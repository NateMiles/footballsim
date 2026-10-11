//! Squad, player and tactics views.

use super::refs::*;
use crate::dto;
use fs_core::attributes::{Attr, Position};
use fs_match::Formation;
use fs_match::selection::{BENCH_SIZE, suitability};
use fs_world::matchday::{snapshot, team_sheet};
use fs_world::people::wage_demand;
use fs_world::scouting;
use fs_world::{ClubId, Foot, PlayerId, Status, World};

/// Players selected today, as (starters, bench).
fn selected(world: &World, club: ClubId) -> (Vec<PlayerId>, Vec<PlayerId>) {
    if world.clubs()[club].squad().len() < 11 {
        return (Vec::new(), Vec::new());
    }
    let sheet = team_sheet(world, club);
    (
        sheet.starters.iter().map(|p| p.id).collect(),
        sheet.bench.iter().map(|p| p.id).collect(),
    )
}

pub fn squad(world: &World, club: ClubId) -> dto::SquadView {
    let (starters, bench) = selected(world, club);
    let p = world.players();
    let players = world.clubs()[club]
        .squad()
        .iter()
        .map(|&id| {
            let c = p.condition(id);
            let age = age(world, id);
            dto::SquadPlayer {
                id: id.raw(),
                name: player_ref(world, id).name,
                number: p.squad_number(id),
                age,
                nation: world.nations()[p.bio(id).nation].code.clone(),
                position: p.positions(id).best().code().into(),
                positions: good_positions(world, id),
                ability: percent_of_1000(p.ability(id)),
                potential: percent_of_1000(p.potential(id)),
                fitness: percent_of_10000(c.fitness),
                sharpness: percent_of_10000(c.sharpness),
                morale: percent_of_10000(c.morale),
                injury: injury(world, id),
                suspension: p.suspension(id),
                contract: contract(world, id),
                season: p.season_stats(id).into(),
                selection: if starters.contains(&id) {
                    dto::Selection::Starter
                } else if bench.contains(&id) {
                    dto::Selection::Bench
                } else {
                    dto::Selection::Reserve
                },
                academy: age < 18,
            }
        })
        .collect();
    dto::SquadView {
        club: club_ref(world, club),
        players,
        honours: super::cup::honours(world, club),
    }
}

