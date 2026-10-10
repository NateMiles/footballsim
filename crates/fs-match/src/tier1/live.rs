//! Playing a tier 1 match step by step, for watching live.

use super::MAX_SUBS;
use super::engine::Engine;
use super::params::Params;
use crate::input::{MatchInput, Tactic};
use crate::instruction::{Instruction, InstructionError, Shout, TimedInstruction};
use crate::log::ActionRecord;
use crate::result::{MatchEvent, MatchResult, Side, TeamStats};
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// A match being played step by step.
///
/// Advance it as far as playback needs, give instructions between steps,
/// and read the action log as it grows. Instructions are recorded with the
/// step they took effect at; playing the same input with those
/// instructions in [`MatchInput::instructions`] reproduces the match
/// exactly, which is how the world confirms a live result.
pub struct LiveMatch {
    engine: Engine,
    /// Actions already handed out by [`LiveMatch::take_actions`].
    taken: usize,
}

impl core::fmt::Debug for LiveMatch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LiveMatch")
            .field("step", &self.engine.steps)
            .field("elapsed", &self.engine.elapsed())
            .finish_non_exhaustive()
    }
}

impl LiveMatch {
    /// Starts a match with the calibrated parameters. Any instructions in
    /// the input are applied at their steps.
    pub fn new(input: &MatchInput) -> Self {
        Self::with_params(input, &Params::default())
    }

    /// Starts a match with custom parameters.
    pub fn with_params(input: &MatchInput, params: &Params) -> Self {
        Self {
            engine: Engine::new(input, params, true),
            taken: 0,
        }
    }

    /// Seconds since kickoff, counting stoppage time.
    pub fn elapsed(&self) -> f64 {
        self.engine.elapsed()
    }

    /// The match minute, as shown on a scoreboard (1-45, then 46-90).
    pub fn minute(&self) -> u8 {
        self.engine.minute()
    }

    /// 0 in the first half, 1 in the second.
    pub fn half(&self) -> u8 {
        self.engine.half
    }

    /// Seconds the first half lasted including stoppage time (45 minutes
    /// until it ends).
    pub fn first_half_length(&self) -> f64 {
        self.engine.first_half_len
    }

    /// Engine steps completed so far.
    pub fn step_index(&self) -> u32 {
        self.engine.steps
    }

    /// Goals, home then away.
    pub fn score(&self) -> [u8; 2] {
        self.engine.goals
    }

    /// Whether the final whistle has gone.
    pub fn is_finished(&self) -> bool {
        self.engine.finished
    }

    /// Plays one step. Returns `false` once the match is over.
    pub fn advance(&mut self) -> bool {
        self.engine.advance()
    }

    /// Plays until the clock reaches `elapsed` seconds or the match ends.
    pub fn advance_until(&mut self, elapsed: f64) {
        while self.engine.elapsed() < elapsed && self.engine.advance() {}
    }

    /// Actions recorded since the last call.
    pub fn take_actions(&mut self) -> Vec<ActionRecord> {
        let log = self.engine.log.as_deref().unwrap_or(&[]);
        let new = log[self.taken..].to_vec();
        self.taken = log.len();
        new
    }

    /// Match events so far (goals, cards, substitutions, injuries).
    pub fn events(&self) -> &[MatchEvent] {
        &self.engine.events
    }

    /// Team stats so far, home then away, with possession filled in.
    pub fn stats(&self) -> [TeamStats; 2] {
        let teams = &self.engine.teams;
        let total = (teams[0].in_play_secs + teams[1].in_play_secs).max(1.0);
        let mut s = [teams[0].stats, teams[1].stats];
        for (k, st) in s.iter_mut().enumerate() {
            st.possession = (teams[k].in_play_secs / total) as f32;
        }
        s
    }

    /// Who is where for one side right now.
    pub fn lineup(&self, side: Side) -> LiveLineup {
        let team = &self.engine.teams[side.index()];
        LiveLineup {
            tactic: team.tactic,
            on_pitch: (0..11)
                .map(|s| {
                    team.on[s].map(|i| LivePlayer {
                        id: team.players[i].snap.id,
                        slot: s as u8,
                        energy: team.players[i].energy as f32,
                        yellow: team.players[i].stats.yellow,
                    })
                })
                .collect(),
            bench: team
                .bench
                .iter()
                .map(|&b| team.players[b].snap.id)
                .collect(),
            substitutions_left: MAX_SUBS.saturating_sub(team.subs_made),
            shout: team.shout,
        }
    }

    /// Gives an instruction now. On success, returns it as recorded.
    ///
    /// # Errors
    /// Fails if the match is over or a substitution is not possible.
    pub fn instruct(
        &mut self,
        side: Side,
        instruction: Instruction,
    ) -> Result<TimedInstruction, InstructionError> {
        self.engine.instruct(side, instruction)
    }

    /// Every instruction applied so far, in order.
    pub fn instructions(&self) -> &[TimedInstruction] {
        &self.engine.applied
    }

    /// Plays any remaining time and returns the result.
    pub fn finish(self) -> MatchResult {
        self.engine.run()
    }
}

/// One side's lineup during a live match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveLineup {
    pub tactic: Tactic,
    /// By formation slot; `None` where a player was sent off.
    pub on_pitch: Vec<Option<LivePlayer>>,
    /// Unused substitutes.
    pub bench: Vec<PlayerId>,
    pub substitutions_left: u8,
    pub shout: Option<Shout>,
}

/// A player on the pitch during a live match.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivePlayer {
    pub id: PlayerId,
    pub slot: u8,
    /// 0-1.
    pub energy: f32,
    pub yellow: bool,
}
