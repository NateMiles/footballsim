//! How people look: the features a portrait is drawn from.
//!
//! Looks are derived on demand from the world seed and the person's id,
//! nation and age, so they cost nothing to store and never change the
//! world. Each nation has its own mix of skin tones and hair; age brings
//! grey hair and receding hairlines.

use crate::ids::{ManagerId, NationId, PlayerId, StaffId};
use crate::people::age_years;
use crate::world::World;
use fs_core::{Rng, Seed, Stream};

/// Number of skin tones, light to dark.
pub const SKIN_TONES: u8 = 8;

/// Hair styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hair {
    Bald,
    Buzz,
    Short,
    SidePart,
    Curly,
    Afro,
    Long,
    Bun,
    Receding,
    Mohawk,
}

/// Hair colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HairColour {
    Black,
    DarkBrown,
    Brown,
    LightBrown,
    Blond,
    Ginger,
    Grey,
    White,
}

/// Facial hair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beard {
    None,
    Stubble,
    Short,
    Full,
    Moustache,
    Goatee,
}

/// Eye colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeColour {
    DarkBrown,
    Brown,
    Green,
    Blue,
}

/// A face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    /// 0 (lightest) to `SKIN_TONES - 1`.
    pub skin: u8,
    /// Head shape, 0-3: narrow, oval, round, square.
    pub face: u8,
    pub hair: Hair,
    pub hair_colour: HairColour,
    pub beard: Beard,
    pub eyes: EyeColour,
    /// Brow weight, 0-2.
    pub brows: u8,
    /// Nose shape, 0-2.
    pub nose: u8,
    /// Mouth width, 0-2.
    pub mouth: u8,
}

/// Who a look belongs to; keeps the streams of different people apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    Player(PlayerId),
    Manager(ManagerId),
    Staff(StaffId),
    /// A journalist, by index.
    Journalist(u32),
}

impl Who {
    fn key(self) -> (u64, u64) {
        match self {
            Who::Player(p) => (1, u64::from(p.raw())),
            Who::Manager(m) => (2, u64::from(m.raw())),
            Who::Staff(s) => (3, u64::from(s.raw())),
            Who::Journalist(j) => (4, u64::from(j)),
        }
    }
}

/// A nation's typical skin tone (a centre and a spread) and how often
/// its people have light hair or eyes.
fn nation_mix(seed: Seed, nation: Option<NationId>) -> (f64, f64, f64) {
    let Some(n) = nation else {
        return (3.5, 2.4, 0.3);
    };
    let mut rng = seed.derive(Stream::Looks, 0, u64::from(n.raw()));
    let centre = rng.range_f64(0.5, f64::from(SKIN_TONES) - 1.5);
    // Most nations are mixed; some much more than others.
    let spread = rng.range_f64(0.9, 2.6);
    let light = (1.0 - centre / f64::from(SKIN_TONES)).clamp(0.0, 1.0) * rng.range_f64(0.3, 0.9);
    (centre, spread, light)
}

fn pick<T: Copy>(rng: &mut Rng, items: &[(T, f64)]) -> T {
    let weights: Vec<f64> = items.iter().map(|&(_, w)| w.max(0.0)).collect();
    items[rng.weighted_index(&weights).unwrap_or(0)].0
}

