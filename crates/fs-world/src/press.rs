//! Press conferences: journalists ask, managers answer, and the answers
//! travel. Before and after every match a human manager's club holds one;
//! the questions come from what is true (form, rumours, the table, the
//! board, unrest, injuries, new signings) and what the other manager said.
//!
//! Answers have consequences, kept small next to football itself: the
//! squad's morale, one player's happiness (praised or criticised in
//! public, backed or put up for sale), the opponents' morale (a
//! dismissive remark fires them up), the board's confidence, and how the
//! press takes to the manager. Quotable answers become news stories.
//!
//! A manager can leave conferences to the assistant, who says nothing
//! worth printing. Ignoring one entirely is noticed.

use crate::command::{CommandError, Event};
use crate::fixture::{self, FixtureRef};
use crate::happiness::{self, Feeling, SquadStatus};
use crate::ids::{ClubId, ManagerId, NationId, PlayerId};
use crate::names::NameRef;
use crate::news::{self, StoryKind};
use crate::personality::trait_of;
use crate::player::CONDITION_MAX;
use crate::world::World;
use fs_core::attributes::HiddenAttr;
use fs_core::{GameDate, Rng, Stream};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The kind of outlet a journalist writes for.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outlet {
    /// A serious national paper: likes measured answers.
    Broadsheet,
    /// A tabloid: lives for a row.
    Tabloid,
    /// Television and radio.
    Broadcaster,
    /// Supporters' websites and podcasts.
    FanSite,
}

impl Outlet {
    pub const ALL: [Outlet; 4] = [
        Outlet::Broadsheet,
        Outlet::Tabloid,
        Outlet::Broadcaster,
        Outlet::FanSite,
    ];
}

/// A journalist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journalist {
    pub first_name: NameRef,
    pub last_name: NameRef,
    pub nation: NationId,
    pub outlet: Outlet,
}

/// What a question is about.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Topic {
    /// How do you see the game?
    Preview { opponent: ClubId },
    /// The other manager has had their say.
    RivalManager { opponent: ClubId, tone: Tone },
    /// The result: goals for and against.
    Result { opponent: ClubId, goals: [u8; 2] },
    /// A player in fine form.
    StarForm { player: PlayerId },
    /// A first-team player struggling.
    PoorForm { player: PlayerId },
    /// Another club wants one of yours.
    Rumour { player: PlayerId, suitor: ClubId },
    /// The board's patience.
    Pressure,
    /// Near the top with the season under way.
    TitleRace,
    /// Near the bottom.
    Relegation,
    /// A key player is out.
    Injury { player: PlayerId },
    /// A new arrival.
    NewSigning { player: PlayerId },
    /// A youngster breaking through.
    Youngster { player: PlayerId },
    /// A player wants to leave.
    Unrest { player: PlayerId },
    /// The manager did not turn up (news only).
    Snub,
    /// Hopes for the season.
    Expectations,
    /// The player to watch.
    KeyPlayer { player: PlayerId },
    /// An AI manager on a human manager's club (news only).
    AboutRival {
        rival: ClubId,
        angle: crate::rivals::Angle,
    },
    /// What a rival manager said about the club: a response?
    RivalComment {
        rival: ClubId,
        tone: Tone,
        angle: crate::rivals::Angle,
    },
    /// A meeting with a manager there is a feud with.
    Grudge { opponent: ClubId },
}

impl Topic {
    /// The player the question is about, if any.
    pub fn player(self) -> Option<PlayerId> {
        match self {
            Topic::StarForm { player }
            | Topic::PoorForm { player }
            | Topic::Rumour { player, .. }
            | Topic::Injury { player }
            | Topic::NewSigning { player }
            | Topic::Youngster { player }
            | Topic::Unrest { player }
            | Topic::KeyPlayer { player } => Some(player),
            _ => None,
        }
    }
}

/// How the manager answers.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    /// Bold: we will win.
    Confident,
    /// Measured: one game at a time.
    Cautious,
    /// Praise for the player or the team.
    Praise,
    /// Public criticism.
    Criticise,
    /// Say nothing of note.
    Deflect,
    /// Stand firm: not for sale, not quitting, not our fault.
    Defiant,
    /// Take responsibility, show respect.
    Humble,
    /// Wave the other side away.
    Dismissive,
}

