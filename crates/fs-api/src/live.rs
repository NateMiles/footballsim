//! A match being watched: the live view, commentary and conversion to
//! view models.

use crate::commentary::{self, Names};
use crate::dto;
use crate::protocol::{ApiError, ErrorCode};
use crate::query::refs::{fixture, player_ref};
use fs_core::ids::PlayerId;
use fs_match::tier1::LiveLineup;
use fs_match::view::{FRAME_HZ, LiveView, PITCH_LENGTH, PITCH_WIDTH};
use fs_match::{MatchResult, Side, TimedInstruction};
use fs_world::FixtureRef;
use fs_world::matchday::fixture_input;
use fs_world::{ManagerId, World};
use std::collections::BTreeMap;

/// Names cached at kickoff.
#[derive(Debug, Clone)]
struct MatchNames {
    players: BTreeMap<PlayerId, String>,
    teams: [String; 2],
}

impl Names for MatchNames {
    fn player(&self, id: PlayerId) -> String {
        self.players
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "Someone".into())
    }

    fn team(&self, side: Side) -> String {
        self.teams[side.index()].clone()
    }
}

/// A match being watched.
#[derive(Debug)]
pub struct LiveSession {
    view: LiveView,
    pub(crate) fixture: FixtureRef,
    side: Side,
    names: MatchNames,
    /// Kick-off announced.
    kicked_off: bool,
    /// Breaks between halves announced.
    breaks: usize,
    /// The end announced.
    ended: bool,
}

impl LiveSession {
    /// Starts watching `manager`'s match today.
    pub fn start(world: &World, manager: ManagerId) -> Result<(Self, dto::LiveStart), ApiError> {
        let watched = world
            .pending()
            .and_then(|p| p.awaiting.iter().find(|w| w.manager == manager))
            .copied()
            .ok_or_else(|| ApiError::new(ErrorCode::NoMatch, "no match to watch today"))?;
        let input = fixture_input(world, watched.fixture);
        let f = fixture(world, watched.fixture);
        let side = if f.home.id == watched.club.raw() {
            Side::Home
        } else {
            Side::Away
        };
        let mut players = Vec::new();
        let mut names = BTreeMap::new();
        for (s, sheet) in [(Side::Home, &input.home), (Side::Away, &input.away)] {
            for p in sheet.starters.iter().chain(&sheet.bench) {
                let r = player_ref(world, p.id);
                names.insert(p.id, r.short_name.clone());
                players.push(dto::LivePlayerInfo {
                    player: r,
                    side: s.into(),
                });
            }
        }
        let view = LiveView::new(&input);
        let fx = watched.fixture.get(world).expect("fixture exists");
        let teams = [
            world.clubs()[fx.home].short_name.clone(),
            world.clubs()[fx.away].short_name.clone(),
        ];
        let session = Self {
            fixture: watched.fixture,
            side,
            names: MatchNames {
                players: names,
                teams,
            },
            view,
            kicked_off: false,
            breaks: 0,
            ended: false,
        };
        let start = dto::LiveStart {
            fixture: f,
            side: side.into(),
            pitch: dto::Pitch {
                length: PITCH_LENGTH,
                width: PITCH_WIDTH,
            },
            frame_hz: FRAME_HZ,
            players,
            lineups: session.lineups(),
            shouts: fs_match::Shout::ALL.iter().map(|&s| s.into()).collect(),
        };
        Ok((session, start))
    }

    /// The side the viewer manages.
    pub fn side(&self) -> Side {
        self.side
    }

    fn lineups(&self) -> [dto::LiveLineup; 2] {
        [
            lineup(&self.view.lineup(Side::Home)),
            lineup(&self.view.lineup(Side::Away)),
        ]
    }

    fn clock_at(&self, t: f64) -> String {
        commentary::clock(t, self.view.engine().half_starts())
    }

