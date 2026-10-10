//! Backroom staff: coaches, scouts, physios and assistants.
//!
//! Staff matter through a club's *ratings*: the quality of its coaching
//! (player development), physiotherapy (injury recovery) and scouting
//! (how much it knows about other players). An average department rates
//! 10 and changes nothing; better staff help, worse staff hurt.

use crate::command::Event;
use crate::ids::{ClubId, NationId, StaffId};
use crate::names::NameRef;
use crate::player::Contract;
use crate::world::World;
use fs_core::{GameDate, Stream};
use serde::{Deserialize, Serialize};

/// What a staff member does.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StaffRole {
    AssistantManager,
    Coach,
    Scout,
    Physio,
}

impl StaffRole {
    pub const ALL: [StaffRole; 4] = [
        StaffRole::AssistantManager,
        StaffRole::Coach,
        StaffRole::Scout,
        StaffRole::Physio,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            StaffRole::AssistantManager => "Assistant manager",
            StaffRole::Coach => "Coach",
            StaffRole::Scout => "Scout",
            StaffRole::Physio => "Physio",
        }
    }

    /// How many a club wants, and the most it may employ.
    pub const fn wanted(self) -> (usize, usize) {
        match self {
            StaffRole::AssistantManager => (1, 1),
            StaffRole::Coach => (3, 6),
            StaffRole::Scout => (2, 6),
            StaffRole::Physio => (2, 4),
        }
    }
}

/// Staff abilities, each 1-20.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffAttrs {
    /// Improving senior players.
    pub coaching: u8,
    /// Improving players under 21.
    pub youth_coaching: u8,
    /// Judging a player's current ability.
    pub judging_ability: u8,
    /// Judging a player's potential.
    pub judging_potential: u8,
    /// Treating and preventing injuries.
    pub physiotherapy: u8,
    /// Getting the best out of a squad: man-management and tactics.
    pub management: u8,
}

impl StaffAttrs {
    /// The rating that matters most for a role.
    pub fn key(&self, role: StaffRole) -> u8 {
        match role {
            StaffRole::AssistantManager => self.management,
            StaffRole::Coach => (self.coaching + self.youth_coaching).div_ceil(2),
            StaffRole::Scout => (self.judging_ability + self.judging_potential).div_ceil(2),
            StaffRole::Physio => self.physiotherapy,
        }
    }
}

/// A staff member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Staff {
    pub first_name: NameRef,
    pub last_name: NameRef,
    pub birth_date: GameDate,
    pub nation: NationId,
    pub role: StaffRole,
    pub attrs: StaffAttrs,
    /// Employer; maintained by [`World`] together with the club's list.
    pub(crate) club: Option<ClubId>,
    pub(crate) contract: Option<Contract>,
}

impl Staff {
    pub fn club(&self) -> Option<ClubId> {
        self.club
    }

    pub fn contract(&self) -> Option<&Contract> {
        self.contract.as_ref()
    }

    /// The wage this staff member asks for, per week.
    pub fn wage_demand(&self) -> u32 {
        staff_wage(self.attrs.key(self.role))
    }
}

/// The weekly wage for a staff member whose key rating is `rating`:
/// a fraction of a player's wage at the kind of club that hires staff of
/// that standard, so backroom costs scale with the pyramid like player
/// wages do.
pub fn staff_wage(rating: u8) -> u32 {
    let quality = f64::from(rating.clamp(1, 20));
    let reputation = reputation_for_quality(quality);
    // Above what the biggest clubs' staff typically rate, the best still
    // command a premium.
    let elite = 1.0 + 0.15 * (quality - 16.0).max(0.0);
    let w = (0.15 * crate::economy::wage_scale(reputation) * elite).max(50.0);
    ((w / 50.0).round() * 50.0) as u32
}

/// A club's departments, each rated 1-20 (10 is average).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaffRatings {
    pub coaching: f64,
    pub youth_coaching: f64,
    pub scouting_ability: f64,
    pub scouting_potential: f64,
    pub physiotherapy: f64,
    pub management: f64,
}

