//! How players feel about their club: squad status, playing time,
//! happiness with visible reasons, and transfer requests.
//!
//! Every player at a club has a squad status, the role they expect, and a
//! running share of the club's match minutes. Once a month each player's
//! happiness is worked out from reasons the manager can see (playing time
//! against status, wages, ambition against the club's level, an expiring
//! contract, the captaincy, the mood of the dressing room's leaders, and
//! memories of talks and promises). Happiness sets the level morale drifts
//! back to each day. Players who stay very unhappy hand in transfer
//! requests: AI clubs list them, human managers answer.
//!
//! The same rules run for every club. For AI clubs the reasons mostly
//! balance out: statuses follow selection, and AI clubs grant requests.

use crate::command::Event;
use crate::ids::{ClubId, PlayerId};
use crate::people::{age_years, wage_demand};
use crate::personality::{leaders_with, natural_captain, trait_of};
use crate::player::CONDITION_MAX;
use crate::talk::Promise;
use crate::transfer::Listing;
use crate::world::World;
use fs_core::attributes::HiddenAttr;
use fs_core::{GameDate, Stream, par};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The role a player expects at their club.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SquadStatus {
    /// One of the first names on the team sheet.
    Key,
    /// A regular starter.
    Regular,
    /// In and out of the team.
    Rotation,
    /// Cover.
    Backup,
    /// A youngster, expecting little first-team football yet.
    Prospect,
}

impl SquadStatus {
    pub const ALL: [SquadStatus; 5] = [
        SquadStatus::Key,
        SquadStatus::Regular,
        SquadStatus::Rotation,
        SquadStatus::Backup,
        SquadStatus::Prospect,
    ];

    /// The share of the club's match minutes the role expects, 0-10000.
    pub fn expected(self) -> u16 {
        match self {
            SquadStatus::Key => 7_000,
            SquadStatus::Regular => 4_500,
            SquadStatus::Rotation => 1_600,
            SquadStatus::Backup => 500,
            SquadStatus::Prospect => 200,
        }
    }

    /// The next role up.
    pub fn above(self) -> SquadStatus {
        match self {
            SquadStatus::Key | SquadStatus::Regular => SquadStatus::Key,
            SquadStatus::Rotation => SquadStatus::Regular,
            SquadStatus::Backup | SquadStatus::Prospect => SquadStatus::Rotation,
        }
    }
}

/// Something a player remembers, for a while.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feeling {
    Praised,
    Criticised,
    /// Took criticism as a challenge.
    FiredUp,
    UnfairCriticism,
    Reassured,
    /// Did not believe the manager's reassurance.
    Doubted,
    /// Waiting for a promise to be kept.
    PromiseMade,
    PromiseKept,
    PromiseBroken,
    RequestRefused,
    RequestAccepted,
    LostCaptaincy,
    Demoted,
    Promoted,
    /// Contract talks broke down.
    TalksFailed,
    /// Praised by the manager in front of the press.
    PraisedPublicly,
    /// Criticised by the manager in front of the press.
    CriticisedPublicly,
    /// Backed by the manager in public.
    BackedPublicly,
    /// Told in public they are not for sale, when they would like to go.
    BlockedMove,
    /// Heard the manager say they could be sold.
    Unsettled,
    /// Told in public to keep their feet on the ground.
    KeptGrounded,
    /// The manager asked how they were and listened.
    Listened,
}

/// A remembered feeling and its weight on happiness until it fades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub feeling: Feeling,
    pub amount: i8,
    pub until: GameDate,
}

/// A transfer request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub club: ClubId,
    pub date: GameDate,
    /// Whether the club agreed to sell.
    pub accepted: bool,
}

