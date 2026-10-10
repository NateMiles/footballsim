//! Formations: eleven slots, each with a position and a typical location
//! in the team's attacking shape.

use fs_core::attributes::Position;
use serde::{Deserialize, Serialize};

/// A slot in a formation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub position: Position,
    /// Depth in the attacking shape: 0 is the team's own goal line, 1 the
    /// opponent's.
    pub x: f64,
    /// Width: 0 is the left touchline, 1 the right.
    pub y: f64,
}

/// A team shape. The numeric order is part of the save format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Formation {
    F442,
    F433,
    F4231,
    F352,
    F532,
    F4141,
}

const fn s(position: Position, x: f64, y: f64) -> Slot {
    Slot { position, x, y }
}

use Position::*;

const GK: Slot = s(Goalkeeper, 0.02, 0.5);
const BACK_FOUR: [Slot; 4] = [
    s(DefenderLeft, 0.26, 0.1),
    s(DefenderCentre, 0.17, 0.36),
    s(DefenderCentre, 0.17, 0.64),
    s(DefenderRight, 0.26, 0.9),
];

impl Formation {
    /// Every formation.
    pub const ALL: [Formation; 6] = [
        Formation::F442,
        Formation::F433,
        Formation::F4231,
        Formation::F352,
        Formation::F532,
        Formation::F4141,
    ];

    /// Display name such as `"4-2-3-1"`.
    pub const fn name(self) -> &'static str {
        match self {
            Formation::F442 => "4-4-2",
            Formation::F433 => "4-3-3",
            Formation::F4231 => "4-2-3-1",
            Formation::F352 => "3-5-2",
            Formation::F532 => "5-3-2",
            Formation::F4141 => "4-1-4-1",
        }
    }

    /// The eleven slots. Slot 0 is always the goalkeeper.
    pub const fn slots(self) -> [Slot; 11] {
        let [dl, dc1, dc2, dr] = BACK_FOUR;
        match self {
            Formation::F442 => [
                GK,
                dl,
                dc1,
                dc2,
                dr,
                s(MidfielderLeft, 0.56, 0.1),
                s(MidfielderCentre, 0.5, 0.38),
                s(MidfielderCentre, 0.5, 0.62),
                s(MidfielderRight, 0.56, 0.9),
                s(Striker, 0.84, 0.38),
                s(Striker, 0.84, 0.62),
            ],
            Formation::F433 => [
                GK,
                dl,
                dc1,
                dc2,
                dr,
                s(DefensiveMidfielder, 0.4, 0.5),
                s(MidfielderCentre, 0.55, 0.3),
                s(MidfielderCentre, 0.55, 0.7),
                s(AttackingMidLeft, 0.78, 0.12),
                s(Striker, 0.87, 0.5),
                s(AttackingMidRight, 0.78, 0.88),
            ],
            Formation::F4231 => [
                GK,
                dl,
                dc1,
                dc2,
                dr,
                s(DefensiveMidfielder, 0.4, 0.38),
                s(DefensiveMidfielder, 0.4, 0.62),
                s(AttackingMidLeft, 0.72, 0.12),
                s(AttackingMidCentre, 0.68, 0.5),
                s(AttackingMidRight, 0.72, 0.88),
                s(Striker, 0.87, 0.5),
            ],
            Formation::F352 => [
                GK,
                s(DefenderCentre, 0.18, 0.25),
                s(DefenderCentre, 0.15, 0.5),
                s(DefenderCentre, 0.18, 0.75),
                s(WingBackLeft, 0.52, 0.05),
                s(MidfielderCentre, 0.5, 0.32),
                s(DefensiveMidfielder, 0.4, 0.5),
                s(MidfielderCentre, 0.5, 0.68),
                s(WingBackRight, 0.52, 0.95),
                s(Striker, 0.84, 0.4),
                s(Striker, 0.84, 0.6),
            ],
            Formation::F532 => [
                GK,
                s(WingBackLeft, 0.36, 0.05),
                s(DefenderCentre, 0.17, 0.28),
                s(DefenderCentre, 0.15, 0.5),
                s(DefenderCentre, 0.17, 0.72),
                s(WingBackRight, 0.36, 0.95),
                s(MidfielderCentre, 0.5, 0.3),
                s(DefensiveMidfielder, 0.42, 0.5),
                s(MidfielderCentre, 0.5, 0.7),
                s(Striker, 0.82, 0.4),
                s(Striker, 0.82, 0.6),
            ],
            Formation::F4141 => [
                GK,
                dl,
                dc1,
                dc2,
                dr,
                s(DefensiveMidfielder, 0.36, 0.5),
                s(MidfielderLeft, 0.6, 0.1),
                s(MidfielderCentre, 0.55, 0.36),
                s(MidfielderCentre, 0.55, 0.64),
                s(MidfielderRight, 0.6, 0.9),
                s(Striker, 0.87, 0.5),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_formation_has_one_keeper_first() {
        for f in Formation::ALL {
            let slots = f.slots();
            assert_eq!(slots[0].position, Goalkeeper, "{}", f.name());
            assert_eq!(slots.iter().filter(|s| s.position == Goalkeeper).count(), 1);
            for s in slots {
                assert!((0.0..=1.0).contains(&s.x) && (0.0..=1.0).contains(&s.y));
            }
        }
    }
}
