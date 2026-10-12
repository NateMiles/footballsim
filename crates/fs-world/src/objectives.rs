//! Board objectives beyond the league table: how far to go in each cup,
//! and an aim that follows the club's ethos (bringing through young
//! players, attacking football, or balancing the books).
//!
//! Boards set them each summer with the league target. Progress is visible
//! all season; at its end each objective is met or missed, and board
//! confidence moves by its importance. Every club has them, so AI and
//! human managers are judged alike; the swings are kept small next to the
//! league verdict.

use crate::command::Event;
use crate::ids::{ClubId, CupId};
use crate::people::{age_years, season_year};
use crate::world::World;
use fs_core::{GameDate, Stream};
use serde::{Deserialize, Serialize};

/// What a club stands for, beyond results. Never changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ethos {
    /// Bring young players through.
    Youth,
    /// Play attacking football.
    Attacking,
    /// Live within its means.
    Prudent,
    /// Results above all.
    Ambitious,
}

/// The club's ethos.
pub fn ethos(world: &World, club: ClubId) -> Ethos {
    let mut rng = world
        .seed()
        .derive(Stream::Board, u64::from(club.raw()), 0xE7405);
    match rng.below(4) {
        0 => Ethos::Youth,
        1 => Ethos::Attacking,
        2 => Ethos::Prudent,
        _ => Ethos::Ambitious,
    }
}

/// An aim for the season.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aim {
    /// Reach this stage of the cup (an index into its format's stages).
    Cup { cup: CupId, stage: u8 },
    /// Players under 22 get at least this percentage of the minutes.
    Youth { share: u8 },
    /// Score at least this many league goals per match, in tenths.
    Goals { tenths: u8 },
    /// End the season without a loss.
    Profit,
}

/// How much an objective matters.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Importance {
    Medium,
    High,
}

/// One of the board's objectives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Objective {
    pub aim: Aim,
    pub importance: Importance,
}

/// Where an objective stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Progress {
    /// Done (a cup stage reached).
    Met,
    /// Can no longer be met (out of the cup).
    Failed,
    /// Would be met if the season ended today.
    OnTrack,
    Behind,
}

/// Where an objective stands, with the measure so far and its target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Standing {
    pub progress: Progress,
    pub value: f64,
    pub target: f64,
}

/// Target share of minutes for under-22s at youth clubs.
const YOUTH_SHARE: u8 = 15;

fn season_start(today: GameDate) -> GameDate {
    GameDate::from_ymd(season_year(today), 7, 1).expect("valid date")
}

/// The furthest format stage the club has played in this edition, if any.
fn furthest(world: &World, cup: CupId, club: ClubId) -> Option<usize> {
    let e = world.cups()[cup].edition()?;
    e.stages
        .iter()
        .filter(|s| s.has(club))
        .map(|s| s.format_index())
        .max()
}

/// The stage a club of this standing among a cup's entrants should reach:
/// counting back from the final, each round halves the field.
fn cup_target(world: &World, cup: CupId, club: ClubId) -> Option<u8> {
    let c = &world.cups()[cup];
    let e = c.edition()?;
    let entry = e.entrants.iter().position(|s| s.contains(&club))?;
    let mut field: Vec<(u16, ClubId)> = e
        .entrants
        .iter()
        .flatten()
        .map(|&x| (world.clubs()[x].reputation, x))
        .collect();
    field.sort_by(|a, b| b.cmp(a));
    let rank = field.iter().position(|&(_, x)| x == club)? + 1;
    let last = c.format.stages.len().checked_sub(1)?;
    // Clubs left `k` rounds before the end: 2^(k+1). Allow some slack:
    // the first k whose field holds one and a half times the rank.
    let mut k = 0;
    while (2usize << k) * 2 < rank * 3 {
        k += 1;
    }
    Some(last.saturating_sub(k).max(entry) as u8)
}

/// Summer (and new worlds): every board sets its objectives.
pub fn set(world: &mut World) {
    let today = world.date();
    let mut all: Vec<(ClubId, Vec<Objective>)> = Vec::new();
    for club in world.clubs().ids() {
        let mut v = Vec::new();
        for (cup, c) in world.cups().iter() {
            if !c
                .edition()
                .is_some_and(|e| e.involves(club) && e.year == season_year(today))
            {
                continue;
            }
            if let Some(stage) = cup_target(world, cup, club) {
                v.push(Objective {
                    aim: Aim::Cup { cup, stage },
                    importance: if c.format.prestige >= 60 {
                        Importance::High
                    } else {
                        Importance::Medium
                    },
                });
            }
        }
        let aim = match ethos(world, club) {
            Ethos::Youth => Some(Aim::Youth { share: YOUTH_SHARE }),
            Ethos::Attacking => world.clubs()[club].league().map(|l| Aim::Goals {
                tenths: league_scoring(world, l),
            }),
            Ethos::Prudent => Some(Aim::Profit),
            Ethos::Ambitious => None,
        };
        if let Some(aim) = aim {
            v.push(Objective {
                aim,
                importance: Importance::Medium,
            });
        }
        all.push((club, v));
    }
    for (club, v) in all {
        world.clubs[club].objectives = v;
    }
}

