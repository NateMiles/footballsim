//! Tunable tier 1 constants.

use super::BANDS;

/// Tunable engine constants. [`Params::default`] holds the calibrated
/// values.
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// Logit change per unit of skill difference in a contest.
    pub skill_slope: f64,
    /// Logit change per unit of skill difference for short passes. Higher
    /// values let technically better teams dominate the ball.
    pub pass_slope: f64,
    /// Logit bonus for the home side's actions.
    pub home_logit: f64,
    /// Base logit of a short pass succeeding, by band.
    pub short_base: [f64; BANDS],
    pub long_base: f64,
    pub through_base: f64,
    pub cross_base: f64,
    pub dribble_base: f64,
    /// Probability per action of a foul away from the ball contest.
    pub foul_rate: f64,
    /// Probability a failed dribble is a foul.
    pub dribble_foul: f64,
    /// Probability a foul earns a yellow card, before temperament.
    pub card_rate: f64,
    pub straight_red: f64,
    /// Base xG by band (5 = penalty area) for central and wide lanes.
    pub xg_central: [f64; BANDS],
    pub xg_wide: [f64; BANDS],
    pub xg_big_chance: f64,
    pub xg_header: f64,
    pub xg_corner_header: f64,
    pub xg_penalty: f64,
    pub xg_free_kick: f64,
    /// Probability a shot is on target, before shooter skill.
    pub on_target_base: f64,
    /// Seconds a short pass takes.
    pub pass_secs: f64,
    /// Probability per action of an injury to an involved player.
    pub injury_rate: f64,
    /// Energy lost per minute on the pitch by an average player.
    pub energy_drain: f64,
    /// Outfielders below this energy are substituted when possible.
    pub sub_energy: f64,
    /// Share of short passes played forward a band.
    pub forward_rate: f64,
    /// Logit penalty for a forward pass (sideways passes get +0.2,
    /// backward +0.6).
    pub forward_risk: f64,
    /// Weight of a shot from the box, before player and tactic modifiers.
    pub box_shot_weight: f64,
    /// Keeper skill (after fatigue) that xG values assume.
    pub average_keeper: f64,
    /// How much each directness step changes the long-ball rate.
    pub direct_effect: f64,
    /// Logit pressure per pressing step in the opponent's half.
    pub press_effect: f64,
    /// Logit bonus near goal per step of the defending side's mentality.
    pub exposure_effect: f64,
    /// Change in the forward-pass share per directness step.
    pub direct_forward: f64,
    /// Depth spread of attacking involvement in the final third; larger
    /// values spread shots and goals beyond the strikers.
    pub final_third_spread: f64,
    /// Extra energy drain from chasing: a side that spends the whole
    /// minute without the ball drains `1 + chase_fatigue / 2` times as fast.
    pub chase_fatigue: f64,
}

impl Default for Params {
    /// Values fitted with `fs-lab`'s calibration report
    /// (docs/CALIBRATION.md).
    fn default() -> Self {
        Self {
            skill_slope: 3.6,
            pass_slope: 5.5,
            home_logit: 0.1,
            short_base: [3.8, 3.4, 1.4, 0.8, 0.5, 0.1],
            long_base: -0.6,
            through_base: -1.7,
            cross_base: -2.3,
            dribble_base: -0.1,
            foul_rate: 0.015,
            dribble_foul: 0.22,
            card_rate: 0.17,
            straight_red: 0.0015,
            xg_central: [0.0, 0.0, 0.0, 0.02, 0.045, 0.10],
            xg_wide: [0.0, 0.0, 0.0, 0.01, 0.025, 0.05],
            xg_big_chance: 0.3,
            xg_header: 0.07,
            xg_corner_header: 0.07,
            xg_penalty: 0.76,
            xg_free_kick: 0.055,
            on_target_base: 0.12,
            pass_secs: 3.3,
            injury_rate: 0.0012,
            energy_drain: 0.0045,
            sub_energy: 0.74,
            forward_rate: 0.25,
            forward_risk: 1.2,
            box_shot_weight: 0.16,
            average_keeper: 0.82,
            direct_effect: 0.6,
            press_effect: 0.15,
            exposure_effect: 0.12,
            chase_fatigue: 1.0,
            final_third_spread: 0.3,
            direct_forward: 0.0,
        }
    }
}
