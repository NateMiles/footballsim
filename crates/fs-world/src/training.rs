//! Training: what a club works on between matches, and how well it knows
//! its tactical shapes.
//!
//! A club's schedule has a focus and an intensity. The focus shifts which
//! attributes grow, without changing how much a player grows in all (the
//! emphasis is normalised), or trades growth for freshness; harder
//! training grows players faster but tires them and injures more. AI
//! clubs train balanced at normal intensity, which leaves the world as it
//! was without training.
//!
//! Each club knows each formation to a degree (0-1000). The shape it plays
//! is drilled every day and unused shapes fade; a team in a shape it has
//! not drilled plays a little below itself ([`cohesion`]).

use crate::attributes::{ATTR_COUNT, Attr, Position};
use crate::ids::ClubId;
use crate::world::World;
use fs_match::Formation;
use serde::{Deserialize, Serialize};

/// What the week's training concentrates on.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Focus {
    #[default]
    Balanced,
    /// Pace, stamina, strength: the physical attributes.
    Fitness,
    /// Finishing, dribbling, passing and movement.
    Attacking,
    /// Tackling, marking, positioning and heading.
    Defending,
    /// Shape and decisions: mental attributes, and formations learnt twice
    /// as fast.
    Tactics,
    /// Rest: less growth, faster recovery, fewer knocks.
    Recovery,
}

impl Focus {
    pub const ALL: [Focus; 6] = [
        Focus::Balanced,
        Focus::Fitness,
        Focus::Attacking,
        Focus::Defending,
        Focus::Tactics,
        Focus::Recovery,
    ];

    /// Attributes the focus works on.
    fn emphasised(self, attr: Attr) -> bool {
        use Attr::*;
        match self {
            Focus::Fitness => attr.group() == "physical",
            Focus::Attacking => matches!(
                attr,
                Finishing
                    | Dribbling
                    | Passing
                    | Crossing
                    | FirstTouch
                    | Technique
                    | LongShots
                    | OffTheBall
                    | Composure
                    | Flair
                    | Vision
            ),
            Focus::Defending => matches!(
                attr,
                Tackling
                    | Marking
                    | Positioning
                    | Heading
                    | Concentration
                    | Anticipation
                    | Bravery
                    | Aggression
                    | JumpingReach
                    | Strength
            ),
            Focus::Tactics => matches!(
                attr,
                Decisions | Positioning | Teamwork | Anticipation | OffTheBall | Vision | WorkRate
            ),
            Focus::Balanced | Focus::Recovery => false,
        }
    }

    /// Growth weight per attribute, averaging 1 over all attributes, so a
    /// focus moves growth between attributes without adding any.
    pub fn weights(self) -> [f64; ATTR_COUNT] {
        let raw: [f64; ATTR_COUNT] = std::array::from_fn(|i| {
            if self.emphasised(Attr::ALL[i]) {
                1.6
            } else {
                1.0
            }
        });
        // Balanced sums to exactly ATTR_COUNT, so its weights stay exactly 1.
        let mean = raw.iter().sum::<f64>() / ATTR_COUNT as f64;
        raw.map(|w| w / mean)
    }
}

/// How hard the squad trains.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Intensity {
    Light,
    #[default]
    Normal,
    Hard,
}

impl Intensity {
    pub const ALL: [Intensity; 3] = [Intensity::Light, Intensity::Normal, Intensity::Hard];
}

/// A club's training schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Schedule {
    pub focus: Focus,
    pub intensity: Intensity,
}

impl Schedule {
    /// Development speed: 1 for balanced, normal training.
    pub fn growth(self) -> f64 {
        let i = match self.intensity {
            Intensity::Light => 0.85,
            Intensity::Normal => 1.0,
            Intensity::Hard => 1.12,
        };
        if self.focus == Focus::Recovery {
            i * 0.8
        } else {
            i
        }
    }

    /// Training injury risk: 1 for normal training.
    pub fn injury_risk(self) -> f64 {
        let i = match self.intensity {
            Intensity::Light => 0.7,
            Intensity::Normal => 1.0,
            Intensity::Hard => 1.5,
        };
        if self.focus == Focus::Recovery {
            i * 0.8
        } else {
            i
        }
    }

