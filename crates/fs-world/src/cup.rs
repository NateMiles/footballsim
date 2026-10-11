//! Cups: knockout rounds and group stages, domestic and continental.
//!
//! A cup's [`CupFormat`] says who enters which stage and how each stage is
//! played. Each season the cup gets a new [`CupEdition`]: the entrants
//! are decided when the season starts (continental places from last
//! season's tables and cups, ranked by nation coefficients), the first
//! stage is drawn at once, and each later stage is drawn the day its
//! previous stage finishes. Draws avoid clashes with a club's other
//! fixtures by moving the cup match a day later.

use crate::command::Event;
use crate::format::{CupFormat, Source, StageKind};
use crate::ids::{ClubId, CupId, NationId};
use crate::league::{Fixture, FixtureResult, TableRow, sort_table};
use crate::world::World;
use fs_core::{GameDate, Stream};
use fs_match::Decider;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Who a cup is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CupScope {
    /// One nation's clubs.
    Nation(NationId),
    /// Clubs from the nations of a continent (index into
    /// [`World::continents`]).
    Continent(u16),
}

/// A cup competition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cup {
    pub format: CupFormat,
    pub scope: CupScope,
    pub(crate) edition: Option<CupEdition>,
    pub(crate) history: Vec<CupSummary>,
}

impl Cup {
    pub fn new(format: CupFormat, scope: CupScope) -> Self {
        Self {
            format,
            scope,
            edition: None,
            history: Vec::new(),
        }
    }

    /// The current season's edition.
    pub fn edition(&self) -> Option<&CupEdition> {
        self.edition.as_ref()
    }

    /// Finished editions, oldest first.
    pub fn history(&self) -> &[CupSummary] {
        &self.history
    }

    /// Whether this is a continental competition.
    pub fn is_continental(&self) -> bool {
        matches!(self.scope, CupScope::Continent(_))
    }
}

/// One season of a cup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CupEdition {
    pub year: i32,
    /// Clubs entering each stage directly.
    pub entrants: Vec<Vec<ClubId>>,
    /// Stages drawn so far.
    pub stages: Vec<StageDraw>,
    /// Every match so far, in date order.
    pub fixtures: Vec<CupFixture>,
    /// Index of the first fixture not yet played.
    pub(crate) next: usize,
    /// Clubs through to the next draw.
    pub(crate) through: Vec<Qualifier>,
    pub winner: Option<ClubId>,
    /// No more matches: won, or abandoned for lack of entrants.
    pub finished: bool,
}

impl CupEdition {
    /// Whether the club entered this edition.
    pub fn involves(&self, club: ClubId) -> bool {
        self.entrants.iter().any(|s| s.contains(&club))
    }

    /// Whether the club is still in it: entering later, through to the
    /// next draw, or still playing in the latest stage.
    pub fn alive(&self, club: ClubId) -> bool {
        if self.finished {
            return self.winner == Some(club);
        }
        self.entrants
            .iter()
            .skip(self.stages.len())
            .any(|s| s.contains(&club))
            || self.through.iter().any(|q| q.club == club)
            || self
                .stages
                .last()
                .is_some_and(|s| s.has(club) && !s.knocked_out(club))
    }
}

/// A club through to the next draw, with what seeds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Qualifier {
    pub club: ClubId,
    /// Group finish (0 for winners); 0 after a knockout round.
    pub position: u8,
    /// The group they came from, to keep group rivals apart.
    pub group: Option<u16>,
}

/// A cup match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CupFixture {
    pub fixture: Fixture,
    /// Index into [`CupEdition::stages`].
    pub stage: u8,
    /// The tie or group within the stage.
    pub tie: u16,
    /// 0 for the first leg (or only match), 1 for the second.
    pub leg: u8,
}

/// A drawn stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageDraw {
    Knockout {
        /// Index into the format's stages (the last is repeated if the
        /// field needs more rounds than the format has).
        format: u8,
        ties: Vec<Tie>,
        /// Clubs through without playing.
        byes: Vec<ClubId>,
    },
    Groups {
        format: u8,
        groups: Vec<Group>,
    },
}

impl StageDraw {
    /// Index into the format's stages.
    pub fn format_index(&self) -> usize {
        match self {
            StageDraw::Knockout { format, .. } | StageDraw::Groups { format, .. } => {
                usize::from(*format)
            }
        }
    }