    /// Plays up to `until` seconds after kickoff.
    pub fn tick(&mut self, until: f64) -> dto::LiveChunk {
        let chunk = self.view.play_until(until);
        let mut lines = Vec::new();
        if !self.kicked_off {
            self.kicked_off = true;
            lines.push(commentary::whistle(0.0, "1".into(), "Kick-off!"));
        }
        for a in &chunk.actions {
            if let Some(l) = commentary::action(a, self.clock_at(f64::from(a.at)), &self.names) {
                lines.push(l);
            }
        }
        let starts = self.view.engine().half_starts().to_vec();
        while self.breaks < starts.len() {
            let t = starts[self.breaks];
            let text = match self.breaks {
                0 => "Half time.",
                1 => "Full time, and level: extra time.",
                _ => "Half time in extra time.",
            };
            self.breaks += 1;
            lines.push(commentary::interval(t as f32, self.clock_at(t - 1.0), text));
        }
        let mut events = Vec::with_capacity(chunk.events.len());
        for ev in &chunk.events {
            // Events carry only a minute: place a goal at its shot, and
            // anything else after the chunk's last action.
            let at_goal = match *ev {
                fs_match::MatchEvent::Goal { scorer, .. } => chunk
                    .actions
                    .iter()
                    .find(|a| a.outcome == fs_match::Outcome::Goal && a.actor == scorer),
                _ => None,
            };
            let t = at_goal
                .or(chunk.actions.last())
                .map_or(chunk.elapsed as f32, |a| a.at + a.duration);
            lines.push(commentary::event(
                ev,
                t,
                self.clock_at(f64::from(t)),
                &self.names,
            ));
            events.push(match_event(ev));
        }
        if chunk.finished && !self.ended {
            self.ended = true;
            let text = match (starts.len() > 1, &chunk.shootout) {
                (_, Some(_)) => "Still level: penalties!",
                (true, None) => "The end of extra time.",
                (false, None) => "Full time.",
            };
            lines.push(commentary::whistle(
                chunk.elapsed as f32,
                self.clock_at(chunk.elapsed),
                text,
            ));
            if let Some(s) = &chunk.shootout {
                let mut score = [0u8; 2];
                for (k, kick) in s.kicks.iter().enumerate() {
                    score[kick.side.index()] += u8::from(kick.scored);
                    lines.push(commentary::kick(
                        chunk.elapsed as f32 + 1.0 + k as f32,
                        kick,
                        score,
                        &self.names,
                    ));
                }
            }
        }
        lines.sort_by(|a, b| a.t.total_cmp(&b.t));
        dto::LiveChunk {
            frames: chunk.frames.iter().map(frame).collect(),
            commentary: lines,
            events,
            score: chunk.score,
            clock: self.clock_at(chunk.elapsed),
            elapsed: chunk.elapsed,
            finished: chunk.finished,
            stats: [team_stats(&chunk.stats[0]), team_stats(&chunk.stats[1])],
            lineups: self.lineups(),
            shootout: chunk.shootout.as_ref().map(shootout),
        }
    }

    /// Gives an instruction for the viewer's side now.
    pub fn instruct(&mut self, instruction: dto::Instruction) -> Result<dto::LiveChunk, ApiError> {
        let ins = match instruction {
            dto::Instruction::Substitute { off, on } => fs_match::Instruction::Substitute {
                off: PlayerId::new(off),
                on: PlayerId::new(on),
            },
            dto::Instruction::ChangeTactic { tactic } => {
                fs_match::Instruction::ChangeTactic(tactic.into())
            }
            dto::Instruction::Shout { shout } => fs_match::Instruction::Shout(shout.into()),
        };
        self.view
            .instruct(self.side, ins)
            .map_err(|e| ApiError::new(ErrorCode::Rejected, format!("{e:?}")))?;
        let now = self.view.engine().elapsed();
        Ok(self.tick(now))
    }

    /// The instructions given so far.
    pub fn instructions(&self) -> Vec<TimedInstruction> {
        self.view.instructions().to_vec()
    }

    /// Plays out the rest unseen.
    pub fn finish(self) -> (Vec<TimedInstruction>, MatchResult) {
        let instructions = self.instructions();
        (instructions, self.view.finish())
    }
}

fn lineup(l: &LiveLineup) -> dto::LiveLineup {
    dto::LiveLineup {
        tactic: l.tactic.into(),
        on_pitch: l
            .on_pitch
            .iter()
            .map(|p| {
                p.map(|p| dto::OnPitch {
                    id: p.id.raw(),
                    slot: p.slot,
                    energy: p.energy,
                    yellow: p.yellow,
                })
            })
            .collect(),
        bench: l.bench.iter().map(|p| p.raw()).collect(),
        substitutions_left: l.substitutions_left,
        shout: l.shout.map(Into::into),
    }
}

