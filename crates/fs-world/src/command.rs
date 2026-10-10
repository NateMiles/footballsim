//! Commands: the only way decisions change the world.
//!
//! A command is validated completely before anything changes, so a
//! rejected command has no effect and is not recorded.

use crate::ids::{ClubId, LeagueId, ManagerId, PlayerId};
use crate::lineup::Lineup;
use crate::live::{self, PendingMatchDay};
use crate::manager::{Control, Manager};
use crate::people::{age_years, wage_demand};
use crate::player::{Contract, Status};
use crate::squad::contract_expiry;
use crate::tick;
use crate::world::World;
use fs_core::GameDate;
use fs_match::{Side, Tactic, TimedInstruction};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
/// Most instructions one watched match may carry.
pub const MAX_MATCH_INSTRUCTIONS: usize = 200;
/// Longest manager name part, in bytes.
pub const MAX_NAME_LEN: usize = 40;

/// A decision that changes the world.
///
/// The variant order is part of the save and replay format. Only append.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Move time forward, running the daily simulation for each day.
    ///
    /// A day on which a manager who watches matches has a fixture stops
    /// before its matches (see [`Event::MatchDayReady`]); the command
    /// ends there. Advancing while such a day is pending first plays it,
    /// with the assistant managing any match nobody watched, and then
    /// moves on `days` days.
    Advance { days: u16 },
    /// Give a player a squad number (1-99), or clear it with `None`.
    SetSquadNumber {
        player: PlayerId,
        number: Option<u8>,
    },
    /// Set the manager's club tactic, or `None` to let the assistant (the
    /// AI) choose.
    SetTactic { tactic: Option<Tactic> },
    /// Offer a contract: a renewal for one of the manager's players, or a
    /// signing for a free agent. `years` counts seasons, including the
    /// current one when offered between July and December.
    OfferContract {
        player: PlayerId,
        wage: u32,
        years: u8,
    },
    /// Release one of the manager's players; they become a free agent.
    ReleasePlayer { player: PlayerId },
    /// Let the assistant handle contracts, releases and free-agent
    /// signings (`true`), or take them back (`false`).
    SetDelegation { squad: bool },
    /// Report a watched match: the instructions given while watching it
    /// (from `fs_match::view::LiveView::instructions`). When every
    /// watched match of the day is in, the day's matches are played.
    PlayMatch { instructions: Vec<TimedInstruction> },
    /// Pick the starting eleven (by formation slot) and substitutes, or
    /// `None` to let the assistant pick each match.
    SetLineup { lineup: Option<Lineup> },
    /// Stop on match days to watch and manage matches live (`true`), or
    /// let them be played in the background (`false`).
    SetWatchMatches { watch: bool },
    /// Appoint a new human manager to a club, replacing its AI manager.
    /// System only. The new manager watches matches and runs their own
    /// squad.
    AppointHuman {
        club: ClubId,
        first_name: String,
        last_name: String,
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
    /// A match in a league with a human-managed club was played.
    MatchPlayed {
        league: LeagueId,
        home: ClubId,
        away: ClubId,
        goals: [u8; 2],
    },
    /// A player at a human-managed club was injured.
    PlayerInjured {
        player: PlayerId,
        club: ClubId,
        until: GameDate,
    },
    /// A human-managed club's player has a contract ending soon.
    ContractExpiring {
        player: PlayerId,
        club: ClubId,
        expires: GameDate,
    },
    /// A player left a club (released or out of contract).
    PlayerLeft { player: PlayerId, club: ClubId },
    /// A player joined a club.
    PlayerSigned { player: PlayerId, club: ClubId },
    /// A human-managed club's player retired.
    PlayerRetired { player: PlayerId, club: ClubId },
    /// Academy graduates joined a human-managed club.
    YouthIntake {
        club: ClubId,
        players: Vec<PlayerId>,
    },
    /// A league season finished.
    SeasonEnded {
        league: LeagueId,
        year: i32,
        champion: ClubId,
    },
    Promoted {
        club: ClubId,
        from: LeagueId,
        to: LeagueId,
    },
    Relegated {
        club: ClubId,
        from: LeagueId,
        to: LeagueId,
    },
    /// A human manager changed tactic.
    TacticChanged { club: ClubId },
    /// A manager can watch today's match: the day waits for
    /// [`Command::PlayMatch`]. `fs_world::matchday::fixture_input` gives
    /// the match to show.
    MatchDayReady {
        manager: ManagerId,
        club: ClubId,
        league: LeagueId,
        /// Index into the league season's fixtures.
        index: usize,
        home: ClubId,
        away: ClubId,
    },
    /// A human manager chose or cleared their lineup.
    LineupChanged { club: ClubId },
    /// A manager took charge of a club.
    ManagerAppointed { manager: ManagerId, club: ClubId },
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
    #[error("{0} does not manage a club")]
    NoManagedClub(ManagerId),
    #[error("{0} has retired")]
    Retired(PlayerId),
    #[error("{0} is at another club")]
    AtAnotherClub(PlayerId),
    #[error("contracts run 1-5 seasons, not {0}")]
    InvalidYears(u8),
    #[error("the player wants at least {demand} a week")]
    ContractRejected { demand: u32 },
    #[error("the squad is full ({0} players)")]
    SquadFull(usize),
    #[error("{0} has no match waiting to be played")]
    NoMatchWaiting(ManagerId),
    #[error("an instruction is for the other side")]
    WrongSide,
    #[error("{0} instructions (at most {MAX_MATCH_INSTRUCTIONS})")]
    TooManyInstructions(usize),
    #[error("invalid lineup: {0}")]
    InvalidLineup(String),
    #[error("not while today's matches are being played")]
    MatchDayInProgress,
    #[error("unknown club {0}")]
    UnknownClub(ClubId),
    #[error("{0} already has a human manager")]
    ClubTaken(ClubId),
    #[error("names must be 1-{MAX_NAME_LEN} bytes")]
    InvalidName,
}