    /// Whether the club plays in (or has a bye through) this stage.
    pub fn has(&self, club: ClubId) -> bool {
        match self {
            StageDraw::Knockout { ties, byes, .. } => {
                byes.contains(&club) || ties.iter().any(|t| t.clubs.contains(&club))
            }
            StageDraw::Groups { groups, .. } => {
                groups.iter().any(|g| g.rows.iter().any(|r| r.club == club))
            }
        }
    }

    /// Every club in the stage.
    pub fn clubs(&self) -> Vec<ClubId> {
        match self {
            StageDraw::Knockout { ties, byes, .. } => ties
                .iter()
                .flat_map(|t| t.clubs)
                .chain(byes.iter().copied())
                .collect(),
            StageDraw::Groups { groups, .. } => groups
                .iter()
                .flat_map(|g| g.rows.iter().map(|r| r.club))
                .collect(),
        }
    }

    /// Whether the club lost a tie in this stage. Groups are settled all
    /// at once, when the next stage is drawn.
    fn knocked_out(&self, club: ClubId) -> bool {
        match self {
            StageDraw::Knockout { ties, .. } => ties
                .iter()
                .any(|t| t.clubs.contains(&club) && t.winner.is_some_and(|w| w != club)),
            StageDraw::Groups { .. } => false,
        }
    }
}

/// A knockout tie over one or two legs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tie {
    /// Home side of the first leg, then the other.
    pub clubs: [ClubId; 2],
    /// Fixture indexes, by leg.
    pub legs: Vec<u32>,
    pub winner: Option<ClubId>,
}

/// A group in a group stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    /// One row per club, in draw order (unsorted).
    pub rows: Vec<TableRow>,
}

impl Group {
    /// The standings.
    pub fn table(&self) -> Vec<TableRow> {
        let mut t = self.rows.clone();
        sort_table(&mut t);
        t
    }
}

/// What a cup remembers about a finished edition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CupSummary {
    pub year: i32,
    pub winner: Option<ClubId>,
    pub runner_up: Option<ClubId>,
}

/// Nations whose clubs meet in continental cups, with their coefficients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continent {
    pub name: String,
    pub nations: Vec<NationId>,
    /// Coefficient points (thousandths) per member nation, one entry per
    /// season, oldest first; the last [`COEFFICIENT_SEASONS`] are kept.
    pub(crate) seasons: Vec<Vec<u32>>,
}

/// Seasons a coefficient counts.
pub const COEFFICIENT_SEASONS: usize = 5;

impl Continent {
    /// A continent whose coefficients start from the nations'
    /// reputations, so the first ranking follows them.
    pub fn new(name: String, nations: Vec<NationId>, reputations: &[u16]) -> Self {
        let start: Vec<u32> = reputations.iter().map(|&r| u32::from(r)).collect();
        Self {
            name,
            nations,
            seasons: vec![start; COEFFICIENT_SEASONS],
        }
    }

    /// A member nation's coefficient (thousandths), the sum of its recent
    /// seasons.
    pub fn coefficient(&self, nation: NationId) -> u32 {
        let Some(k) = self.nations.iter().position(|&n| n == nation) else {
            return 0;
        };
        self.seasons
            .iter()
            .map(|s| s.get(k).copied().unwrap_or(0))
            .sum()
    }

    /// Member nations, best coefficient first.
    pub fn ranking(&self) -> Vec<NationId> {
        let mut n = self.nations.clone();
        n.sort_by_key(|&x| (std::cmp::Reverse(self.coefficient(x)), x));
        n
    }

    /// Coefficient points per season (thousandths), oldest first.
    pub fn seasons(&self) -> &[Vec<u32>] {
        &self.seasons
    }
}

/// Clubs too strong for an open draw: ordered by finish, then
/// reputation.
fn seed_order(world: &World, field: &mut [Qualifier]) {
    field.sort_by_key(|q| {
        (
            q.position,
            std::cmp::Reverse(world.clubs[q.club].reputation),
            q.club,
        )
    });
}

/// Clubs a knockout round sends through by default: half the field, or
/// down to a power of two first.
fn default_winners(n: usize) -> usize {
    if n <= 1 {
        n
    } else if n.is_power_of_two() {
        n / 2
    } else {
        1 << (usize::BITS - 1 - n.leading_zeros())
    }
}