/// The answers that make sense for a topic.
pub fn options(topic: Topic) -> &'static [Tone] {
    use Tone::*;
    match topic {
        Topic::Preview { .. } => &[Confident, Cautious, Humble, Dismissive],
        Topic::RivalManager { .. } => &[Dismissive, Humble, Defiant, Deflect],
        Topic::Result { goals, .. } if goals[0] > goals[1] => &[Praise, Humble, Confident, Deflect],
        Topic::Result { goals, .. } if goals[0] == goals[1] => {
            &[Praise, Cautious, Criticise, Defiant]
        }
        Topic::Result { .. } => &[Humble, Criticise, Defiant, Deflect],
        Topic::StarForm { .. } => &[Praise, Cautious, Deflect],
        Topic::PoorForm { .. } => &[Praise, Criticise, Deflect],
        Topic::Rumour { .. } => &[Defiant, Cautious, Deflect],
        Topic::Pressure => &[Confident, Humble, Defiant, Deflect],
        Topic::TitleRace => &[Confident, Cautious, Deflect],
        Topic::Relegation => &[Confident, Humble, Defiant],
        Topic::Injury { .. } => &[Confident, Cautious, Deflect],
        Topic::NewSigning { .. } => &[Praise, Cautious, Deflect],
        Topic::Youngster { .. } => &[Praise, Cautious, Deflect],
        Topic::Unrest { .. } => &[Defiant, Humble, Criticise, Deflect],
        Topic::Snub => &[Deflect],
        Topic::Expectations => &[Confident, Cautious, Humble],
        Topic::KeyPlayer { .. } => &[Praise, Cautious, Deflect],
        Topic::AboutRival { .. } => &[Deflect],
        Topic::RivalComment { .. } => &[Dismissive, Humble, Defiant, Deflect],
        Topic::Grudge { .. } => &[Dismissive, Humble, Deflect],
    }
}

/// A quotable answer, as reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub topic: Topic,
    pub tone: Tone,
}

/// What an answer did.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// Squad morale, per mille of the scale.
    Squad(i16),
    /// The opponents' morale, per mille.
    Opponents { club: ClubId, by: i16 },
    /// One player's happiness.
    Player { player: PlayerId, by: i8 },
    /// Board confidence, points.
    Board(i8),
    /// How the press takes to the manager.
    Rapport(i8),
    /// It made the news.
    Headline(u32),
}

/// One question and, once answered, the answer and what it did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    /// Index into the press's journalists.
    pub journalist: u32,
    pub topic: Topic,
    pub answer: Option<Tone>,
    pub effects: Vec<Effect>,
}

/// Why a conference is held.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Occasion {
    PreMatch {
        fixture: FixtureRef,
        opponent: ClubId,
    },
    PostMatch {
        fixture: FixtureRef,
        opponent: ClubId,
    },
}

/// A press conference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conference {
    pub id: u32,
    pub club: ClubId,
    pub manager: ManagerId,
    pub occasion: Occasion,
    pub questions: Vec<Question>,
    pub opened: GameDate,
    /// The last day it can be attended.
    pub closes: GameDate,
}

impl Conference {
    pub fn finished(&self) -> bool {
        self.questions.iter().all(|q| q.answer.is_some())
    }
}

/// The press: its journalists, open conferences and how it takes to each
/// manager.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Press {
    journalists: Vec<Journalist>,
    pub(crate) conferences: Vec<Conference>,
    next: u32,
    /// -100 (hostile) to 100 (adored).
    rapport: BTreeMap<ManagerId, i16>,
    /// Managers who leave conferences to their assistant.
    pub(crate) delegated: BTreeSet<ManagerId>,
    /// Heat between a human manager and a rival manager (see `rivals`).
    pub(crate) heat: BTreeMap<(ManagerId, ManagerId), i16>,
}

impl Press {
    pub fn journalists(&self) -> &[Journalist] {
        &self.journalists
    }

    pub fn journalist(&self, i: u32) -> Option<&Journalist> {
        self.journalists.get(i as usize)
    }

    /// Open conferences, oldest first.
    pub fn conferences(&self) -> &[Conference] {
        &self.conferences
    }

    pub fn conference(&self, id: u32) -> Option<&Conference> {
        self.conferences.iter().find(|c| c.id == id)
    }

    pub fn rapport(&self, manager: ManagerId) -> i16 {
        self.rapport.get(&manager).copied().unwrap_or(0)
    }

