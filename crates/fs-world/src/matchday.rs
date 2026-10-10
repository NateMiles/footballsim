//! Bridges the world and the match engine: snapshots of players, and team
//! selection for a club.

use crate::ids::{ClubId, PlayerId};
use crate::world::World;
use fs_core::Stream;
use fs_match::{Formation, PlayerSnapshot, Tactic, TeamSheet, pick_team};

/// A player's state at kickoff.
pub fn snapshot(world: &World, id: PlayerId) -> PlayerSnapshot {
    let p = world.players();
    let c = p.condition(id);
    PlayerSnapshot {
        id,
        attributes: *p.attributes(id),
        hidden: *p.hidden(id),
        positions: *p.positions(id),
        fitness: c.fitness,
        sharpness: c.sharpness,
        morale: c.morale,
    }
}

/// Snapshots of a club's whole squad.
pub fn squad(world: &World, club: ClubId) -> Vec<PlayerSnapshot> {
    world.clubs()[club]
        .squad()
        .iter()
        .map(|&p| snapshot(world, p))
        .collect()
}

/// The club's preferred tactic until AI managers choose their own (M2):
/// the formation that fits the squad best, with sliders that give each
/// club a stable, distinct style.
pub fn default_tactic(world: &World, club: ClubId, squad: &[PlayerSnapshot]) -> Tactic {
    let mut rng = world.seed().derive(Stream::Ai, u64::from(club.raw()), 1);
    let mut slider = || rng.range_i32(-1, 1) as i8;
    let style = Tactic {
        formation: Formation::F442,
        mentality: slider(),
        tempo: slider(),
        width: slider(),
        pressing: slider(),
        defensive_line: slider(),
        directness: slider(),
    };
    let formation = Formation::ALL
        .into_iter()
        .max_by(|&a, &b| {
            sheet_quality(squad, a)
                .total_cmp(&sheet_quality(squad, b))
                .then((b as u8).cmp(&(a as u8)))
        })
        .unwrap_or(Formation::F442);
    Tactic { formation, ..style }
}

fn sheet_quality(squad: &[PlayerSnapshot], formation: Formation) -> f64 {
    let sheet = pick_team(squad, Tactic::balanced(formation));
    let slots = formation.slots();
    sheet
        .starters
        .iter()
        .zip(slots)
        .map(|(p, s)| fs_match::selection::suitability(p, s.position))
        .sum()
}

/// The club's team sheet with its default tactic.
///
/// # Panics
/// Panics if the club has fewer than eleven players.
pub fn team_sheet(world: &World, club: ClubId) -> TeamSheet {
    let squad = squad(world, club);
    let tactic = default_tactic(world, club, &squad);
    pick_team(&squad, tactic)
}
