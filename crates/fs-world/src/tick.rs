//! The daily simulation step.
//!
//! M0 version: physical and mental recovery between matches. Training,
//! injuries and development arrive in M2. Everything here uses integer
//! maths and per-player RNG streams, so it parallelises without changing
//! results.

use crate::attributes::{ATTR_MAX, Attr, HiddenAttr};
use crate::player::{CONDITION_MAX, Condition, Players};
use crate::world::World;
use fs_core::Stream;
use fs_core::par;

/// Morale drifts back toward this value.
const MORALE_BASELINE: u16 = CONDITION_MAX / 2;
/// Sharpness never decays below this without match play.
const SHARPNESS_FLOOR: u16 = 2_000;

/// Runs one day for the whole world. `world.date` is already the new day.
pub(crate) fn daily(world: &mut World) {
    let seed = world.seed;
    let day = u64::from(world.date.days());
    let Players {
        condition,
        attributes,
        hidden,
        ..
    } = &mut world.players;

    par::for_each_indexed_mut(condition, |i, c| {
        let mut rng = seed.derive(Stream::DailyTick, day, i as u64);
        let natural_fitness = u32::from(attributes[i].get(Attr::NaturalFitness));
        let professionalism = u32::from(hidden[i].get(HiddenAttr::Professionalism));
        recover(c, natural_fitness, professionalism, &mut rng);
    });
}

/// One day of recovery for one player. Attribute inputs are 0-1000.
fn recover(c: &mut Condition, natural_fitness: u32, professionalism: u32, rng: &mut fs_core::Rng) {
    let max = u32::from(CONDITION_MAX);
    let attr_max = u32::from(ATTR_MAX);

    // Fitness closes 15% to 30% of the gap to full each day, faster with
    // better natural fitness, so a tired player is fresh within a week.
    let fitness = u32::from(c.fitness);
    let rate = 150 + natural_fitness * 150 / attr_max; // per mille
    let gain = (max - fitness) * rate / 1000 + rng.below(20);
    c.fitness = (fitness + gain).min(max) as u16;

    // Fatigue sheds 5% to 9% a day.
    let fatigue = u32::from(c.fatigue);
    let shed = fatigue * (50 + natural_fitness * 40 / attr_max) / 1000 + 1;
    c.fatigue = fatigue.saturating_sub(shed) as u16;

    // Sharpness fades slowly without games; professional players keep it
    // better.
    let sharpness = u32::from(c.sharpness);
    if sharpness > u32::from(SHARPNESS_FLOOR) {
        let loss = sharpness * (6 - professionalism * 3 / attr_max) / 1000;
        c.sharpness = sharpness
            .saturating_sub(loss)
            .max(u32::from(SHARPNESS_FLOOR)) as u16;
    }

    // Morale reverts 2% toward neutral each day, plus small noise.
    let morale = i32::from(c.morale);
    let baseline = i32::from(MORALE_BASELINE);
    let pull = (baseline - morale) * 2 / 100;
    let noise = rng.range_i32(-30, 30);
    c.morale = (morale + pull + noise).clamp(0, max as i32) as u16;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_core::Rng;

    #[test]
    fn tired_player_recovers() {
        let mut c = Condition {
            fitness: 6_000,
            sharpness: 9_000,
            fatigue: 5_000,
            morale: 5_000,
        };
        let mut rng = Rng::from_u64(1);
        for _ in 0..14 {
            recover(&mut c, 800, 500, &mut rng);
        }
        assert!(c.fitness > 9_400, "{c:?}");
        assert!(c.fatigue < 2_000, "{c:?}");
        assert!(
            c.sharpness < 9_000 && c.sharpness >= SHARPNESS_FLOOR,
            "{c:?}"
        );
        assert!((4_000..=6_000).contains(&c.morale), "{c:?}");
    }

    #[test]
    fn values_stay_in_range() {
        let mut rng = Rng::from_u64(2);
        for nf in [0, 500, 1000] {
            let mut c = Condition {
                fitness: CONDITION_MAX,
                sharpness: SHARPNESS_FLOOR,
                fatigue: 0,
                morale: CONDITION_MAX,
            };
            for _ in 0..1000 {
                recover(&mut c, nf, nf, &mut rng);
                assert!(c.fitness <= CONDITION_MAX);
                assert!(c.morale <= CONDITION_MAX);
                assert!(c.sharpness >= SHARPNESS_FLOOR);
            }
        }
    }
}
