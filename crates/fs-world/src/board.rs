//! Boards and the managers' job market.
//!
//! Each summer a board sets a target league position from the club's
//! standing in its league. Confidence in the manager moves with every
//! result against what a club with that target should get, with the
//! league position through the season, and with the club's money. A
//! manager whose board loses confidence is sacked; human and AI managers
//! alike. Vacant clubs are run by the AI until a new manager arrives:
//! after [`VACANCY_DAYS`] the board appoints the best available AI
//! manager, unless a human applied first ([`crate::Command::ApplyForJob`]).
//!
//! Club reputations drift with results: toward the level a club finishes
//! at, so promoted clubs grow and relegated ones shrink, over seasons.

use crate::command::Event;
use crate::economy::revenue_potential;
use crate::ids::{ClubId, ManagerId};
use crate::manager::{Control, Manager};
use crate::world::World;
use fs_core::{GameDate, Stream};
use serde::{Deserialize, Serialize};

/// Starting confidence, and where it settles.
pub const NEUTRAL_CONFIDENCE: u8 = 60;
/// Below this the manager is sacked.
pub const SACK_CONFIDENCE: u8 = 20;
/// Human managers are warned when confidence falls below this.
pub const WARNING_CONFIDENCE: u8 = 35;
/// Days a club waits before appointing an AI manager.
pub const VACANCY_DAYS: u32 = 7;

/// A club's board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    /// The league position the board expects, or better (1 is first).
    /// Zero when the club has no league season.
    pub target: u8,
    /// Confidence in the manager, 0-100. The manager is sacked when it
    /// falls too low.
    pub confidence: u8,
    /// When the club lost its manager, if it has none.
    pub vacant_since: Option<GameDate>,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            target: 0,
            confidence: NEUTRAL_CONFIDENCE,
            vacant_since: None,
        }
    }
}

fn adjust(board: &mut Board, delta: f64) {
    board.confidence = (f64::from(board.confidence) + delta)
        .round()
        .clamp(0.0, 100.0) as u8;
}

/// Points per game a club finishing `target` of `n` usually takes.
fn expected_points(target: u8, n: usize) -> f64 {
    let n = n.max(2) as f64;
    2.3 - 1.5 * (f64::from(target.max(1)) - 1.0) / (n - 1.0)
}

/// Summer (and new worlds): every board sets its target from the club's
/// reputation rank in its league, with a little slack.
pub fn set_targets(world: &mut World) {
    let mut targets: Vec<(ClubId, u8)> = Vec::new();
    for (_, league) in world.leagues().iter() {
        let mut clubs: Vec<ClubId> = league.clubs().to_vec();
        clubs.sort_by_key(|&c| (std::cmp::Reverse(world.clubs()[c].reputation), c));
        let n = clubs.len();
        let slack = (n / 8).max(1);
        for (rank, &c) in clubs.iter().enumerate() {
            targets.push((c, (rank + 1 + slack).min(n) as u8));
        }
    }
    for (c, t) in targets {
        world.clubs[c].board.target = t;
    }
}

/// After a league match: confidence moves with the points won against
/// what a club with this target should average.
pub(crate) fn after_match(world: &mut World, club: ClubId, points: u8, events: &mut Vec<Event>) {
    let Some(league) = world.clubs[club].league else {
        return;
    };
    let n = world.leagues[league].clubs.len();
    let board = &mut world.clubs[club].board;
    let delta = (f64::from(points) - expected_points(board.target, n)) * 2.2;
    let before = board.confidence;
    adjust(board, delta);
    let after = board.confidence;
    if let Some(m) = world.clubs[club].manager
        && world.managers[m].control == Control::Human
        && before >= WARNING_CONFIDENCE
        && after < WARNING_CONFIDENCE
    {
        events.push(Event::BoardWarning {
            club,
            confidence: after,
        });
    }
    check_sacking(world, club, events);
}