/// Relations between players and their clubs: what players remember,
/// promises made to them, and their transfer requests. Only players at
/// clubs appear; leaving a club wipes the slate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relations {
    pub(crate) memories: BTreeMap<PlayerId, Vec<Memory>>,
    pub(crate) promises: Vec<Promise>,
    pub(crate) requests: BTreeMap<PlayerId, Request>,
    /// When each player last had a talk with the manager.
    pub(crate) talks: BTreeMap<PlayerId, GameDate>,
    /// Transfer requests handed in since the world began.
    pub(crate) requests_made: u32,
    /// Contract talks under way (or recently broken down), free agents
    /// included.
    pub(crate) negotiations: BTreeMap<PlayerId, crate::contract::Talks>,
}

impl Relations {
    pub fn memories(&self, player: PlayerId) -> &[Memory] {
        self.memories.get(&player).map_or(&[], Vec::as_slice)
    }

    /// Open promises, oldest first.
    pub fn promises(&self) -> &[Promise] {
        &self.promises
    }

    pub fn request(&self, player: PlayerId) -> Option<Request> {
        self.requests.get(&player).copied()
    }

    pub fn requests(&self) -> &BTreeMap<PlayerId, Request> {
        &self.requests
    }

    pub fn last_talk(&self, player: PlayerId) -> Option<GameDate> {
        self.talks.get(&player).copied()
    }

    /// Contract talks with a player, if any.
    pub fn negotiation(&self, player: PlayerId) -> Option<crate::contract::Talks> {
        self.negotiations.get(&player).copied()
    }

    pub fn requests_made(&self) -> u32 {
        self.requests_made
    }

    pub fn remembers(&self, player: PlayerId, feeling: Feeling) -> bool {
        self.memories(player).iter().any(|m| m.feeling == feeling)
    }
}

/// Why a player is happy or unhappy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Playing time against the role they expect.
    PlayingTime,
    /// Paid well below (or well above) their worth.
    Wages,
    /// Too good for the club.
    Ambition,
    /// The club doing better or worse than expected.
    Results,
    /// Proud to play at a club above their level.
    Prestige,
    /// The contract is running out.
    Future,
    Captain,
    /// The mood of the dressing room's leaders.
    DressingRoom,
    /// Put up for sale without asking.
    Listed,
    /// Waiting for an answer to a transfer request.
    RequestPending,
    Memory(Feeling),
}

/// One reason and how much it moves happiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Factor {
    pub reason: Reason,
    pub effect: i16,
}

/// A player's happiness now, with its reasons, largest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub happiness: i8,
    pub factors: Vec<Factor>,
}

/// At or below this, a player is very unhappy.
pub const UNHAPPY: i8 = -30;
/// Morale per point of happiness, around the neutral 5000.
const MORALE_PER_POINT: i32 = 15;

/// The level a player's morale drifts back to.
pub fn morale_baseline(happiness: i8) -> u16 {
    (i32::from(CONDITION_MAX / 2) + i32::from(happiness) * MORALE_PER_POINT)
        .clamp(0, i32::from(CONDITION_MAX)) as u16
}

/// What a club's players are measured against.
struct Context {
    /// Mean ability of the club's best eleven.
    bar: u16,
    /// League places above (positive) or below the board's target, once
    /// the season is under way.
    against_target: Option<i32>,
    /// The squad's abilities, worked out once.
    abilities: Vec<(PlayerId, u16)>,
}

impl Context {
    fn ability(&self, world: &World, p: PlayerId) -> u16 {
        self.abilities
            .iter()
            .find(|a| a.0 == p)
            .map_or_else(|| world.players().ability(p), |a| a.1)
    }
}

fn context(world: &World, club: ClubId) -> Context {
    let ps = world.players();
    let abilities: Vec<(PlayerId, u16)> = world.clubs()[club]
        .squad()
        .iter()
        .map(|&p| (p, ps.ability(p)))
        .collect();
    let mut sorted: Vec<u16> = abilities.iter().map(|a| a.1).collect();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let top = &sorted[..sorted.len().min(11)];
    let bar = top.iter().map(|&a| u32::from(a)).sum::<u32>() / top.len().max(1) as u32;
    let target = world.clubs()[club].board().target;
    let against_target = world.clubs()[club]
        .league()
        .and_then(|l| world.leagues()[l].season())
        .filter(|_| target > 0)
        .and_then(|s| {
            let table = s.table();
            let k = table.iter().position(|r| r.club == club)?;
            (table[k].played >= 6).then_some(i32::from(target) - (k as i32 + 1))
        });
    Context {
        bar: bar as u16,
        against_target,
        abilities,
    }
}

