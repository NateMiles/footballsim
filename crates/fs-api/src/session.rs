//! A session: one UI talking to one game.
//!
//! The session is the host side of the protocol. It owns the game (world
//! plus command log), the viewer's inbox and any match being watched, and
//! turns each [`Request`] into world commands and views. All changes to
//! the world still go through `Game::submit`, so the log stays complete.

use crate::dto;
use crate::inbox::Inbox;
use crate::live::{LiveSession, report};
use crate::protocol::{ApiError, ErrorCode, PROTOCOL_VERSION, Request, Response};
use crate::query;
use crate::save;
use fs_gen::{GenConfig, Size, generate};
use fs_match::MatchInput;
use fs_world::matchday::fixture_input;
use fs_world::{
    Actor, ClubId, Command, Event, Game, LeagueId, Lineup, ManagerId, PlayerId, StaffId, World,
};

/// Days [`Request::Continue`] moves on at most.
const CONTINUE_DAYS: u32 = 7;

/// A game in progress, seen by one human manager.
#[derive(Debug)]
pub(crate) struct Career {
    pub(crate) game: Game,
    pub(crate) manager: ManagerId,
    pub(crate) inbox: Inbox,
    live: Option<(LiveSession, MatchInput)>,
}

#[derive(Debug, Default)]
enum Stage {
    #[default]
    Empty,
    /// A new world, waiting for the manager to pick a club.
    Choosing(Box<World>),
    Playing(Box<Career>),
}

/// The host side of the protocol.
#[derive(Debug, Default)]
pub struct Session {
    stage: Stage,
    /// A performance test world, kept apart from any game.
    bench: Option<Box<Game>>,
}

type Result<T> = std::result::Result<T, ApiError>;

fn world_size(size: dto::WorldSize) -> Size {
    match size {
        dto::WorldSize::Tiny => Size::Tiny,
        dto::WorldSize::Small => Size::Small,
        dto::WorldSize::Medium => Size::Medium,
    }
}

