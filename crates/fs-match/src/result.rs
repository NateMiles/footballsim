//! What a match produces.

use crate::shootout::Shootout;
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// Home or away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Side {
    Home,
    Away,
}

impl Side {
    /// 0 for home, 1 for away.
    pub const fn index(self) -> usize {
        match self {
            Side::Home => 0,
            Side::Away => 1,
        }
    }

    /// The other side.
    pub const fn other(self) -> Side {
        match self {
            Side::Home => Side::Away,
            Side::Away => Side::Home,
        }
    }

    /// Side from an index (0 home, anything else away).
    pub const fn from_index(i: usize) -> Side {
        if i == 0 { Side::Home } else { Side::Away }
    }
}

/// How a goal was scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalKind {
    OpenPlay,
    Header,
    LongShot,
    Penalty,
    FreeKick,
    Corner,
}

/// A notable moment, for the match report and timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchEvent {
    Goal {
        minute: u8,
        side: Side,
        scorer: PlayerId,
        assist: Option<PlayerId>,
        kind: GoalKind,
        xg: f32,
    },
    PenaltyMissed {
        minute: u8,
        side: Side,
        taker: PlayerId,
    },
    Card {
        minute: u8,
        side: Side,
        player: PlayerId,
        red: bool,
    },
    Substitution {
        minute: u8,
        side: Side,
        off: PlayerId,
        on: PlayerId,
    },
    Injury {
        minute: u8,
        side: Side,
        player: PlayerId,
    },
}

impl MatchEvent {
    /// Minute the event happened.
    pub fn minute(&self) -> u8 {
        match *self {
            MatchEvent::Goal { minute, .. }
            | MatchEvent::PenaltyMissed { minute, .. }
            | MatchEvent::Card { minute, .. }
            | MatchEvent::Substitution { minute, .. }
            | MatchEvent::Injury { minute, .. } => minute,
        }
    }
}

/// Team totals.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TeamStats {
    /// Share of time in possession, 0-1.
    pub possession: f32,
    pub shots: u16,
    pub shots_on_target: u16,
    pub xg: f32,
    pub passes: u16,
    pub passes_completed: u16,
    pub crosses: u16,
    pub crosses_completed: u16,
    pub dribbles: u16,
    pub dribbles_won: u16,
    pub tackles: u16,
    pub interceptions: u16,
    pub fouls: u16,
    pub yellows: u16,
    pub reds: u16,
    pub corners: u16,
    pub offsides: u16,
    pub saves: u16,
}

impl TeamStats {
    /// Completed passes as a share of attempts (0 when none attempted).
    pub fn pass_completion(&self) -> f64 {
        if self.passes == 0 {
            0.0
        } else {
            f64::from(self.passes_completed) / f64::from(self.passes)
        }
    }
}

/// One player's match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerMatchStats {
    pub player: PlayerId,
    pub started: bool,
    pub minutes: u8,
    pub goals: u8,
    pub assists: u8,
    pub shots: u8,
    pub shots_on_target: u8,
    pub xg: f32,
    pub passes: u16,
    pub passes_completed: u16,
    pub key_passes: u8,
    pub dribbles: u8,
    pub dribbles_won: u8,
    pub tackles: u8,
    pub interceptions: u8,
    pub fouls: u8,
    pub yellow: bool,
    pub red: bool,
    pub saves: u8,
    pub goals_conceded: u8,
    pub injured: bool,
    /// Energy at the final whistle (or when subbed off), 0-1.
    pub energy: f32,
    /// Match rating, 3.0-10.0.
    pub rating: f32,
}

impl PlayerMatchStats {
    /// Empty stats for a player.
    pub fn new(player: PlayerId, started: bool) -> Self {
        Self {
            player,
            started,
            minutes: 0,
            goals: 0,
            assists: 0,
            shots: 0,
            shots_on_target: 0,
            xg: 0.0,
            passes: 0,
            passes_completed: 0,
            key_passes: 0,
            dribbles: 0,
            dribbles_won: 0,
            tackles: 0,
            interceptions: 0,
            fouls: 0,
            yellow: false,
            red: false,
            saves: 0,
            goals_conceded: 0,
            injured: false,
            energy: 1.0,
            rating: 6.0,
        }
    }
}

/// The outcome of a match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchResult {
    /// Goals, home then away.
    pub goals: [u8; 2],
    /// Team stats, home then away.
    pub stats: [TeamStats; 2],
    /// Everyone who played, home then away; starters first.
    pub players: [Vec<PlayerMatchStats>; 2],
    /// Timeline, in minute order.
    pub events: Vec<MatchEvent>,
    /// Whether the match went to extra time (goals include it).
    #[serde(default)]
    pub extra_time: bool,
    /// The penalty shootout that settled a level tie.
    #[serde(default)]
    pub shootout: Option<Shootout>,
}

impl MatchResult {
    /// The side that won a tie this match decided, given the goals carried
    /// from earlier legs: on aggregate, else on penalties.
    pub fn tie_winner(&self, carried: [u8; 2]) -> Option<Side> {
        if let Some(s) = &self.shootout {
            return Some(s.winner());
        }
        let total = |s: usize| u32::from(self.goals[s]) + u32::from(carried[s]);
        match total(0).cmp(&total(1)) {
            core::cmp::Ordering::Greater => Some(Side::Home),
            core::cmp::Ordering::Less => Some(Side::Away),
            core::cmp::Ordering::Equal => None,
        }
    }

    /// Minutes played, 90 or 120.
    pub fn length(&self) -> u8 {
        if self.extra_time { 120 } else { 90 }
    }

    /// +1 home win, 0 draw, -1 away win.
    pub fn outcome(&self) -> i8 {
        match self.goals[0].cmp(&self.goals[1]) {
            core::cmp::Ordering::Greater => 1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Less => -1,
        }
    }
}

/// Fills in match ratings from the stats. Shared by both tiers so ratings
/// mean the same thing everywhere.
pub fn rate_players(result: &mut MatchResult, defensive: &[Vec<bool>; 2]) {
    let goals = result.goals;
    for side in 0..2 {
        let scored = i32::from(goals[side]);
        let conceded = i32::from(goals[1 - side]);
        let team_bonus = match scored.cmp(&conceded) {
            core::cmp::Ordering::Greater => 0.3,
            core::cmp::Ordering::Equal => 0.0,
            core::cmp::Ordering::Less => -0.3,
        };
        for (i, p) in result.players[side].iter_mut().enumerate() {
            if p.minutes == 0 {
                p.rating = 6.0;
                continue;
            }
            let mut r = 6.0 + team_bonus;
            r += 1.0 * f64::from(p.goals) + 0.6 * f64::from(p.assists);
            r += 0.15 * f64::from(p.key_passes) + 0.08 * f64::from(p.dribbles_won);
            r += 0.1 * f64::from(p.tackles) + 0.08 * f64::from(p.interceptions);
            r += 0.2 * f64::from(p.saves);
            if p.passes >= 10 {
                let completion = f64::from(p.passes_completed) / f64::from(p.passes);
                r += (completion - 0.8) * 2.0;
            }
            r -= 0.05 * f64::from(p.fouls);
            if p.yellow {
                r -= 0.3;
            }
            if p.red {
                r -= 1.5;
            }
            if defensive[side].get(i).copied().unwrap_or(false) {
                r -= 0.25 * f64::from(p.goals_conceded);
                if p.goals_conceded == 0 && p.minutes >= 60 {
                    r += 0.4;
                }
            }
            p.rating = ((r.clamp(3.0, 10.0) * 10.0).round() / 10.0) as f32;
        }
    }
}
