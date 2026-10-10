//! Procedural generation of a fictional football world.
//!
//! Everything is derived from the config's seed, so the same config always
//! produces the same world, on every platform.

pub mod names;
pub mod scenario;

use fs_core::{GameDate, Rng, Seed, Stream};
use fs_world::attributes::Position;
use fs_world::people::{Age, PlayerSpec, new_player};
use fs_world::{Club, ClubId, Control, League, Manager, NameRef, Nation, NationId, World};
use names::{NameStyle, STYLES};

/// What to generate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenConfig {
    pub seed: Seed,
    pub start: GameDate,
    pub nations: u16,
    pub clubs_per_nation: u16,
    /// Clubs per division; each nation's clubs are split into divisions
    /// of this size, strongest first.
    pub clubs_per_league: u16,
    pub players_per_club: u16,
    pub free_agents: u32,
    /// Clubs (by creation index) whose managers are human.
    pub human_clubs: Vec<u32>,
}

/// Preset world sizes, matching the memory budgets in
/// `docs/ARCHITECTURE.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// One nation, 20 clubs. For tests and quick experiments.
    Tiny,
    /// About 60,000 players. Default for the browser build.
    Small,
    /// About 150,000 players.
    Medium,
    /// About 500,000 players. Native builds only.
    Large,
}

impl GenConfig {
    /// A preset world with one human-managed club (the first).
    pub fn preset(size: Size, seed: u64) -> Self {
        let (nations, clubs_per_nation, free_agents) = match size {
            Size::Tiny => (1, 20, 50),
            Size::Small => (24, 100, 2_000),
            Size::Medium => (60, 100, 5_000),
            Size::Large => (100, 200, 20_000),
        };
        Self {
            seed: Seed(seed),
            start: GameDate::from_ymd(2026, 7, 1).expect("valid date"),
            nations,
            clubs_per_nation,
            clubs_per_league: 20,
            players_per_club: 25,
            free_agents,
            human_clubs: vec![0],
        }
    }
}

/// Positions filled, in order, when building a squad. Larger squads wrap
/// around.
const SQUAD_TEMPLATE: [Position; 25] = {
    use Position::*;
    [
        Goalkeeper,
        DefenderRight,
        DefenderCentre,
        DefenderCentre,
        DefenderLeft,
        DefensiveMidfielder,
        MidfielderCentre,
        MidfielderCentre,
        AttackingMidRight,
        Striker,
        AttackingMidLeft,
        Goalkeeper,
        DefenderCentre,
        DefenderRight,
        DefenderLeft,
        DefensiveMidfielder,
        MidfielderCentre,
        MidfielderRight,
        MidfielderLeft,
        AttackingMidCentre,
        Striker,
        DefenderCentre,
        MidfielderCentre,
        AttackingMidCentre,
        Goalkeeper,
    ]
};