pub fn player(world: &World, id: PlayerId, viewer: Option<ClubId>) -> dto::PlayerView {
    let p = world.players();
    let (first_name, last_name) = player_names(world, id);
    let bio = p.bio(id);
    let c = p.condition(id);
    let attrs = p.attributes(id);
    // What the viewer's club believes; exact for its own players and for
    // a viewer without a club.
    let estimate = viewer.map(|c| scouting::estimate(world, c, id));
    let mut groups: Vec<dto::AttributeGroup> = Vec::new();
    let is_keeper = p.positions(id).best() == Position::Goalkeeper;
    for (k, a) in Attr::ALL.into_iter().enumerate() {
        let group = a.group();
        if group == "goalkeeping" && !is_keeper {
            continue;
        }
        let name = capitalise(group);
        if groups.last().is_none_or(|g| g.name != name) {
            groups.push(dto::AttributeGroup {
                name,
                attributes: Vec::new(),
            });
        }
        groups
            .last_mut()
            .expect("pushed above")
            .attributes
            .push(match &estimate {
                Some(e) => dto::AttributeValue {
                    name: a.label().into(),
                    value: e.attributes[k].value as u8,
                    low: e.attributes[k].low as u8,
                    high: e.attributes[k].high as u8,
                },
                None => {
                    let v = attrs.display(a);
                    dto::AttributeValue {
                        name: a.label().into(),
                        value: v,
                        low: v,
                        high: v,
                    }
                }
            });
    }
    let age = age(world, id);
    let exact = |v: u16| scouting::Range {
        value: v,
        low: v,
        high: v,
    };
    let (ability, potential) = match &estimate {
        Some(e) => (e.ability, e.potential),
        None => (
            exact(p.ability(id)),
            exact(p.potential(id).max(p.ability(id))),
        ),
    };
    dto::PlayerView {
        id: id.raw(),
        name: format!("{first_name} {last_name}"),
        first_name,
        last_name,
        age,
        birth_date: date(bio.birth_date),
        nation: world.nations()[bio.nation].name.clone(),
        club: p.club(id).map(|c| club_ref(world, c)),
        number: p.squad_number(id),
        foot: match bio.foot {
            Foot::Left => "left",
            Foot::Right => "right",
            Foot::Both => "both",
        }
        .into(),
        height_cm: bio.height_cm,
        weight_kg: bio.weight_kg,
        position: p.positions(id).best().code().into(),
        position_ratings: Position::ALL
            .iter()
            .map(|&pos| p.positions(id).get(pos))
            .collect(),
        attributes: groups,
        knowledge: estimate.as_ref().map_or(100, |e| e.knowledge),
        ability: percent_of_1000(ability.value),
        ability_low: percent_of_1000(ability.low),
        ability_high: percent_of_1000(ability.high),
        potential: percent_of_1000(potential.value),
        potential_low: percent_of_1000(potential.low),
        potential_high: percent_of_1000(potential.high),
        value: if p.club(id).is_some() {
            super::market::estimated_value(
                world,
                id,
                f64::from(ability.value),
                f64::from(potential.value),
            )
        } else {
            0
        },
        asking_price: p
            .club(id)
            .filter(|&c| Some(c) != viewer && world.ai_runs_squad(c))
            .and_then(|c| fs_world::market::asking_price(world, c, id)),
        listing: world.market().listing(id).map(Into::into),
        loan: world
            .market()
            .loan(id)
            .map(|l| super::market::loan(world, id, l)),
        scouting: viewer.is_some_and(|c| world.scouting().assignments(c).contains(&id)),
        offer: viewer.and_then(|c| {
            world
                .market()
                .open_offer(c, id)
                .map(|o| super::market::offer(world, o))
        }),
        fitness: percent_of_10000(c.fitness),
        sharpness: percent_of_10000(c.sharpness),
        morale: percent_of_10000(c.morale),
        injury: injury(world, id),
        suspension: p.suspension(id),
        contract: contract(world, id),
        wage_demand: wage_demand(p.ability(id), p.potential(id), u32::from(age)),
        season: p.season_stats(id).into(),
        career: p
            .career(id)
            .iter()
            .map(|s| dto::CareerRow {
                season: season_label(s.year),
                club: s.club.map(|c| club_ref(world, c)),
                ability: percent_of_1000(s.ability),
                stats: (&s.stats).into(),
            })
            .collect(),
        retired: p.status(id) == Status::Retired,
        own: viewer.is_some() && p.club(id) == viewer,
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Full position names, in [`Position::ALL`] order.
pub fn positions() -> Vec<dto::PositionInfo> {
    Position::ALL
        .iter()
        .map(|&p| dto::PositionInfo {
            code: p.code().into(),
            name: position_name(p).into(),
        })
        .collect()
}

fn position_name(p: Position) -> &'static str {
    match p {
        Position::Goalkeeper => "Goalkeeper",
        Position::DefenderRight => "Right back",
        Position::DefenderCentre => "Centre back",
        Position::DefenderLeft => "Left back",
        Position::WingBackRight => "Right wing-back",
        Position::WingBackLeft => "Left wing-back",
        Position::DefensiveMidfielder => "Defensive midfielder",
        Position::MidfielderRight => "Right midfielder",
        Position::MidfielderCentre => "Central midfielder",
        Position::MidfielderLeft => "Left midfielder",
        Position::AttackingMidRight => "Right winger",
        Position::AttackingMidCentre => "Attacking midfielder",
        Position::AttackingMidLeft => "Left winger",
        Position::Striker => "Striker",
    }
}

/// Every formation's shape.
pub fn formations() -> Vec<dto::FormationInfo> {
    Formation::ALL
        .iter()
        .map(|&f| dto::FormationInfo {
            formation: f.into(),
            slots: f
                .slots()
                .iter()
                .map(|s| dto::SlotInfo {
                    position: s.position as u8,
                    x: s.x as f32,
                    y: s.y as f32,
                })
                .collect(),
        })
        .collect()
}

/// The tactic sliders.
pub fn sliders() -> Vec<dto::SliderInfo> {
    let s = |key: &str, name: &str, low: &str, high: &str| dto::SliderInfo {
        key: key.into(),
        name: name.into(),
        low: low.into(),
        high: high.into(),
    };
    vec![
        s("mentality", "Mentality", "Defensive", "Attacking"),
        s("tempo", "Tempo", "Patient", "Fast"),
        s("width", "Width", "Narrow", "Wide"),
        s("pressing", "Pressing", "Low block", "High press"),
        s("defensiveLine", "Defensive line", "Deep", "High"),
        s("directness", "Passing", "Short", "Direct"),
    ]
}

pub fn tactics(world: &World, club: ClubId) -> dto::TacticsView {
    let sheet = team_sheet(world, club);
    let c = &world.clubs()[club];
    let today = world.date();
    let players = c
        .squad()
        .iter()
        .map(|&id| {
            let snap = snapshot(world, id);
            let r = player_ref(world, id);
            dto::SelectablePlayer {
                id: id.raw(),
                name: r.name,
                short_name: r.short_name,
                number: r.number,
                position: r.position,
                available: world.players().available(id, today),
                fitness: percent_of_10000(snap.fitness),
                suitability: Position::ALL
                    .iter()
                    .map(|&pos| (suitability(&snap, pos) / 10.0).clamp(0.0, 100.0) as u8)
                    .collect(),
            }
        })
        .collect();
    dto::TacticsView {
        club: club_ref(world, club),
        tactic: sheet.tactic.into(),
        own_tactic: c.manager_tactic().is_some(),
        own_lineup: c.lineup().is_some(),
        starters: sheet.starters.iter().map(|p| p.id.raw()).collect(),
        bench: sheet.bench.iter().map(|p| p.id.raw()).collect(),
        players,
        formations: formations(),
        positions: positions(),
        sliders: sliders(),
        bench_size: BENCH_SIZE as u8,
    }
}