/// Starts every cup's edition for the season starting in `year`. Leagues
/// must already have their fixtures, so draws can avoid them.
pub(crate) fn start_editions(world: &mut World, year: i32, events: &mut Vec<Event>) {
    // Continental places: a club plays in at most one competition, the
    // most prestigious it qualifies for.
    let mut taken: BTreeSet<ClubId> = BTreeSet::new();
    let ids: Vec<CupId> = world.cups.ids().collect();
    let entrants: Vec<Vec<Vec<ClubId>>> = ids
        .iter()
        .map(|&c| qualify(world, c, year, &mut taken))
        .collect();
    for (&cup, entrants) in ids.iter().zip(entrants) {
        world.cups[cup].edition = Some(CupEdition {
            year,
            entrants,
            stages: Vec::new(),
            fixtures: Vec::new(),
            next: 0,
            through: Vec::new(),
            winner: None,
            finished: false,
        });
        draw_stage(world, cup, 0, events);
        progress(world, cup, events);
    }
}

/// Who enters each stage of a cup this season.
fn qualify(world: &World, cup: CupId, year: i32, taken: &mut BTreeSet<ClubId>) -> Vec<Vec<ClubId>> {
    let c = &world.cups[cup];
    let mut stages = vec![Vec::new(); c.format.stages.len()];
    let mut entered: BTreeSet<ClubId> = BTreeSet::new();
    let ranking = match c.scope {
        CupScope::Continent(k) => world
            .continents
            .get(usize::from(k))
            .map(Continent::ranking)
            .unwrap_or_default(),
        CupScope::Nation(_) => Vec::new(),
    };
    let ranked = |r: [u16; 2]| -> Vec<NationId> {
        let lo = usize::from(r[0]).saturating_sub(1);
        let hi = usize::from(r[1]).min(ranking.len());
        ranking
            .get(lo..hi)
            .map(<[NationId]>::to_vec)
            .unwrap_or_default()
    };
    for e in &c.format.entries {
        let mut clubs: Vec<ClubId> = Vec::new();
        match (&e.source, c.scope) {
            (Source::Levels { levels }, CupScope::Nation(n)) => {
                for (_, l) in world.leagues.iter() {
                    if l.nation == n && (levels[0]..=levels[1]).contains(&l.level) {
                        clubs.extend(l.clubs.iter().copied());
                    }
                }
            }
            (Source::LeaguePlaces { ranks, places }, CupScope::Continent(_)) => {
                for nation in ranked(*ranks) {
                    let order = final_order(world, nation);
                    let want = usize::from(places[1] - places[0]) + 1;
                    clubs.extend(
                        order
                            .iter()
                            .skip(usize::from(places[0]) - 1)
                            .filter(|c| !taken.contains(c) && !entered.contains(c))
                            .take(want),
                    );
                }
            }
            (Source::CupWinners { ranks }, CupScope::Continent(_)) => {
                for nation in ranked(*ranks) {
                    let winner = main_cup_winner(world, nation, year - 1)
                        .filter(|c| !taken.contains(c) && !entered.contains(c));
                    // A winner already in Europe hands the place to the
                    // best-placed league club not yet in.
                    let pick = winner.or_else(|| {
                        final_order(world, nation)
                            .into_iter()
                            .find(|c| !taken.contains(c) && !entered.contains(c))
                    });
                    clubs.extend(pick);
                }
            }
            _ => {}
        }
        for club in clubs {
            if entered.insert(club) {
                stages[usize::from(e.stage)].push(club);
            }
        }
    }
    if c.is_continental() {
        taken.extend(entered);
    }
    stages
}

/// A nation's top division in finishing order last season, or by
/// reputation before any season has finished.
fn final_order(world: &World, nation: NationId) -> Vec<ClubId> {
    let Some((_, top)) = world
        .leagues
        .iter()
        .filter(|(_, l)| l.nation == nation)
        .min_by_key(|(id, l)| (l.level, *id))
    else {
        return Vec::new();
    };
    match top.history.last() {
        Some(s) => s.table.iter().map(|r| r.club).collect(),
        None => {
            let mut clubs = top.clubs.clone();
            clubs.sort_by_key(|&c| (std::cmp::Reverse(world.clubs[c].reputation), c));
            clubs
        }
    }
}