/// Everything but the dressing room.
/// Adds every reason but the dressing room to `out`.
fn personal(world: &World, ctx: &Context, club: ClubId, player: PlayerId, out: &mut Vec<Factor>) {
    let ps = world.players();
    let today = world.date();
    let i = player.index();
    let age = age_years(ps.bio(player).birth_date, today);
    let ambition = trait_of(world, player, HiddenAttr::Ambition);
    let loyalty = trait_of(world, player, HiddenAttr::Loyalty);
    let ability = ctx.ability(world, player);
    let mut push = |reason, effect: f64| {
        let effect = effect.round() as i16;
        if effect != 0 {
            out.push(Factor { reason, effect });
        }
    };

    // Playing time, unless injured or still in the academy.
    let status = ps.squad_status[i];
    let senior = age >= 18.0 && status != SquadStatus::Prospect;
    if senior && ps.injury(player).is_none() {
        let expected = f64::from(status.expected()) / 10_000.0;
        let share = f64::from(ps.playing_time[i]) / 10_000.0;
        if share < expected {
            let deficit = ((expected - share) / expected - 0.2).max(0.0) / 0.8;
            push(
                Reason::PlayingTime,
                -deficit * 40.0 * (0.6 + 0.6 * ambition),
            );
        } else if status <= SquadStatus::Regular {
            push(Reason::PlayingTime, 6.0);
        }
    }

    // Wages against what the player could now earn, for established
    // players who expect to play.
    if let Some(c) = ps.contract(player) {
        if age >= 21.0 && status <= SquadStatus::Rotation {
            let demand = wage_demand(ability, ps.potential(player), age as u32);
            let ratio = f64::from(c.wage) / f64::from(demand.max(1));
            if ratio < 0.5 {
                let w = ((0.5 - ratio) / 0.5 * 30.0 * (0.4 + ambition)).min(20.0);
                push(Reason::Wages, -w);
            } else if ratio >= 1.3 {
                push(Reason::Wages, 4.0);
            }
        }
        // A contract running out, for players who want to stay.
        let left = c.expires.days_since(today);
        if left < 183 && age < 33.0 && ps.squad_status[i] != SquadStatus::Prospect {
            push(Reason::Future, -(6.0 + 8.0 * loyalty));
        }
    }

    // Ambition: a player far better than the team.
    let gap = f64::from(ability) - f64::from(ctx.bar);
    if gap > 40.0 {
        let w = ((gap - 40.0) / 100.0 * 30.0 * ambition * 1.5 * (1.2 - loyalty)).min(30.0);
        push(Reason::Ambition, -w);
    } else if gap < -60.0 && age >= 18.0 {
        push(Reason::Prestige, 4.0);
    }

    if let Some(d) = ctx.against_target
        && senior
    {
        push(Reason::Results, (f64::from(d) * 2.0).clamp(-8.0, 12.0));
    }

    if world.clubs()[club].captain == Some(player) {
        push(Reason::Captain, 8.0);
    }
    let request = world.relations.request(player);
    match (request, world.market().listing(player)) {
        (Some(r), _) if !r.accepted => push(Reason::RequestPending, -10.0),
        (None, Some(Listing::Transfer)) => push(Reason::Listed, -(6.0 + 14.0 * loyalty)),
        _ => {}
    }
    for m in world.relations.memories(player) {
        push(Reason::Memory(m.feeling), f64::from(m.amount));
    }
}

