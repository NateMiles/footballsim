//! Match skills: attributes combined into the handful of numbers the
//! engines actually use. All skills are on a 0-1 scale.

use crate::input::PlayerSnapshot;
use fs_core::attributes::{Attr, Attributes, HiddenAttr, Position};

/// A player's skills for one match, before fatigue.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Skills {
    pub pass_short: f64,
    pub pass_long: f64,
    pub through: f64,
    pub cross: f64,
    pub dribble: f64,
    pub shoot: f64,
    pub long_shot: f64,
    pub header: f64,
    pub off_ball: f64,
    pub tackle: f64,
    pub defend: f64,
    pub aerial_def: f64,
    pub keeper: f64,
    pub keeper_aerial: f64,
    pub penalty: f64,
    pub free_kick: f64,
    pub corner: f64,
    /// How readily the player commits fouls (0 never, 1 constantly).
    pub foul_tendency: f64,
    /// How likely a foul becomes a card (0 calm, 1 hot-headed).
    pub hot_head: f64,
    /// Endurance: slows the in-match energy drain.
    pub stamina: f64,
    pub injury_proneness: f64,
    /// Composite used for team selection and summaries.
    pub overall: f64,
}

fn weighted(a: &Attributes, parts: &[(Attr, f64)]) -> f64 {
    parts
        .iter()
        .map(|&(attr, w)| f64::from(a.get(attr)) * w)
        .sum::<f64>()
        / 1000.0
}

/// Multiplier for playing out of position: 1.0 when natural (20), 0.7
/// when completely unfamiliar (0).
pub fn familiarity_factor(familiarity: u8) -> f64 {
    0.7 + 0.3 * f64::from(familiarity.min(20)) / 20.0
}

impl Skills {
    /// Skills of `p` playing in `position`, including familiarity,
    /// sharpness and morale, but not in-match fatigue.
    pub fn of(p: &PlayerSnapshot, position: Position) -> Self {
        use Attr::*;
        let a = &p.attributes;
        let h = |x: HiddenAttr| f64::from(p.hidden.get(x)) / 1000.0;
        let form = familiarity_factor(p.positions.get(position))
            * (0.94 + 0.06 * f64::from(p.sharpness) / 10_000.0)
            * (0.97 + 0.06 * f64::from(p.morale) / 10_000.0);
        let f = |x: f64| x * form;
        Self {
            pass_short: f(weighted(
                a,
                &[
                    (Passing, 0.4),
                    (Technique, 0.2),
                    (Decisions, 0.2),
                    (FirstTouch, 0.1),
                    (Composure, 0.1),
                ],
            )),
            pass_long: f(weighted(
                a,
                &[
                    (Passing, 0.4),
                    (Vision, 0.3),
                    (Technique, 0.2),
                    (Decisions, 0.1),
                ],
            )),
            through: f(weighted(
                a,
                &[
                    (Vision, 0.45),
                    (Passing, 0.35),
                    (Technique, 0.1),
                    (Flair, 0.1),
                ],
            )),
            cross: f(weighted(
                a,
                &[(Crossing, 0.7), (Technique, 0.2), (Decisions, 0.1)],
            )),
            dribble: f(weighted(
                a,
                &[
                    (Dribbling, 0.35),
                    (Technique, 0.15),
                    (Agility, 0.15),
                    (Balance, 0.1),
                    (Acceleration, 0.1),
                    (Pace, 0.1),
                    (Flair, 0.05),
                ],
            )),
            shoot: f(weighted(
                a,
                &[
                    (Finishing, 0.45),
                    (Composure, 0.25),
                    (Technique, 0.15),
                    (Decisions, 0.15),
                ],
            )),
            long_shot: f(weighted(
                a,
                &[(LongShots, 0.6), (Technique, 0.25), (Composure, 0.15)],
            )),
            header: f(weighted(
                a,
                &[
                    (Heading, 0.45),
                    (JumpingReach, 0.35),
                    (Strength, 0.1),
                    (Bravery, 0.1),
                ],
            )),
            off_ball: f(weighted(
                a,
                &[
                    (OffTheBall, 0.5),
                    (Anticipation, 0.2),
                    (Pace, 0.15),
                    (Acceleration, 0.15),
                ],
            )),
            tackle: f(weighted(
                a,
                &[
                    (Tackling, 0.4),
                    (Positioning, 0.2),
                    (Anticipation, 0.15),
                    (Strength, 0.1),
                    (Bravery, 0.1),
                    (Aggression, 0.05),
                ],
            )),
            defend: f(weighted(
                a,
                &[
                    (Positioning, 0.3),
                    (Anticipation, 0.25),
                    (Marking, 0.2),
                    (Concentration, 0.15),
                    (WorkRate, 0.1),
                ],
            )),
            aerial_def: f(weighted(
                a,
                &[
                    (Heading, 0.35),
                    (JumpingReach, 0.35),
                    (Marking, 0.15),
                    (Strength, 0.15),
                ],
            )),
            keeper: f(weighted(
                a,
                &[
                    (Reflexes, 0.3),
                    (Handling, 0.2),
                    (OneOnOnes, 0.2),
                    (Positioning, 0.15),
                    (Agility, 0.15),
                ],
            )),
            keeper_aerial: f(weighted(
                a,
                &[(AerialReach, 0.4), (CommandOfArea, 0.4), (Handling, 0.2)],
            )),
            penalty: weighted(a, &[(Penalties, 0.7), (Composure, 0.3)]),
            free_kick: weighted(a, &[(FreeKicks, 0.7), (Technique, 0.3)]),
            corner: weighted(a, &[(Corners, 0.7), (Technique, 0.3)]),
            foul_tendency: 0.5 * f64::from(a.get(Aggression)) / 1000.0
                + 0.5 * h(HiddenAttr::Dirtiness),
            hot_head: 1.0 - h(HiddenAttr::Temperament),
            stamina: weighted(a, &[(Stamina, 0.7), (NaturalFitness, 0.3)]),
            injury_proneness: h(HiddenAttr::InjuryProneness),
            overall: f(f64::from(position.role_rating(a)) / 1000.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_core::attributes::{ATTR_COUNT, Hidden, PositionRatings};
    use fs_core::ids::PlayerId;

    fn player(level: u16, natural: Position) -> PlayerSnapshot {
        let mut positions = PositionRatings::default();
        positions.set(natural, 20);
        PlayerSnapshot {
            id: PlayerId::new(0),
            attributes: Attributes([level; ATTR_COUNT]),
            hidden: Hidden::default(),
            positions,
            fitness: 10_000,
            sharpness: 10_000,
            morale: 5_000,
        }
    }

    #[test]
    fn skills_scale_with_attributes() {
        let good = Skills::of(&player(800, Position::Striker), Position::Striker);
        let poor = Skills::of(&player(300, Position::Striker), Position::Striker);
        assert!(good.shoot > poor.shoot + 0.4);
        assert!(good.shoot <= 1.0 && poor.shoot >= 0.0);
    }

    #[test]
    fn out_of_position_is_worse() {
        let p = player(700, Position::Striker);
        let natural = Skills::of(&p, Position::Striker);
        let wrong = Skills::of(&p, Position::DefenderCentre);
        assert!(wrong.tackle < natural.tackle * 0.8);
    }
}