fn not_found(what: &str) -> ApiError {
    ApiError::new(ErrorCode::NotFound, format!("no such {what}"))
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles one JSON request and returns the JSON response. Never
    /// fails: errors become [`Response::Error`].
    pub fn call_json(&mut self, request: &str) -> String {
        let response = match serde_json::from_str::<Request>(request) {
            Ok(r) => self.handle(r),
            Err(e) => Response::Error {
                error: ApiError::new(ErrorCode::BadRequest, e.to_string()),
            },
        };
        serde_json::to_string(&response).expect("responses serialise")
    }

    /// Handles one request.
    pub fn handle(&mut self, request: Request) -> Response {
        self.dispatch(request)
            .unwrap_or_else(|error| Response::Error { error })
    }

    fn benchmark_status(&self) -> Response {
        let (active, players, clubs, leagues, cups, date, matches) = match &self.bench {
            Some(g) => {
                let w = g.world();
                let played = |f: &fs_world::Fixture| f.result.is_some();
                let league_matches: usize = w
                    .leagues()
                    .iter()
                    .filter_map(|(_, l)| l.season())
                    .map(|s| s.fixtures.iter().filter(|f| played(f)).count())
                    .sum();
                let cup_matches: usize = w
                    .cups()
                    .iter()
                    .filter_map(|(_, c)| c.edition())
                    .map(|e| e.fixtures.iter().filter(|f| played(&f.fixture)).count())
                    .sum();
                (
                    true,
                    w.players().len() as u32,
                    w.clubs().len() as u32,
                    w.leagues().len() as u32,
                    w.cups().len() as u32,
                    query::refs::date(w.date()),
                    (league_matches + cup_matches) as u32,
                )
            }
            None => (false, 0, 0, 0, 0, String::new(), 0),
        };
        Response::Benchmark {
            benchmark: dto::BenchmarkStatus {
                active,
                players,
                clubs,
                leagues,
                cups,
                date,
                matches,
                memory_bytes: crate::diagnostics::memory_bytes(),
                threads: crate::diagnostics::threads(),
            },
        }
    }

    /// The game, if one is being played.
    pub fn game(&self) -> Option<&Game> {
        match &self.stage {
            Stage::Playing(c) => Some(&c.game),
            _ => None,
        }
    }

    fn career(&mut self) -> Result<&mut Career> {
        match &mut self.stage {
            Stage::Playing(c) => Ok(c),
            _ => Err(ApiError::new(ErrorCode::NoGame, "no game loaded")),
        }
    }

    fn dispatch(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::Hello => Ok(Response::Hello {
                protocol: PROTOCOL_VERSION,
                version: env!("CARGO_PKG_VERSION").into(),
                in_game: matches!(self.stage, Stage::Playing(_)),
            }),
            Request::BenchmarkStart { size } => {
                let world = generate(&GenConfig::preset(world_size(size), 1));
                self.bench = Some(Box::new(Game::new(world)));
                Ok(self.benchmark_status())
            }
            Request::BenchmarkRun { days } => {
                let game = self
                    .bench
                    .as_mut()
                    .ok_or_else(|| ApiError::new(ErrorCode::NoGame, "start a test first"))?;
                game.submit(
                    Actor::System,
                    Command::Advance {
                        days: days.clamp(1, 30),
                    },
                )?;
                Ok(self.benchmark_status())
            }
            Request::BenchmarkEnd => {
                self.bench = None;
                Ok(self.benchmark_status())
            }
            Request::NewWorld { seed, size } => {
                let mut cfg = GenConfig::preset(world_size(size), u64::from(seed));
                cfg.human_clubs.clear();
                let world = generate(&cfg);
                let choices = query::game::club_choices(&world);
                self.stage = Stage::Choosing(Box::new(world));
                Ok(Response::ClubChoices { choices })
            }
            Request::StartCareer {
                club,
                first_name,
                last_name,
            } => {
                let Stage::Choosing(world) = std::mem::take(&mut self.stage) else {
                    return Err(ApiError::new(ErrorCode::NoGame, "generate a world first"));
                };
                let mut game = Game::new(*world);
                let appointed = game.submit(
                    Actor::System,
                    Command::AppointHuman {
                        club: ClubId::new(club),
                        first_name,
                        last_name,
                    },
                );
                let events = match appointed {
                    Ok(e) => e,
                    Err(e) => {
                        // Keep the world so the manager can choose again.
                        self.stage = Stage::Choosing(Box::new(game.into_parts().0));
                        return Err(e.into());
                    }
                };
                let Some(&Event::ManagerAppointed { manager, .. }) = events.first() else {
                    unreachable!("AppointHuman reports the appointment");
                };
                let mut career = Career {
                    game,
                    manager,
                    inbox: Inbox::default(),
                    live: None,
                };
                career.record(&events);
                self.stage = Stage::Playing(Box::new(career));
                self.overview()
            }
            Request::Save => {
                let c = self.career()?;
                Ok(Response::Saved {
                    save: save::save(c),
                })
            }
            Request::Load { data } => {
                let (game, manager, inbox) = save::load(&data)?;
                self.stage = Stage::Playing(Box::new(Career {
                    game,
                    manager,
                    inbox,
                    live: None,
                }));
                self.overview()
            }
            Request::Overview => self.overview(),
            Request::Inbox => Ok(Response::Inbox {
                inbox: self.career()?.inbox.view(),
            }),
            Request::MarkRead { id } => {
                let c = self.career()?;
                c.inbox.mark_read(id);
                Ok(Response::Inbox {
                    inbox: c.inbox.view(),
                })
            }
            Request::Squad { club } => {
                let c = self.career()?;
                let club = match club {
                    Some(id) => c.existing_club(id)?,
                    None => c.club()?,
                };
                Ok(Response::Squad {
                    squad: query::club::squad(c.world(), club),
                })
            }
            Request::Player { player } => {
                let c = self.career()?;
                let id = c.existing_player(player)?;
                Ok(Response::Player {
                    player: query::club::player(c.world(), id, c.club().ok()),
                })
            }
            Request::Tactics => {
                let c = self.career()?;
                Ok(Response::Tactics {
                    tactics: query::club::tactics(c.world(), c.club()?),
                })
            }
            Request::SetTactic { tactic } => {
                let c = self.career()?;
                c.submit_outside_match(Command::SetTactic {
                    tactic: tactic.map(Into::into),
                })?;
                Ok(Response::Tactics {
                    tactics: query::club::tactics(c.world(), c.club()?),
                })
            }
            Request::SetInstructions {
                roles,
                penalty_taker,
                free_kick_taker,
                corner_taker,
                marking,
            } => {
                let c = self.career()?;
                let club = c.club()?;
                let mut takers = [None; 3];
                for (out, id) in
                    takers
                        .iter_mut()
                        .zip([penalty_taker, free_kick_taker, corner_taker])
                {
                    if let Some(id) = id {
                        *out = Some(c.existing_player(id)?);
                    }
                }
                for m in &marking {
                    c.existing_player(m.player)?;
                }
                let formation = fs_world::matchday::team_sheet(c.world(), club)
                    .tactic
                    .formation;
                let instructions =
                    query::instructions::from_choices(formation, &roles, takers, &marking);
                c.submit_outside_match(Command::SetInstructions {
                    instructions: Some(instructions),
                })?;
                Ok(Response::Tactics {
                    tactics: query::club::tactics(c.world(), club),
                })
            }
            Request::SetLineup { lineup } => {
                let c = self.career()?;
                c.submit_outside_match(Command::SetLineup {
                    lineup: lineup.map(|l| Lineup {
                        starters: l.starters.into_iter().map(PlayerId::new).collect(),
                        bench: l.bench.into_iter().map(PlayerId::new).collect(),
                    }),
                })?;
                Ok(Response::Tactics {
                    tactics: query::club::tactics(c.world(), c.club()?),
                })
            }
            Request::Fixtures { league, club } => {
                let c = self.career()?;
                let (league, club) = match (league, club) {
                    (Some(l), club) => (
                        c.existing_league(l)?,
                        club.map(|id| c.existing_club(id)).transpose()?,
                    ),
                    (None, Some(id)) => {
                        let club = c.existing_club(id)?;
                        (c.league_of(club)?, Some(club))
                    }
                    (None, None) => {
                        let club = c.club()?;
                        (c.league_of(club)?, Some(club))
                    }
                };
                Ok(Response::Fixtures {
                    fixtures: query::competition::fixtures(c.world(), league, club),
                })
            }
            Request::Table { league } => {
                let c = self.career()?;
                let league = match league {
                    Some(l) => c.existing_league(l)?,
                    None => c.league_of(c.club()?)?,
                };
                Ok(Response::Table {
                    table: query::competition::table(c.world(), league),
                })
            }
            Request::Schedule { club } => {
                let c = self.career()?;
                let club = match club {
                    Some(id) => c.existing_club(id)?,
                    None => c.club()?,
                };
                Ok(Response::Schedule {
                    schedule: query::cup::schedule(c.world(), club),
                })
            }
            Request::Cup { cup } => {
                let c = self.career()?;
                let viewer = c.club().ok();
                let cup = match cup {
                    Some(id) => {
                        let cup = fs_world::CupId::new(id);
                        c.world()
                            .cups()
                            .get(cup)
                            .map(|_| cup)
                            .ok_or_else(|| not_found("cup"))?
                    }
                    None => query::cup::default_cup(c.world(), viewer)
                        .ok_or_else(|| not_found("cup"))?,
                };
                Ok(Response::Cup {
                    cup: query::cup::cup(c.world(), cup, viewer),
                })
            }
            Request::Competitions => {
                let c = self.career()?;
                Ok(Response::Competitions {
                    competitions: query::cup::competitions(c.world()),
                })
            }
            Request::OppositionReport { club } => {
                let c = self.career()?;
                let me = c.club()?;
                let opponent = match club {
                    Some(id) => c.existing_club(id)?,
                    None => query::preparation::next_match(c.world(), me)
                        .map(|(_, o)| o)
                        .ok_or_else(|| ApiError::new(ErrorCode::NoMatch, "no match to come"))?,
                };
                Ok(Response::OppositionReport {
                    report: query::preparation::report(c.world(), me, opponent),
                })
            }
            Request::Predict { tactic, runs } => {
                let c = self.career()?;
                let me = c.club()?;
                let prediction =
                    query::preparation::predict(c.world(), me, tactic.map(Into::into), runs)
                        .ok_or_else(|| ApiError::new(ErrorCode::NoMatch, "no match to come"))?;
                Ok(Response::Prediction { prediction })
            }
            Request::Projections { league } => {
                let c = self.career()?;
                let league = match league {
                    Some(l) => c.existing_league(l)?,
                    None => c.league_of(c.club()?)?,
                };
                Ok(Response::Projections {
                    projections: query::preparation::projections(c.world(), league),
                })
            }
            Request::SetWatchMatches { watch } => {
                self.career()?
                    .submit_outside_match(Command::SetWatchMatches { watch })?;
                self.overview()
            }
            Request::SetDelegation { squad } => {
                self.career()?
                    .submit_outside_match(Command::SetDelegation { squad })?;
                self.overview()
            }
            Request::OfferContract {
                player,
                wage,
                years,
            } => {
                let c = self.career()?;
                let id = c.existing_player(player)?;
                c.submit_outside_match(Command::OfferContract {
                    player: id,
                    wage,
                    years,
                })?;
                Ok(Response::Player {
                    player: query::club::player(c.world(), id, c.club().ok()),
                })
            }
            Request::ReleasePlayer { player } => {
                let c = self.career()?;
                let id = c.existing_player(player)?;
                c.submit_outside_match(Command::ReleasePlayer { player: id })?;
                Ok(Response::Squad {
                    squad: query::club::squad(c.world(), c.club()?),
                })
            }
            Request::Continue => Ok(Response::Continued {
                continued: self.career()?.continue_on()?,
            }),
            Request::StartMatch => {
                let c = self.career()?;
                let (live, start) = LiveSession::start(c.world(), c.manager)?;
                let input = fixture_input(c.world(), live.fixture);
                c.live = Some((live, input));
                Ok(Response::LiveStart { start })
            }
            Request::LiveTick { until } => {
                let (live, _) = self.career()?.live_mut()?;
                Ok(Response::LiveChunk {
                    chunk: live.tick(until),
                })
            }
            Request::LiveInstruct { instruction } => {
                let (live, _) = self.career()?.live_mut()?;
                Ok(Response::LiveChunk {
                    chunk: live.instruct(instruction)?,
                })
            }
            Request::FinishMatch => Ok(Response::MatchReport {
                report: self.career()?.finish_match()?,
            }),
            Request::SkipMatch => {
                let c = self.career()?;
                c.live = None;
                c.submit(Command::SkipMatch)?;
                self.overview()
            }
            Request::Finances => {
                let c = self.career()?;
                Ok(Response::Finances {
                    finances: query::management::finances(c.world(), c.club()?),
                })
            }
            Request::Staff => self.staff_view(),
            Request::Training => {
                let c = self.career()?;
                Ok(Response::Training {
                    training: query::training::view(c.world(), c.club()?),
                })
            }
            Request::SetTraining { focus, intensity } => {
                let c = self.career()?;
                c.submit_quiet(Command::SetTraining {
                    schedule: query::training::schedule(focus, intensity),
                })?;
                Ok(Response::Training {
                    training: query::training::view(c.world(), c.club()?),
                })
            }
            Request::SetIndividualTraining { player, training } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                let training = match &training {
                    Some(t) => Some(query::training::individual_from(t).ok_or_else(|| {
                        ApiError::new(ErrorCode::BadRequest, "unknown position or attribute")
                    })?),
                    None => None,
                };
                c.submit_quiet(Command::SetIndividualTraining { player, training })?;
                Ok(Response::Training {
                    training: query::training::view(c.world(), c.club()?),
                })
            }
            Request::Dynamics => self.dynamics(),
            Request::TalkToPlayer { player, talk } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                let talk = talk.into();
                let events = c.submit_quiet(Command::TalkToPlayer { player, talk })?;
                let reaction = events
                    .iter()
                    .find_map(|e| match e {
                        fs_world::Event::PlayerReacted { reaction, .. } => Some(*reaction),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        ApiError::new(ErrorCode::Rejected, "the player did not answer")
                    })?;
                Ok(Response::Talk {
                    result: dto::TalkResult {
                        reaction: reaction.into(),
                        message: query::people::reaction_text(talk, reaction).into(),
                        said: query::people::talk_line(talk).into(),
                        reply: query::people::reply_text(c.world(), player, talk, reaction),
                        expression: query::people::reaction_expression(reaction),
                        player: query::club::player(c.world(), player, c.club().ok()),
                    },
                })
            }
            Request::AnswerTransferRequest { player, accept } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                c.submit_quiet(Command::AnswerTransferRequest { player, accept })?;
                self.dynamics()
            }
            Request::SetCaptain { player } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                c.submit_quiet(Command::SetCaptain { player })?;
                self.dynamics()
            }
            Request::SetSquadStatus { player, status } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                c.submit_quiet(Command::SetSquadStatus {
                    player,
                    status: status.into(),
                })?;
                self.dynamics()
            }
            Request::HireStaff { staff, wage, years } => {
                self.career()?.submit(Command::HireStaff {
                    staff: StaffId::new(staff),
                    wage,
                    years,
                })?;
                self.staff_view()
            }
            Request::ReleaseStaff { staff } => {
                self.career()?.submit(Command::ReleaseStaff {
                    staff: StaffId::new(staff),
                })?;
                self.staff_view()
            }
            Request::Transfers => self.transfers_view(),
            Request::SearchPlayers { search } => {
                let c = self.career()?;
                Ok(Response::Search {
                    search: query::market::search(c.world(), c.club().ok(), &search),
                })
            }
            Request::MakeOffer {
                player,
                kind,
                wage,
                years,
                terms,
                sell_on,
            } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                let command = match (kind, terms) {
                    (dto::OfferKind::Transfer { fee }, Some(t)) => Command::MakeTransferOffer {
                        player,
                        fee,
                        sell_on: sell_on.unwrap_or(0),
                        terms: t.into(),
                    },
                    (kind, _) => Command::MakeOffer {
                        player,
                        kind: kind.into(),
                        wage,
                        years,
                    },
                };
                c.submit_quiet(command)?;
                self.transfers_view()
            }
            Request::MatchAnalysis { fixture } => {
                let c = self.career()?;
                let analysis = query::analysis::match_analysis(
                    c.world(),
                    query::analysis::fixture_ref(fixture),
                )
                .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "no analysis for that match"))?;
                Ok(Response::MatchAnalysis { analysis })
            }
            Request::News { scope } => {
                let c = self.career()?;
                Ok(Response::News {
                    news: query::media::news(c.world(), c.club().ok(), scope),
                })
            }
            Request::Press => self.press_view(),
            Request::AnswerQuestion {
                conference,
                question,
                tone,
            } => {
                let c = self.career()?;
                c.submit_quiet(Command::AnswerQuestion {
                    conference,
                    question,
                    tone: query::media::tone_from(tone),
                })?;
                self.press_view()
            }
            Request::LeaveConference { conference } => {
                let c = self.career()?;
                c.submit_quiet(Command::LeaveConference { conference })?;
                self.press_view()
            }
            Request::SetPressDelegation { delegate } => {
                let c = self.career()?;
                c.submit_quiet(Command::SetPressDelegation { delegate })?;
                self.press_view()
            }
            Request::SeasonStats { club } => {
                let c = self.career()?;
                let club = match club {
                    Some(id) => c.existing_club(id)?,
                    None => c.club()?,
                };
                Ok(Response::SeasonStats {
                    stats: query::analysis::season_stats(c.world(), club),
                })
            }
            Request::ContractTalks { player } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                Ok(Response::Negotiation {
                    negotiation: query::contracts::negotiation(c.world(), c.club()?, player),
                })
            }
            Request::NegotiateContract { player, terms } => {
                let c = self.career()?;
                let player = c.existing_player(player)?;
                let events = c.submit_outside_match(Command::NegotiateContract {
                    player,
                    terms: terms.into(),
                })?;
                let outcome = events
                    .iter()
                    .find_map(|e| match e {
                        fs_world::Event::ContractTalks { outcome, .. } => Some(*outcome),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        ApiError::new(ErrorCode::Rejected, "no answer from the agent")
                    })?;
                let (outcome, message) = query::contracts::outcome(outcome);
                Ok(Response::Talks {
                    result: dto::TalksResult {
                        outcome,
                        message: message.into(),
                        negotiation: query::contracts::negotiation(c.world(), c.club()?, player),
                    },
                })
            }
            Request::RespondToOffer { offer, response } => {
                self.career()?.submit_quiet(Command::RespondToOffer {
                    offer,
                    response: match response {
                        dto::OfferResponse::Accept => fs_world::OfferResponse::Accept,
                        dto::OfferResponse::Reject => fs_world::OfferResponse::Reject,
                        dto::OfferResponse::Counter { fee } => {
                            fs_world::OfferResponse::Counter { fee }
                        }
                    },
                })?;
                self.transfers_view()
            }
            Request::AcceptCounter { offer } => {
                self.career()?
                    .submit_quiet(Command::AcceptCounter { offer })?;
                self.transfers_view()
            }
            Request::WithdrawOffer { offer } => {
                self.career()?
                    .submit_quiet(Command::WithdrawOffer { offer })?;
                self.transfers_view()
            }
            Request::SetListing { player, listing } => {
                let c = self.career()?;
                let id = c.existing_player(player)?;
                c.submit_quiet(Command::SetListing {
                    player: id,
                    listing: listing.map(Into::into),
                })?;
                Ok(Response::Player {
                    player: query::club::player(c.world(), id, c.club().ok()),
                })
            }
            Request::ScoutPlayer { player } => {
                let c = self.career()?;
                let id = c.existing_player(player)?;
                c.submit_quiet(Command::ScoutPlayer { player: id })?;
                Ok(Response::Player {
                    player: query::club::player(c.world(), id, c.club().ok()),
                })
            }
            Request::CancelScouting { player } => {
                let c = self.career()?;
                let id = c.existing_player(player)?;
                c.submit_quiet(Command::CancelScouting { player: id })?;
                Ok(Response::Player {
                    player: query::club::player(c.world(), id, c.club().ok()),
                })
            }
            Request::Jobs => {
                let c = self.career()?;
                Ok(Response::Jobs {
                    jobs: query::management::jobs(c.world(), c.manager),
                })
            }
            Request::ApplyForJob { club } => {
                let c = self.career()?;
                let club = c.existing_club(club)?;
                c.submit(Command::ApplyForJob { club })?;
                self.overview()
            }
            Request::Resign => {
                let c = self.career()?;
                c.live = None;
                c.submit(Command::Resign)?;
                self.overview()
            }
        }
    }

    fn overview(&mut self) -> Result<Response> {
        let c = self.career()?;
        Ok(Response::Overview {
            overview: query::game::overview(c.world(), c.manager, c.club().ok(), c.inbox.unread()),
        })
    }

    fn press_view(&mut self) -> Result<Response> {
        let c = self.career()?;
        Ok(Response::Press {
            press: query::media::press(c.world(), c.manager, c.club().ok()),
        })
    }

    fn dynamics(&mut self) -> Result<Response> {
        let c = self.career()?;
        Ok(Response::Dynamics {
            dynamics: query::people::dynamics(c.world(), c.club()?),
        })
    }

    fn staff_view(&mut self) -> Result<Response> {
        let c = self.career()?;
        Ok(Response::Staff {
            staff: query::management::staff(c.world(), c.club()?),
        })
    }

    fn transfers_view(&mut self) -> Result<Response> {
        let c = self.career()?;
        Ok(Response::Transfers {
            transfers: query::market::transfers(c.world(), c.club()?),
        })
    }
}