fn total(factors: &[Factor]) -> i32 {
    factors.iter().map(|f| i32::from(f.effect)).sum()
}

fn clamp(v: i32) -> i8 {
    v.clamp(-100, 100) as i8
}

/// The leaders of a dressing room and their moods.
struct Leaders(Vec<(PlayerId, i32)>);

impl Leaders {
    fn of(world: &World, ctx: &Context, club: ClubId) -> Self {
        let mut buf = Vec::new();
        Leaders(
            leaders_with(world, club, |p| ctx.ability(world, p))
                .into_iter()
                .map(|p| {
                    buf.clear();
                    personal(world, ctx, club, p, &mut buf);
                    (p, total(&buf).clamp(-100, 100))
                })
                .collect(),
        )
    }

    /// The dressing room's effect on a player: a share of the mood of the
    /// leaders other than themselves.
    fn effect_on(&self, player: PlayerId) -> Option<Factor> {
        let (sum, n) = self
            .0
            .iter()
            .filter(|(p, _)| *p != player)
            .fold((0, 0), |(s, n), &(_, m)| (s + m, n + 1));
        if n == 0 {
            return None;
        }
        let effect = (sum / n * 3 / 10) as i16;
        (effect.abs() >= 3).then_some(Factor {
            reason: Reason::DressingRoom,
            effect,
        })
    }
}

/// A player's happiness as things stand, with every reason. `None` for
/// players without a club.
pub fn assess(world: &World, player: PlayerId) -> Option<Assessment> {
    let club = world.players().club(player)?;
    let ctx = context(world, club);
    let leaders = Leaders::of(world, &ctx, club);
    let mut factors = Vec::new();
    personal(world, &ctx, club, player, &mut factors);
    factors.extend(leaders.effect_on(player));
    factors.sort_by_key(|f| std::cmp::Reverse(f.effect.abs()));
    Some(Assessment {
        happiness: clamp(total(&factors)),
        factors,
    })
}

/// The statuses a club would give its players now: its best eleven
/// (regardless of injuries) are key players or regulars, the bench rotates
/// in, and the rest are backups or prospects.
pub fn natural_statuses(world: &World, club: ClubId) -> Vec<(PlayerId, SquadStatus)> {
    let ps = world.players();
    let today = world.date();
    let squad = world.clubs()[club].squad();
    let adult = |p: PlayerId| age_years(ps.bio(p).birth_date, today) >= 18.0;
    let mut pool: Vec<PlayerId> = squad.iter().copied().filter(|&p| adult(p)).collect();
    if pool.len() < 11 {
        pool = squad.to_vec();
    }
    let mut out: BTreeMap<PlayerId, SquadStatus> = BTreeMap::new();
    if pool.len() >= 11 {
        let snaps: Vec<_> = pool
            .iter()
            .map(|&p| crate::matchday::snapshot(world, p))
            .collect();
        let tactic = world.clubs()[club]
            .tactic()
            .unwrap_or_else(|| crate::matchday::default_tactic(world, club, &snaps));
        let sheet = fs_match::pick_team(&snaps, tactic);
        let mut starters: Vec<PlayerId> = sheet.starters.iter().map(|s| s.id).collect();
        starters.sort_by(|&a, &b| ps.ability(b).cmp(&ps.ability(a)).then(a.cmp(&b)));
        for (k, &p) in starters.iter().enumerate() {
            let s = if k < 5 {
                SquadStatus::Key
            } else {
                SquadStatus::Regular
            };
            out.insert(p, s);
        }
        // The bench rotates in, except the reserve goalkeeper.
        for s in &sheet.bench {
            let keeper = ps.positions(s.id).best() == fs_core::attributes::Position::Goalkeeper;
            let status = if keeper {
                SquadStatus::Backup
            } else {
                SquadStatus::Rotation
            };
            out.entry(s.id).or_insert(status);
        }
    }
    squad
        .iter()
        .map(|&p| {
            let s = out.get(&p).copied().unwrap_or_else(|| {
                if age_years(ps.bio(p).birth_date, today) < 21.0 {
                    SquadStatus::Prospect
                } else {
                    SquadStatus::Backup
                }
            });
            (p, s)
        })
        .collect()
}

