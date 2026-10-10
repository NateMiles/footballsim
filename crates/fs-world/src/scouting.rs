//! Scouting: what each club knows about players it does not employ.
//!
//! Attributes are hidden true values. A club sees an *estimate*: the truth
//! plus an error that is fixed for that club and player (so the numbers do
//! not flicker) and shrinks as the club's knowledge grows. Knowledge comes
//! from proximity (same league, same nation), from the strength of the
//! scouting department, and from scouting the player or playing against
//! them. Two clubs can genuinely disagree about a player.

use crate::attributes::{ATTR_COUNT, Attr};
use crate::command::Event;
use crate::ids::{ClubId, PlayerId};
use crate::staff::{self, StaffRatings, StaffRole};
use crate::world::World;
use fs_core::{Rng, Stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Full knowledge.
pub const FULL: u8 = 100;
/// A scouting assignment ends when knowledge reaches this.
pub const REPORT_AT: u8 = 95;
/// Most players one club can have under assignment.
pub const MAX_ASSIGNMENTS: usize = 40;
/// Knowledge gained about every opposing player in a match.
pub const MATCH_KNOWLEDGE: u8 = 15;

/// Scouting state for the whole world.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scouting {
    /// Knowledge gained by scouting or playing against, 0-100. Only
    /// kept where it beats what a club knows anyway.
    known: BTreeMap<(ClubId, PlayerId), u8>,
    /// Players each club's scouts are watching, oldest first.
    assignments: BTreeMap<ClubId, Vec<PlayerId>>,
}

impl Scouting {
    /// Players `club` is scouting.
    pub fn assignments(&self, club: ClubId) -> &[PlayerId] {
        self.assignments.get(&club).map_or(&[], Vec::as_slice)
    }

    /// Knowledge gained directly, if any.
    pub fn gained(&self, club: ClubId, player: PlayerId) -> u8 {
        self.known.get(&(club, player)).copied().unwrap_or(0)
    }

    fn learn(&mut self, club: ClubId, player: PlayerId, amount: u8) {
        let k = self.known.entry((club, player)).or_insert(0);
        *k = k.saturating_add(amount).min(FULL);
    }

    pub(crate) fn assign(&mut self, club: ClubId, player: PlayerId) {
        let list = self.assignments.entry(club).or_default();
        if !list.contains(&player) {
            list.push(player);
        }
    }

    pub(crate) fn cancel(&mut self, club: ClubId, player: PlayerId) {
        if let Some(list) = self.assignments.get_mut(&club) {
            list.retain(|&p| p != player);
        }
    }
}

/// What a club knows without scouting a player: more about players
/// nearby, and more with a stronger scouting department.
fn background(world: &World, club: ClubId, player: PlayerId, ratings: &StaffRatings) -> u8 {
    let c = &world.clubs()[club];
    let theirs = world.players().club(player);
    let near = match theirs {
        Some(t)
            if world.clubs()[t].league().is_some() && world.clubs()[t].league() == c.league() =>
        {
            40.0
        }
        Some(t) if world.clubs()[t].nation == c.nation => 22.0,
        None if world.players().bio(player).nation == c.nation => 18.0,
        _ => 8.0,
    };
    let network = ((ratings.scouting_ability - 4.0) * 1.5).max(0.0);
    (near + network).min(90.0) as u8
}

/// How much `club` knows about `player`, 0-100.
pub fn knowledge(world: &World, club: ClubId, player: PlayerId) -> u8 {
    knowledge_with(world, club, player, &staff::ratings(world, club))
}

/// [`knowledge`] with the club's staff ratings already worked out.
pub fn knowledge_with(world: &World, club: ClubId, player: PlayerId, ratings: &StaffRatings) -> u8 {
    if world.players().club(player) == Some(club) {
        return FULL;
    }
    background(world, club, player, ratings).max(world.scouting().gained(club, player))
}

/// An estimated value with its plausible range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub value: u16,
    pub low: u16,
    pub high: u16,
}

/// What a club believes about a player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Estimate {
    /// 0-100.
    pub knowledge: u8,
    /// Each attribute on the 1-20 scale, in [`Attr::ALL`] order.
    pub attributes: Vec<Range>,
    /// Current ability, 0-1000.
    pub ability: Range,
    /// Potential ability, 0-1000.
    pub potential: Range,
}

/// The fixed errors a club makes about a player, each about N(0, 1):
/// ability, then potential, then one per attribute. Only the first `N`
/// are drawn, so callers needing just the ability pay for one draw.
fn errors<const N: usize>(world: &World, club: ClubId, player: PlayerId) -> [f64; N] {
    let mut rng: Rng = world.seed().derive(
        Stream::Scouting,
        u64::from(club.raw()),
        u64::from(player.raw()),
    );
    std::array::from_fn(|_| rng.normal(0.0, 1.0).clamp(-2.5, 2.5))
}

