//! Commands: the only way decisions change the world.
//!
//! A command is validated completely before anything changes, so a
//! rejected command has no effect and is not recorded.

use crate::ids::{ClubId, ManagerId, PlayerId};
use crate::manager::Control;
use crate::tick;
use crate::world::World;
use fs_core::GameDate;
use serde::{Deserialize, Serialize};

/// Who issued a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor {
    /// The game itself: the single-player host, or a league server.
    System,
    /// A specific manager.
    Manager(ManagerId),
}

/// Largest number of days one [`Command::Advance`] may cover.
pub const MAX_ADVANCE_DAYS: u16 = 366;

/// A decision that changes the world.
///
/// The variant order is part of the save and replay format. Only append.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Move time forward, running the daily simulation for each day.
    Advance { days: u16 },
    /// Give a player a squad number (1-99), or clear it with `None`.
    SetSquadNumber {
        player: PlayerId,
        number: Option<u8>,
    },
}

/// An applied command, as stored in the command log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    /// Position in the log, starting at 0.
    pub seq: u64,
    /// The world date when the command was applied.
    pub date: GameDate,
    pub actor: Actor,
    pub command: Command,
}

/// Something that happened, for the UI to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// A new day began and the daily simulation ran.
    DayAdvanced { date: GameDate },
    /// A squad number changed.
    SquadNumberChanged {
        club: ClubId,
        player: PlayerId,
        number: Option<u8>,
    },
}

/// Why a command was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("unknown manager {0}")]
    UnknownManager(ManagerId),
    #[error("unknown player {0}")]
    UnknownPlayer(PlayerId),
    #[error("{0} has no club")]
    NoClub(PlayerId),
    #[error("only the system or a human manager can do this")]
    NotPermitted,
    #[error("{0} does not manage this player's club")]
    NotYourPlayer(ManagerId),
    #[error("cannot advance {0} days (must be 1-{MAX_ADVANCE_DAYS})")]
    InvalidDays(u16),
    #[error("squad number {0} is out of range (must be 1-99)")]
    InvalidSquadNumber(u8),
    #[error("squad number {number} is already worn by {holder}")]
    SquadNumberTaken { number: u8, holder: PlayerId },
}

impl World {
    /// Validates and applies a command. Callers outside this crate go
    /// through [`crate::Game::submit`], which also records it.
    pub(crate) fn apply(
        &mut self,
        actor: Actor,
        command: &Command,
    ) -> Result<Vec<Event>, CommandError> {
        match *command {
            Command::Advance { days } => {
                self.require_system_or_human(actor)?;
                if days == 0 || days > MAX_ADVANCE_DAYS {
                    return Err(CommandError::InvalidDays(days));
                }
                let mut events = Vec::with_capacity(usize::from(days));
                for _ in 0..days {
                    self.date = self.date.add_days(1);
                    tick::daily(self);
                    events.push(Event::DayAdvanced { date: self.date });
                }
                Ok(events)
            }
            Command::SetSquadNumber { player, number } => {
                if !self.players.contains(player) {
                    return Err(CommandError::UnknownPlayer(player));
                }
                let club = self.players.club(player);
                if let Actor::Manager(m) = actor {
                    let manager = self
                        .managers
                        .get(m)
                        .ok_or(CommandError::UnknownManager(m))?;
                    if club.is_none() || manager.club != club {
                        return Err(CommandError::NotYourPlayer(m));
                    }
                }
                let club = club.ok_or(CommandError::NoClub(player))?;
                if let Some(n) = number {
                    if !(1..=99).contains(&n) {
                        return Err(CommandError::InvalidSquadNumber(n));
                    }
                    if let Some(&holder) = self.clubs[club]
                        .squad
                        .iter()
                        .find(|&&p| p != player && self.players.squad_number(p) == Some(n))
                    {
                        return Err(CommandError::SquadNumberTaken { number: n, holder });
                    }
                }
                self.players.squad_number[player.index()] = number;
                Ok(vec![Event::SquadNumberChanged {
                    club,
                    player,
                    number,
                }])
            }
        }
    }

    fn require_system_or_human(&self, actor: Actor) -> Result<(), CommandError> {
        match actor {
            Actor::System => Ok(()),
            Actor::Manager(m) => match self.managers.get(m) {
                None => Err(CommandError::UnknownManager(m)),
                Some(mgr) if mgr.control == Control::Human => Ok(()),
                Some(_) => Err(CommandError::NotPermitted),
            },
        }
    }
}