fn set_status(world: &mut World, player: PlayerId, status: SquadStatus) {
    let i = player.index();
    if world.players.squad_status[i] != status {
        world.players.squad_status[i] = status;
        world.players.playing_time[i] = status.expected();
    }
}

/// A new world: statuses from selection, and a captain at every club.
pub fn initialise(world: &mut World) {
    let clubs: Vec<ClubId> = world.clubs().ids().collect();
    for club in clubs {
        for (p, s) in natural_statuses(world, club) {
            set_status(world, p, s);
        }
        world.clubs[club].captain = natural_captain(world, club);
    }
}

/// The status a newcomer expects: the highest role whose current holders
/// they are at least as good as (keepers measured against keepers). Much
/// cheaper than picking the team again for every signing.
pub(crate) fn newcomer_status(world: &World, club: ClubId, player: PlayerId) -> SquadStatus {
    let ps = world.players();
    let today = world.date();
    let age = age_years(ps.bio(player).birth_date, today);
    if age < 18.0 {
        return SquadStatus::Prospect;
    }
    let ability = ps.ability(player);
    let keeper = |p: PlayerId| ps.positions(p).best() == fs_core::attributes::Position::Goalkeeper;
    let others = || {
        world.clubs()[club]
            .squad()
            .iter()
            .copied()
            .filter(move |&p| p != player)
    };
    let weakest = |s: SquadStatus| {
        others()
            .filter(|&p| ps.squad_status(p) == s && !keeper(p))
            .map(|p| ps.ability(p))
            .min()
    };
    if keeper(player) {
        let best_other = others().filter(|&p| keeper(p)).map(|p| ps.ability(p)).max();
        return if best_other.is_some_and(|b| b >= ability) {
            SquadStatus::Backup
        } else if weakest(SquadStatus::Key).is_some_and(|m| ability >= m) {
            SquadStatus::Key
        } else {
            SquadStatus::Regular
        };
    }
    [
        SquadStatus::Key,
        SquadStatus::Regular,
        SquadStatus::Rotation,
    ]
    .into_iter()
    .find(|&s| weakest(s).is_some_and(|m| ability >= m))
    .unwrap_or(if age < 21.0 {
        SquadStatus::Prospect
    } else {
        SquadStatus::Backup
    })
}

/// A player joined a club (signed or on loan): a fresh start, with the
/// status their ability earns them there.
pub(crate) fn joined(world: &mut World, player: PlayerId, club: ClubId) {
    let status = newcomer_status(world, club, player);
    let i = player.index();
    world.players.squad_status[i] = status;
    world.players.playing_time[i] = status.expected();
    world.players.happiness[i] = 0;
    world.players.unsettled[i] = 0;
}

/// A player left a club: memories, promises and requests go with them.
pub(crate) fn left(world: &mut World, player: PlayerId, club: ClubId) {
    let r = &mut world.relations;
    r.memories.remove(&player);
    r.requests.remove(&player);
    r.talks.remove(&player);
    r.promises.retain(|p| p.player != player);
    if world.clubs[club].captain == Some(player) {
        world.clubs[club].captain = None;
    }
}

/// Adds a memory, replacing an older one of the same feeling.
pub(crate) fn remember(
    world: &mut World,
    player: PlayerId,
    feeling: Feeling,
    amount: i8,
    days: u32,
) {
    let until = world.date().add_days(days);
    let list = world.relations.memories.entry(player).or_default();
    list.retain(|m| m.feeling != feeling);
    list.push(Memory {
        feeling,
        amount,
        until,
    });
}

pub(crate) fn forget(world: &mut World, player: PlayerId, feeling: Feeling) {
    if let Some(list) = world.relations.memories.get_mut(&player) {
        list.retain(|m| m.feeling != feeling);
    }
}