/// The winner of a nation's main cup in the season starting in `year`.
fn main_cup_winner(world: &World, nation: NationId, year: i32) -> Option<ClubId> {
    let (_, cup) = world
        .cups
        .iter()
        .find(|(_, c)| c.scope == CupScope::Nation(nation))?;
    cup.history
        .iter()
        .rev()
        .find(|s| s.year == year)
        .and_then(|s| s.winner)
}

/// Match days for a stage: the format's dates, but never sooner than two
/// days from now, and at least three days apart.
fn stage_dates(
    world: &World,
    cup: CupId,
    format_stage: usize,
    year: i32,
    n: usize,
) -> Vec<GameDate> {
    let rules = &world.cups[cup].format.stages[format_stage].dates;
    let mut earliest = world.date.add_days(2);
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let d = match rules.get(k) {
            Some(r) => r.resolve(year),
            None => out.last().map_or(earliest, |&l: &GameDate| l.add_days(7)),
        };
        let d = d.max(earliest);
        out.push(d);
        earliest = d.add_days(3);
    }
    out
}

/// The first day from `date` on which neither club already plays, in
/// the world or among the matches being drawn. Books the day.
fn free_date(
    world: &World,
    booked: &mut BTreeSet<(ClubId, GameDate)>,
    clubs: [ClubId; 2],
    date: GameDate,
) -> GameDate {
    let mut d = date;
    for _ in 0..7 {
        if clubs
            .iter()
            .all(|&c| !booked.contains(&(c, d)) && crate::fixture::on(world, c, d).is_none())
        {
            break;
        }
        d = d.add_days(1);
    }
    for c in clubs {
        booked.insert((c, d));
    }
    d
}

