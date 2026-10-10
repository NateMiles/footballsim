//! Commands: the only way decisions change the world.
//!
//! A command is validated completely before anything changes, so a
//! rejected command has no effect and is not recorded.

use crate::finance::Money;
use crate::ids::{ClubId, LeagueId, ManagerId, PlayerId, StaffId};
use crate::lineup::Lineup;
use crate::live::{self, PendingMatchDay};
use crate::manager::{Control, Manager};
use crate::people::{age_years, wage_demand};
use crate::player::{Contract, Status};
use crate::squad::contract_expiry;
use crate::tick;
use crate::transfer::{Listing, Offer, OfferKind, OfferStatus, Refusal};
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
    /// Don't watch today's match: the assistant manages it, making the
    /// substitutions. Like [`Command::PlayMatch`], the day's matches are
    /// played once every watched match is accounted for.
    SkipMatch,
    /// Hire a free staff member, or renew the contract of one of yours.
    HireStaff {
        staff: StaffId,
        wage: u32,
        years: u8,
    },
    /// Dismiss a staff member, paying three months' wages.
    ReleaseStaff { staff: StaffId },
    /// Take a vacant job (for a manager without a club). The board must
    /// rate the manager highly enough.
    ApplyForJob { club: ClubId },
    /// Leave your club.
    Resign,
    /// Send the scouts to watch a player.
    ScoutPlayer { player: PlayerId },
    /// Stop scouting a player.
    CancelScouting { player: PlayerId },
    /// Bid for a player at another club during a transfer window. For a
    /// transfer, `wage` and `years` are the contract offered to the player.
    MakeOffer {
        player: PlayerId,
        kind: OfferKind,
        wage: u32,
        years: u8,
    },
    /// Answer an offer for one of your players.
    RespondToOffer { offer: u32, response: OfferResponse },
    /// Accept the fee a selling club asked for in reply to your offer.
    AcceptCounter { offer: u32 },
    /// Withdraw one of your offers.
    WithdrawOffer { offer: u32 },
    /// List one of your players for sale or loan, or take them off the list.
    SetListing {
        player: PlayerId,
        listing: Option<Listing>,
    },
}