/// Recomputes one player's stored happiness (after a talk or decision).
pub(crate) fn refresh(world: &mut World, player: PlayerId) {
    if let Some(a) = assess(world, player) {
        world.players.happiness[player.index()] = a.happiness;
    }
}

/// After a match: each squad member's share of the minutes. Injured and
/// suspended players are not counted, so absence is not held against them.
pub(crate) fn record_minutes(world: &mut World, club: ClubId, minutes: &[(PlayerId, u8)]) {
    let today = world.date;
    let World { clubs, players, .. } = world;
    for &p in &clubs[club].squad {
        let played = minutes.iter().find(|m| m.0 == p).map_or(0, |m| m.1);
        if played == 0 && !players.available(p, today) {
            continue;
        }
        let share = i32::from(played.min(90)) * 10_000 / 90;
        let pt = &mut players.playing_time[p.index()];
        *pt = (i32::from(*pt) + (share - i32::from(*pt)) / 6).clamp(0, 10_000) as u16;
    }
}

/// Daily: promises fall due; on the 1st, the monthly review.
pub(crate) fn daily(world: &mut World, events: &mut Vec<Event>) {
    crate::talk::settle(world, events);
    if world.date().ymd().2 == 1 {
        monthly(world, events);
    }
}

fn monthly(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    for list in world.relations.memories.values_mut() {
        list.retain(|m| m.until > today);
    }
    world.relations.memories.retain(|_, l| !l.is_empty());
    crate::contract::tidy(world);

    // An agreed sale that the club went back on.
    let reneged: Vec<PlayerId> = world
        .relations
        .requests
        .iter()
        .filter(|&(&p, r)| r.accepted && world.market().listing(p) != Some(Listing::Transfer))
        .map(|(&p, _)| p)
        .collect();
    for p in reneged {
        world.relations.requests.remove(&p);
        forget(world, p, Feeling::RequestAccepted);
        remember(world, p, Feeling::RequestRefused, -25, 180);
    }

    // Captains for clubs without one.
    let clubs: Vec<ClubId> = world.clubs().ids().collect();
    for &c in &clubs {
        if world.clubs[c].captain.is_none() {
            world.clubs[c].captain = natural_captain(world, c);
        }
    }

    // Everyone's happiness, club by club.
    let mut moods: Vec<Vec<(PlayerId, i8)>> = vec![Vec::new(); clubs.len()];
    crate::profile::time("people: review", || {
        let w = &*world;
        par::for_each_indexed_mut(&mut moods, |k, out| {
            let club = ClubId::new(k as u32);
            let ctx = context(w, club);
            let leaders = Leaders::of(w, &ctx, club);
            let mut buf = Vec::with_capacity(8);
            for &p in w.clubs()[club].squad() {
                buf.clear();
                personal(w, &ctx, club, p, &mut buf);
                let mut v = total(&buf);
                if let Some(f) = leaders.effect_on(p) {
                    v += i32::from(f.effect);
                }
                out.push((p, clamp(v)));
            }
        });
    });

    for (k, list) in moods.into_iter().enumerate() {
        let club = ClubId::new(k as u32);
        let human = world.is_human_club(club);
        for (p, h) in list {
            let i = p.index();
            let before = world.players.happiness[i];
            world.players.happiness[i] = h;
            if h <= UNHAPPY {
                world.players.unsettled[i] = world.players.unsettled[i].saturating_add(1);
                if human && before > UNHAPPY {
                    events.push(Event::PlayerUnhappy { player: p, club });
                }
            } else if h > UNHAPPY / 2 {
                world.players.unsettled[i] = 0;
            }
            maybe_request(world, events, club, p);
        }
    }
}

