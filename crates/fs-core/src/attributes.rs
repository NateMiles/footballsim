//! Player attributes and positions.
//!
//! True attribute values are hidden and stored on a 0-1000 scale. The UI
//! shows the familiar 1-20 scale, and later shows it as a club-specific
//! scouting estimate rather than the truth (see `docs/PLAN.md`).

use serde::{Deserialize, Serialize};

/// Highest value on the internal attribute scale.
pub const ATTR_MAX: u16 = 1000;

macro_rules! attr_enum {
    (
        $(#[$meta:meta])*
        $name:ident, $count:ident, $all:ident {
            $($group:ident: [$($variant:ident => $label:literal),* $(,)?]),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[repr(u8)]
        pub enum $name {
            $($($variant,)*)*
        }

        /// Number of variants.
        pub const $count: usize = [$($($name::$variant,)*)*].len();

        impl $name {
            /// Every variant, in declaration order.
            pub const $all: [$name; $count] = [$($($name::$variant,)*)*];

            /// Display label.
            pub const fn label(self) -> &'static str {
                match self {
                    $($($name::$variant => $label,)*)*
                }
            }

            /// Group the attribute belongs to.
            pub const fn group(self) -> &'static str {
                match self {
                    $($($name::$variant => stringify!($group),)*)*
                }
            }
        }
    };
}

attr_enum! {
    /// A visible player attribute. The numeric order is part of the save
    /// format: only ever append new variants.
    Attr, ATTR_COUNT, ALL {
        technical: [
            Corners => "Corners",
            Crossing => "Crossing",
            Dribbling => "Dribbling",
            Finishing => "Finishing",
            FirstTouch => "First Touch",
            FreeKicks => "Free Kicks",
            Heading => "Heading",
            LongShots => "Long Shots",
            LongThrows => "Long Throws",
            Marking => "Marking",
            Passing => "Passing",
            Penalties => "Penalties",
            Tackling => "Tackling",
            Technique => "Technique",
        ],
        mental: [
            Aggression => "Aggression",
            Anticipation => "Anticipation",
            Bravery => "Bravery",
            Composure => "Composure",
            Concentration => "Concentration",
            Decisions => "Decisions",
            Determination => "Determination",
            Flair => "Flair",
            Leadership => "Leadership",
            OffTheBall => "Off the Ball",
            Positioning => "Positioning",
            Teamwork => "Teamwork",
            Vision => "Vision",
            WorkRate => "Work Rate",
        ],
        physical: [
            Acceleration => "Acceleration",
            Agility => "Agility",
            Balance => "Balance",
            JumpingReach => "Jumping Reach",
            NaturalFitness => "Natural Fitness",
            Pace => "Pace",
            Stamina => "Stamina",
            Strength => "Strength",
        ],
        goalkeeping: [
            AerialReach => "Aerial Reach",
            CommandOfArea => "Command of Area",
            Communication => "Communication",
            Eccentricity => "Eccentricity",
            Handling => "Handling",
            Kicking => "Kicking",
            OneOnOnes => "One on Ones",
            Punching => "Punching",
            Reflexes => "Reflexes",
            RushingOut => "Rushing Out",
            Throwing => "Throwing",
        ],
    }
}

attr_enum! {
    /// A hidden personality or medical trait. Never shown directly; the UI
    /// only hints at these through staff reports.
    HiddenAttr, HIDDEN_COUNT, ALL {
        hidden: [
            Adaptability => "Adaptability",
            Ambition => "Ambition",
            Consistency => "Consistency",
            Dirtiness => "Dirtiness",
            ImportantMatches => "Important Matches",
            InjuryProneness => "Injury Proneness",
            Loyalty => "Loyalty",
            Pressure => "Pressure",
            Professionalism => "Professionalism",
            Temperament => "Temperament",
            Versatility => "Versatility",
        ],
    }
}

macro_rules! attr_array {
    ($(#[$meta:meta])* $name:ident, $key:ty, $count:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(#[serde(with = "serde_big_array::BigArray")] pub [u16; $count]);

        impl Default for $name {
            fn default() -> Self {
                Self([ATTR_MAX / 2; $count])
            }
        }

        impl $name {
            /// Value on the internal 0-1000 scale.
            #[inline]
            pub fn get(&self, key: $key) -> u16 {
                self.0[key as usize]
            }

            /// Sets a value, clamped to 0-1000.
            #[inline]
            pub fn set(&mut self, key: $key, value: u16) {
                self.0[key as usize] = value.min(ATTR_MAX);
            }

            /// Value on the 1-20 display scale.
            #[inline]
            pub fn display(&self, key: $key) -> u8 {
                to_display(self.get(key))
            }
        }
    };
}

attr_array!(
    /// A player's true visible attributes, 0-1000.
    Attributes, Attr, ATTR_COUNT
);
attr_array!(
    /// A player's true hidden traits, 0-1000.
    Hidden, HiddenAttr, HIDDEN_COUNT
);

/// Converts an internal 0-1000 value to the 1-20 display scale.
#[inline]
pub fn to_display(v: u16) -> u8 {
    let v = u32::from(v.min(ATTR_MAX));
    (1 + (v * 19 + 500) / 1000) as u8
}

/// A position on the pitch. The numeric order is part of the save format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Position {
    Goalkeeper,
    DefenderRight,
    DefenderCentre,
    DefenderLeft,
    WingBackRight,
    WingBackLeft,
    DefensiveMidfielder,
    MidfielderRight,
    MidfielderCentre,
    MidfielderLeft,
    AttackingMidRight,
    AttackingMidCentre,
    AttackingMidLeft,
    Striker,
}

