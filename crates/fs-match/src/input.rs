//! What a match needs to know: who plays, how, and where.

use crate::formation::Formation;
use fs_core::attributes::{Attributes, Hidden, PositionRatings};
use fs_core::ids::PlayerId;
use fs_core::{Rng, Seed, Stream};
use serde::{Deserialize, Serialize};

/// A player as the match engine sees them: a copy of the relevant state
/// at kickoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: PlayerId,
    pub attributes: Attributes,
    pub hidden: Hidden,
    pub positions: PositionRatings,
    /// 0-10000, 10000 fully fit.
    pub fitness: u16,
    /// 0-10000.
    pub sharpness: u16,
    /// 0-10000, 5000 neutral.
    pub morale: u16,
}

/// Team instructions. Every slider runs from -2 to +2; 0 is balanced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tactic {
    pub formation: Formation,
    /// Defensive (-2) to attacking (+2): risk taking and shot appetite.
    pub mentality: i8,
    /// Patient (-2) to fast (+2) ball circulation.
    pub tempo: i8,
    /// Narrow (-2) to wide (+2) attacking play.
    pub width: i8,
    /// Low block (-2) to high press (+2).
    pub pressing: i8,
    /// Deep (-2) to high (+2) defensive line.
    pub defensive_line: i8,
    /// Short passing (-2) to direct long balls (+2).
    pub directness: i8,
}

impl Tactic {
    /// A balanced tactic in the given formation.
    pub const fn balanced(formation: Formation) -> Self {
        Self {
            formation,
            mentality: 0,
            tempo: 0,
            width: 0,
            pressing: 0,
            defensive_line: 0,
            directness: 0,
        }
    }

    /// Names of the sliders, in [`Tactic::sliders`] order.
    pub const SLIDER_NAMES: [&'static str; 6] = [
        "mentality",
        "tempo",
        "width",
        "pressing",
        "defensive line",
        "directness",
    ];

    /// The six sliders, clamped, in a fixed order: mentality, tempo,
    /// width, pressing, defensive line, directness.
    pub fn sliders(self) -> [i8; 6] {
        let t = self.clamped();
        [
            t.mentality,
            t.tempo,
            t.width,
            t.pressing,
            t.defensive_line,
            t.directness,
        ]
    }

    /// The same tactic with every slider clamped to -2..=2.
    pub fn clamped(self) -> Self {
        let c = |v: i8| v.clamp(-2, 2);
        Self {
            formation: self.formation,
            mentality: c(self.mentality),
            tempo: c(self.tempo),
            width: c(self.width),
            pressing: c(self.pressing),
            defensive_line: c(self.defensive_line),
            directness: c(self.directness),
        }
    }
}

impl Default for Tactic {
    fn default() -> Self {
        Self::balanced(Formation::F442)
    }
}

/// A team's selection for one match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSheet {
    pub tactic: Tactic,
    /// Exactly eleven players. `starters[i]` plays formation slot `i`.
    pub starters: Vec<PlayerSnapshot>,
    /// Substitutes, in order of preference.
    pub bench: Vec<PlayerSnapshot>,
}

/// Everything needed to play a match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchInput {
    pub home: TeamSheet,
    pub away: TeamSheet,
    /// True when neither side has home advantage (cup finals).
    pub neutral: bool,
    /// World seed.
    pub seed: Seed,
    /// Identifies this match among all matches of the world. Different
    /// keys give independent random streams.
    pub key: u64,
}

impl MatchInput {
    /// The match's random stream.
    pub fn rng(&self) -> Rng {
        self.seed.derive(Stream::Match, self.key, 0)
    }

    /// Checks the sheets are playable.
    ///
    /// # Errors
    /// Returns a description of the first problem.
    pub fn validate(&self) -> Result<(), String> {
        for (side, sheet) in [("home", &self.home), ("away", &self.away)] {
            if sheet.starters.len() != 11 {
                return Err(format!("{side} has {} starters", sheet.starters.len()));
            }
            let mut ids: Vec<_> = sheet
                .starters
                .iter()
                .chain(&sheet.bench)
                .map(|p| p.id)
                .collect();
            ids.sort_unstable();
            if ids.windows(2).any(|w| w[0] == w[1]) {
                return Err(format!("{side} lists a player twice"));
            }
        }
        Ok(())
    }
}
