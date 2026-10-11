//! Cups, competitions, schedules and honours.

use super::refs::*;
use crate::dto;
use fs_world::cup::{CupEdition, CupScope, StageDraw};
use fs_world::{ClubId, CupId, FixtureRef, World};

fn cup_fixture(cup: CupId, index: u32) -> FixtureRef {
    FixtureRef::Cup { cup, index }
}

/// The cups a viewer would look at first: their nation's, then their
/// continent's, then the rest.
fn cup_order(world: &World, viewer: Option<ClubId>) -> Vec<CupId> {
    let nation = viewer.map(|c| world.clubs()[c].nation);
    let continent = nation.and_then(|n| {
        world
            .continents()
            .iter()
            .position(|k| k.nations.contains(&n))
            .map(|k| k as u16)
    });
    let mut ids: Vec<CupId> = world.cups().ids().collect();
    ids.sort_by_key(|&id| {
        let near = match world.cups()[id].scope {
            CupScope::Nation(n) if Some(n) == nation => 0,
            CupScope::Continent(k) if Some(k) == continent => 1,
            CupScope::Continent(_) => 2,
            CupScope::Nation(_) => 3,
        };
        (near, id)
    });
    ids
}

/// The cup to show by default: the viewer's national cup.
pub fn default_cup(world: &World, viewer: Option<ClubId>) -> Option<CupId> {
    cup_order(world, viewer).into_iter().next()
}

pub fn cup(world: &World, id: CupId, viewer: Option<ClubId>) -> dto::CupView {
    let c = &world.cups()[id];
    let e = c.edition();
    let stages = e.map(|e| stages(world, id, e)).unwrap_or_default();
    let drawn = e.map_or(0, |e| e.stages.len());
    let upcoming = if e.is_some_and(|e| e.finished) {
        Vec::new()
    } else {
        c.format
            .stages
            .iter()
            .skip(drawn)
            .map(|s| s.name.clone())
            .collect()
    };
    let mut history: Vec<dto::CupWinner> = c
        .history()
        .iter()
        .map(|h| dto::CupWinner {
            season: season_label(h.year),
            winner: h.winner.map(|w| club_ref(world, w)),
            runner_up: h.runner_up.map(|w| club_ref(world, w)),
        })
        .collect();
    history.reverse();
    dto::CupView {
        cup: cup_ref(world, id),
        season: e.map(|e| season_label(e.year)).unwrap_or_default(),
        stages,
        upcoming,
        winner: e.and_then(|e| e.winner).map(|w| club_ref(world, w)),
        history,
        cups: cup_order(world, viewer)
            .into_iter()
            .map(|c| cup_ref(world, c))
            .collect(),
    }
}

fn stages(world: &World, id: CupId, e: &CupEdition) -> Vec<dto::CupStage> {
    let format = &world.cups()[id].format;
    e.stages
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let name = format.stages[s.format_index()].name.clone();
            match s {
                StageDraw::Knockout { ties, byes, .. } => dto::CupStage {
                    name,
                    ties: ties
                        .iter()
                        .map(|t| {
                            let legs: Vec<dto::Fixture> = t
                                .legs
                                .iter()
                                .map(|&i| fixture(world, cup_fixture(id, i)))
                                .collect();
                            let played: Vec<_> = t
                                .legs
                                .iter()
                                .filter_map(|&i| {
                                    let f = &e.fixtures[i as usize].fixture;
                                    f.result.as_ref().map(|r| (f.home, r.goals))
                                })
                                .collect();
                            let aggregate = (t.legs.len() > 1 && !played.is_empty()).then(|| {
                                let mut agg = [0u8; 2];
                                for (home, goals) in &played {
                                    let first_home = *home == t.clubs[0];
                                    agg[0] += if first_home { goals[0] } else { goals[1] };
                                    agg[1] += if first_home { goals[1] } else { goals[0] };
                                }
                                agg
                            });
                            dto::Tie {
                                clubs: [club_ref(world, t.clubs[0]), club_ref(world, t.clubs[1])],
                                legs,
                                aggregate,
                                winner: t.winner.map(|w| w.raw()),
                            }
                        })
                        .collect(),
                    groups: Vec::new(),
                    byes: byes.iter().map(|&b| club_ref(world, b)).collect(),
                },
                StageDraw::Groups { groups, .. } => {
                    let advance = match format.stages[s.format_index()].kind {
                        fs_world::format::StageKind::Groups { advance, .. } => usize::from(advance),
                        fs_world::format::StageKind::Knockout { .. } => 1,
                    };
                    dto::CupStage {
                        name,
                        ties: Vec::new(),
                        groups: groups
                            .iter()
                            .enumerate()
                            .map(|(gi, g)| dto::GroupView {
                                name: group_name(gi),
                                rows: g
                                    .table()
                                    .iter()
                                    .enumerate()
                                    .map(|(i, r)| dto::TableRow {
                                        position: (i + 1) as u8,
                                        club: club_ref(world, r.club),
                                        played: r.played,
                                        won: r.won,
                                        drawn: r.drawn,
                                        lost: r.lost,
                                        scored: r.scored,
                                        conceded: r.conceded,
                                        goal_difference: r.goal_difference() as i16,
                                        points: r.points(),
                                        form: Vec::new(),
                                        zone: (i < advance).then_some(dto::TableZone::Qualify),
                                    })
                                    .collect(),
                                fixtures: e
                                    .fixtures
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, f)| {
                                        usize::from(f.stage) == k && usize::from(f.tie) == gi
                                    })
                                    .map(|(i, _)| fixture(world, cup_fixture(id, i as u32)))
                                    .collect(),
                            })
                            .collect(),
                        byes: Vec::new(),
                    }
                }
            }
        })
        .collect()
}