fn range(truth: f64, error: f64, spread: f64, lo: f64, hi: f64) -> Range {
    let value = (truth + error * spread * 0.7).clamp(lo, hi);
    Range {
        value: value.round() as u16,
        low: (value - spread).clamp(lo, hi).round() as u16,
        high: (value + spread).clamp(lo, hi).round() as u16,
    }
}

/// What `club` believes about `player`.
pub fn estimate(world: &World, club: ClubId, player: PlayerId) -> Estimate {
    let ratings = staff::ratings(world, club);
    let k = knowledge_with(world, club, player, &ratings);
    let unknown = f64::from(FULL - k) / 100.0;
    // Better judges are less wrong: from 1.4x for poor scouts to 0.6x for
    // the best.
    let judge = staff::effect(ratings.scouting_ability, -0.4);
    let judge_potential = staff::effect(ratings.scouting_potential, -0.4);
    let e: [f64; ATTR_COUNT + 2] = errors(world, club, player);
    let ps = world.players();
    let attrs = ps.attributes(player);
    let attributes = Attr::ALL
        .iter()
        .enumerate()
        .map(|(i, &a)| {
            range(
                f64::from(attrs.display(a)),
                e[2 + i],
                unknown * 5.0 * judge,
                1.0,
                20.0,
            )
        })
        .collect();
    let ability = range(
        f64::from(ps.ability(player)),
        e[0],
        unknown * 150.0 * judge,
        0.0,
        1000.0,
    );
    let potential = range(
        f64::from(ps.potential(player).max(ps.ability(player))),
        e[1],
        unknown * 250.0 * judge_potential,
        0.0,
        1000.0,
    );
    Estimate {
        knowledge: k,
        attributes,
        ability,
        potential,
    }
}

/// The ability (0-1000) `club` believes `player` has: what AI clubs act
/// on when buying.
pub fn perceived_ability(
    world: &World,
    club: ClubId,
    player: PlayerId,
    ratings: &StaffRatings,
) -> f64 {
    let k = knowledge_with(world, club, player, ratings);
    let unknown = f64::from(FULL - k) / 100.0;
    let judge = staff::effect(ratings.scouting_ability, -0.4);
    let [e] = errors::<1>(world, club, player);
    f64::from(world.players().ability(player)) + e * unknown * 105.0 * judge
}

/// Daily: scouts work through their club's assignments. Each scout watches
/// one player at a time; better scouts learn faster. Human clubs get a
/// report when a player is fully assessed.
pub(crate) fn daily(world: &mut World, events: &mut Vec<Event>) {
    let clubs: Vec<ClubId> = world.scouting.assignments.keys().copied().collect();
    for club in clubs {
        let mut scouts: Vec<u8> = world.clubs[club]
            .staff()
            .iter()
            .map(|&s| &world.staff()[s])
            .filter(|s| s.role == StaffRole::Scout)
            .map(|s| s.attrs.judging_ability.max(s.attrs.judging_potential))
            .collect();
        scouts.sort_unstable_by(|a, b| b.cmp(a));
        let watching: Vec<PlayerId> = world
            .scouting
            .assignments(club)
            .iter()
            .copied()
            .take(scouts.len())
            .collect();
        let ratings = staff::ratings(world, club);
        for (&player, &skill) in watching.iter().zip(&scouts) {
            let before = knowledge_with(world, club, player, &ratings);
            let gain = 2 + skill / 3;
            let gained = world.scouting.gained(club, player).max(before);
            let target = gained.saturating_add(gain).min(FULL);
            world
                .scouting
                .learn(club, player, target - world.scouting.gained(club, player));
            if target >= REPORT_AT {
                world.scouting.cancel(club, player);
                if world.is_human_club(club) {
                    events.push(Event::ScoutReport { club, player });
                }
            }
        }
    }
    world.scouting.assignments.retain(|_, v| !v.is_empty());
}

/// After a match: a human-managed club learns about the players it faced.
pub(crate) fn after_match(world: &mut World, club: ClubId, opponents: &[PlayerId]) {
    if !world.is_human_club(club) {
        return;
    }
    for &p in opponents {
        world.scouting.learn(club, p, MATCH_KNOWLEDGE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_hold_the_estimate_and_stay_on_scale() {
        let r = range(18.0, 2.0, 5.0, 1.0, 20.0);
        assert!(r.low <= r.value && r.value <= r.high);
        assert_eq!(r.high, 20);
        let exact = range(12.0, 2.0, 0.0, 1.0, 20.0);
        assert_eq!((exact.low, exact.value, exact.high), (12, 12, 12));
    }
}