/// Draws stage `k` of a cup's edition from the clubs through and the
/// stage's direct entrants.
fn draw_stage(world: &mut World, cup: CupId, k: usize, events: &mut Vec<Event>) {
    let format_len = world.cups[cup].format.stages.len();
    let format_stage = k.min(format_len - 1);
    let (year, mut field) = {
        let e = world.cups[cup].edition.as_mut().expect("edition");
        let mut field: Vec<Qualifier> = std::mem::take(&mut e.through);
        if let Some(direct) = e.entrants.get(k).filter(|_| k < format_len) {
            field.extend(direct.iter().map(|&club| Qualifier {
                club,
                position: 0,
                group: None,
            }));
        }
        (e.year, field)
    };
    seed_order(world, &mut field);
    let stage = world.cups[cup].format.stages[format_stage].clone();
    let mut rng = world.seed.derive(
        Stream::Cup,
        ((year as u64) << 20) | u64::from(cup.raw()),
        k as u64,
    );
    // Stage money for everyone taking part.
    if stage.prize > 0 {
        for q in &field {
            world.clubs[q.club]
                .finances
                .earn(|l| &mut l.prize_money, stage.prize);
        }
    }
    let base = world.cups[cup]
        .edition
        .as_ref()
        .expect("edition")
        .fixtures
        .len();
    let mut booked: BTreeSet<(ClubId, GameDate)> = BTreeSet::new();
    // (date, tie, leg, home, away)
    let mut matches: Vec<(GameDate, u16, u8, ClubId, ClubId)> = Vec::new();
    let draw = match stage.kind {
        StageKind::Knockout {
            legs,
            winners,
            seeded,
            ..
        } => {
            let n = field.len();
            let last = k + 1 >= format_len;
            let target = if last {
                1
            } else {
                winners.map_or_else(|| default_winners(n), usize::from)
            };
            let ties = n.saturating_sub(target.max(1)).min(n / 2);
            let bye_count = n - 2 * ties;
            let byes: Vec<ClubId> = field[..bye_count].iter().map(|q| q.club).collect();
            let rest = &field[bye_count..];
            let pairs = if seeded {
                seeded_pairs(world, rest, &mut rng)
            } else {
                let mut r: Vec<ClubId> = rest.iter().map(|q| q.club).collect();
                rng.shuffle(&mut r);
                r.chunks_exact(2).map(|p| [p[0], p[1]]).collect()
            };
            let dates = stage_dates(world, cup, format_stage, year, usize::from(legs));
            let mut tie_list = Vec::with_capacity(pairs.len());
            for (t, clubs) in pairs.into_iter().enumerate() {
                for leg in 0..legs {
                    let (home, away) = if leg % 2 == 0 {
                        (clubs[0], clubs[1])
                    } else {
                        (clubs[1], clubs[0])
                    };
                    let date = free_date(world, &mut booked, clubs, dates[usize::from(leg)]);
                    matches.push((date, t as u16, leg, home, away));
                }
                tie_list.push(Tie {
                    clubs,
                    legs: Vec::new(),
                    winner: None,
                });
            }
            StageDraw::Knockout {
                format: format_stage as u8,
                ties: tie_list,
                byes,
            }
        }
        StageKind::Groups {
            groups, meetings, ..
        } => {
            let g = usize::from(groups.max(1));
            let mut members: Vec<Vec<ClubId>> = vec![Vec::new(); g];
            for (p, pot) in field.chunks(g).enumerate() {
                let mut pot: Vec<ClubId> = pot.iter().map(|q| q.club).collect();
                rng.shuffle(&mut pot);
                for club in pot {
                    let nation = world.clubs[club].nation;
                    let open = |m: &Vec<ClubId>| m.len() <= p;
                    let pick = (0..g)
                        .find(|&i| {
                            open(&members[i])
                                && members[i].iter().all(|&o| world.clubs[o].nation != nation)
                        })
                        .or_else(|| (0..g).find(|&i| open(&members[i])))
                        .unwrap_or(0);
                    members[pick].push(club);
                }
            }
            let largest = members.iter().map(Vec::len).max().unwrap_or(0);
            let per_meeting = crate::competition::double_round_robin(largest).len() / 2;
            let rounds = per_meeting * usize::from(meetings);
            let dates = stage_dates(world, cup, format_stage, year, rounds);
            for (gi, m) in members.iter().enumerate() {
                let schedule = crate::competition::double_round_robin(m.len());
                let half = schedule.len() / 2;
                for meeting in 0..usize::from(meetings) {
                    for (r, games) in schedule.iter().enumerate() {
                        // Odd meetings play the first half's venues, even
                        // ones the second half's.
                        let first_half = r < half;
                        if (meeting % 2 == 0) != first_half {
                            continue;
                        }
                        let round = meeting * half + (r % half.max(1));
                        for &(h, a) in games {
                            let clubs = [m[h], m[a]];
                            let day = dates[round.min(dates.len() - 1)];
                            let date = free_date(world, &mut booked, clubs, day);
                            matches.push((date, gi as u16, (round % 256) as u8, m[h], m[a]));
                        }
                    }
                }
            }
            StageDraw::Groups {
                format: format_stage as u8,
                groups: members
                    .into_iter()
                    .map(|m| Group {
                        rows: m.into_iter().map(TableRow::new).collect(),
                    })
                    .collect(),
            }
        }
    };
    matches.sort_by_key(|m| (m.0, m.1, m.2));
    let mut draw = draw;
    {
        let e = world.cups[cup].edition.as_mut().expect("edition");
        let is_knockout = matches!(draw, StageDraw::Knockout { .. });
        for (i, &(date, tie, leg, home, away)) in matches.iter().enumerate() {
            let index = (base + i) as u32;
            if let StageDraw::Knockout { ties, .. } = &mut draw {
                ties[usize::from(tie)].legs.push(index);
            }
            e.fixtures.push(CupFixture {
                fixture: Fixture {
                    date,
                    home,
                    away,
                    result: None,
                },
                stage: k as u8,
                tie,
                leg: if is_knockout { leg } else { 0 },
            });
        }
        // Legs in order, whatever dates they ended up on.
        if let StageDraw::Knockout { ties, .. } = &mut draw {
            for t in ties.iter_mut() {
                t.legs.sort_by_key(|&i| e.fixtures[i as usize].leg);
            }
        }
        e.stages.push(draw);
    }
    announce_draw(world, cup, k, events);
}