    pub fn delegated(&self, manager: ManagerId) -> bool {
        self.delegated.contains(&manager)
    }
}

/// Journalists per nation, one per outlet.
const PER_NATION: usize = 4;

/// The nation's journalists, created the first time they are needed from
/// names already in use there.
fn journalists_of(world: &mut World, nation: NationId) -> Vec<u32> {
    let found: Vec<u32> = world
        .press
        .journalists
        .iter()
        .enumerate()
        .filter(|(_, j)| j.nation == nation)
        .map(|(i, _)| i as u32)
        .collect();
    if !found.is_empty() {
        return found;
    }
    let mut firsts = Vec::new();
    let mut lasts = Vec::new();
    let ps = world.players();
    for i in 0..ps.len() {
        let bio = ps.bio(PlayerId::new(i as u32));
        if bio.nation == nation {
            firsts.push(bio.first_name);
            lasts.push(bio.last_name);
            if firsts.len() >= 400 {
                break;
            }
        }
    }
    if firsts.is_empty() {
        let bio = ps.bio(PlayerId::new(0));
        firsts.push(bio.first_name);
        lasts.push(bio.last_name);
    }
    let mut rng = world
        .seed()
        .derive(Stream::Media, 1, u64::from(nation.raw()));
    let start = world.press.journalists.len() as u32;
    for k in 0..PER_NATION {
        let first_name = firsts[rng.below(firsts.len() as u32) as usize];
        let last_name = lasts[rng.below(lasts.len() as u32) as usize];
        world.press.journalists.push(Journalist {
            first_name,
            last_name,
            nation,
            outlet: Outlet::ALL[k % Outlet::ALL.len()],
        });
    }
    (start..start + PER_NATION as u32).collect()
}

/// Every morning: conferences past their day close; human managers' clubs
/// playing tomorrow hold their pre-match conference today.
pub(crate) fn morning(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let mut closed = Vec::new();
    world.press.conferences.retain(|c| {
        if c.closes < today {
            closed.push(c.clone());
            false
        } else {
            true
        }
    });
    for c in closed {
        if c.questions.iter().all(|q| q.answer.is_none())
            && !world.press.delegated(c.manager)
            && world.clubs()[c.club].manager == Some(c.manager)
        {
            // Nobody came: the press notices.
            shift_rapport(world, c.manager, -4);
            news::report(
                world,
                StoryKind::Quote {
                    manager: c.manager,
                    club: c.club,
                    quote: Quote {
                        topic: Topic::Snub,
                        tone: Tone::Deflect,
                    },
                },
            );
        }
    }

    // Rivals speak first, so today's conferences can ask about it.
    crate::rivals::daily(world, events);

    let tomorrow = today.add_days(1);
    let clubs: Vec<(ClubId, ManagerId)> = world
        .clubs()
        .iter()
        .filter_map(|(c, club)| club.manager.map(|m| (c, m)))
        .filter(|&(c, m)| world.is_human_club(c) && !world.press.delegated(m))
        .collect();
    for (club, manager) in clubs {
        let Some(f) = fixture::on(world, club, tomorrow) else {
            continue;
        };
        let Some(fx) = f.get(world) else { continue };
        let opponent = if fx.home == club { fx.away } else { fx.home };
        open(
            world,
            club,
            manager,
            Occasion::PreMatch {
                fixture: f,
                opponent,
            },
            // Over before the match day begins, so nothing said changes a
            // match being watched.
            today,
            events,
        );
    }
}

/// After a human manager's club has played.
pub(crate) fn after_match(
    world: &mut World,
    fixture: FixtureRef,
    club: ClubId,
    opponent: ClubId,
    events: &mut Vec<Event>,
) {
    let Some(manager) = world.clubs()[club].manager else {
        return;
    };
    if !world.is_human_club(club) || world.press.delegated(manager) {
        return;
    }
    // The pre-match conference is over once the match is played.
    world.press.conferences.retain(|c| {
        !(c.club == club
            && matches!(c.occasion, Occasion::PreMatch { fixture: f, .. } if f == fixture))
    });
    let closes = world.date().add_days(2);
    open(
        world,
        club,
        manager,
        Occasion::PostMatch { fixture, opponent },
        closes,
        events,
    );
}

