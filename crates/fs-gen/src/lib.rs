//! Procedural generation of a fictional football world.
//!
//! Everything is derived from the config's seed, so the same config always
//! produces the same world, on every platform.

pub mod names;
pub mod scenario;

use fs_core::{GameDate, Rng, Seed, Stream};
use fs_world::attributes::{ATTR_MAX, Attr, HiddenAttr, Position};
use fs_world::player::CONDITION_MAX;
use fs_world::{
    Attributes, Club, ClubId, Condition, Control, Foot, Hidden, Manager, NameRef, Nation, NationId,
    PlayerBio, PlayerInit, PositionRatings, World,
};
use names::{NameStyle, STYLES};

/// What to generate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenConfig {
    pub seed: Seed,
    pub start: GameDate,
    pub nations: u16,
    pub clubs_per_nation: u16,
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
            players_per_club: 25,
            free_agents,
            human_clubs: vec![0],
        }
    }
}

/// First and last name pools for one nation.
struct NationNames {
    style: &'static NameStyle,
    first: Vec<NameRef>,
    last: Vec<NameRef>,
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
    let mut pools = Vec::with_capacity(usize::from(cfg.nations));
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
        world.add_nation(Nation {
            name,
            code: code.to_uppercase(),
            reputation,
        });
        let first = (0..300)
            .map(|_| world.names_mut().intern(&style.first_name(&mut nrng)))
            .collect();
        let last = (0..800)
            .map(|_| world.names_mut().intern(&style.surname(&mut nrng)))
            .collect();
        pools.push(NationNames { style, first, last });
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
                &pools[usize::from(n)],
                nation_rep,
                rank,
                cfg.clubs_per_nation,
            );
            let pool = &pools[usize::from(n)];
            let first = pool.first[crng.index(pool.first.len())];
            let last = pool.last[crng.index(pool.last.len())];
            world.add_manager(Manager {
                first_name: first,
                last_name: last,
                control: if cfg.human_clubs.contains(&index) {
                    Control::Human
                } else {
                    Control::Ai
                },
                club: Some(club_id),
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
                let p = make_player(
                    &mut crng,
                    cfg.start,
                    &pools,
                    nationality,
                    position,
                    level,
                    Some(club_id),
                    number,
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
        let p = make_player(
            &mut frng, cfg.start, &pools, nation, position, level, None, None,
        );
        world.add_player(p);
    }

    debug_assert_eq!(world.check_invariants(), Ok(()));
    world
}

fn add_club(
    world: &mut World,
    rng: &mut Rng,
    nation: NationId,
    pool: &NationNames,
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
    let town = pool.style.town(rng);
    let suffix = SUFFIXES[rng.index(SUFFIXES.len())];
    let name = match suffix {
        "Real" | "Sporting" | "Dynamo" | "Olympic" => format!("{suffix} {town}"),
        _ => format!("{town} {suffix}"),
    };
    // Reputation falls from the top of the national pyramid to the bottom.
    let drop = i32::from(rank) * nation_rep * 7 / (10 * i32::from(clubs.max(1)));
    let reputation = (nation_rep - drop + rng.range_i32(-400, 400)).clamp(200, 10_000) as u16;
    world.add_club(Club::new(name, town, nation, reputation))
}

/// Which attributes matter most for a position: (key, secondary).
fn profile(position: Position) -> (&'static [Attr], &'static [Attr]) {
    use Attr::*;
    use Position::*;
    match position {
        Goalkeeper => (
            &[
                Reflexes,
                Handling,
                OneOnOnes,
                AerialReach,
                CommandOfArea,
                Positioning,
            ],
            &[
                Kicking,
                Throwing,
                Communication,
                RushingOut,
                Concentration,
                Agility,
            ],
        ),
        DefenderCentre => (
            &[
                Tackling,
                Marking,
                Heading,
                Positioning,
                Strength,
                JumpingReach,
            ],
            &[Anticipation, Bravery, Concentration, Composure, Passing],
        ),
        DefenderRight | DefenderLeft | WingBackRight | WingBackLeft => (
            &[Tackling, Marking, Pace, Stamina, Crossing, Positioning],
            &[Acceleration, WorkRate, Dribbling, Teamwork, Anticipation],
        ),
        DefensiveMidfielder => (
            &[
                Tackling,
                Positioning,
                Passing,
                Anticipation,
                Teamwork,
                Stamina,
            ],
            &[Marking, Decisions, Concentration, Strength, WorkRate],
        ),
        MidfielderCentre => (
            &[Passing, Vision, Decisions, Technique, FirstTouch, Stamina],
            &[Teamwork, WorkRate, Composure, LongShots, Tackling],
        ),
        MidfielderRight | MidfielderLeft => (
            &[Crossing, Pace, Dribbling, Stamina, Passing, WorkRate],
            &[Acceleration, Technique, FirstTouch, OffTheBall, Teamwork],
        ),
        AttackingMidRight | AttackingMidLeft => (
            &[Dribbling, Pace, Acceleration, Technique, Flair, Crossing],
            &[Agility, FirstTouch, OffTheBall, Finishing, Passing],
        ),
        AttackingMidCentre => (
            &[Passing, Vision, Technique, FirstTouch, Flair, Decisions],
            &[Dribbling, LongShots, OffTheBall, Composure, Agility],
        ),
        Striker => (
            &[Finishing, OffTheBall, Composure, FirstTouch, Pace, Heading],
            &[Acceleration, Strength, Dribbling, Technique, Anticipation],
        ),
    }
}

/// Neighbouring positions a player can usually also play.
fn neighbours(position: Position) -> &'static [Position] {
    use Position::*;
    match position {
        Goalkeeper => &[],
        DefenderRight => &[WingBackRight, DefenderCentre, MidfielderRight],
        DefenderLeft => &[WingBackLeft, DefenderCentre, MidfielderLeft],
        DefenderCentre => &[DefensiveMidfielder, DefenderRight, DefenderLeft],
        WingBackRight => &[DefenderRight, MidfielderRight],
        WingBackLeft => &[DefenderLeft, MidfielderLeft],
        DefensiveMidfielder => &[MidfielderCentre, DefenderCentre],
        MidfielderCentre => &[DefensiveMidfielder, AttackingMidCentre],
        MidfielderRight => &[AttackingMidRight, WingBackRight, MidfielderCentre],
        MidfielderLeft => &[AttackingMidLeft, WingBackLeft, MidfielderCentre],
        AttackingMidRight => &[MidfielderRight, Striker, AttackingMidLeft],
        AttackingMidLeft => &[MidfielderLeft, Striker, AttackingMidRight],
        AttackingMidCentre => &[MidfielderCentre, Striker],
        Striker => &[AttackingMidCentre, AttackingMidRight, AttackingMidLeft],
    }
}