/// The look of anyone, from the seed, who they are, their nation and age.
pub fn look(seed: Seed, who: Who, nation: Option<NationId>, age: f64) -> Look {
    let (kind, id) = who.key();
    let mut rng = seed.derive(Stream::Looks, kind, id);
    let (centre, spread, light) = nation_mix(seed, nation);
    let skin = rng
        .normal(centre, spread * 0.5)
        .round()
        .clamp(0.0, f64::from(SKIN_TONES - 1)) as u8;
    let dark = f64::from(skin) / f64::from(SKIN_TONES - 1);

    let greying = ((age - 32.0) / 30.0).clamp(0.0, 0.9);
    let hair_colour = if rng.chance(greying) {
        if age > 58.0 && rng.chance(0.4) {
            HairColour::White
        } else {
            HairColour::Grey
        }
    } else {
        let fair = light * (1.0 - dark);
        pick(
            &mut rng,
            &[
                (HairColour::Black, 0.35 + dark * 2.0),
                (HairColour::DarkBrown, 0.9),
                (HairColour::Brown, 0.6 + fair),
                (HairColour::LightBrown, fair * 1.4),
                (HairColour::Blond, fair * 1.2),
                (HairColour::Ginger, fair * 0.35),
            ],
        )
    };

    let thinning = ((age - 26.0) / 40.0).clamp(0.0, 0.6);
    let hair = if rng.chance(thinning) {
        if rng.chance(0.45) {
            Hair::Bald
        } else {
            Hair::Receding
        }
    } else {
        pick(
            &mut rng,
            &[
                (Hair::Buzz, 1.2),
                (Hair::Short, 1.6),
                (Hair::SidePart, 1.0 - dark * 0.4),
                (Hair::Curly, 0.5 + dark * 0.6),
                (Hair::Afro, dark * dark * 1.0),
                (Hair::Long, 0.35),
                (Hair::Bun, 0.25),
                (Hair::Mohawk, 0.15),
                (Hair::Bald, 0.15),
            ],
        )
    };

    let beard = pick(
        &mut rng,
        &[
            (Beard::None, 2.2 - (age / 40.0).min(1.2)),
            (Beard::Stubble, 1.0),
            (Beard::Short, 0.6 + age / 80.0),
            (Beard::Full, 0.2 + age / 60.0),
            (Beard::Moustache, 0.08 + age / 200.0),
            (Beard::Goatee, 0.25),
        ],
    );

    let eyes = if rng.chance(light * (1.0 - dark) * 0.8) {
        if rng.chance(0.55) {
            EyeColour::Blue
        } else {
            EyeColour::Green
        }
    } else if rng.chance(0.5) {
        EyeColour::Brown
    } else {
        EyeColour::DarkBrown
    };

    Look {
        skin,
        face: rng.below(4) as u8,
        hair,
        hair_colour,
        beard,
        eyes,
        brows: rng.below(3) as u8,
        nose: rng.below(3) as u8,
        mouth: rng.below(3) as u8,
    }
}

/// A player's look.
pub fn player(world: &World, p: PlayerId) -> Look {
    let bio = world.players().bio(p);
    look(
        world.seed(),
        Who::Player(p),
        Some(bio.nation),
        age_years(bio.birth_date, world.date()),
    )
}

/// A manager's look. Managers have no recorded age; they are drawn in
/// their forties, from their club's nation when they have one.
pub fn manager(world: &World, m: ManagerId) -> Look {
    let nation = world.managers()[m].club.map(|c| world.clubs()[c].nation);
    let mut rng = world.seed().derive(Stream::Looks, 5, u64::from(m.raw()));
    let age = rng.range_f64(36.0, 60.0);
    look(world.seed(), Who::Manager(m), nation, age)
}

/// A staff member's look.
pub fn staff(world: &World, s: StaffId) -> Look {
    let st = &world.staff()[s];
    look(
        world.seed(),
        Who::Staff(s),
        Some(st.nation),
        age_years(st.birth_date, world.date()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_are_stable_and_varied() {
        let seed = Seed(11);
        let a = look(seed, Who::Journalist(3), None, 30.0);
        assert_eq!(a, look(seed, Who::Journalist(3), None, 30.0));
        let all: Vec<Look> = (0..400)
            .map(|i| look(seed, Who::Journalist(i), None, 25.0))
            .collect();
        let tones: std::collections::BTreeSet<u8> = all.iter().map(|l| l.skin).collect();
        assert!(tones.len() >= 6, "{tones:?}");
        let bald = all.iter().filter(|l| l.hair == Hair::Bald).count();
        assert!(bald < 80, "{bald} bald 25-year-olds");
        let old: Vec<Look> = (0..400)
            .map(|i| look(seed, Who::Journalist(i), None, 65.0))
            .collect();
        let grey = old
            .iter()
            .filter(|l| matches!(l.hair_colour, HairColour::Grey | HairColour::White))
            .count();
        assert!(grey > 200, "{grey} grey 65-year-olds");
    }
}