/// A player unsettled for three months may ask to leave.
fn maybe_request(world: &mut World, events: &mut Vec<Event>, club: ClubId, player: PlayerId) {
    let i = player.index();
    if world.players.unsettled[i] < 3
        || world.relations.requests.contains_key(&player)
        || world.relations.remembers(player, Feeling::RequestRefused)
        || age_years(world.players.bio[i].birth_date, world.date()) < 18.0
        || world.market().loan(player).is_some()
    {
        return;
    }
    let loyalty = trait_of(world, player, HiddenAttr::Loyalty);
    let professionalism = trait_of(world, player, HiddenAttr::Professionalism);
    let chance = 0.3 * (1.3 - loyalty) * (1.2 - 0.4 * professionalism);
    let mut rng = world.seed().derive(
        Stream::People,
        u64::from(world.date().days()),
        u64::from(player.raw()),
    );
    if !rng.chance(chance) {
        return;
    }
    world.relations.requests_made += 1;
    crate::news::report(
        world,
        crate::news::StoryKind::TransferRequest { player, club },
    );
    world.relations.requests.insert(
        player,
        Request {
            club,
            date: world.date(),
            accepted: false,
        },
    );
    if world.ai_runs_squad(club) && !world.is_human_club(club) {
        answer_request(world, player, true);
    } else {
        events.push(Event::TransferRequested { player, club });
    }
}

/// The club's answer to a transfer request: list the player, or refuse.
pub(crate) fn answer_request(world: &mut World, player: PlayerId, accept: bool) {
    if accept {
        if let Some(r) = world.relations.requests.get_mut(&player) {
            r.accepted = true;
        }
        world.market.listed.insert(player, Listing::Transfer);
        remember(world, player, Feeling::RequestAccepted, 12, 365);
    } else {
        world.relations.requests.remove(&player);
        world.players.unsettled[player.index()] = 0;
        let ambition = trait_of(world, player, HiddenAttr::Ambition);
        remember(
            world,
            player,
            Feeling::RequestRefused,
            -(15.0 + 15.0 * ambition) as i8,
            180,
        );
    }
    refresh(world, player);
}

/// The world's mood, for health checks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mood {
    /// Mean happiness of senior players at clubs.
    pub mean: f64,
    /// Share of them at or below [`UNHAPPY`].
    pub unhappy: f64,
    /// Share of them at 20 or above.
    pub happy: f64,
}

impl Mood {
    pub fn of(world: &World) -> Self {
        let ps = world.players();
        let today = world.date();
        let (mut n, mut sum, mut low, mut high) = (0usize, 0i64, 0usize, 0usize);
        for (_, c) in world.clubs().iter() {
            for &p in c.squad() {
                if age_years(ps.bio(p).birth_date, today) < 18.0 {
                    continue;
                }
                let h = ps.happiness(p);
                n += 1;
                sum += i64::from(h);
                low += usize::from(h <= UNHAPPY);
                high += usize::from(h >= 20);
            }
        }
        let n = n.max(1) as f64;
        Mood {
            mean: sum as f64 / n,
            unhappy: low as f64 / n,
            happy: high as f64 / n,
        }
    }
}

/// The summer: AI clubs reset statuses to their new pecking order, and
/// every club without a captain names one.
pub(crate) fn rollover(world: &mut World) {
    let clubs: Vec<ClubId> = world.clubs().ids().collect();
    for club in clubs {
        if !world.is_human_club(club) {
            for (p, s) in natural_statuses(world, club) {
                set_status(world, p, s);
            }
        }
        if world.clubs[club].captain.is_none() {
            world.clubs[club].captain = natural_captain(world, club);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_is_neutral_at_zero() {
        assert_eq!(morale_baseline(0), CONDITION_MAX / 2);
        assert!(morale_baseline(-100) < morale_baseline(100));
    }

    #[test]
    fn statuses_expect_less_down_the_order() {
        let e: Vec<u16> = SquadStatus::ALL.iter().map(|s| s.expected()).collect();
        assert!(e.windows(2).all(|w| w[0] > w[1]));
    }
}