/// Sacks the manager if the board has lost faith.
fn check_sacking(world: &mut World, club: ClubId, events: &mut Vec<Event>) {
    if world.clubs[club].board.confidence >= SACK_CONFIDENCE {
        return;
    }
    let Some(m) = world.clubs[club].manager else {
        return;
    };
    if world.managers[m].control == Control::Human && !world.rules.sack_humans {
        world.clubs[club].board.confidence = SACK_CONFIDENCE;
        return;
    }
    world.clubs[club].manager = None;
    world.managers[m].club = None;
    world.managers[m].reputation = world.managers[m].reputation.saturating_sub(600);
    world.clubs[club].board.vacant_since = Some(world.date);
    world.clubs[club].board.confidence = NEUTRAL_CONFIDENCE;
    events.push(Event::ManagerSacked { manager: m, club });
}

/// Ends a manager's time at a club (resignation).
pub(crate) fn leave(world: &mut World, manager: ManagerId) -> Option<ClubId> {
    let club = world.managers[manager].club.take()?;
    world.clubs[club].manager = None;
    world.clubs[club].board.vacant_since = Some(world.date);
    world.clubs[club].board.confidence = NEUTRAL_CONFIDENCE;
    Some(club)
}

/// Appoints a manager to a vacant club.
pub(crate) fn appoint(world: &mut World, manager: ManagerId, club: ClubId) {
    world.managers[manager].club = Some(club);
    world.clubs[club].manager = Some(manager);
    let b = &mut world.clubs[club].board;
    b.vacant_since = None;
    b.confidence = NEUTRAL_CONFIDENCE;
}

/// Whether a club's board would hire a manager of this reputation.
pub fn would_hire(world: &World, club: ClubId, reputation: u16) -> bool {
    let c = &world.clubs()[club];
    u32::from(reputation) + 3_000 >= u32::from(c.reputation) || c.reputation <= 3_000
}

/// The 1st of the month: boards look at the table and the bank.
pub(crate) fn monthly(world: &mut World, events: &mut Vec<Event>) {
    let clubs: Vec<ClubId> = world.clubs().ids().collect();
    for club in clubs {
        if world.clubs[club].manager.is_none() {
            continue;
        }
        let mut delta = 0.0;
        if let Some(l) = world.clubs[club].league
            && let Some(season) = world.leagues[l].season()
        {
            let table = season.table();
            let n = table.len();
            if let Some(pos) = table.iter().position(|r| r.club == club)
                && table[pos].played >= 8
            {
                let target = usize::from(world.clubs[club].board.target.max(1));
                if pos + 1 > target + n / 4 {
                    delta -= 3.0;
                } else if pos < target {
                    delta += 1.0;
                }
            }
        }
        let potential = revenue_potential(world.clubs[club].reputation);
        if world.clubs[club].finances.balance < -potential / 3 {
            delta -= 2.0;
        }
        // Patience returns slowly.
        let c = f64::from(world.clubs[club].board.confidence);
        delta += (f64::from(NEUTRAL_CONFIDENCE) - c) * 0.05;
        adjust(&mut world.clubs[club].board, delta);
        check_sacking(world, club, events);
    }
}