/// Number of positions.
pub const POSITION_COUNT: usize = 14;

impl Position {
    /// Every position, in declaration order.
    pub const ALL: [Position; POSITION_COUNT] = [
        Position::Goalkeeper,
        Position::DefenderRight,
        Position::DefenderCentre,
        Position::DefenderLeft,
        Position::WingBackRight,
        Position::WingBackLeft,
        Position::DefensiveMidfielder,
        Position::MidfielderRight,
        Position::MidfielderCentre,
        Position::MidfielderLeft,
        Position::AttackingMidRight,
        Position::AttackingMidCentre,
        Position::AttackingMidLeft,
        Position::Striker,
    ];

    /// Short code such as `"DC"`.
    pub const fn code(self) -> &'static str {
        match self {
            Position::Goalkeeper => "GK",
            Position::DefenderRight => "DR",
            Position::DefenderCentre => "DC",
            Position::DefenderLeft => "DL",
            Position::WingBackRight => "WBR",
            Position::WingBackLeft => "WBL",
            Position::DefensiveMidfielder => "DM",
            Position::MidfielderRight => "MR",
            Position::MidfielderCentre => "MC",
            Position::MidfielderLeft => "ML",
            Position::AttackingMidRight => "AMR",
            Position::AttackingMidCentre => "AMC",
            Position::AttackingMidLeft => "AML",
            Position::Striker => "ST",
        }
    }

    /// Attributes that matter most in this position, then the ones that
    /// matter somewhat.
    pub const fn profile(self) -> (&'static [Attr], &'static [Attr]) {
        use Attr::*;
        use Position::*;
        match self {
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
    pub const fn neighbours(self) -> &'static [Position] {
        use Position::*;
        match self {
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

    /// Key attributes for this position.
    pub const fn key_attributes(self) -> &'static [Attr] {
        self.profile().0
    }

    /// Secondary attributes for this position.
    pub const fn secondary_attributes(self) -> &'static [Attr] {
        self.profile().1
    }

    /// How good a player with these attributes is in this position, on the
    /// 0-1000 scale, ignoring familiarity. Key attributes count three times,
    /// secondary attributes twice.
    pub fn role_rating(self, attrs: &Attributes) -> u16 {
        let (key, secondary) = self.profile();
        let mut total = 0u32;
        for &a in key {
            total += 3 * u32::from(attrs.get(a));
        }
        for &a in secondary {
            total += 2 * u32::from(attrs.get(a));
        }
        let weight = 3 * key.len() as u32 + 2 * secondary.len() as u32;
        (total / weight) as u16
    }
}

/// How familiar a player is with each position, 0 (never) to 20 (natural).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionRatings(pub [u8; POSITION_COUNT]);

impl PositionRatings {
    /// Familiarity with a position.
    pub fn get(&self, p: Position) -> u8 {
        self.0[p as usize]
    }

    /// Sets familiarity, clamped to 0-20.
    pub fn set(&mut self, p: Position, v: u8) {
        self.0[p as usize] = v.min(20);
    }

    /// The most familiar position (the first one on ties).
    pub fn best(&self) -> Position {
        let mut best = Position::Goalkeeper;
        for p in Position::ALL {
            if self.get(p) > self.get(best) {
                best = p;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        assert_eq!(ATTR_COUNT, 47);
        assert_eq!(HIDDEN_COUNT, 11);
        assert_eq!(Attr::ALL[Attr::Pace as usize], Attr::Pace);
        assert_eq!(Attr::Reflexes.group(), "goalkeeping");
        assert_eq!(Position::ALL.len(), POSITION_COUNT);
        for (i, p) in Position::ALL.iter().enumerate() {
            assert_eq!(*p as usize, i);
        }
    }

    #[test]
    fn display_scale() {
        assert_eq!(to_display(0), 1);
        assert_eq!(to_display(1000), 20);
        assert_eq!(to_display(500), 11);
        assert_eq!(to_display(5000), 20);
    }

    #[test]
    fn set_clamps() {
        let mut a = Attributes::default();
        a.set(Attr::Pace, 4000);
        assert_eq!(a.get(Attr::Pace), ATTR_MAX);
    }

    #[test]
    fn role_rating_weights_key_attributes() {
        let mut a = Attributes([100; ATTR_COUNT]);
        for &k in Position::Striker.key_attributes() {
            a.set(k, 900);
        }
        assert!(Position::Striker.role_rating(&a) > Position::Goalkeeper.role_rating(&a) + 400);
        for p in Position::ALL {
            assert!(!p.key_attributes().is_empty());
        }
    }

    #[test]
    fn best_position() {
        let mut r = PositionRatings::default();
        r.set(Position::Striker, 20);
        r.set(Position::AttackingMidCentre, 15);
        assert_eq!(r.best(), Position::Striker);
    }
}