/// A tenth more than the league's average goals per club and match last
/// season, in tenths (1.4 when unknown).
fn league_scoring(world: &World, league: crate::ids::LeagueId) -> u8 {
    let avg = world.leagues()[league]
        .history()
        .last()
        .map(|s| {
            let goals: u32 = s.table.iter().map(|r| u32::from(r.scored)).sum();
            let played: u32 = s.table.iter().map(|r| u32::from(r.played)).sum();
            f64::from(goals) / f64::from(played.max(1))
        })
        .unwrap_or(1.3);
    ((avg * 1.1) * 10.0).round().clamp(8.0, 30.0) as u8
}

/// Where an objective stands today.
pub fn standing(world: &World, club: ClubId, o: &Objective) -> Standing {
    match o.aim {
        Aim::Cup { cup, stage } => {
            let reached = furthest(world, cup, club);
            let won = world.cups()[cup]
                .edition()
                .is_some_and(|e| e.winner == Some(club));
            let value = reached.map_or(-1.0, |s| s as f64) + if won { 1.0 } else { 0.0 };
            let alive = world.cups()[cup].edition().is_some_and(|e| e.alive(club));
            let progress = if won || reached.is_some_and(|s| s >= usize::from(stage)) {
                Progress::Met
            } else if alive {
                Progress::OnTrack
            } else {
                Progress::Failed
            };
            Standing {
                progress,
                value,
                target: f64::from(stage),
            }
        }
        Aim::Youth { share } => {
            let ps = world.players();
            let start = season_start(world.date());
            let (mut young, mut all) = (0u64, 0u64);
            for &p in world.clubs()[club].squad() {
                let m = u64::from(ps.season_stats(p).minutes);
                all += m;
                if age_years(ps.bio(p).birth_date, start) < 22.0 {
                    young += m;
                }
            }
            let value = 100.0 * young as f64 / all.max(1) as f64;
            let target = f64::from(share);
            Standing {
                // Nothing to judge before a ball is kicked.
                progress: if all == 0 || value >= target {
                    Progress::OnTrack
                } else {
                    Progress::Behind
                },
                value,
                target,
            }
        }
        Aim::Goals { tenths } => {
            let row = world.clubs()[club]
                .league()
                .and_then(|l| world.leagues()[l].season())
                .and_then(|s| s.table().into_iter().find(|r| r.club == club));
            let played = row.map_or(0, |r| r.played);
            let value = row.map_or(0.0, |r| f64::from(r.scored) / f64::from(r.played.max(1)));
            let target = f64::from(tenths) / 10.0;
            Standing {
                progress: if played == 0 || value >= target {
                    Progress::OnTrack
                } else {
                    Progress::Behind
                },
                value,
                target,
            }
        }
        Aim::Profit => {
            let value = world.clubs()[club].finances().season.net() as f64;
            Standing {
                progress: if value >= 0.0 {
                    Progress::OnTrack
                } else {
                    Progress::Behind
                },
                value,
                target: 0.0,
            }
        }
    }
}

/// The end of the season (before cups and ledgers are archived): each
/// objective met or missed moves the board's confidence.
pub(crate) fn judge(world: &mut World, events: &mut Vec<Event>) {
    let clubs: Vec<ClubId> = world.clubs().ids().collect();
    for club in clubs {
        let objectives = world.clubs[club].objectives.clone();
        if objectives.is_empty() {
            continue;
        }
        let mut met = 0u8;
        let mut swing = 0.0;
        for o in &objectives {
            let done = matches!(
                standing(world, club, o).progress,
                Progress::Met | Progress::OnTrack
            );
            met += u8::from(done);
            swing += match (done, o.importance) {
                (true, Importance::High) => 6.0,
                (true, Importance::Medium) => 4.0,
                (false, Importance::High) => -8.0,
                (false, Importance::Medium) => -5.0,
            };
        }
        crate::board::adjust_confidence(world, club, swing);
        if world.is_human_club(club) {
            events.push(Event::ObjectivesJudged {
                club,
                met,
                total: objectives.len() as u8,
            });
        }
    }
}