fn open(
    world: &mut World,
    club: ClubId,
    manager: ManagerId,
    occasion: Occasion,
    closes: GameDate,
    events: &mut Vec<Event>,
) {
    let today = world.date();
    let id = world.press.next;
    let mut rng = world.seed().derive(
        Stream::Media,
        u64::from(today.days()),
        u64::from(club.raw()),
    );
    let mut topics = match occasion {
        Occasion::PreMatch { opponent, .. } => {
            let mut t = vec![Topic::Preview { opponent }];
            if let Some(rival) = world.clubs()[opponent].manager
                && crate::rivals::heat(world, manager, rival) >= crate::rivals::FEUD
            {
                t.push(Topic::Grudge { opponent });
            }
            if let Some(rival) = rival_says(world, &mut rng, club, opponent) {
                t.push(rival);
            }
            t
        }
        Occasion::PostMatch { fixture, opponent } => {
            let goals = fixture
                .get(world)
                .and_then(|f| {
                    let r = f.result.as_ref()?;
                    Some(if f.home == club {
                        r.goals
                    } else {
                        [r.goals[1], r.goals[0]]
                    })
                })
                .unwrap_or([0, 0]);
            vec![Topic::Result { opponent, goals }]
        }
    };
    let want = match occasion {
        Occasion::PreMatch { .. } => 4,
        Occasion::PostMatch { .. } => 3,
    };
    let mut context = context(world, club);
    // A fixed order of what matters most, shuffled a little.
    while topics.len() < want && !context.is_empty() {
        let i = if context.len() > 1 && rng.chance(0.35) {
            1
        } else {
            0
        };
        let t = context.remove(i);
        if !topics.contains(&t) {
            topics.push(t);
        }
    }
    let nation = world.clubs()[club].nation;
    let pool = journalists_of(world, nation);
    let mut questions = Vec::with_capacity(topics.len());
    for (k, topic) in topics.into_iter().enumerate() {
        // Tabloids ask about rows; broadsheets about the game.
        let preferred = match topic {
            Topic::RivalManager { .. } | Topic::Unrest { .. } | Topic::Rumour { .. } => {
                Some(Outlet::Tabloid)
            }
            Topic::Preview { .. } | Topic::Result { .. } => Some(Outlet::Broadcaster),
            Topic::Youngster { .. } | Topic::NewSigning { .. } => Some(Outlet::FanSite),
            _ => None,
        };
        let journalist = preferred
            .and_then(|o| {
                pool.iter()
                    .copied()
                    .find(|&j| world.press.journalists[j as usize].outlet == o)
            })
            .unwrap_or(pool[(k + rng.below(pool.len() as u32) as usize) % pool.len()]);
        questions.push(Question {
            journalist,
            topic,
            answer: None,
            effects: Vec::new(),
        });
    }
    world.press.next += 1;
    world.press.conferences.push(Conference {
        id,
        club,
        manager,
        occasion,
        questions,
        opened: today,
        closes,
    });
    events.push(Event::PressConference {
        manager,
        club,
        conference: id,
    });
}

/// Sometimes the other manager talks first, and it is reported.
fn rival_says(world: &mut World, rng: &mut Rng, club: ClubId, opponent: ClubId) -> Option<Topic> {
    let manager = world.clubs()[opponent].manager?;
    if world.is_human_club(opponent) || !rng.chance(0.45) {
        return None;
    }
    let ours = i32::from(world.clubs()[club].reputation);
    let theirs = i32::from(world.clubs()[opponent].reputation);
    let tone = if theirs > ours + 1_000 {
        Tone::Dismissive
    } else if theirs + 1_000 < ours {
        Tone::Humble
    } else {
        Tone::Confident
    };
    news::report(
        world,
        StoryKind::Quote {
            manager,
            club: opponent,
            quote: Quote {
                topic: Topic::Preview { opponent: club },
                tone,
            },
        },
    );
    Some(Topic::RivalManager { opponent, tone })
}