/// Every competition, by nation and continent.
pub fn competitions(world: &World) -> dto::CompetitionsView {
    let nations = world
        .nations()
        .iter()
        .map(|(n, nation)| dto::NationCompetitions {
            nation: nation.name.clone(),
            code: nation.code.clone(),
            leagues: world
                .leagues()
                .iter()
                .filter(|(_, l)| l.nation == n)
                .map(|(id, _)| league_ref(world, id))
                .collect(),
            cups: world
                .cups()
                .iter()
                .filter(|(_, c)| c.scope == CupScope::Nation(n))
                .map(|(id, _)| cup_ref(world, id))
                .collect(),
        })
        .collect();
    let continents = world
        .continents()
        .iter()
        .enumerate()
        .map(|(k, c)| dto::ContinentView {
            name: c.name.clone(),
            cups: world
                .cups()
                .iter()
                .filter(|(_, cup)| cup.scope == CupScope::Continent(k as u16))
                .map(|(id, _)| cup_ref(world, id))
                .collect(),
            ranking: c
                .ranking()
                .into_iter()
                .enumerate()
                .map(|(i, n)| {
                    let slot = c.nations.iter().position(|&x| x == n).unwrap_or(0);
                    dto::CoefficientRow {
                        rank: (i + 1) as u16,
                        nation: world.nations()[n].name.clone(),
                        code: world.nations()[n].code.clone(),
                        points: c.coefficient(n) as f32 / 1000.0,
                        seasons: c
                            .seasons()
                            .iter()
                            .map(|s| s.get(slot).copied().unwrap_or(0) as f32 / 1000.0)
                            .collect(),
                    }
                })
                .collect(),
        })
        .collect();
    dto::CompetitionsView {
        nations,
        continents,
    }
}

/// A club's matches this season in every competition.
pub fn schedule(world: &World, club: ClubId) -> dto::ScheduleView {
    dto::ScheduleView {
        club: club_ref(world, club),
        fixtures: fs_world::fixture::of_club(world, club)
            .into_iter()
            .map(|r| fixture(world, r))
            .collect(),
    }
}

/// How a club is doing in each cup it entered this season.
pub fn statuses(world: &World, club: ClubId) -> Vec<dto::CupStatus> {
    cup_order(world, Some(club))
        .into_iter()
        .filter_map(|id| {
            let c = &world.cups()[id];
            let e = c.edition()?;
            if !e.involves(club) {
                return None;
            }
            let stage_name = |k: usize| {
                let fi = e.stages.get(k).map_or(k, |s| s.format_index());
                c.format.stages[fi.min(c.format.stages.len() - 1)]
                    .name
                    .clone()
            };
            let alive = e.alive(club);
            let last_with_club = e.stages.iter().rposition(|s| s.has(club));
            let status = if e.winner == Some(club) {
                "Winners".to_owned()
            } else if alive {
                // The stage being played, the one the club enters later,
                // or the next one to be drawn.
                let enters = e
                    .entrants
                    .iter()
                    .enumerate()
                    .skip(e.stages.len())
                    .find(|(_, s)| s.contains(&club))
                    .map(|(k, _)| k);
                match (last_with_club, enters) {
                    (Some(k), _) if k + 1 == e.stages.len() => stage_name(k),
                    (_, Some(k)) => format!("Enters in the {}", stage_name(k)),
                    _ => stage_name(e.stages.len()),
                }
            } else {
                let k = last_with_club.unwrap_or(0);
                if e.finished && k + 1 == e.stages.len() {
                    "Runners-up".to_owned()
                } else {
                    format!("Out in the {}", stage_name(k))
                }
            };
            Some(dto::CupStatus {
                cup: cup_ref(world, id),
                status,
                alive,
            })
        })
        .collect()
}

/// Trophies a club has won: league titles and cups.
pub fn honours(world: &World, club: ClubId) -> Vec<dto::Honour> {
    let mut out = Vec::new();
    for (_, l) in world.leagues().iter() {
        let seasons: Vec<String> = l
            .history()
            .iter()
            .filter(|s| s.champion() == Some(club))
            .map(|s| season_label(s.year))
            .collect();
        if !seasons.is_empty() {
            out.push(dto::Honour {
                competition: l.name.clone(),
                seasons,
            });
        }
    }
    for (_, c) in world.cups().iter() {
        let seasons: Vec<String> = c
            .history()
            .iter()
            .filter(|s| s.winner == Some(club))
            .map(|s| season_label(s.year))
            .collect();
        if !seasons.is_empty() {
            out.push(dto::Honour {
                competition: c.format.name.clone(),
                seasons,
            });
        }
    }
    out
}