/// Pairs the strongest half of a field with the weakest half, keeping
/// clubs from the same group or nation apart where possible. The stronger
/// club is second in each pair (at home in the second leg).
fn seeded_pairs(world: &World, field: &[Qualifier], rng: &mut fs_core::Rng) -> Vec<[ClubId; 2]> {
    let half = field.len() / 2;
    let (top, bottom) = field.split_at(half);
    let mut bottom: Vec<Qualifier> = bottom[..half].to_vec();
    rng.shuffle(&mut bottom);
    let clash = |a: &Qualifier, b: &Qualifier| {
        (a.group.is_some() && a.group == b.group)
            || world.clubs[a.club].nation == world.clubs[b.club].nation
    };
    for i in 0..half {
        if clash(&top[i], &bottom[i])
            && let Some(j) =
                (i + 1..half).find(|&j| !clash(&top[i], &bottom[j]) && !clash(&top[j], &bottom[i]))
        {
            bottom.swap(i, j);
        }
    }
    top.iter()
        .zip(bottom)
        .map(|(t, b)| [b.club, t.club])
        .collect()
}

fn announce_draw(world: &World, cup: CupId, k: usize, events: &mut Vec<Event>) {
    let e = world.cups[cup].edition.as_ref().expect("edition");
    let Some(stage) = e.stages.get(k) else { return };
    for club in stage.clubs() {
        if !world.is_human_club(club) {
            continue;
        }
        let opponent = match stage {
            StageDraw::Knockout { ties, .. } => {
                ties.iter().find(|t| t.clubs.contains(&club)).map(|t| {
                    if t.clubs[0] == club {
                        t.clubs[1]
                    } else {
                        t.clubs[0]
                    }
                })
            }
            StageDraw::Groups { .. } => None,
        };
        events.push(Event::CupDrawn {
            cup,
            club,
            stage: k as u8,
            opponent,
        });
    }
}

/// The rules for a cup match: neutral ground, and how a level tie is
/// settled.
pub fn match_rules(world: &World, cup: CupId, index: usize) -> (bool, Option<Decider>) {
    let c = &world.cups[cup];
    let Some(e) = c.edition() else {
        return (false, None);
    };
    let f = &e.fixtures[index];
    let stage = &e.stages[usize::from(f.stage)];
    let kind = c.format.stages[stage.format_index()].kind;
    let StageKind::Knockout {
        legs,
        extra_time,
        neutral,
        ..
    } = kind
    else {
        return (false, None);
    };
    if f.leg + 1 < legs {
        return (false, None);
    }
    let carried = match stage {
        StageDraw::Knockout { ties, .. } => {
            let tie = &ties[usize::from(f.tie)];
            let mut carried = [0u8; 2];
            for &i in &tie.legs {
                let earlier = &e.fixtures[i as usize];
                if earlier.leg >= f.leg {
                    continue;
                }
                if let Some(r) = &earlier.fixture.result {
                    for (side, &club) in [f.fixture.home, f.fixture.away].iter().enumerate() {
                        let k = usize::from(earlier.fixture.home != club);
                        carried[side] = carried[side].saturating_add(r.goals[k]);
                    }
                }
            }
            carried
        }
        StageDraw::Groups { .. } => [0, 0],
    };
    (
        neutral && legs == 1,
        Some(Decider {
            carried,
            extra_time,
        }),
    )
}

/// Records a played cup match (and the group table, in a group stage).
pub(crate) fn record_result(world: &mut World, cup: CupId, index: usize, result: FixtureResult) {
    let e = world.cups[cup].edition.as_mut().expect("edition");
    let goals = result.goals;
    let f = &mut e.fixtures[index];
    f.fixture.result = Some(result);
    let (home, away, stage, tie) = (f.fixture.home, f.fixture.away, f.stage, f.tie);
    if let Some(StageDraw::Groups { groups, .. }) = e.stages.get_mut(usize::from(stage)) {
        let g = &mut groups[usize::from(tie)];
        for (side, club) in [home, away].into_iter().enumerate() {
            if let Some(row) = g.rows.iter_mut().find(|r| r.club == club) {
                row.record(goals[side], goals[1 - side]);
            }
        }
    }
    while e.next < e.fixtures.len() && e.fixtures[e.next].fixture.result.is_some() {
        e.next += 1;
    }
}

/// After the day's matches: settles finished stages and draws the next.
pub(crate) fn daily(world: &mut World, events: &mut Vec<Event>) {
    let ids: Vec<CupId> = world.cups.ids().collect();
    for cup in ids {
        progress(world, cup, events);
    }
}

