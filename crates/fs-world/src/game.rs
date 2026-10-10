//! A running game: the world plus its command log.

use crate::command::{Actor, Command, CommandError, CommandRecord, Event};
use crate::world::World;
use fs_core::GameDate;

/// The world plus every command applied to it since it was created or
/// loaded.
#[derive(Debug, Clone)]
pub struct Game {
    world: World,
    log: Vec<CommandRecord>,
}

impl Game {
    /// Starts a game from a world, with an empty log.
    pub fn new(world: World) -> Self {
        Self {
            world,
            log: Vec::new(),
        }
    }

    /// Resumes a saved game: the world as saved, and the commands that led
    /// to it (from whatever world the log began with). New commands are
    /// numbered on from the log.
    pub fn resume(world: World, log: Vec<CommandRecord>) -> Self {
        Self { world, log }
    }

    /// The current world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Commands applied so far, oldest first.
    pub fn log(&self) -> &[CommandRecord] {
        &self.log
    }

    /// Validates and applies a command. On success the command is appended
    /// to the log and its events are returned. On failure nothing changes.
    pub fn submit(&mut self, actor: Actor, command: Command) -> Result<Vec<Event>, CommandError> {
        let date = self.world.date;
        let events = self.world.apply(actor, &command)?;
        self.log.push(CommandRecord {
            seq: self.log.len() as u64,
            date,
            actor,
            command,
        });
        Ok(events)
    }

    /// Splits the game into its world and log.
    pub fn into_parts(self) -> (World, Vec<CommandRecord>) {
        (self.world, self.log)
    }
}

/// Why a replay failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplayError {
    #[error("record {index} has sequence number {seq}")]
    BadSequence { index: usize, seq: u64 },
    #[error("record {seq} was issued on {expected} but the replay is on {actual}")]
    Diverged {
        seq: u64,
        expected: GameDate,
        actual: GameDate,
    },
    #[error("record {seq} was rejected on replay: {error}")]
    Rejected { seq: u64, error: CommandError },
}

/// Rebuilds a game by applying `log` to `initial`. Because the simulation
/// is deterministic, the result is identical to the original game.
pub fn replay(initial: World, log: &[CommandRecord]) -> Result<Game, ReplayError> {
    let mut game = Game::new(initial);
    for (index, rec) in log.iter().enumerate() {
        if rec.seq != index as u64 {
            return Err(ReplayError::BadSequence {
                index,
                seq: rec.seq,
            });
        }
        if rec.date != game.world.date {
            return Err(ReplayError::Diverged {
                seq: rec.seq,
                expected: rec.date,
                actual: game.world.date,
            });
        }
        game.submit(rec.actor, rec.command.clone())
            .map_err(|error| ReplayError::Rejected {
                seq: rec.seq,
                error,
            })?;
    }
    Ok(game)
}