impl Career {
    fn world(&self) -> &World {
        self.game.world()
    }

    fn club(&self) -> Result<ClubId> {
        self.world()
            .human_club(self.manager)
            .ok_or_else(|| ApiError::new(ErrorCode::NoGame, "you have no club"))
    }

    fn existing_club(&self, id: u32) -> Result<ClubId> {
        let club = ClubId::new(id);
        self.world()
            .clubs()
            .get(club)
            .map(|_| club)
            .ok_or_else(|| not_found("club"))
    }

    fn existing_league(&self, id: u32) -> Result<LeagueId> {
        let league = LeagueId::new(id);
        self.world()
            .leagues()
            .get(league)
            .map(|_| league)
            .ok_or_else(|| not_found("league"))
    }

    fn existing_player(&self, id: u32) -> Result<PlayerId> {
        let player = PlayerId::new(id);
        if self.world().players().contains(player) {
            Ok(player)
        } else {
            Err(not_found("player"))
        }
    }

    fn league_of(&self, club: ClubId) -> Result<LeagueId> {
        self.world().clubs()[club]
            .league()
            .ok_or_else(|| not_found("league for this club"))
    }

    fn live_mut(&mut self) -> Result<&mut (LiveSession, MatchInput)> {
        self.live
            .as_mut()
            .ok_or_else(|| ApiError::new(ErrorCode::NoMatch, "no match is being watched"))
    }