/// A selling club's answer to an offer.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferResponse {
    Accept,
    Reject,
    /// Name the fee you would accept.
    Counter {
        fee: Money,
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
    /// A club hired a staff member (or renewed their contract).
    StaffHired { staff: StaffId, club: ClubId },
    /// A staff member left a club.
    StaffLeft { staff: StaffId, club: ClubId },
    /// A board sacked its manager.
    ManagerSacked { manager: ManagerId, club: ClubId },
    /// A manager resigned.
    ManagerLeft { manager: ManagerId, club: ClubId },
    /// A human manager's board is losing patience.
    BoardWarning { club: ClubId, confidence: u8 },
    /// Another club bid for a human-managed club's player.
    OfferReceived {
        offer: u32,
        player: PlayerId,
        buyer: ClubId,
        seller: ClubId,
    },
    /// An offer involving a human-managed club changed status.
    OfferUpdated {
        offer: u32,
        player: PlayerId,
        buyer: ClubId,
        seller: ClubId,
        status: OfferStatus,
    },
    /// A deal involving a human-managed club went through.
    TransferCompleted {
        player: PlayerId,
        from: ClubId,
        to: ClubId,
        /// Zero for loans.
        fee: Money,
        loan: bool,
    },
    /// A loan ended and the player went back.
    LoanEnded {
        player: PlayerId,
        from: ClubId,
        to: ClubId,
    },
    /// The scouts finished assessing a player.
    ScoutReport { club: ClubId, player: PlayerId },
    /// A human-managed club's staff member has a contract ending soon.
    StaffContractExpiring {
        staff: StaffId,
        club: ClubId,
        expires: GameDate,
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
    #[error("unknown staff member {0}")]
    UnknownStaff(StaffId),
    #[error("{0} works for another club")]
    StaffAtAnotherClub(StaffId),
    #[error("{0} does not want to join a club of this standing")]
    StaffNotInterested(StaffId),
    #[error("the club already has {0} {1}s")]
    StaffRoleFull(usize, &'static str),
    #[error("the staff member wants at least {demand} a week")]
    StaffWageTooLow { demand: u32 },
    #[error("{0} does not work for your club")]
    NotYourStaff(StaffId),
    #[error("the scouts already have {0} players to watch")]
    TooManyAssignments(usize),
    #[error("{0} is already one of your players")]
    OwnPlayer(PlayerId),
    #[error("the transfer window is closed")]
    WindowClosed,
    #[error("unknown offer {0}")]
    UnknownOffer(u32),
    #[error("that offer is not yours to answer")]
    NotYourOffer,
    #[error("that offer has already been settled")]
    OfferClosed,
    #[error("the transfer budget is {budget}")]
    OverBudget { budget: Money },
    #[error("you already have an offer in for {0}")]
    AlreadyOffered(PlayerId),
    #[error("{0} is on loan")]
    OnLoan(PlayerId),
    #[error("a loan's wage share must be 0-100%")]
    InvalidWageShare,
    #[error("fees cannot be negative")]
    NegativeFee,
    #[error("{0} is not looking for a manager")]
    NotVacant(ClubId),
    #[error("the board of {0} wants someone with a bigger reputation")]
    ApplicationRejected(ClubId),
    #[error("you already manage a club")]
    AlreadyEmployed,
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
                if self.market.loans.contains_key(&player) {
                    return Err(CommandError::OnLoan(player));
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
                if self.market.loans.contains_key(&player) {
                    return Err(CommandError::OnLoan(player));
                }
                self.release_player(player);
                Ok(vec![Event::PlayerLeft { player, club }])
            }
            Command::PlayMatch { ref instructions } => {
                let watched = self.awaited_match(actor)?;
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
                pending
                    .instructions
                    .insert(watched.club, instructions.clone());
                Ok(self.match_reported(watched.manager))
            }
            Command::SkipMatch => {
                let watched = self.awaited_match(actor)?;
                Ok(self.match_reported(watched.manager))
            }
            Command::HireStaff { staff, wage, years } => {
                self.hire_staff_command(actor, staff, wage, years)
            }
            Command::ReleaseStaff { staff } => self.release_staff_command(actor, staff),
            Command::ScoutPlayer { player } => {
                let club = self.managed_club(actor)?;
                if !self.players.contains(player) {
                    return Err(CommandError::UnknownPlayer(player));
                }
                if self.players.status(player) == Status::Retired {
                    return Err(CommandError::Retired(player));
                }
                if self.players.club(player) == Some(club) {
                    return Err(CommandError::OwnPlayer(player));
                }
                let n = self.scouting.assignments(club).len();
                if n >= crate::scouting::MAX_ASSIGNMENTS {
                    return Err(CommandError::TooManyAssignments(n));
                }
                self.scouting.assign(club, player);
                Ok(Vec::new())
            }
            Command::CancelScouting { player } => {
                let club = self.managed_club(actor)?;
                self.scouting.cancel(club, player);
                Ok(Vec::new())
            }
            Command::MakeOffer {
                player,
                kind,
                wage,
                years,
            } => self.make_offer(actor, player, kind, wage, years),
            Command::RespondToOffer { offer, response } => {
                self.respond_to_offer(actor, offer, response)
            }
            Command::AcceptCounter { offer } => self.accept_counter(actor, offer),
            Command::WithdrawOffer { offer } => {
                let club = self.managed_club(actor)?;
                let o = *self
                    .market
                    .offer(offer)
                    .ok_or(CommandError::UnknownOffer(offer))?;
                if o.buyer != club {
                    return Err(CommandError::NotYourOffer);
                }
                if !o.status.is_open() || o.status == OfferStatus::Agreed {
                    return Err(CommandError::OfferClosed);
                }
                Ok(self.settle(offer, OfferStatus::Refused(Refusal::Withdrawn)))
            }
            Command::SetListing { player, listing } => {
                let club = self.managed_club(actor)?;
                if !self.players.contains(player) {
                    return Err(CommandError::UnknownPlayer(player));
                }
                if self.players.club(player) != Some(club) {
                    return Err(CommandError::NotYourPlayer(match actor {
                        Actor::Manager(m) => m,
                        Actor::System => unreachable!("managed_club rejects the system"),
                    }));
                }
                if self.market.loans.contains_key(&player) {
                    return Err(CommandError::OnLoan(player));
                }
                match listing {
                    Some(l) => self.market.listed.insert(player, l),
                    None => self.market.listed.remove(&player),
                };
                Ok(Vec::new())
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
                    // A newcomer: known, but with everything to prove.
                    reputation: self.clubs[club].reputation / 2,
                });
                let board = &mut self.clubs[club].board;
                board.vacant_since = None;
                board.confidence = crate::board::NEUTRAL_CONFIDENCE;
                Ok(vec![Event::ManagerAppointed { manager, club }])
            }
            Command::ApplyForJob { club } => {
                let m = self.unemployed_human(actor)?;
                if self.clubs.get(club).is_none() {
                    return Err(CommandError::UnknownClub(club));
                }
                if self.clubs[club].manager.is_some() || self.clubs[club].league.is_none() {
                    return Err(CommandError::NotVacant(club));
                }
                if !crate::board::would_hire(self, club, self.managers[m].reputation) {
                    return Err(CommandError::ApplicationRejected(club));
                }
                crate::board::appoint(self, m, club);
                Ok(vec![Event::ManagerAppointed { manager: m, club }])
            }
            Command::Resign => {
                let club = self.managed_club(actor)?;
                if self.pending.is_some() {
                    return Err(CommandError::MatchDayInProgress);
                }
                let Actor::Manager(m) = actor else {
                    unreachable!("managed_club rejects the system")
                };
                crate::board::leave(self, m);
                Ok(vec![Event::ManagerLeft { manager: m, club }])
            }
        }
    }

    fn hire_staff_command(
        &mut self,
        actor: Actor,
        staff: StaffId,
        wage: u32,
        years: u8,
    ) -> Result<Vec<Event>, CommandError> {
        let club = self.managed_club(actor)?;
        let st = self
            .staff
            .get(staff)
            .ok_or(CommandError::UnknownStaff(staff))?;
        let renewal = st.club == Some(club);
        if !renewal {
            if st.club.is_some() {
                return Err(CommandError::StaffAtAnotherClub(staff));
            }
            if !crate::staff::available(self, staff, self.date) {
                return Err(CommandError::StaffNotInterested(staff));
            }
            if !st.would_join(self.clubs[club].reputation) {
                return Err(CommandError::StaffNotInterested(staff));
            }
            let (_, max) = st.role.wanted();
            let have = self.clubs[club]
                .staff
                .iter()
                .filter(|&&s| self.staff[s].role == st.role)
                .count();
            if have >= max {
                return Err(CommandError::StaffRoleFull(have, st.role.label()));
            }
        }
        if !(1..=5).contains(&years) {
            return Err(CommandError::InvalidYears(years));
        }
        let demand = st.wage_demand();
        if u64::from(wage) * 10 < u64::from(demand) * 9 {
            return Err(CommandError::StaffWageTooLow { demand });
        }
        let contract = Contract {
            wage,
            expires: contract_expiry(self.date, years),
        };
        if renewal {
            self.staff[staff].contract = Some(contract);
        } else {
            self.hire_staff(staff, club, contract);
        }
        Ok(vec![Event::StaffHired { staff, club }])
    }

    fn release_staff_command(
        &mut self,
        actor: Actor,
        staff: StaffId,
    ) -> Result<Vec<Event>, CommandError> {
        let club = self.managed_club(actor)?;
        let st = self
            .staff
            .get(staff)
            .ok_or(CommandError::UnknownStaff(staff))?;
        if st.club != Some(club) {
            return Err(CommandError::NotYourStaff(staff));
        }
        let severance = st.contract.map_or(0, |c| i64::from(c.wage) * 13);
        self.clubs[club]
            .finances
            .spend(|l| &mut l.staff_wages, severance);
        self.release_staff(staff);
        Ok(vec![Event::StaffLeft { staff, club }])
    }

    fn make_offer(
        &mut self,
        actor: Actor,
        player: PlayerId,
        kind: OfferKind,
        wage: u32,
        years: u8,
    ) -> Result<Vec<Event>, CommandError> {
        let club = self.managed_club(actor)?;
        if !crate::transfer::window_open(self.date) {
            return Err(CommandError::WindowClosed);
        }
        if !self.players.contains(player) {
            return Err(CommandError::UnknownPlayer(player));
        }
        let seller = self
            .players
            .club(player)
            .ok_or(CommandError::NoClub(player))?;
        if seller == club {
            return Err(CommandError::OwnPlayer(player));
        }
        if self.market.loans.contains_key(&player) {
            return Err(CommandError::OnLoan(player));
        }
        if self.market.open_offer(club, player).is_some() {
            return Err(CommandError::AlreadyOffered(player));
        }
        if self.clubs[club].squad.len() >= MAX_SQUAD {
            return Err(CommandError::SquadFull(self.clubs[club].squad.len()));
        }
        match kind {
            OfferKind::Transfer { fee } => {
                if fee < 0 {
                    return Err(CommandError::NegativeFee);
                }
                let budget = self.clubs[club].finances.transfer_budget;
                if fee > budget {
                    return Err(CommandError::OverBudget { budget });
                }
                if !(1..=5).contains(&years) {
                    return Err(CommandError::InvalidYears(years));
                }
            }
            OfferKind::Loan { wage_share } => {
                if wage_share > 100 {
                    return Err(CommandError::InvalidWageShare);
                }
            }
        }
        let today = self.date;
        let offer = Offer {
            id: 0,
            date: today,
            buyer: club,
            seller,
            player,
            kind,
            wage,
            years,
            status: OfferStatus::Pending,
            respond_by: crate::market::respond_by(self, seller),
            updated: today,
        };
        let id = self.market.add(offer);
        let mut events = Vec::new();
        if self.is_human_club(seller) && !self.ai_runs_squad(seller) {
            events.push(Event::OfferReceived {
                offer: id,
                player,
                buyer: club,
                seller,
            });
        }
        Ok(events)
    }

    fn respond_to_offer(
        &mut self,
        actor: Actor,
        offer: u32,
        response: OfferResponse,
    ) -> Result<Vec<Event>, CommandError> {
        let club = self.managed_club(actor)?;
        let o = *self
            .market
            .offer(offer)
            .ok_or(CommandError::UnknownOffer(offer))?;
        if o.seller != club {
            return Err(CommandError::NotYourOffer);
        }
        if o.status != OfferStatus::Pending {
            return Err(CommandError::OfferClosed);
        }
        let status = match response {
            OfferResponse::Reject => OfferStatus::Refused(Refusal::TooLow),
            OfferResponse::Counter { fee } => {
                if fee < 0 {
                    return Err(CommandError::NegativeFee);
                }
                if let Some(x) = self.market.offer_mut(offer) {
                    x.respond_by = self.date.add_days(crate::transfer::HUMAN_RESPONSE_DAYS);
                }
                OfferStatus::Countered { fee }
            }
            OfferResponse::Accept => self.player_decides(&o),
        };
        Ok(self.settle(offer, status))
    }

    fn accept_counter(&mut self, actor: Actor, offer: u32) -> Result<Vec<Event>, CommandError> {
        let club = self.managed_club(actor)?;
        let o = *self
            .market
            .offer(offer)
            .ok_or(CommandError::UnknownOffer(offer))?;
        if o.buyer != club {
            return Err(CommandError::NotYourOffer);
        }
        let OfferStatus::Countered { fee } = o.status else {
            return Err(CommandError::OfferClosed);
        };
        let budget = self.clubs[club].finances.transfer_budget;
        if fee > budget {
            return Err(CommandError::OverBudget { budget });
        }
        let x = self.market.offer_mut(offer).expect("exists");
        x.kind = OfferKind::Transfer { fee };
        let updated = *x;
        let status = self.player_decides(&updated);
        Ok(self.settle(offer, status))
    }

    /// The player's verdict on an offer the clubs have agreed.
    fn player_decides(&self, o: &Offer) -> OfferStatus {
        let yes = match o.kind {
            OfferKind::Transfer { .. } => {
                crate::transfer::accepts_terms(self, o.player, o.buyer, o.wage)
            }
            OfferKind::Loan { .. } => crate::transfer::accepts_loan(self, o.player, o.buyer),
        };
        if yes {
            OfferStatus::Agreed
        } else {
            OfferStatus::Refused(Refusal::PlayerRefused)
        }
    }

    /// Sets an offer's status and reports it to the clubs involved.
    fn settle(&mut self, offer: u32, status: OfferStatus) -> Vec<Event> {
        let today = self.date;
        let o = self.market.offer_mut(offer).expect("exists");
        o.status = status;
        o.updated = today;
        let o = *o;
        vec![Event::OfferUpdated {
            offer,
            player: o.player,
            buyer: o.buyer,
            seller: o.seller,
            status,
        }]
    }

    /// The match `actor` is due to watch today.
    fn awaited_match(&self, actor: Actor) -> Result<live::WatchedFixture, CommandError> {
        let Actor::Manager(m) = actor else {
            return Err(CommandError::NotPermitted);
        };
        self.pending
            .as_ref()
            .and_then(|p| p.awaiting.iter().find(|w| w.manager == m))
            .copied()
            .ok_or(CommandError::NoMatchWaiting(m))
    }

    /// Marks a manager's match as accounted for, and plays the day once
    /// nobody else is watching.
    fn match_reported(&mut self, manager: ManagerId) -> Vec<Event> {
        let pending = self.pending.as_mut().expect("a pending match day");
        pending.awaiting.retain(|w| w.manager != manager);
        let mut events = Vec::new();
        if pending.awaiting.is_empty() {
            self.finish_match_day(&mut events);
        }
        events
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

    /// A human manager without a club.
    fn unemployed_human(&self, actor: Actor) -> Result<ManagerId, CommandError> {
        let Actor::Manager(m) = actor else {
            return Err(CommandError::NotPermitted);
        };
        let mgr = self
            .managers
            .get(m)
            .ok_or(CommandError::UnknownManager(m))?;
        if mgr.control != Control::Human {
            return Err(CommandError::NotPermitted);
        }
        if mgr.club.is_some() {
            return Err(CommandError::AlreadyEmployed);
        }
        Ok(m)
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