impl StaffRatings {
    /// A club with no staff at all: well below average.
    pub const NONE: StaffRatings = StaffRatings {
        coaching: 4.0,
        youth_coaching: 4.0,
        scouting_ability: 4.0,
        scouting_potential: 4.0,
        physiotherapy: 4.0,
        management: 4.0,
    };
}

/// Mean of the best `n` values, or `fallback` if there are none. Missing
/// staff below `n` count as `fallback`.
fn best_mean(mut values: Vec<u8>, n: usize, fallback: f64) -> f64 {
    values.sort_unstable_by(|a, b| b.cmp(a));
    let total: f64 = (0..n)
        .map(|i| values.get(i).map_or(fallback, |&v| f64::from(v)))
        .sum();
    total / n as f64
}

/// Rates a club's departments from its staff.
pub fn ratings(world: &World, club: ClubId) -> StaffRatings {
    let staff: Vec<&Staff> = world.clubs()[club]
        .staff()
        .iter()
        .map(|&s| &world.staff()[s])
        .collect();
    let of = |role: StaffRole, f: fn(&StaffAttrs) -> u8| -> Vec<u8> {
        staff
            .iter()
            .filter(|s| s.role == role)
            .map(|s| f(&s.attrs))
            .collect()
    };
    let none = StaffRatings::NONE;
    StaffRatings {
        coaching: best_mean(of(StaffRole::Coach, |a| a.coaching), 3, none.coaching),
        youth_coaching: best_mean(
            of(StaffRole::Coach, |a| a.youth_coaching),
            3,
            none.youth_coaching,
        ),
        scouting_ability: best_mean(
            of(StaffRole::Scout, |a| a.judging_ability),
            2,
            none.scouting_ability,
        ),
        scouting_potential: best_mean(
            of(StaffRole::Scout, |a| a.judging_potential),
            2,
            none.scouting_potential,
        ),
        physiotherapy: best_mean(
            of(StaffRole::Physio, |a| a.physiotherapy),
            2,
            none.physiotherapy,
        ),
        management: best_mean(
            of(StaffRole::AssistantManager, |a| a.management),
            1,
            none.management,
        ),
    }
}

/// How a club's medical team scales time out injured.
pub fn treatment_factor(r: &StaffRatings) -> f64 {
    effect(r.physiotherapy, -0.25)
}

/// How a club's medical team scales the risk of training injuries.
pub fn prevention_factor(r: &StaffRatings) -> f64 {
    effect(r.physiotherapy, -0.15)
}

/// How a club's coaches and training ground scale player growth, for
/// senior players and for under-21s.
pub fn coaching_factors(r: &StaffRatings, training: u8) -> (f64, f64) {
    let ground = effect(f64::from(training), 0.06);
    (
        effect(r.coaching, 0.12) * ground,
        effect(r.youth_coaching, 0.18) * ground,
    )
}

/// A 1-20 rating as a multiplier around 1: 10 gives 1, 20 gives
/// `1 + spread`, 0 gives `1 - spread`.
pub fn effect(rating: f64, spread: f64) -> f64 {
    1.0 + (rating - 10.0) / 10.0 * spread
}

/// Creates a staff member of `role` whose ratings centre on `quality`
/// (1-20).
pub fn new_staff(
    nations: &fs_core::IdVec<crate::ids::NationKind, crate::nation::Nation>,
    rng: &mut fs_core::Rng,
    today: GameDate,
    nation: NationId,
    role: StaffRole,
    quality: f64,
) -> Staff {
    let nat = &nations[nation];
    let first_name = nat.first_names[rng.index(nat.first_names.len())];
    let last_name = nat.last_names[rng.index(nat.last_names.len())];
    let age = rng.range_i32(32, 64);
    let (y, m, d) = today.ymd();
    let birth_date = GameDate::from_ymd(y - age, m, d.min(28))
        .expect("valid date")
        .add_days(rng.below(365));
    let mut rate = |centre: f64| rng.normal(centre, 2.0).round().clamp(1.0, 20.0) as u8;
    // Strong in what the role needs, ordinary elsewhere.
    let main = quality;
    let side = (quality * 0.7).max(3.0);
    let (c, y, ja, jp, p, m) = match role {
        StaffRole::AssistantManager => (side, side, side, side, 4.0, main),
        StaffRole::Coach => (main, main, side, side, 4.0, side),
        StaffRole::Scout => (side * 0.8, side * 0.8, main, main, 3.0, side * 0.8),
        StaffRole::Physio => (3.0, 3.0, 3.0, 3.0, main, 4.0),
    };
    let attrs = StaffAttrs {
        coaching: rate(c),
        youth_coaching: rate(y),
        judging_ability: rate(ja),
        judging_potential: rate(jp),
        physiotherapy: rate(p),
        management: rate(m),
    };
    Staff {
        first_name,
        last_name,
        birth_date,
        nation,
        role,
        attrs,
        club: None,
        contract: None,
    }
}