/// Generates a world.
///
/// # Panics
/// Panics if `human_clubs` names a club index that is not generated.
pub fn generate(cfg: &GenConfig) -> World {
    let mut world = World::new(cfg.seed, cfg.start);

    // Nations and their name pools.
    let mut styles: Vec<&NameStyle> = Vec::with_capacity(usize::from(cfg.nations));
    for n in 0..cfg.nations {
        let style = &STYLES[usize::from(n) % STYLES.len()];
        let mut nrng = cfg.seed.derive(Stream::Generation, 1, u64::from(n));
        let name = style.town(&mut nrng);
        let code: String = name
            .chars()
            .filter(char::is_ascii_alphabetic)
            .take(3)
            .collect();
        // Strongest nations first, with some noise.
        let reputation = (9_000 - i32::from(n) * 6_000 / i32::from(cfg.nations.max(1))
            + nrng.range_i32(-500, 500))
        .clamp(1_000, 10_000) as u16;
        let first_names: Vec<NameRef> = (0..300)
            .map(|_| world.names_mut().intern(&style.first_name(&mut nrng)))
            .collect();
        let last_names: Vec<NameRef> = (0..800)
            .map(|_| world.names_mut().intern(&style.surname(&mut nrng)))
            .collect();
        world.add_nation(Nation {
            name,
            code: code.to_uppercase(),
            reputation,
            first_names,
            last_names,
        });
        styles.push(style);
    }

    // Clubs, managers and squads.
    for n in 0..cfg.nations {
        let nation = NationId::new(u32::from(n));
        let nation_rep = i32::from(world.nations()[nation].reputation);
        for rank in 0..cfg.clubs_per_nation {
            let index = world.clubs().len() as u32;
            let mut crng = cfg.seed.derive(Stream::Generation, 2, u64::from(index));
            let club_id = add_club(
                &mut world,
                &mut crng,
                nation,
                styles[usize::from(n)],
                nation_rep,
                rank,
                cfg.clubs_per_nation,
            );
            let nat = &world.nations()[nation];
            let first = nat.first_names[crng.index(nat.first_names.len())];
            let last = nat.last_names[crng.index(nat.last_names.len())];
            world.add_manager(Manager {
                first_name: first,
                last_name: last,
                control: if cfg.human_clubs.contains(&index) {
                    Control::Human
                } else {
                    Control::Ai
                },
                club: Some(club_id),
                delegate_squad: false,
                watch_matches: false,
            });
            let level = i32::from(world.clubs()[club_id].reputation) / 10;
            for slot in 0..cfg.players_per_club {
                let position = SQUAD_TEMPLATE[usize::from(slot) % SQUAD_TEMPLATE.len()];
                let number = u8::try_from(slot + 1).ok().filter(|n| *n <= 99);
                let nationality = if crng.chance(0.85) || cfg.nations == 1 {
                    nation
                } else {
                    NationId::new(crng.below(u32::from(cfg.nations)))
                };
                let p = new_player(
                    world.nations(),
                    &mut crng,
                    cfg.start,
                    PlayerSpec {
                        nation: nationality,
                        position,
                        level,
                        age: Age::Senior,
                        club: Some(club_id),
                        squad_number: number,
                    },
                );
                world.add_player(p);
            }
            // The academy: about three players per year group, 15 to 17.
            for k in 0..9u8 {
                let position = SQUAD_TEMPLATE[crng.index(SQUAD_TEMPLATE.len())];
                let p = new_player(
                    world.nations(),
                    &mut crng,
                    cfg.start,
                    PlayerSpec {
                        nation,
                        position,
                        level,
                        age: Age::Youth(15 + k / 3),
                        club: Some(club_id),
                        squad_number: u8::try_from(cfg.players_per_club + 1 + u16::from(k))
                            .ok()
                            .filter(|n| *n <= 99),
                    },
                );
                world.add_player(p);
            }
        }
    }
    for &h in &cfg.human_clubs {
        assert!(
            (h as usize) < world.clubs().len(),
            "human club {h} does not exist"
        );
    }

    // Unattached players.
    for i in 0..cfg.free_agents {
        let mut frng = cfg.seed.derive(Stream::Generation, 3, u64::from(i));
        let nation = NationId::new(frng.below(u32::from(cfg.nations)));
        let position = SQUAD_TEMPLATE[frng.index(SQUAD_TEMPLATE.len())];
        let level = frng.range_i32(150, 550);
        let p = new_player(
            world.nations(),
            &mut frng,
            cfg.start,
            PlayerSpec {
                nation,
                position,
                level,
                age: Age::Senior,
                club: None,
                squad_number: None,
            },
        );
        world.add_player(p);
    }

    // Divisions: each nation's clubs, strongest first, in leagues of
    // `clubs_per_league`.
    let per = usize::from(cfg.clubs_per_league.max(2));
    let mut first_club = 0;
    for n in 0..cfg.nations {
        let nation = NationId::new(u32::from(n));
        let clubs: Vec<ClubId> = (first_club..first_club + usize::from(cfg.clubs_per_nation))
            .map(|c| ClubId::new(c as u32))
            .collect();
        first_club += usize::from(cfg.clubs_per_nation);
        for (level, chunk) in clubs.chunks(per).enumerate() {
            let name = format!(
                "{} {} Division",
                world.nations()[nation].name,
                ORDINALS.get(level).copied().unwrap_or("Lower")
            );
            world.add_league(League::new(name, nation, level as u8 + 1, 3), chunk);
        }
    }
    fs_world::season::start_first_season(&mut world);

    debug_assert_eq!(world.check_invariants(), Ok(()));
    world
}