/// Largest squad a club may register.
pub const MAX_SQUAD: usize = 40;

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
                if self.pending.is_some() {
                    self.finish_match_day(&mut events);
                }
                for _ in 0..days {
                    self.date = self.date.add_days(1);
                    events.push(Event::DayAdvanced { date: self.date });
                    tick::morning(self, &mut events);
                    let watched = live::watched_fixtures(self);
                    if !watched.is_empty() {
                        for w in &watched {
                            let f =
                                &self.leagues[w.league].season().expect("season").fixtures[w.index];
                            events.push(Event::MatchDayReady {
                                manager: w.manager,
                                club: w.club,
                                league: w.league,
                                index: w.index,
                                home: f.home,
                                away: f.away,
                            });
                        }
                        self.pending = Some(PendingMatchDay {
                            awaiting: watched,
                            instructions: BTreeMap::new(),
                        });
                        break;
                    }
                    tick::afternoon(self, &mut events, &BTreeMap::new());
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
            Command::SetTactic { tactic } => {
                let club = self.managed_club(actor)?;
                self.require_unlocked(club)?;
                self.clubs[club].tactic = tactic.map(Tactic::clamped);
                Ok(vec![Event::TacticChanged { club }])
            }
            Command::OfferContract {
                player,
                wage,
                years,
            } => {
                let club = self.managed_club(actor)?;
                self.require_unlocked(club)?;
                if !self.players.contains(player) {
                    return Err(CommandError::UnknownPlayer(player));
                }
                if self.players.status(player) == Status::Retired {
                    return Err(CommandError::Retired(player));
                }
                let current = self.players.club(player);
                if current.is_some_and(|c| c != club) {
                    return Err(CommandError::AtAnotherClub(player));
                }
                if !(1..=5).contains(&years) {
                    return Err(CommandError::InvalidYears(years));
                }
                if current.is_none() && self.clubs[club].squad.len() >= MAX_SQUAD {
                    return Err(CommandError::SquadFull(self.clubs[club].squad.len()));
                }
                let age = age_years(self.players.bio(player).birth_date, self.date) as u32;
                let demand = wage_demand(
                    self.players.ability(player),
                    self.players.potential(player),
                    age,
                );
                if u64::from(wage) * 10 < u64::from(demand) * 9 {
                    return Err(CommandError::ContractRejected { demand });
                }
                let contract = Contract {
                    wage,
                    expires: contract_expiry(self.date, years),
                };
                if current.is_some() {
                    self.players.contract[player.index()] = Some(contract);
                    Ok(Vec::new())
                } else {
                    self.sign_player(player, club, contract);
                    Ok(vec![Event::PlayerSigned { player, club }])
                }
            }
            Command::SetDelegation { squad } => {
                self.managed_club(actor)?;
                if let Actor::Manager(m) = actor {
                    self.managers[m].delegate_squad = squad;
                }
                Ok(Vec::new())
            }
            Command::ReleasePlayer { player } => {
                let club = self.managed_club(actor)?;
                self.require_unlocked(club)?;
                if !self.players.contains(player) {
                    return Err(CommandError::UnknownPlayer(player));
                }
                if self.players.club(player) != Some(club) {
                    return Err(CommandError::NotYourPlayer(match actor {
                        Actor::Manager(m) => m,
                        Actor::System => unreachable!("managed_club rejects the system"),
                    }));
                }
                self.release_player(player);
                Ok(vec![Event::PlayerLeft { player, club }])
            }
            Command::PlayMatch { ref instructions } => {
                let Actor::Manager(m) = actor else {
                    return Err(CommandError::NotPermitted);
                };
                let watched = self
                    .pending
                    .as_ref()
                    .and_then(|p| p.awaiting.iter().find(|w| w.manager == m))
                    .copied()
                    .ok_or(CommandError::NoMatchWaiting(m))?;
                if instructions.len() > MAX_MATCH_INSTRUCTIONS {
                    return Err(CommandError::TooManyInstructions(instructions.len()));
                }
                let f = &self.leagues[watched.league]
                    .season()
                    .expect("season")
                    .fixtures[watched.index];
                let side = if f.home == watched.club {
                    Side::Home
                } else {
                    Side::Away
                };
                if instructions.iter().any(|i| i.side != side) {
                    return Err(CommandError::WrongSide);
                }
                let pending = self.pending.as_mut().expect("checked above");
                pending.awaiting.retain(|w| w.manager != m);
                pending
                    .instructions
                    .insert(watched.club, instructions.clone());
                let mut events = Vec::new();
                if pending.awaiting.is_empty() {
                    self.finish_match_day(&mut events);
                }
                Ok(events)
            }
            Command::SetLineup { ref lineup } => {
                let club = self.managed_club(actor)?;
                self.require_unlocked(club)?;
                if let Some(l) = lineup {
                    l.validate().map_err(CommandError::InvalidLineup)?;
                    if let Some(&p) =
                        l.starters.iter().chain(&l.bench).find(|&&p| {
                            !self.players.contains(p) || self.players.club(p) != Some(club)
                        })
                    {
                        return Err(CommandError::InvalidLineup(format!(
                            "{p} is not at the club"
                        )));
                    }
                }
                self.clubs[club].lineup = lineup.clone();
                Ok(vec![Event::LineupChanged { club }])
            }
            Command::SetWatchMatches { watch } => {
                self.managed_club(actor)?;
                if self.pending.is_some() {
                    return Err(CommandError::MatchDayInProgress);
                }
                if let Actor::Manager(m) = actor {
                    self.managers[m].watch_matches = watch;
                }
                Ok(Vec::new())
            }
            Command::AppointHuman {
                club,
                ref first_name,
                ref last_name,
            } => {
                if actor != Actor::System {
                    return Err(CommandError::NotPermitted);
                }
                if self.pending.is_some() {
                    return Err(CommandError::MatchDayInProgress);
                }
                if self.clubs.get(club).is_none() {
                    return Err(CommandError::UnknownClub(club));
                }
                if self.is_human_club(club) {
                    return Err(CommandError::ClubTaken(club));
                }
                for name in [first_name, last_name] {
                    if name.trim().is_empty() || name.len() > MAX_NAME_LEN {
                        return Err(CommandError::InvalidName);
                    }
                }
                if let Some(old) = self.clubs[club].manager.take() {
                    self.managers[old].club = None;
                }
                let first_name = self.names.intern(first_name.trim());
                let last_name = self.names.intern(last_name.trim());
                let manager = self.add_manager(Manager {
                    first_name,
                    last_name,
                    control: Control::Human,
                    club: Some(club),
                    delegate_squad: false,
                    watch_matches: true,
                });
                Ok(vec![Event::ManagerAppointed { manager, club }])
            }
        }
    }

    /// Plays the rest of a pending match day, with the instructions
    /// submitted so far.
    fn finish_match_day(&mut self, events: &mut Vec<Event>) {
        let pending = self.pending.take().expect("a pending match day");
        tick::afternoon(self, events, &pending.instructions);
    }

    /// Rejects changes that would alter a match already shown or being
    /// shown live: while a match day is pending, a club playing today may
    /// change only while its own manager, alone, is still to watch it.
    fn require_unlocked(&self, club: ClubId) -> Result<(), CommandError> {
        let Some(pending) = &self.pending else {
            return Ok(());
        };
        let Some((league, index)) = live::fixture_today(self, club) else {
            return Ok(());
        };
        let mut watchers = pending
            .awaiting
            .iter()
            .filter(|w| w.league == league && w.index == index);
        match (watchers.next(), watchers.next()) {
            (Some(w), None) if w.club == club => Ok(()),
            _ => Err(CommandError::MatchDayInProgress),
        }
    }

    /// The club a human manager runs.
    fn managed_club(&self, actor: Actor) -> Result<ClubId, CommandError> {
        match actor {
            Actor::System => Err(CommandError::NotPermitted),
            Actor::Manager(m) => {
                let mgr = self
                    .managers
                    .get(m)
                    .ok_or(CommandError::UnknownManager(m))?;
                if mgr.control != Control::Human {
                    return Err(CommandError::NotPermitted);
                }
                mgr.club.ok_or(CommandError::NoManagedClub(m))
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
