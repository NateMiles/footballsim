//! Team strength summaries. Tier 2 turns these into expected goals; the
//! calibration tools regress tier 1 results on them.

use crate::input::TeamSheet;
use crate::skills::Skills;
use serde::{Deserialize, Serialize};

/// A team's strength in a few numbers, each roughly 0-1.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TeamStrength {
    /// Chance creation: passing, dribbling and movement, weighted toward
    /// the front of the formation.
    pub attack: f64,
    /// Chance prevention, weighted toward the back.
    pub defence: f64,
    /// Ball retention across the outfield.
    pub control: f64,
    /// Finishing of the forward players.
    pub finishing: f64,
    pub keeper: f64,
    /// Average foul tendency.
    pub discipline: f64,
    /// Average endurance.
    pub stamina: f64,
    /// Average suitability of the eleven for their positions.
    pub overall: f64,
}

impl TeamStrength {
    /// Summarises a team sheet.
    pub fn of(sheet: &TeamSheet) -> Self {
        Self::with_skills(sheet).0
    }

    /// Summarises a team sheet and also returns each starter's skills in
    /// their slot, so callers need not compute them again.
    pub fn with_skills(sheet: &TeamSheet) -> (Self, Vec<Skills>) {
        let slots = sheet.tactic.formation.slots();
        let mut all = Vec::with_capacity(11);
        let mut acc = Acc::default();
        let mut keeper = 0.0;
        let mut overall = Mean::default();
        for (i, p) in sheet.starters.iter().enumerate().take(11) {
            let s = Skills::of(p, slots[i].position);
            all.push(s);
            let fat = 0.82 + 0.18 * f64::from(p.fitness) / 10_000.0;
            overall.push(1.0, s.overall * fat);
            if i == 0 {
                keeper = s.keeper * fat;
                continue;
            }
            let x = slots[i].x;
            let front = x * x;
            let back = (1.0 - x) * (1.0 - x);
            let mid = fs_core::math::exp(-((x - 0.5) * (x - 0.5)) / 0.08);
            acc.attack.push(
                front,
                (0.3 * s.pass_short
                    + 0.2 * s.through
                    + 0.2 * s.dribble
                    + 0.2 * s.shoot
                    + 0.1 * s.off_ball)
                    * fat,
            );
            acc.defence.push(
                back,
                (0.45 * s.defend + 0.35 * s.tackle + 0.2 * s.aerial_def) * fat,
            );
            acc.control.push(mid + 0.2, s.pass_short * fat);
            acc.finishing.push(front * x, s.shoot * fat);
            acc.discipline.push(1.0, s.foul_tendency);
            acc.stamina.push(1.0, s.stamina);
        }
        let strength = Self {
            attack: acc.attack.mean(),
            defence: acc.defence.mean(),
            control: acc.control.mean(),
            finishing: acc.finishing.mean(),
            keeper,
            discipline: acc.discipline.mean(),
            stamina: acc.stamina.mean(),
            overall: overall.mean(),
        };
        (strength, all)
    }
}

#[derive(Default)]
struct Mean {
    sum: f64,
    weight: f64,
}

impl Mean {
    fn push(&mut self, w: f64, v: f64) {
        self.sum += w * v;
        self.weight += w;
    }
    fn mean(&self) -> f64 {
        if self.weight > 0.0 {
            self.sum / self.weight
        } else {
            0.0
        }
    }
}

#[derive(Default)]
struct Acc {
    attack: Mean,
    defence: Mean,
    control: Mean,
    finishing: Mean,
    discipline: Mean,
    stamina: Mean,
}