const ORDINALS: [&str; 10] = [
    "First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth", "Ninth", "Tenth",
];

fn add_club(
    world: &mut World,
    rng: &mut Rng,
    nation: NationId,
    style: &NameStyle,
    nation_rep: i32,
    rank: u16,
    clubs: u16,
) -> ClubId {
    const SUFFIXES: &[&str] = &[
        "FC",
        "United",
        "City",
        "Athletic",
        "Rovers",
        "Town",
        "Wanderers",
        "Sporting",
        "Albion",
        "Rangers",
        "Dynamo",
        "Real",
        "Olympic",
    ];
    let town = style.town(rng);
    let suffix = SUFFIXES[rng.index(SUFFIXES.len())];
    let name = match suffix {
        "Real" | "Sporting" | "Dynamo" | "Olympic" => format!("{suffix} {town}"),
        _ => format!("{town} {suffix}"),
    };
    // Reputation falls down the national pyramid at a fixed rate per
    // place, as if the nation had 100 clubs: a 20-club world is a top
    // division, not a whole country squeezed into one league.
    let _ = clubs;
    let drop = i32::from(rank) * nation_rep * 7 / 1000;
    let reputation = (nation_rep - drop + rng.range_i32(-400, 400)).clamp(200, 10_000) as u16;
    world.add_club(Club::new(name, town, nation, reputation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_world::attributes::Attr;

    #[test]
    fn tiny_world_is_consistent() {
        let cfg = GenConfig::preset(Size::Tiny, 1);
        let world = generate(&cfg);
        assert_eq!(world.check_invariants(), Ok(()));
        assert_eq!(world.clubs().len(), 20);
        // 25 seniors and a 9-player academy per club, plus free agents.
        assert_eq!(world.players().len(), 20 * (25 + 9) + 50);
        assert_eq!(world.human_managers().count(), 1);
        let first_club = &world.clubs()[ClubId::new(0)];
        assert_eq!(first_club.squad().len(), 34);
        assert_eq!(world.leagues().len(), 1);
        assert_eq!(
            world.leagues()[fs_world::LeagueId::new(0)].clubs().len(),
            20
        );
        assert!(
            world.leagues()[fs_world::LeagueId::new(0)]
                .season()
                .is_some()
        );
    }

    #[test]
    fn same_seed_same_world() {
        let cfg = GenConfig::preset(Size::Tiny, 77);
        assert_eq!(generate(&cfg).state_hash(), generate(&cfg).state_hash());
        let other = GenConfig::preset(Size::Tiny, 78);
        assert_ne!(generate(&cfg).state_hash(), generate(&other).state_hash());
    }

    #[test]
    fn stronger_clubs_have_better_players() {
        let world = generate(&GenConfig {
            clubs_per_nation: 40,
            ..GenConfig::preset(Size::Tiny, 3)
        });
        let avg = |c: u32| {
            let club = &world.clubs()[ClubId::new(c)];
            let total: u32 = club
                .squad()
                .iter()
                .map(|&p| u32::from(world.players().attributes(p).get(Attr::Decisions)))
                .sum();
            total / club.squad().len() as u32
        };
        let top: u32 = (0..5).map(avg).sum::<u32>() / 5;
        let bottom: u32 = (35..40).map(avg).sum::<u32>() / 5;
        assert!(top > bottom + 100, "top {top} bottom {bottom}");
    }
}