/// Settles and draws stages for as long as the latest one is complete.
fn progress(world: &mut World, cup: CupId, events: &mut Vec<Event>) {
    // A guard against a format that never narrows the field.
    for _ in 0..64 {
        let Some(e) = world.cups[cup].edition.as_ref() else {
            return;
        };
        if e.finished || e.stages.is_empty() {
            return;
        }
        let k = e.stages.len() - 1;
        if e.fixtures
            .iter()
            .any(|f| usize::from(f.stage) == k && f.fixture.result.is_none())
        {
            return;
        }
        settle_stage(world, cup, k, events);
        let e = world.cups[cup].edition.as_ref().expect("edition");
        if e.finished {
            return;
        }
        draw_stage(world, cup, k + 1, events);
    }
}

/// Decides who goes through from a completed stage.
fn settle_stage(world: &mut World, cup: CupId, k: usize, events: &mut Vec<Event>) {
    let prestige = world.cups[cup].format.prestige;
    let format_len = world.cups[cup].format.stages.len();
    let kinds: Vec<StageKind> = world.cups[cup]
        .format
        .stages
        .iter()
        .map(|s| s.kind)
        .collect();
    let mut through: Vec<Qualifier> = Vec::new();
    let mut out: Vec<(ClubId, Option<ClubId>)> = Vec::new();
    let mut winners: Vec<ClubId> = Vec::new();
    {
        let e = world.cups[cup].edition.as_mut().expect("edition");
        let fixtures = &e.fixtures;
        match &mut e.stages[k] {
            StageDraw::Knockout { ties, byes, .. } => {
                through.extend(byes.iter().map(|&club| Qualifier {
                    club,
                    position: 0,
                    group: None,
                }));
                for t in ties.iter_mut() {
                    let winner = tie_winner(t, fixtures);
                    let loser = if winner == t.clubs[0] {
                        t.clubs[1]
                    } else {
                        t.clubs[0]
                    };
                    t.winner = Some(winner);
                    winners.push(winner);
                    through.push(Qualifier {
                        club: winner,
                        position: 0,
                        group: None,
                    });
                    out.push((loser, Some(winner)));
                }
            }
            StageDraw::Groups { format, groups } => {
                let advance = match kinds[usize::from(*format)] {
                    StageKind::Groups { advance, .. } => usize::from(advance),
                    StageKind::Knockout { .. } => 1,
                };
                for (gi, g) in groups.iter().enumerate() {
                    for (pos, row) in g.table().iter().enumerate() {
                        if pos < advance {
                            through.push(Qualifier {
                                club: row.club,
                                position: pos as u8,
                                group: Some(gi as u16),
                            });
                        } else {
                            out.push((row.club, None));
                        }
                    }
                }
            }
        }
    }
    for &(club, by) in &out {
        crate::board::cup_exit(world, club, by, prestige, events);
        if world.is_human_club(club) {
            events.push(Event::CupEliminated { cup, club, by });
        }
    }
    for &club in &winners {
        crate::board::cup_progress(world, club, prestige);
    }
    let last_stage = k + 1 >= format_len;
    let e = world.cups[cup].edition.as_mut().expect("edition");
    if through.len() <= 1 && (last_stage || through.is_empty()) {
        e.finished = true;
        e.winner = through.first().map(|q| q.club);
        let runner_up = out.first().map(|&(c, _)| c);
        let summary = CupSummary {
            year: e.year,
            winner: e.winner,
            runner_up: runner_up.filter(|_| e.winner.is_some()),
        };
        let year = e.year;
        world.cups[cup].history.push(summary);
        if let Some(w) = through.first().map(|q| q.club) {
            let prize = world.cups[cup].format.winner_prize;
            world.clubs[w].finances.earn(|l| &mut l.prize_money, prize);
            crate::board::trophy(world, w, prestige);
            events.push(Event::CupWon {
                cup,
                year,
                winner: w,
            });
        }
    } else {
        e.through = through;
    }
}