    /// Adds the events that concern the viewer to the inbox.
    fn record(&mut self, events: &[Event]) {
        let club = self.game.world().human_club(self.manager);
        self.inbox
            .record(self.game.world(), self.manager, club, events);
    }

    /// Submits a command whose result the viewer sees at once, so nothing
    /// goes to the inbox (market moves, scouting).
    fn submit_quiet(&mut self, command: Command) -> Result<Vec<Event>> {
        if self.live.is_some() {
            return Err(ApiError::new(
                ErrorCode::Rejected,
                "not while a match is being watched",
            ));
        }
        Ok(self.game.submit(Actor::Manager(self.manager), command)?)
    }

    /// Submits a command as the viewer and files its events.
    fn submit(&mut self, command: Command) -> Result<Vec<Event>> {
        let events = self.game.submit(Actor::Manager(self.manager), command)?;
        self.record(&events);
        Ok(events)
    }

    /// As [`Career::submit`], refusing while a match is being watched:
    /// the match on screen must stay the one the world will play.
    fn submit_outside_match(&mut self, command: Command) -> Result<Vec<Event>> {
        if self.live.is_some() {
            return Err(ApiError::new(
                ErrorCode::Rejected,
                "not while a match is being watched",
            ));
        }
        self.submit(command)
    }

