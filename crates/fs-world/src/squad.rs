//! AI squad management: contract renewals, free-agent signings and
//! trimming bloated squads. Human-managed clubs are left alone; their
//! managers only get warnings.

use crate::command::Event;
use crate::ids::{ClubId, PlayerId};
use crate::people::{age_years, season_end, season_year, wage_cap, wage_demand};
use crate::player::{Contract, Status};
use crate::world::World;
use fs_core::GameDate;
use fs_core::attributes::Position;

/// Broad position groups for squad planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Goalkeeper,
    Defender,
    Midfielder,
    Attacker,
}

impl Group {
    /// The group a position belongs to.
    pub fn of(p: Position) -> Group {
        use Position::*;
        match p {
            Goalkeeper => Group::Goalkeeper,
            DefenderCentre | DefenderLeft | DefenderRight | WingBackLeft | WingBackRight => {
                Group::Defender
            }
            DefensiveMidfielder | MidfielderCentre | MidfielderLeft | MidfielderRight
            | AttackingMidCentre => Group::Midfielder,
            AttackingMidLeft | AttackingMidRight | Striker => Group::Attacker,
        }
    }
}

/// Minimum players per group an AI club wants.
const NEEDS: [(Group, usize); 4] = [
    (Group::Goalkeeper, 2),
    (Group::Defender, 7),
    (Group::Midfielder, 7),
    (Group::Attacker, 4),
];
/// AI clubs top their squads up to this size.
pub const TARGET_SIZE: usize = 24;
/// Squads above this are trimmed...
pub const MAX_SIZE: usize = 30;
/// ...down to this.
pub const TRIM_TO: usize = 27;

/// How much a club values a player: current ability, plus some of the
/// remaining potential for youngsters.
pub fn value(world: &World, p: PlayerId, today: GameDate) -> f64 {
    let ps = world.players();
    let ability = f64::from(ps.ability(p));
    let age = age_years(ps.bio(p).birth_date, today);
    if age < 22.0 {
        ability + 0.3 * (f64::from(ps.potential(p)) - ability).max(0.0)
    } else if age > 32.0 {
        ability - (age - 32.0) * 25.0
    } else {
        ability
    }
}

/// A contract of `years` seasons, counting the current one if it is
/// still July to December, otherwise starting next season.
pub fn contract_expiry(today: GameDate, years: u8) -> GameDate {
    let (_, m, _) = today.ymd();
    let first = if m >= 7 {
        season_year(today)
    } else {
        season_year(today) + 1
    };
    season_end(first + i32::from(years.max(1)) - 1)
}

/// Contract length an AI club offers at this age.
fn years_for_age(age: f64) -> u8 {
    if age < 24.0 {
        4
    } else if age < 29.0 {
        3
    } else if age < 32.0 {
        2
    } else {
        1
    }
}

/// Players under this age are academy players: they can play, but do not
/// count toward squad size or positional needs.
pub const ACADEMY_AGE: f64 = 18.0;

/// Whether a player is old enough to count toward the first-team squad.
pub fn first_team_age(world: &World, p: PlayerId, today: GameDate) -> bool {
    age_years(world.players().bio(p).birth_date, today) >= ACADEMY_AGE
}

/// The first-team players (18 and over), most valuable first.
fn ranked_first_team(world: &World, club: ClubId, today: GameDate) -> Vec<PlayerId> {
    let mut squad: Vec<PlayerId> = world.clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&p| first_team_age(world, p, today))
        .collect();
    squad.sort_by(|&a, &b| {
        value(world, b, today)
            .total_cmp(&value(world, a, today))
            .then(a.cmp(&b))
    });
    squad
}

/// Average ability of a club's best 18: the bar a youngster must be able
/// to reach.
fn first_team_bar(world: &World, ranked: &[PlayerId]) -> u32 {
    let top: Vec<u32> = ranked
        .iter()
        .take(18)
        .map(|&p| u32::from(world.players().ability(p)))
        .collect();
    top.iter().sum::<u32>() / top.len().max(1) as u32
}

/// 1 June: AI clubs renew the expiring contracts of players worth keeping;
/// human managers are reminded about theirs.
pub(crate) fn renew_contracts(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let ending = season_end(season_year(today));
    for club in world.clubs.ids() {
        let expiring: Vec<PlayerId> = world.clubs[club]
            .squad
            .iter()
            .copied()
            .filter(|&p| {
                world
                    .players
                    .contract(p)
                    .is_some_and(|c| c.expires <= ending)
            })
            .collect();
        if !world.ai_runs_squad(club) {
            for p in expiring {
                events.push(Event::ContractExpiring {
                    player: p,
                    club,
                    expires: ending,
                });
            }
            continue;
        }
        let ranked = ranked_first_team(world, club, today);
        let bar = first_team_bar(world, &ranked);
        let keep: Vec<PlayerId> = ranked.iter().copied().take(TARGET_SIZE - 2).collect();
        for p in expiring {
            let age = age_years(world.players.bio(p).birth_date, today);
            let core = ranked.iter().position(|&q| q == p).is_some_and(|k| k < 14);
            let prospect = age < 21.0 && u32::from(world.players.potential(p)) + 30 >= bar;
            if prospect || (keep.contains(&p) && (age < 33.0 || (age < 35.0 && core))) {
                let ps = &world.players;
                let wage = wage_demand(ps.ability(p), ps.potential(p), age as u32);
                world.players.contract[p.index()] = Some(Contract {
                    wage,
                    expires: contract_expiry(today, years_for_age(age)),
                });
            }
        }
    }
}