fn clamp_attr(x: f64) -> u16 {
    x.round().clamp(1.0, f64::from(ATTR_MAX)) as u16
}

#[allow(clippy::too_many_arguments)]
fn make_player(
    rng: &mut Rng,
    start: GameDate,
    pools: &[NationNames],
    nation: NationId,
    position: Position,
    level: i32,
    club: Option<ClubId>,
    squad_number: Option<u8>,
) -> PlayerInit {
    let pool = &pools[nation.index()];
    let age = rng.normal(25.5, 4.2).round().clamp(16.0, 38.0) as i32;
    let (y, m, d) = start.ymd();
    let birth_year = y - age - 1;
    let birth = GameDate::from_ymd(birth_year, m, d.min(28))
        .expect("valid birth date")
        .add_days(rng.below(365));

    // Overall ability on the 0-1000 scale. Young players have not peaked;
    // veterans have begun to decline physically.
    let mut ability = rng.normal(f64::from(level) * 0.85 + 60.0, 70.0);
    if age < 24 {
        ability -= f64::from(24 - age) * 22.0;
    }
    let ability = ability.clamp(60.0, 950.0);

    let (key, secondary) = profile(position);
    let is_gk = position == Position::Goalkeeper;
    let mut attributes = Attributes::default();
    for attr in Attr::ALL {
        let weight = if key.contains(&attr) {
            1.12
        } else if secondary.contains(&attr) {
            0.95
        } else if attr.group() == "goalkeeping" {
            if is_gk { 0.8 } else { 0.12 }
        } else if is_gk && attr.group() == "technical" {
            0.45
        } else {
            0.72
        };
        let mut value = rng.normal(ability * weight, 55.0);
        if attr.group() == "physical" && age > 30 && attr != Attr::Strength {
            value -= f64::from(age - 30) * 18.0;
        }
        attributes.set(attr, clamp_attr(value));
    }

    let mut hidden = Hidden::default();
    for h in HiddenAttr::ALL {
        hidden.set(h, clamp_attr(rng.normal(500.0, 160.0)));
    }

    let mut positions = PositionRatings::default();
    positions.set(position, 20);
    for &n in neighbours(position) {
        if rng.chance(0.6) {
            positions.set(n, 8 + rng.below(10) as u8);
        }
    }

    let foot = {
        let left_sided = matches!(
            position,
            Position::DefenderLeft
                | Position::WingBackLeft
                | Position::MidfielderLeft
                | Position::AttackingMidLeft
        );
        let roll = rng.unit_f64();
        let left_p = if left_sided { 0.65 } else { 0.18 };
        if roll < left_p {
            Foot::Left
        } else if roll < left_p + 0.07 {
            Foot::Both
        } else {
            Foot::Right
        }
    };
    let tall = matches!(position, Position::Goalkeeper | Position::DefenderCentre);
    let height = rng
        .normal(if tall { 188.0 } else { 179.0 }, 6.0)
        .round()
        .clamp(160.0, 205.0);
    let weight = (height - 105.0 + rng.normal(0.0, 4.0))
        .round()
        .clamp(55.0, 105.0);

    PlayerInit {
        bio: PlayerBio {
            first_name: pool.first[rng.index(pool.first.len())],
            last_name: pool.last[rng.index(pool.last.len())],
            birth_date: birth,
            nation,
            foot,
            height_cm: height as u8,
            weight_kg: weight as u8,
        },
        club,
        attributes,
        hidden,
        positions,
        condition: Condition {
            fitness: CONDITION_MAX - rng.below(800) as u16,
            sharpness: 4_000 + rng.below(4_000) as u16,
            fatigue: rng.below(1_000) as u16,
            morale: 4_500 + rng.below(1_000) as u16,
        },
        squad_number,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_world_is_consistent() {
        let cfg = GenConfig::preset(Size::Tiny, 1);
        let world = generate(&cfg);
        assert_eq!(world.check_invariants(), Ok(()));
        assert_eq!(world.clubs().len(), 20);
        assert_eq!(world.players().len(), 20 * 25 + 50);
        assert_eq!(world.human_managers().count(), 1);
        let first_club = &world.clubs()[ClubId::new(0)];
        assert_eq!(first_club.squad().len(), 25);
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