/// What the press wants to ask about this club, most pressing first.
fn context(world: &World, club: ClubId) -> Vec<Topic> {
    let today = world.date();
    let c = &world.clubs()[club];
    let ps = world.players();
    let squad = c.squad();
    let mut out = Vec::new();

    // What a rival manager has just said about the club.
    if let Some((rival, tone, angle)) = crate::rivals::recent_comment(world, club, 2) {
        out.push(Topic::RivalComment { rival, tone, angle });
    }
    // Unrest: a transfer request waiting.
    if let Some(&p) = squad.iter().find(|&&p| {
        world
            .relations
            .requests
            .get(&p)
            .is_some_and(|r| !r.accepted)
    }) {
        out.push(Topic::Unrest { player: p });
    }
    // A rumour about one of ours in the last three weeks.
    if let Some(s) = world.news.stories().iter().rev().find(|s| {
        today.days_since(s.date) <= 21
            && matches!(s.kind, StoryKind::Rumour { owner, player, .. }
                if owner == club && ps.club(player) == Some(club))
    }) && let StoryKind::Rumour {
        player,
        club: suitor,
        ..
    } = s.kind
    {
        out.push(Topic::Rumour { player, suitor });
    }
    if c.board.confidence < 40 {
        out.push(Topic::Pressure);
    }
    if let Some(league) = c.league()
        && let Some(season) = world.leagues()[league].season()
    {
        let table = season.table();
        if let Some(pos) = table.iter().position(|r| r.club == club)
            && table[pos].played >= 8
        {
            if pos < 2 {
                out.push(Topic::TitleRace);
            } else if pos + 3 >= table.len() {
                out.push(Topic::Relegation);
            }
        }
    }
    // Form: the best and the most worrying regulars.
    let rated = |p: PlayerId| {
        let s = ps.season_stats(p);
        (s.appearances >= 3).then(|| s.average_rating()).flatten()
    };
    if let Some((p, _)) = squad
        .iter()
        .filter_map(|&p| rated(p).map(|r| (p, r)))
        .filter(|&(_, r)| r >= 7.2)
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
    {
        out.push(Topic::StarForm { player: p });
    }
    if let Some((p, _)) = squad
        .iter()
        .filter(|&&p| matches!(ps.squad_status(p), SquadStatus::Key | SquadStatus::Regular))
        .filter_map(|&p| rated(p).map(|r| (p, r)))
        .filter(|&(_, r)| r < 6.4)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    {
        out.push(Topic::PoorForm { player: p });
    }
    // A key player out for a while.
    if let Some(&p) = squad
        .iter()
        .filter(|&&p| ps.squad_status(p) == SquadStatus::Key)
        .find(|&&p| {
            ps.injury(p)
                .is_some_and(|i| i.until.days_since(today) >= 14)
        })
    {
        out.push(Topic::Injury { player: p });
    }
    // A new arrival this month.
    if let Some(s) = world.news.stories().iter().rev().find(|s| {
        today.days_since(s.date) <= 30
            && matches!(s.kind, StoryKind::Transfer { to, loan: false, player, .. }
                if to == club && ps.club(player) == Some(club))
    }) && let StoryKind::Transfer { player, .. } = s.kind
    {
        out.push(Topic::NewSigning { player });
    }
    // Always something to ask: the season's hopes and the star.
    let fallback = {
        let mut f = Vec::new();
        if let Some(&p) = squad
            .iter()
            .filter(|&&p| ps.injury(p).is_none())
            .max_by_key(|&&p| (ps.ability(p), std::cmp::Reverse(p)))
        {
            f.push(Topic::KeyPlayer { player: p });
        }
        f.push(Topic::Expectations);
        f
    };
    // A youngster getting games.
    if let Some(&p) = squad.iter().find(|&&p| {
        crate::people::age_years(ps.bio(p).birth_date, today) < 21.0
            && ps.season_stats(p).appearances >= 2
    }) {
        out.push(Topic::Youngster { player: p });
    }
    out.extend(fallback);
    out
}

fn shift_rapport(world: &mut World, manager: ManagerId, by: i16) {
    let r = world.press.rapport.entry(manager).or_insert(0);
    *r = (*r + by).clamp(-100, 100);
}

fn shift_morale(world: &mut World, player: PlayerId, by: i32) {
    let c = &mut world.players.condition[player.index()];
    c.morale = (i32::from(c.morale) + by).clamp(0, i32::from(CONDITION_MAX)) as u16;
}

/// Morale for a whole squad, in per mille of the scale.
pub(crate) fn squad_morale(world: &mut World, club: ClubId, per_mille: i16) {
    let by = i32::from(per_mille) * i32::from(CONDITION_MAX) / 1000;
    let squad = world.clubs()[club].squad().to_vec();
    for p in squad {
        shift_morale(world, p, by);
    }
}