/// 1 January: reminds human managers about contracts ending in June.
pub(crate) fn warn_expiring(world: &World, events: &mut Vec<Event>) {
    let today = world.date();
    let ending = season_end(season_year(today));
    for club in world.clubs().ids().filter(|&c| !world.ai_runs_squad(c)) {
        for &p in world.clubs()[club].squad() {
            if world
                .players()
                .contract(p)
                .is_some_and(|c| c.expires <= ending)
            {
                events.push(Event::ContractExpiring {
                    player: p,
                    club,
                    expires: ending,
                });
            }
        }
    }
}

/// Transfer-window housekeeping for AI clubs: trim oversized squads, then
/// sign free agents to fill positional gaps, richest clubs first.
pub(crate) fn maintain_squads(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let ai_clubs: Vec<ClubId> = world
        .clubs
        .ids()
        .filter(|&c| world.ai_runs_squad(c))
        .collect();

    // Academy cut: in summer, youngsters with no realistic first-team
    // future are let go.
    let summer = today.ymd().1 == 7;
    for &club in ai_clubs.iter().filter(|_| summer) {
        let ranked = ranked_first_team(world, club, today);
        let bar = first_team_bar(world, &ranked);
        for &p in ranked.iter().skip(18) {
            let age = age_years(world.players.bio(p).birth_date, today);
            if (18.0..22.0).contains(&age) && u32::from(world.players.potential(p)) + 60 < bar {
                world.release_player(p);
                if world.is_human_club(club) {
                    events.push(Event::PlayerLeft { player: p, club });
                }
            }
        }
    }

    // Trim: release the least valuable first-team players.
    for &club in &ai_clubs {
        let ranked = ranked_first_team(world, club, today);
        if ranked.len() > MAX_SIZE {
            for &p in ranked.iter().skip(TRIM_TO) {
                world.release_player(p);
                if world.is_human_club(club) {
                    events.push(Event::PlayerLeft { player: p, club });
                }
            }
        }
    }

    // Fill: free agents by ability, best first, with their demands worked
    // out once.
    let mut free: Vec<(PlayerId, Group, u32)> = world
        .players
        .ids()
        .filter(|&p| world.players.status(p) == Status::Active && world.players.club(p).is_none())
        .map(|p| {
            let ps = &world.players;
            let age = age_years(ps.bio(p).birth_date, today) as u32;
            (
                p,
                Group::of(ps.positions(p).best()),
                wage_demand(ps.ability(p), ps.potential(p), age),
            )
        })
        .collect();
    free.sort_by(|a, b| {
        world
            .players
            .ability(b.0)
            .cmp(&world.players.ability(a.0))
            .then(a.0.cmp(&b.0))
    });
    let mut order = ai_clubs;
    order.sort_by(|&a, &b| {
        world.clubs[b]
            .reputation
            .cmp(&world.clubs[a].reputation)
            .then(a.cmp(&b))
    });
    for club in order {
        let cap = wage_cap(world.clubs[club].reputation);
        for _ in 0..8 {
            let Some(need) = most_needed(world, club, today) else {
                break;
            };
            let pick = free
                .iter()
                .position(|&(_, g, demand)| need.is_none_or(|n| n == g) && demand <= cap);
            let Some(k) = pick else { break };
            let (p, _, wage) = free.remove(k);
            let age = age_years(world.players.bio(p).birth_date, today);
            let contract = Contract {
                wage,
                expires: contract_expiry(today, years_for_age(age)),
            };
            world.sign_player(p, club, contract);
            if world.is_human_club(club) {
                events.push(Event::PlayerSigned { player: p, club });
            }
        }
    }
}

/// The group a club most needs, `Some(None)` if it only needs bodies, or
/// `None` if the squad is complete.
fn most_needed(world: &World, club: ClubId, today: GameDate) -> Option<Option<Group>> {
    let squad: Vec<PlayerId> = world.clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&p| first_team_age(world, p, today))
        .collect();
    let count = |g: Group| {
        squad
            .iter()
            .filter(|&&p| Group::of(world.players().positions(p).best()) == g)
            .count()
    };
    for (g, n) in NEEDS {
        if count(g) < n {
            return Some(Some(g));
        }
    }
    (squad.len() < TARGET_SIZE).then_some(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_expiry_counts_seasons() {
        let autumn = GameDate::from_ymd(2026, 9, 1).unwrap();
        assert_eq!(
            contract_expiry(autumn, 1),
            GameDate::from_ymd(2027, 6, 30).unwrap()
        );
        let spring = GameDate::from_ymd(2027, 3, 1).unwrap();
        assert_eq!(
            contract_expiry(spring, 1),
            GameDate::from_ymd(2028, 6, 30).unwrap()
        );
        assert_eq!(
            contract_expiry(spring, 3),
            GameDate::from_ymd(2030, 6, 30).unwrap()
        );
    }
}