    /// Daily fitness recovery, per mille of the usual rate.
    pub fn recovery_per_mille(self) -> u32 {
        let i = match self.intensity {
            Intensity::Light => 1150,
            Intensity::Normal => 1000,
            Intensity::Hard => 850,
        };
        match self.focus {
            Focus::Recovery => i * 125 / 100,
            Focus::Fitness => i * 105 / 100,
            _ => i,
        }
    }

    /// Daily gain in familiarity with the formation in use.
    pub fn drill(self) -> u16 {
        let base = match self.intensity {
            Intensity::Light => 9,
            Intensity::Normal => 12,
            Intensity::Hard => 15,
        };
        match self.focus {
            Focus::Tactics => base * 2,
            Focus::Recovery => base / 2,
            _ => base,
        }
    }
}

/// What one player works on besides the squad's schedule.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Individual {
    /// Learn to play a position (familiarity rises a point a month, up to
    /// natural).
    Position(Position),
    /// Work on one attribute: it grows half as fast again, the rest a
    /// little slower.
    Attribute(Attr),
}

impl Individual {
    /// Growth weights for this player's attributes, starting from the
    /// squad's.
    pub fn weights(self, squad: [f64; ATTR_COUNT]) -> [f64; ATTR_COUNT] {
        match self {
            Individual::Position(_) => squad,
            Individual::Attribute(a) => {
                let mut w = squad.map(|x| x * 0.97);
                w[a as usize] = squad[a as usize] * 1.5;
                w
            }
        }
    }
}

/// How well a club knows each formation, 0-1000, in [`Formation::ALL`]
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Familiarity(pub [u16; Formation::ALL.len()]);

/// Fully drilled.
pub const DRILLED: u16 = 1000;
/// Familiarity at or above which a shape costs nothing.
pub const SETTLED: u16 = 900;
/// Unused shapes fade no lower than this.
pub const FLOOR: u16 = 300;

impl Default for Familiarity {
    fn default() -> Self {
        Self([FLOOR; Formation::ALL.len()])
    }
}

impl Familiarity {
    pub fn get(&self, f: Formation) -> u16 {
        self.0[f as usize]
    }

    fn set(&mut self, f: Formation, v: u16) {
        self.0[f as usize] = v.min(DRILLED);
    }
}

/// Team cohesion in a formation, as the match engine takes it (0-10000):
/// full from [`SETTLED`] familiarity up, down to none at zero.
pub fn cohesion(familiarity: u16) -> u16 {
    let f = u32::from(familiarity.min(SETTLED));
    (f * 10_000 / u32::from(SETTLED)) as u16
}

/// A new world: each club knows the formation it starts with.
pub fn initialise(world: &mut World) {
    let clubs: Vec<ClubId> = world.clubs.ids().collect();
    for c in clubs {
        if let Some(t) = world.clubs[c].tactic() {
            world.clubs[c].familiarity.set(t.formation, DRILLED);
        }
    }
}

/// Daily: each club drills the shape it plays; the others fade.
pub(crate) fn daily(world: &mut World) {
    for club in world.clubs.as_mut_slice() {
        let Some(current) = club.tactic() else {
            continue;
        };
        let gain = club.training.drill();
        for f in Formation::ALL {
            let v = club.familiarity.get(f);
            if f == current.formation {
                club.familiarity.set(f, v.saturating_add(gain));
            } else if v > FLOOR {
                club.familiarity.set(f, (v - 2).max(FLOOR));
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // exactly 1 is the point: AI clubs must train as before
mod tests {
    use super::*;

    #[test]
    fn focus_weights_average_one() {
        for f in Focus::ALL {
            let w = f.weights();
            let mean = w.iter().sum::<f64>() / w.len() as f64;
            assert!((mean - 1.0).abs() < 1e-12, "{f:?}");
        }
        assert!(Focus::Balanced.weights().iter().all(|&w| w == 1.0));
        let att = Focus::Attacking.weights();
        assert!(att[Attr::Finishing as usize] > att[Attr::Tackling as usize]);
    }

    #[test]
    fn normal_balanced_training_changes_nothing() {
        let s = Schedule::default();
        assert_eq!(s.growth(), 1.0);
        assert_eq!(s.injury_risk(), 1.0);
        assert_eq!(s.recovery_per_mille(), 1000);
    }

    #[test]
    fn cohesion_is_full_once_settled() {
        assert_eq!(cohesion(DRILLED), 10_000);
        assert_eq!(cohesion(SETTLED), 10_000);
        assert!(cohesion(FLOOR) < 4_000);
        assert_eq!(cohesion(0), 0);
    }
}