fn frame(f: &fs_match::view::Frame) -> dto::Frame {
    use fs_match::view::Pose as P;
    dto::Frame {
        t: f.t,
        ball: f.ball,
        players: f
            .players
            .iter()
            .map(|p| dto::PlayerFrame {
                id: p.id.raw(),
                side: p.side.into(),
                x: p.x,
                y: p.y,
                facing: p.facing,
                pose: match p.pose {
                    P::Standing => dto::Pose::Standing,
                    P::Running => dto::Pose::Running,
                    P::Passing => dto::Pose::Passing,
                    P::Shooting => dto::Pose::Shooting,
                    P::Heading => dto::Pose::Heading,
                    P::Tackling => dto::Pose::Tackling,
                    P::Diving => dto::Pose::Diving,
                    P::Celebrating => dto::Pose::Celebrating,
                },
            })
            .collect(),
    }
}

pub(crate) fn team_stats(s: &fs_match::TeamStats) -> dto::TeamStats {
    dto::TeamStats {
        possession: s.possession,
        shots: s.shots,
        shots_on_target: s.shots_on_target,
        xg: s.xg,
        passes: s.passes,
        passes_completed: s.passes_completed,
        crosses: s.crosses,
        tackles: s.tackles,
        fouls: s.fouls,
        yellows: s.yellows,
        reds: s.reds,
        corners: s.corners,
        offsides: s.offsides,
        saves: s.saves,
    }
}

pub(crate) fn match_event(e: &fs_match::MatchEvent) -> dto::MatchEvent {
    use fs_match::{GoalKind as G, MatchEvent as E};
    match *e {
        E::Goal {
            minute,
            side,
            scorer,
            assist,
            kind,
            xg,
        } => dto::MatchEvent::Goal {
            minute,
            side: side.into(),
            scorer: scorer.raw(),
            assist: assist.map(|a| a.raw()),
            kind: match kind {
                G::OpenPlay => "openPlay",
                G::Header => "header",
                G::LongShot => "longShot",
                G::Penalty => "penalty",
                G::FreeKick => "freeKick",
                G::Corner => "corner",
            }
            .into(),
            xg,
        },
        E::PenaltyMissed {
            minute,
            side,
            taker,
        } => dto::MatchEvent::PenaltyMissed {
            minute,
            side: side.into(),
            taker: taker.raw(),
        },
        E::Card {
            minute,
            side,
            player,
            red,
        } => dto::MatchEvent::Card {
            minute,
            side: side.into(),
            player: player.raw(),
            red,
        },
        E::Substitution {
            minute,
            side,
            off,
            on,
        } => dto::MatchEvent::Substitution {
            minute,
            side: side.into(),
            off: off.raw(),
            on: on.raw(),
        },
        E::Injury {
            minute,
            side,
            player,
        } => dto::MatchEvent::Injury {
            minute,
            side: side.into(),
            player: player.raw(),
        },
    }
}

/// A shootout as the UI sees it.
pub(crate) fn shootout(s: &fs_match::Shootout) -> dto::Shootout {
    dto::Shootout {
        kicks: s
            .kicks
            .iter()
            .map(|k| dto::Kick {
                side: k.side.into(),
                taker: k.taker.raw(),
                scored: k.scored,
            })
            .collect(),
        score: s.score,
    }
}

/// The report for a finished match.
pub(crate) fn report(world: &World, r: FixtureRef, result: &MatchResult) -> dto::MatchReport {
    let ratings = |side: usize| {
        result.players[side]
            .iter()
            .filter(|p| p.minutes > 0)
            .map(|p| dto::PlayerRating {
                player: player_ref(world, p.player),
                minutes: p.minutes,
                goals: p.goals,
                assists: p.assists,
                rating: p.rating,
            })
            .collect()
    };
    dto::MatchReport {
        fixture: fixture(world, r),
        stats: [team_stats(&result.stats[0]), team_stats(&result.stats[1])],
        events: result.events.iter().map(match_event).collect(),
        ratings: [ratings(0), ratings(1)],
        shootout: result.shootout.as_ref().map(shootout),
    }
}