/// How a player takes being talked about in public: their memory of it
/// and its weight.
fn player_feels(
    world: &mut World,
    player: PlayerId,
    feeling: Feeling,
    by: i8,
    days: u32,
) -> Effect {
    happiness::remember(world, player, feeling, by, days);
    shift_morale(world, player, i32::from(by) * 60);
    happiness::refresh(world, player);
    Effect::Player { player, by }
}

/// Answers one question. The caller has checked that the manager runs
/// `club`.
pub(crate) fn answer(
    world: &mut World,
    club: ClubId,
    conference: u32,
    question: u8,
    tone: Tone,
) -> Result<Vec<Event>, CommandError> {
    let c = world
        .press
        .conferences
        .iter()
        .find(|c| c.id == conference && c.club == club)
        .ok_or(CommandError::NoSuchConference(conference))?;
    let q = c
        .questions
        .get(usize::from(question))
        .ok_or(CommandError::NoSuchConference(conference))?;
    if q.answer.is_some() {
        return Err(CommandError::AlreadyAnswered(conference));
    }
    if !options(q.topic).contains(&tone) {
        return Err(CommandError::UnsuitableAnswer(conference));
    }
    let topic = q.topic;
    let outlet = world.press.journalists[q.journalist as usize].outlet;
    let manager = c.manager;
    let professional = |w: &World, p: PlayerId| trait_of(w, p, HiddenAttr::Professionalism);
    let temperament = |w: &World, p: PlayerId| trait_of(w, p, HiddenAttr::Temperament);
    let mut effects = Vec::new();
    let mut quotable = false;

    use Tone::*;
    match (topic, tone) {
        (Topic::Preview { .. }, Confident) => {
            squad_morale(world, club, 25);
            effects.push(Effect::Squad(25));
            quotable = true;
        }
        (Topic::Preview { .. }, Cautious) => {
            squad_morale(world, club, 10);
            effects.push(Effect::Squad(10));
        }
        (Topic::Preview { opponent }, Humble) => {
            squad_morale(world, opponent, -10);
            effects.push(Effect::Opponents {
                club: opponent,
                by: -10,
            });
        }
        (Topic::Preview { opponent }, Dismissive)
        | (Topic::RivalManager { opponent, .. }, Dismissive)
        | (
            Topic::RivalComment {
                rival: opponent, ..
            },
            Dismissive,
        )
        | (Topic::Grudge { opponent }, Dismissive) => {
            squad_morale(world, club, 20);
            squad_morale(world, opponent, 30);
            effects.push(Effect::Squad(20));
            effects.push(Effect::Opponents {
                club: opponent,
                by: 30,
            });
            crate::rivals::warm(world, manager, opponent, 15);
            quotable = true;
        }
        (Topic::RivalComment { rival, .. }, Humble)
        | (Topic::Grudge { opponent: rival }, Humble) => {
            squad_morale(world, club, 5);
            effects.push(Effect::Squad(5));
            crate::rivals::warm(world, manager, rival, -15);
        }
        (Topic::RivalComment { rival, .. }, Defiant) => {
            squad_morale(world, club, 20);
            effects.push(Effect::Squad(20));
            crate::rivals::warm(world, manager, rival, 5);
            quotable = true;
        }
        (Topic::RivalManager { .. }, Humble) => {
            squad_morale(world, club, 5);
            effects.push(Effect::Squad(5));
        }
        (Topic::RivalManager { .. }, Defiant) => {
            squad_morale(world, club, 25);
            effects.push(Effect::Squad(25));
            quotable = true;
        }
        (Topic::Result { goals, .. }, Praise) => {
            let by = if goals[0] > goals[1] { 25 } else { 15 };
            squad_morale(world, club, by);
            effects.push(Effect::Squad(by));
        }
        (Topic::Result { .. }, Humble) => {
            squad_morale(world, club, 5);
            crate::board::adjust_confidence(world, club, 1.0);
            effects.push(Effect::Squad(5));
            effects.push(Effect::Board(1));
        }
        (Topic::Result { .. }, Confident) => {
            squad_morale(world, club, 20);
            effects.push(Effect::Squad(20));
            quotable = true;
        }
        (Topic::Result { .. }, Cautious) => {}
        (Topic::Result { .. }, Criticise) => {
            // A public dressing-down: the professionals take it as a
            // challenge, the rest sulk.
            let squad = world.clubs()[club].squad().to_vec();
            let mut net = 0i32;
            for p in squad {
                let by = if professional(world, p) > 0.6 {
                    300
                } else if temperament(world, p) < 0.35 {
                    -500
                } else {
                    -200
                };
                net += by;
                shift_morale(world, p, by);
            }
            let n = world.clubs()[club].squad().len().max(1) as i32;
            let per_mille = (net * 1000 / i32::from(CONDITION_MAX) / n) as i16;
            effects.push(Effect::Squad(per_mille));
            quotable = true;
        }
        (Topic::Result { .. }, Defiant) => {
            squad_morale(world, club, 15);
            effects.push(Effect::Squad(15));
            shift_rapport(world, manager, -2);
            effects.push(Effect::Rapport(-2));
            quotable = true;
        }
        (Topic::Expectations, Confident) => {
            squad_morale(world, club, 20);
            effects.push(Effect::Squad(20));
            quotable = true;
        }
        (Topic::Expectations, Cautious) | (Topic::KeyPlayer { .. }, Cautious) => {
            squad_morale(world, club, 5);
            effects.push(Effect::Squad(5));
        }
        (Topic::Expectations, Humble) => {
            crate::board::adjust_confidence(world, club, 1.0);
            effects.push(Effect::Board(1));
        }
        (Topic::StarForm { player }, Praise)
        | (Topic::NewSigning { player }, Praise)
        | (Topic::KeyPlayer { player }, Praise) => {
            effects.push(player_feels(world, player, Feeling::PraisedPublicly, 6, 45));
            quotable = true;
        }
        (Topic::StarForm { player }, Cautious) | (Topic::Youngster { player }, Cautious) => {
            let by = if professional(world, player) > 0.5 {
                3
            } else {
                -2
            };
            effects.push(player_feels(world, player, Feeling::KeptGrounded, by, 30));
        }
        (Topic::NewSigning { player }, Cautious) => {
            effects.push(player_feels(world, player, Feeling::KeptGrounded, 2, 30));
        }
        (Topic::Youngster { player }, Praise) => {
            // Hype: a professional takes it in their stride.
            let by = if professional(world, player) > 0.5 {
                6
            } else {
                3
            };
            effects.push(player_feels(
                world,
                player,
                Feeling::PraisedPublicly,
                by,
                45,
            ));
            quotable = true;
        }
        (Topic::PoorForm { player }, Praise) => {
            effects.push(player_feels(world, player, Feeling::BackedPublicly, 6, 60));
        }
        (Topic::PoorForm { player }, Criticise) => {
            let by = if professional(world, player) > 0.65 {
                3
            } else if temperament(world, player) < 0.35 {
                -14
            } else {
                -9
            };
            effects.push(player_feels(
                world,
                player,
                Feeling::CriticisedPublicly,
                by,
                75,
            ));
            quotable = true;
        }
        (Topic::Rumour { player, .. }, Defiant) | (Topic::Unrest { player }, Defiant) => {
            // Not for sale: the loyal are flattered, the ambitious feel
            // trapped.
            let loyal = trait_of(world, player, HiddenAttr::Loyalty);
            let ambition = trait_of(world, player, HiddenAttr::Ambition);
            let wants_out = matches!(topic, Topic::Unrest { .. });
            let by = if wants_out || ambition > loyal + 0.2 {
                -6
            } else {
                5
            };
            let feeling = if by > 0 {
                Feeling::BackedPublicly
            } else {
                Feeling::BlockedMove
            };
            effects.push(player_feels(world, player, feeling, by, 60));
            quotable = true;
        }
        (Topic::Rumour { player, .. }, Cautious) => {
            // Everyone has a price.
            let ambition = trait_of(world, player, HiddenAttr::Ambition);
            let by = if ambition > 0.65 { 2 } else { -7 };
            effects.push(player_feels(world, player, Feeling::Unsettled, by, 60));
            quotable = true;
        }
        (Topic::Unrest { player }, Humble) => {
            effects.push(player_feels(world, player, Feeling::Reassured, 3, 30));
        }
        (Topic::Unrest { player }, Criticise) => {
            effects.push(player_feels(
                world,
                player,
                Feeling::CriticisedPublicly,
                -12,
                90,
            ));
            squad_morale(world, club, 10);
            effects.push(Effect::Squad(10));
            quotable = true;
        }
        (Topic::Pressure, Confident) | (Topic::Relegation, Confident) => {
            squad_morale(world, club, 15);
            effects.push(Effect::Squad(15));
            quotable = true;
        }
        (Topic::Pressure, Humble) | (Topic::Relegation, Humble) => {
            crate::board::adjust_confidence(world, club, 1.5);
            effects.push(Effect::Board(2));
        }
        (Topic::Pressure, Defiant) | (Topic::Relegation, Defiant) => {
            squad_morale(world, club, 15);
            crate::board::adjust_confidence(world, club, -1.0);
            effects.push(Effect::Squad(15));
            effects.push(Effect::Board(-1));
            quotable = true;
        }
        (Topic::TitleRace, Confident) => {
            squad_morale(world, club, 25);
            effects.push(Effect::Squad(25));
            quotable = true;
        }
        (Topic::TitleRace, Cautious) => {
            squad_morale(world, club, 10);
            effects.push(Effect::Squad(10));
        }
        (Topic::Injury { .. }, Confident) => {
            squad_morale(world, club, 15);
            effects.push(Effect::Squad(15));
        }
        (_, Deflect) => {
            shift_rapport(world, manager, -1);
            effects.push(Effect::Rapport(-1));
        }
        _ => {}
    }

    // How the room takes it.
    let liked = match (outlet, tone) {
        (Outlet::Tabloid, Dismissive | Defiant | Criticise) => 2,
        (Outlet::Broadsheet, Humble | Cautious) => 2,
        (Outlet::FanSite, Praise | Confident) => 2,
        (Outlet::Broadcaster, Confident | Praise | Humble) => 1,
        _ => 0,
    };
    if liked != 0 {
        shift_rapport(world, manager, liked);
        effects.push(Effect::Rapport(liked as i8));
    }
    if quotable
        && let Some(story) = news::report(
            world,
            StoryKind::Quote {
                manager,
                club,
                quote: Quote { topic, tone },
            },
        )
    {
        effects.push(Effect::Headline(story));
    }

    let c = world
        .press
        .conferences
        .iter_mut()
        .find(|c| c.id == conference)
        .expect("conference found above");
    let q = &mut c.questions[usize::from(question)];
    q.answer = Some(tone);
    q.effects = effects;
    Ok(vec![Event::PressAnswered {
        club,
        conference,
        question,
    }])
}