/// The club that won a completed tie: on aggregate, then on penalties.
fn tie_winner(t: &Tie, fixtures: &[CupFixture]) -> ClubId {
    let mut goals = [0u32; 2];
    let mut penalties = None;
    for &i in &t.legs {
        let f = &fixtures[i as usize];
        let Some(r) = &f.fixture.result else { continue };
        for (side, club) in [f.fixture.home, f.fixture.away].into_iter().enumerate() {
            let k = usize::from(club != t.clubs[0]);
            goals[k] += u32::from(r.goals[side]);
        }
        if let Some(p) = r.penalties {
            // Penalties from the home side's view of that match.
            penalties = Some(if f.fixture.home == t.clubs[0] {
                p
            } else {
                [p[1], p[0]]
            });
        }
    }
    let by_goals = goals[0].cmp(&goals[1]);
    let first_wins = match by_goals {
        core::cmp::Ordering::Greater => true,
        core::cmp::Ordering::Less => false,
        core::cmp::Ordering::Equal => penalties.is_none_or(|p| p[0] > p[1]),
    };
    if first_wins { t.clubs[0] } else { t.clubs[1] }
}

/// 1 July: archives every edition and updates continental coefficients.
pub(crate) fn close_editions(world: &mut World) {
    update_coefficients(world);
    let ids: Vec<CupId> = world.cups.ids().collect();
    for cup in ids {
        let Some(e) = world.cups[cup].edition.take() else {
            continue;
        };
        if !e.finished {
            world.cups[cup].history.push(CupSummary {
                year: e.year,
                winner: None,
                runner_up: None,
            });
        }
    }
}

/// A season's coefficient points for each continent's nations: two for a
/// win and one for a draw in continental matches, plus one for every
/// stage after the first a club plays in, averaged over the nation's
/// entrants.
fn update_coefficients(world: &mut World) {
    for k in 0..world.continents.len() {
        let nations = world.continents[k].nations.clone();
        let mut points = vec![0u64; nations.len()];
        let mut clubs: Vec<BTreeSet<ClubId>> = vec![BTreeSet::new(); nations.len()];
        let index_of = |club: ClubId, world: &World| {
            let n = world.clubs[club].nation;
            nations.iter().position(|&x| x == n)
        };
        for (_, cup) in world.cups.iter() {
            if cup.scope != CupScope::Continent(k as u16) {
                continue;
            }
            let Some(e) = cup.edition() else { continue };
            for f in &e.fixtures {
                let Some(r) = &f.fixture.result else { continue };
                for (side, club) in [f.fixture.home, f.fixture.away].into_iter().enumerate() {
                    let Some(n) = index_of(club, world) else {
                        continue;
                    };
                    clubs[n].insert(club);
                    points[n] += match r.goals[side].cmp(&r.goals[1 - side]) {
                        core::cmp::Ordering::Greater => 2000,
                        core::cmp::Ordering::Equal => 1000,
                        core::cmp::Ordering::Less => 0,
                    };
                }
            }
            for stage in e.stages.iter().skip(1) {
                for club in stage.clubs() {
                    if let Some(n) = index_of(club, world) {
                        points[n] += 1000;
                    }
                }
            }
        }
        let season: Vec<u32> = points
            .iter()
            .zip(&clubs)
            .map(|(&p, c)| {
                if c.is_empty() {
                    0
                } else {
                    (p / c.len() as u64) as u32
                }
            })
            .collect();
        let continent = &mut world.continents[k];
        continent.seasons.push(season);
        if continent.seasons.len() > COEFFICIENT_SEASONS {
            continent.seasons.remove(0);
        }
    }
}

/// Matches each club has played this season in cups, indexed by club.
pub fn matches_played(world: &World) -> Vec<u32> {
    let mut played = vec![0u32; world.clubs.len()];
    for (_, cup) in world.cups.iter() {
        let Some(e) = cup.edition() else { continue };
        for f in &e.fixtures[..e.next] {
            if f.fixture.result.is_some() {
                played[f.fixture.home.index()] += 1;
                played[f.fixture.away.index()] += 1;
            }
        }
    }
    played
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rounds_reach_a_power_of_two() {
        assert_eq!(default_winners(2), 1);
        assert_eq!(default_winners(32), 16);
        assert_eq!(default_winners(60), 32);
        assert_eq!(default_winners(33), 32);
        assert_eq!(default_winners(20), 16);
        assert_eq!(default_winners(1), 1);
        assert_eq!(default_winners(3), 2);
    }
}