/// The staff quality a club of this reputation (0-10000) attracts.
pub fn quality_for_reputation(reputation: u16) -> f64 {
    4.0 + 12.0 * f64::from(reputation) / 10_000.0
}

/// The reputation of a club whose staff are typically of this quality.
pub fn reputation_for_quality(quality: f64) -> u16 {
    ((quality - 4.0) / 12.0 * 10_000.0).clamp(0.0, 10_000.0) as u16
}

// Purposes for staff random streams, keyed by year (generation uses
// years 1 and 2, which never occur in play).
const RENEWAL: u64 = 1;
const NEWCOMER: u64 = 2;
const HIRE: u64 = 3;
const POOL: u64 = 4;

fn key(purpose: u64, id: u64) -> u64 {
    (purpose << 48) | id
}

/// Staff stop working at this age.
pub const RETIREMENT_AGE: f64 = 66.0;

impl Staff {
    /// Their key rating as a quality, 1-20.
    pub fn quality(&self) -> f64 {
        f64::from(self.attrs.key(self.role))
    }

    /// Whether they would join a club of this reputation: good staff want
    /// a club near their own standing.
    pub fn would_join(&self, reputation: u16) -> bool {
        reputation_for_quality(self.quality()) <= reputation.saturating_add(1_500)
    }
}

/// Whether a staff member is available to hire.
pub fn available(world: &World, staff: StaffId, today: GameDate) -> bool {
    let s = &world.staff()[staff];
    s.club.is_none() && crate::people::age_years(s.birth_date, today) < RETIREMENT_AGE
}

/// 1 June: human managers are told about staff contracts ending.
pub(crate) fn warn_expiring(world: &World, events: &mut Vec<Event>) {
    let ending = crate::people::season_end(crate::people::season_year(world.date()));
    for club in world.clubs().ids().filter(|&c| !world.ai_runs_squad(c)) {
        for &s in world.clubs()[club].staff() {
            if world.staff()[s]
                .contract
                .is_some_and(|c| c.expires <= ending)
            {
                events.push(Event::StaffContractExpiring {
                    staff: s,
                    club,
                    expires: ending,
                });
            }
        }
    }
}

/// 1 July: staff contracts end. Clubs whose squads the AI runs renew the
/// staff worth keeping and fill gaps; human managers lose staff they did
/// not renew and hire their own.
pub(crate) fn rollover(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let year = crate::people::season_year(today) as u64;
    let seed = world.seed();

    let expired: Vec<StaffId> = world
        .staff()
        .ids()
        .filter(|&s| {
            let st = &world.staff()[s];
            st.club.is_some() && st.contract.is_some_and(|c| c.expires < today)
        })
        .collect();
    for s in expired {
        let club = world.staff[s].club.expect("filtered");
        let st = &world.staff[s];
        let old = crate::people::age_years(st.birth_date, today) >= RETIREMENT_AGE - 1.0;
        let keep = world.ai_runs_squad(club)
            && !old
            && st.quality() >= quality_for_reputation(world.clubs[club].reputation) - 3.0;
        if keep {
            let mut rng = seed.derive(Stream::Staff, year, key(RENEWAL, u64::from(s.raw())));
            let years = 2 + rng.below(2) as u8;
            let wage = st.wage_demand();
            world.staff[s].contract = Some(Contract {
                wage,
                expires: crate::squad::contract_expiry(today, years),
            });
        } else {
            world.release_staff(s);
            if world.is_human_club(club) {
                events.push(Event::StaffLeft { staff: s, club });
            }
        }
    }
    fill_vacancies(world, events);
}

