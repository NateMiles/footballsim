//! Small types shared by many views.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// An entity's index. Players, clubs, leagues and managers each have their
/// own numbering.
pub type Id = u32;

/// A calendar date as `YYYY-MM-DD`.
pub type Date = String;

/// A club as other views refer to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClubRef {
    pub id: Id,
    pub name: String,
    pub short_name: String,
    pub colors: Colors,
}

/// Kit colours as CSS hex strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Colors {
    pub primary: String,
    pub secondary: String,
}

/// A league as other views refer to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LeagueRef {
    pub id: Id,
    pub name: String,
    pub nation: String,
    /// 1 for a nation's top division.
    pub level: u8,
}

/// A player as other views refer to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRef {
    pub id: Id,
    pub name: String,
    /// Initial and surname, for tight spaces.
    pub short_name: String,
    pub number: Option<u8>,
    /// Best position code, such as `"DC"`.
    pub position: String,
}

/// Home or away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Home,
    Away,
}

impl From<fs_match::Side> for Side {
    fn from(s: fs_match::Side) -> Self {
        match s {
            fs_match::Side::Home => Side::Home,
            fs_match::Side::Away => Side::Away,
        }
    }
}

impl From<Side> for fs_match::Side {
    fn from(s: Side) -> Self {
        match s {
            Side::Home => fs_match::Side::Home,
            Side::Away => fs_match::Side::Away,
        }
    }
}

/// A formation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum Formation {
    #[serde(rename = "4-4-2")]
    F442,
    #[serde(rename = "4-3-3")]
    F433,
    #[serde(rename = "4-2-3-1")]
    F4231,
    #[serde(rename = "3-5-2")]
    F352,
    #[serde(rename = "5-3-2")]
    F532,
    #[serde(rename = "4-1-4-1")]
    F4141,
}

impl From<fs_match::Formation> for Formation {
    fn from(f: fs_match::Formation) -> Self {
        use fs_match::Formation as F;
        match f {
            F::F442 => Formation::F442,
            F::F433 => Formation::F433,
            F::F4231 => Formation::F4231,
            F::F352 => Formation::F352,
            F::F532 => Formation::F532,
            F::F4141 => Formation::F4141,
        }
    }
}

impl From<Formation> for fs_match::Formation {
    fn from(f: Formation) -> Self {
        use fs_match::Formation as F;
        match f {
            Formation::F442 => F::F442,
            Formation::F433 => F::F433,
            Formation::F4231 => F::F4231,
            Formation::F352 => F::F352,
            Formation::F532 => F::F532,
            Formation::F4141 => F::F4141,
        }
    }
}

/// Team instructions. Every slider runs from -2 to +2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Tactic {
    pub formation: Formation,
    pub mentality: i8,
    pub tempo: i8,
    pub width: i8,
    pub pressing: i8,
    pub defensive_line: i8,
    pub directness: i8,
}

impl From<fs_match::Tactic> for Tactic {
    fn from(t: fs_match::Tactic) -> Self {
        Self {
            formation: t.formation.into(),
            mentality: t.mentality,
            tempo: t.tempo,
            width: t.width,
            pressing: t.pressing,
            defensive_line: t.defensive_line,
            directness: t.directness,
        }
    }
}

impl From<Tactic> for fs_match::Tactic {
    fn from(t: Tactic) -> Self {
        fs_match::Tactic {
            formation: t.formation.into(),
            mentality: t.mentality,
            tempo: t.tempo,
            width: t.width,
            pressing: t.pressing,
            defensive_line: t.defensive_line,
            directness: t.directness,
        }
        .clamped()
    }
}

/// A season's numbers for one player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStats {
    pub appearances: u16,
    pub starts: u16,
    pub minutes: u32,
    pub goals: u16,
    pub assists: u16,
    pub yellows: u16,
    pub reds: u16,
    pub clean_sheets: u16,
    /// Average match rating, if they played.
    pub rating: Option<f32>,
}

impl From<&fs_world::SeasonStats> for SeasonStats {
    fn from(s: &fs_world::SeasonStats) -> Self {
        Self {
            appearances: s.appearances,
            starts: s.starts,
            minutes: s.minutes,
            goals: s.goals,
            assists: s.assists,
            yellows: s.yellows,
            reds: s.reds,
            clean_sheets: s.clean_sheets,
            rating: s.average_rating(),
        }
    }
}