    /// Moves time on until there is something to look at.
    fn continue_on(&mut self) -> Result<dto::Continued> {
        let before = self.inbox.len();
        let mut stopped = dto::StopReason::Week;
        if query::game::match_day(self.world(), self.manager).is_some() {
            stopped = dto::StopReason::MatchDay;
        } else {
            for _ in 0..CONTINUE_DAYS {
                let events = self
                    .game
                    .submit(Actor::System, Command::Advance { days: 1 })?;
                self.record(&events);
                if query::game::match_day(self.world(), self.manager).is_some() {
                    stopped = dto::StopReason::MatchDay;
                    break;
                }
                if self.inbox.len() > before {
                    stopped = dto::StopReason::Messages;
                    break;
                }
            }
        }
        Ok(dto::Continued {
            date: query::refs::date(self.world().date()),
            stopped,
            new_messages: (self.inbox.len() - before) as u32,
            match_day: query::game::match_day(self.world(), self.manager),
        })
    }

    /// Plays out the watched match, reports it to the world, and returns
    /// the world's record of it.
    fn finish_match(&mut self) -> Result<dto::MatchReport> {
        let (live, input) = self
            .live
            .take()
            .ok_or_else(|| ApiError::new(ErrorCode::NoMatch, "no match is being watched"))?;
        let r = live.fixture;
        if fixture_input(self.world(), r) != input {
            return Err(ApiError::new(
                ErrorCode::Rejected,
                "the teams changed during the match",
            ));
        }
        let (instructions, result) = live.finish();
        self.submit(Command::PlayMatch { instructions })?;
        let recorded = r
            .get(self.world())
            .and_then(|f| f.result.as_ref())
            .map(|x| x.goals);
        debug_assert_eq!(
            recorded,
            Some(result.goals),
            "the world replays the watched match"
        );
        Ok(report(self.world(), r, &result))
    }
}