/// AI clubs hire staff for every department below strength: the best
/// available who would join, richest clubs first, or a newcomer when
/// nobody suitable is free.
pub(crate) fn fill_vacancies(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let year = crate::people::season_year(today) as u64;
    let seed = world.seed();
    let mut clubs: Vec<ClubId> = world
        .clubs()
        .ids()
        .filter(|&c| world.ai_runs_squad(c))
        .collect();
    clubs.sort_by_key(|&c| (std::cmp::Reverse(world.clubs()[c].reputation), c));
    // Free staff by role, best first.
    let mut pool: Vec<Vec<StaffId>> = StaffRole::ALL
        .iter()
        .map(|&role| {
            let mut v: Vec<StaffId> = world
                .staff()
                .ids()
                .filter(|&s| world.staff()[s].role == role && available(world, s, today))
                .collect();
            v.sort_by(|&a, &b| {
                world.staff()[b]
                    .quality()
                    .total_cmp(&world.staff()[a].quality())
                    .then(a.cmp(&b))
            });
            v
        })
        .collect();
    for club in clubs {
        let rep = world.clubs[club].reputation;
        for (k, &role) in StaffRole::ALL.iter().enumerate() {
            let have = world.clubs[club]
                .staff
                .iter()
                .filter(|&&s| world.staff[s].role == role)
                .count();
            for n in have..role.wanted().0 {
                let pick = pool[k].iter().position(|&s| world.staff[s].would_join(rep));
                let s = match pick {
                    Some(i) => pool[k].remove(i),
                    None => {
                        let id = (u64::from(club.raw()) * 8 + n as u64) * 4 + k as u64;
                        let mut rng = seed.derive(Stream::Staff, year, key(NEWCOMER, id));
                        let nation = world.clubs[club].nation;
                        let quality = quality_for_reputation(rep) + rng.normal(-1.0, 1.5);
                        let st = new_staff(&world.nations, &mut rng, today, nation, role, quality);
                        world.add_staff(st)
                    }
                };
                let mut rng = seed.derive(Stream::Staff, year, key(HIRE, u64::from(s.raw())));
                let contract = Contract {
                    wage: world.staff[s].wage_demand(),
                    expires: crate::squad::contract_expiry(today, 2 + rng.below(2) as u8),
                };
                world.hire_staff(s, club, contract);
                if world.is_human_club(club) {
                    events.push(Event::StaffHired { staff: s, club });
                }
            }
        }
    }
    refresh_pool(world);
}

/// Keeps a pool of free staff for human managers to choose from: about
/// one for every four clubs, of every standard.
fn refresh_pool(world: &mut World) {
    let today = world.date();
    let year = crate::people::season_year(today) as u64;
    let target = world.clubs().len() / 4 + 20;
    let free = world
        .staff()
        .ids()
        .filter(|&s| available(world, s, today))
        .count();
    let seed = world.seed();
    for i in free..target {
        let mut rng = seed.derive(Stream::Staff, year, key(POOL, i as u64));
        let nation = crate::ids::NationId::new(rng.below(world.nations.len() as u32));
        let role = StaffRole::ALL[rng.index(StaffRole::ALL.len())];
        let quality = rng.range_f64(3.0, 16.0);
        let st = new_staff(&world.nations, &mut rng, today, nation, role, quality);
        world.add_staff(st);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wages_rise_with_rating() {
        assert_eq!(staff_wage(1), 50);
        assert!(staff_wage(10) > 200 && staff_wage(10) < 1_500);
        assert!(staff_wage(20) > 50_000);
        assert!(staff_wage(16) < staff_wage(17));
    }

    #[test]
    fn best_staff_count_and_missing_staff_hurt() {
        assert!((best_mean(vec![10, 12, 14, 2], 3, 4.0) - 12.0).abs() < 1e-9);
        assert!((best_mean(vec![16], 2, 4.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn average_staff_change_nothing() {
        assert!((effect(10.0, 0.2) - 1.0).abs() < 1e-12);
        assert!((effect(20.0, 0.2) - 1.2).abs() < 1e-9);
    }
}