/// Leaves the rest of a conference to the assistant.
pub(crate) fn leave(
    world: &mut World,
    club: ClubId,
    conference: u32,
) -> Result<Vec<Event>, CommandError> {
    let before = world.press.conferences.len();
    world
        .press
        .conferences
        .retain(|c| !(c.id == conference && c.club == club));
    if world.press.conferences.len() == before {
        return Err(CommandError::NoSuchConference(conference));
    }
    Ok(Vec::new())
}

/// A manager's choice to send the assistant to every conference.
pub(crate) fn delegate(world: &mut World, manager: ManagerId, on: bool) {
    if on {
        world.press.delegated.insert(manager);
        world.press.conferences.retain(|c| c.manager != manager);
    } else {
        world.press.delegated.remove(&manager);
    }
}

/// A manager left their club: their conferences go with them.
pub(crate) fn manager_left(world: &mut World, manager: ManagerId) {
    world.press.conferences.retain(|c| c.manager != manager);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_has_answers() {
        let c = ClubId::new(0);
        let p = PlayerId::new(0);
        for t in [
            Topic::Preview { opponent: c },
            Topic::RivalManager {
                opponent: c,
                tone: Tone::Confident,
            },
            Topic::Result {
                opponent: c,
                goals: [2, 1],
            },
            Topic::Result {
                opponent: c,
                goals: [1, 1],
            },
            Topic::Result {
                opponent: c,
                goals: [0, 1],
            },
            Topic::StarForm { player: p },
            Topic::PoorForm { player: p },
            Topic::Rumour {
                player: p,
                suitor: c,
            },
            Topic::Pressure,
            Topic::TitleRace,
            Topic::Relegation,
            Topic::Injury { player: p },
            Topic::NewSigning { player: p },
            Topic::Youngster { player: p },
            Topic::Unrest { player: p },
        ] {
            assert!(options(t).len() >= 3, "{t:?}");
        }
    }
}