/// The end of a season, before promotion and relegation: boards judge the
/// final position, managers' and clubs' reputations move.
pub(crate) fn season_end(world: &mut World, events: &mut Vec<Event>) {
    let mut verdicts: Vec<(ClubId, i32, usize, usize, f64)> = Vec::new();
    for (_, league) in world.leagues().iter() {
        let Some(season) = league.season() else {
            continue;
        };
        let table = season.table();
        let n = table.len();
        // The reputation a position in this league implies: the league's
        // spread of reputations, mapped onto the table.
        let mut reps: Vec<u16> = league
            .clubs()
            .iter()
            .map(|&c| world.clubs()[c].reputation)
            .collect();
        reps.sort_unstable_by(|a, b| b.cmp(a));
        for (i, row) in table.iter().enumerate() {
            let target = i32::from(world.clubs()[row.club].board.target.max(1));
            let implied = f64::from(reps[i]);
            verdicts.push((row.club, target - (i as i32 + 1), i, n, implied));
        }
    }
    for (club, beat, pos, n, implied) in verdicts {
        // Reputation: a tenth of the way toward what the finish implies.
        // Within a league this only reshuffles reputations; promotion and
        // relegation move them between levels.
        let c = &mut world.clubs[club];
        let rep = f64::from(c.reputation);
        c.reputation = (rep + (implied - rep) * 0.1).clamp(200.0, 10_000.0) as u16;
        let swing = if beat >= 0 {
            12.0 + 3.0 * f64::from(beat.min(5))
        } else if -beat as usize > n / 4 {
            -25.0
        } else {
            -6.0
        };
        adjust(&mut c.board, swing);
        if let Some(m) = c.manager {
            let mr = &mut world.managers[m].reputation;
            let delta = (beat * 80).clamp(-500, 600) + if pos == 0 { 400 } else { 0 };
            *mr = (i32::from(*mr) + delta).clamp(0, 10_000) as u16;
        }
        // A disastrous season ends it, whatever the confidence.
        if world.clubs[club].board.confidence < SACK_CONFIDENCE + 10 {
            world.clubs[club].board.confidence = 0;
        }
        check_sacking(world, club, events);
    }
}

/// Daily: clubs that have been without a manager for a week appoint the
/// best available AI manager they can attract, or promote a newcomer.
pub(crate) fn fill_vacancies(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date;
    let vacant: Vec<ClubId> = world
        .clubs()
        .ids()
        .filter(|&c| {
            world.clubs()[c]
                .board
                .vacant_since
                .is_some_and(|d| today.days_since(d) >= i64::from(VACANCY_DAYS))
        })
        .collect();
    for club in vacant {
        let rep = world.clubs[club].reputation;
        let pick = world
            .managers()
            .ids()
            .filter(|&m| {
                let mg = &world.managers()[m];
                mg.control == Control::Ai
                    && mg.club.is_none()
                    && mg.reputation <= rep.saturating_add(1_000)
                    && would_hire(world, club, mg.reputation)
            })
            .max_by_key(|&m| (world.managers()[m].reputation, std::cmp::Reverse(m)));
        let manager = match pick {
            Some(m) => m,
            None => {
                let mut rng = world.seed().derive(
                    Stream::Board,
                    u64::from(today.days()),
                    u64::from(club.raw()),
                );
                let nation = &world.nations[world.clubs[club].nation];
                let first_name = nation.first_names[rng.index(nation.first_names.len())];
                let last_name = nation.last_names[rng.index(nation.last_names.len())];
                let reputation = (f64::from(rep) * rng.range_f64(0.6, 0.9)) as u16;
                world.add_manager(Manager {
                    first_name,
                    last_name,
                    control: Control::Ai,
                    club: None,
                    delegate_squad: false,
                    watch_matches: false,
                    reputation,
                })
            }
        };
        appoint(world, manager, club);
        events.push(Event::ManagerAppointed { manager, club });
    }
}

/// Clubs looking for a manager.
pub fn vacancies(world: &World) -> Vec<ClubId> {
    world
        .clubs()
        .ids()
        .filter(|&c| world.clubs()[c].manager.is_none() && world.clubs()[c].league().is_some())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favourites_are_expected_to_win_more() {
        assert!(expected_points(1, 20) > 2.2);
        assert!(expected_points(20, 20) < 0.9);
        assert!(expected_points(10, 20) > expected_points(11, 20));
    }

    #[test]
    fn confidence_stays_in_range() {
        let mut b = Board::default();
        adjust(&mut b, -500.0);
        assert_eq!(b.confidence, 0);
        adjust(&mut b, 500.0);
        assert_eq!(b.confidence, 100);
    }
}
