//! A watched match: the live engine plus the frames that show it.

use super::director::Director;
use super::frame::Frame;
use crate::input::MatchInput;
use crate::instruction::{Instruction, InstructionError, TimedInstruction};
use crate::log::ActionRecord;
use crate::result::{MatchEvent, MatchResult, Side, TeamStats};
use crate::tier1::{LiveLineup, LiveMatch};
use serde::{Deserialize, Serialize};

/// What happened between two calls to [`LiveView::play_until`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveChunk {
    pub frames: Vec<Frame>,
    /// Actions played since the last chunk, for commentary.
    pub actions: Vec<ActionRecord>,
    /// Goals, cards, substitutions and injuries since the last chunk.
    pub events: Vec<MatchEvent>,
    pub score: [u8; 2],
    pub minute: u8,
    /// Seconds since kickoff reached by the engine.
    pub elapsed: f64,
    pub finished: bool,
    pub stats: [TeamStats; 2],
}

/// A match being watched.
#[derive(Debug)]
pub struct LiveView {
    live: LiveMatch,
    director: Director,
    events_sent: usize,
}

impl LiveView {
    /// Starts watching a match from kickoff.
    pub fn new(input: &MatchInput) -> Self {
        Self {
            live: LiveMatch::new(input),
            director: Director::new(),
            events_sent: 0,
        }
    }

    /// Plays until `t` seconds after kickoff (or the final whistle) and
    /// returns everything new.
    pub fn play_until(&mut self, t: f64) -> LiveChunk {
        self.live.advance_until(t);
        let actions = self.live.take_actions();
        self.director.push(&actions);
        let split = (self.live.half() == 1).then(|| self.live.first_half_length());
        let until = t.min(self.live.elapsed());
        let lineups = [self.live.lineup(Side::Home), self.live.lineup(Side::Away)];
        let frames = self
            .director
            .render(until, [&lineups[0], &lineups[1]], split);
        let events = self.live.events()[self.events_sent..].to_vec();
        self.events_sent = self.live.events().len();
        LiveChunk {
            frames,
            actions,
            events,
            score: self.live.score(),
            minute: self.live.minute(),
            elapsed: self.live.elapsed(),
            finished: self.live.is_finished(),
            stats: self.live.stats(),
        }
    }

    /// The live engine, for reading state.
    pub fn engine(&self) -> &LiveMatch {
        &self.live
    }

    /// One side's lineup right now.
    pub fn lineup(&self, side: Side) -> LiveLineup {
        self.live.lineup(side)
    }

    /// Gives an instruction at the current point of the match.
    ///
    /// # Errors
    /// As [`LiveMatch::instruct`].
    pub fn instruct(
        &mut self,
        side: Side,
        instruction: Instruction,
    ) -> Result<TimedInstruction, InstructionError> {
        self.live.instruct(side, instruction)
    }

    /// Instructions given so far.
    pub fn instructions(&self) -> &[TimedInstruction] {
        self.live.instructions()
    }

    /// Plays out the rest of the match unseen and returns the result.
    pub fn finish(self) -> MatchResult {
        self.live.finish()
    }
}
