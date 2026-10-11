//! The world: every entity plus the calendar.

use crate::club::Club;
use crate::cup::{Continent, Cup};
use crate::happiness::Relations;
use crate::ids::{
    ClubId, ClubKind, CupId, CupKind, LeagueId, LeagueKind, ManagerId, ManagerKind, NationId,
    NationKind, PlayerId, StaffId, StaffKind,
};
use crate::league::League;
use crate::live::PendingMatchDay;
use crate::manager::{Control, Manager};
use crate::names::NamePool;
use crate::nation::Nation;
use crate::player::{Contract, Status};
use crate::player::{PlayerInit, Players};
use crate::scouting::Scouting;
use crate::staff::Staff;
use crate::transfer::Market;
use fs_core::{GameDate, IdVec, Seed, StateHash, state_hash};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Rules a game is played under, chosen when it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    /// Whether boards may sack human managers.
    pub sack_humans: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self { sack_humans: true }
    }
}

/// The complete simulation state.
///
/// Read access is public. Mutation goes through [`crate::Game::submit`]
/// (for decisions) or the `add_*` builders (for world generation), so that
/// cross-references such as club squads always stay consistent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub(crate) seed: Seed,
    pub(crate) date: GameDate,
    pub(crate) names: NamePool,
    pub(crate) nations: IdVec<NationKind, Nation>,
    pub(crate) clubs: IdVec<ClubKind, Club>,
    pub(crate) managers: IdVec<ManagerKind, Manager>,
    pub(crate) leagues: IdVec<LeagueKind, League>,
    pub(crate) cups: IdVec<CupKind, Cup>,
    pub(crate) continents: Vec<Continent>,
    pub(crate) players: Players,
    pub(crate) staff: IdVec<StaffKind, Staff>,
    pub(crate) scouting: Scouting,
    pub(crate) market: Market,
    pub(crate) relations: Relations,
    pub(crate) rules: Rules,
    /// Today's matches, waiting for human managers to watch them.
    pub(crate) pending: Option<PendingMatchDay>,
}

impl World {
    /// An empty world starting on `date`.
    pub fn new(seed: Seed, date: GameDate) -> Self {
        Self {
            seed,
            date,
            names: NamePool::default(),
            nations: IdVec::new(),
            clubs: IdVec::new(),
            managers: IdVec::new(),
            leagues: IdVec::new(),
            cups: IdVec::new(),
            continents: Vec::new(),
            players: Players::default(),
            staff: IdVec::new(),
            scouting: Scouting::default(),
            market: Market::default(),
            relations: Relations::default(),
            rules: Rules::default(),
            pending: None,
        }
    }

    /// Root seed. Every random stream derives from it.
    pub fn seed(&self) -> Seed {
        self.seed
    }

    /// Today's date.
    pub fn date(&self) -> GameDate {
        self.date
    }

    /// Interned names.
    pub fn names(&self) -> &NamePool {
        &self.names
    }

    /// Interned names, for world builders.
    pub fn names_mut(&mut self) -> &mut NamePool {
        &mut self.names
    }

    /// All nations.
    pub fn nations(&self) -> &IdVec<NationKind, Nation> {
        &self.nations
    }

    /// All clubs.
    pub fn clubs(&self) -> &IdVec<ClubKind, Club> {
        &self.clubs
    }

    /// All staff members, employed or not.
    pub fn staff(&self) -> &IdVec<StaffKind, Staff> {
        &self.staff
    }

    /// Game rules chosen when the world was made.
    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    /// Sets the game rules (world generation).
    pub fn set_rules(&mut self, rules: Rules) {
        self.rules = rules;
    }

    /// The transfer market: offers, loans, listings and history.
    pub fn market(&self) -> &Market {
        &self.market
    }

    /// Players' memories, promises and transfer requests.
    pub fn relations(&self) -> &Relations {
        &self.relations
    }

    /// What clubs know about players.
    pub fn scouting(&self) -> &Scouting {
        &self.scouting
    }

    /// All managers.
    pub fn managers(&self) -> &IdVec<ManagerKind, Manager> {
        &self.managers
    }

    /// All leagues.
    pub fn leagues(&self) -> &IdVec<LeagueKind, League> {
        &self.leagues
    }

    /// All cups, domestic and continental.
    pub fn cups(&self) -> &IdVec<CupKind, Cup> {
        &self.cups
    }

    /// Groups of nations that meet in continental cups.
    pub fn continents(&self) -> &[Continent] {
        &self.continents
    }

    /// All players.
    pub fn players(&self) -> &Players {
        &self.players
    }

    /// Managers controlled by people, in ID order.
    pub fn human_managers(&self) -> impl Iterator<Item = ManagerId> + '_ {
        self.managers
            .iter()
            .filter(|(_, m)| m.control == Control::Human)
            .map(|(id, _)| id)
    }

    /// Full display name of a player.
    pub fn player_name(&self, id: PlayerId) -> String {
        let bio = self.players.bio(id);
        format!(
            "{} {}",
            self.names.get(bio.first_name),
            self.names.get(bio.last_name)
        )
    }

    /// Adds a nation.
    pub fn add_nation(&mut self, nation: Nation) -> NationId {
        self.nations.push(nation)
    }

    /// Adds a club.
    ///
    /// # Panics
    /// Panics if the club's nation does not exist, or if it already has a
    /// manager or squad (use [`World::add_manager`] and
    /// [`World::add_player`] for those).
    pub fn add_club(&mut self, club: Club) -> ClubId {
        assert!(self.nations.get(club.nation).is_some(), "unknown nation");
        assert!(club.manager.is_none() && club.squad.is_empty());
        self.clubs.push(club)
    }

    /// Adds a manager and, if they have a club, appoints them there.
    ///
    /// # Panics
    /// Panics if the club does not exist or already has a manager.
    pub fn add_manager(&mut self, manager: Manager) -> ManagerId {
        let club = manager.club;
        let id = self.managers.push(manager);
        if let Some(c) = club {
            let club = &mut self.clubs[c];
            assert!(club.manager.is_none(), "club already has a manager");
            club.manager = Some(id);
        }
        id
    }

    /// Adds a staff member and, if they have a club, adds them to its
    /// staff.
    ///
    /// # Panics
    /// Panics if the club does not exist.
    pub fn add_staff(&mut self, staff: Staff) -> StaffId {
        let club = staff.club;
        let id = self.staff.push(staff);
        if let Some(c) = club {
            self.clubs[c].staff.push(id);
        }
        id
    }

    /// Adds a staff member employed by `club` (world generation).
    ///
    /// # Panics
    /// Panics if the club does not exist.
    pub fn add_staff_at(&mut self, mut staff: Staff, club: ClubId, contract: Contract) -> StaffId {
        staff.club = Some(club);
        staff.contract = Some(contract);
        self.add_staff(staff)
    }

    /// Sets a club's facilities (world generation).
    pub fn set_facilities(&mut self, club: ClubId, facilities: crate::finance::Facilities) {
        self.clubs[club].facilities = facilities;
    }

    /// Hires a staff member on a new contract, leaving any current club.
    pub(crate) fn hire_staff(&mut self, staff: StaffId, club: ClubId, contract: Contract) {
        self.release_staff(staff);
        self.staff[staff].club = Some(club);
        self.staff[staff].contract = Some(contract);
        self.clubs[club].staff.push(staff);
    }

    /// Ends a staff member's employment.
    pub(crate) fn release_staff(&mut self, staff: StaffId) {
        if let Some(c) = self.staff[staff].club.take() {
            self.clubs[c].staff.retain(|&s| s != staff);
        }
        self.staff[staff].contract = None;
    }

    /// Adds a player and, if they have a club, adds them to its squad.
    ///
    /// # Panics
    /// Panics if the club or nation does not exist.
    pub fn add_player(&mut self, player: PlayerInit) -> PlayerId {
        assert!(
            self.nations.get(player.bio.nation).is_some(),
            "unknown nation"
        );
        let club = player.club;
        if let Some(c) = club {
            assert!(self.clubs.get(c).is_some(), "unknown club");
        }
        let id = self.players.push(player);
        if let Some(c) = club {
            self.clubs[c].squad.push(id);
        }
        id
    }

    /// Adds a league and makes `clubs` its members.
    ///
    /// # Panics
    /// Panics if a club does not exist or is already in a league.
    pub fn add_league(&mut self, mut league: League, clubs: &[ClubId]) -> LeagueId {
        league.clubs.clear();
        let id = self.leagues.push(league);
        for &c in clubs {
            let club = &mut self.clubs[c];
            assert!(club.league.is_none(), "{c} is already in a league");
            club.league = Some(id);
            self.leagues[id].clubs.push(c);
        }
        id
    }

    /// Adds a continent (world generation). Returns its index.
    ///
    /// # Panics
    /// Panics if a nation does not exist.
    pub fn add_continent(&mut self, continent: Continent) -> u16 {
        assert!(
            continent
                .nations
                .iter()
                .all(|&n| self.nations.get(n).is_some())
        );
        self.continents.push(continent);
        (self.continents.len() - 1) as u16
    }

    /// Adds a cup (world generation). Its first edition starts with the
    /// first season.
    ///
    /// # Panics
    /// Panics if its nation or continent does not exist.
    pub fn add_cup(&mut self, cup: Cup) -> CupId {
        match cup.scope {
            crate::cup::CupScope::Nation(n) => assert!(self.nations.get(n).is_some()),
            crate::cup::CupScope::Continent(k) => {
                assert!(usize::from(k) < self.continents.len());
            }
        }
        self.cups.push(cup)
    }

    /// Moves a club from its league to another (promotion and relegation).
    pub(crate) fn move_club(&mut self, club: ClubId, to: LeagueId) {
        if let Some(from) = self.clubs[club].league {
            self.leagues[from].clubs.retain(|&c| c != club);
        }
        self.clubs[club].league = Some(to);
        self.leagues[to].clubs.push(club);
    }

    /// Signs a player to a club on a new contract, releasing them from any
    /// current club. Gives them the lowest free squad number.
    pub(crate) fn sign_player(&mut self, player: PlayerId, club: ClubId, contract: Contract) {
        self.release_player(player);
        let i = player.index();
        self.players.club[i] = Some(club);
        self.players.contract[i] = Some(contract);
        self.players.seasons_unattached[i] = 0;
        let used: BTreeSet<u8> = self.clubs[club]
            .squad
            .iter()
            .filter_map(|&p| self.players.squad_number(p))
            .collect();
        self.players.squad_number[i] = (1..=99).find(|n| !used.contains(n));
        self.clubs[club].squad.push(player);
        crate::happiness::joined(self, player, club);
    }

    /// Moves a player to another club's squad without touching their
    /// contract (loans). Gives them the lowest free squad number there.
    pub(crate) fn move_player(&mut self, player: PlayerId, club: ClubId) {
        let i = player.index();
        if let Some(c) = self.players.club[i].take() {
            self.clubs[c].squad.retain(|&p| p != player);
            self.clubs[c].individual.remove(&player);
            crate::happiness::left(self, player, c);
        }
        let used: BTreeSet<u8> = self.clubs[club]
            .squad
            .iter()
            .filter_map(|&p| self.players.squad_number(p))
            .collect();
        self.players.squad_number[i] = (1..=99).find(|n| !used.contains(n));
        self.players.club[i] = Some(club);
        self.clubs[club].squad.push(player);
        crate::happiness::joined(self, player, club);
    }

    /// Removes a player from their club, if any, ending their contract.
    pub(crate) fn release_player(&mut self, player: PlayerId) {
        self.market.listed.remove(&player);
        self.market.loans.remove(&player);
        let i = player.index();
        if let Some(c) = self.players.club[i].take() {
            self.clubs[c].squad.retain(|&p| p != player);
            self.clubs[c].individual.remove(&player);
            crate::happiness::left(self, player, c);
        }
        self.players.contract[i] = None;
        self.players.squad_number[i] = None;
    }

    /// Retires a player.
    pub(crate) fn retire_player(&mut self, player: PlayerId) {
        self.release_player(player);
        let i = player.index();
        self.players.status[i] = Status::Retired;
        self.players.injury[i] = None;
        self.players.suspension[i] = 0;
    }

    /// The club managed by a human, if `manager` is one.
    pub fn human_club(&self, manager: ManagerId) -> Option<ClubId> {
        self.managers
            .get(manager)
            .filter(|m| m.control == Control::Human)
            .and_then(|m| m.club)
    }

    /// Whether a human manages this club.
    pub fn is_human_club(&self, club: ClubId) -> bool {
        self.clubs[club]
            .manager
            .is_some_and(|m| self.managers[m].control == Control::Human)
    }

    /// The match day waiting for human managers, if today is one.
    pub fn pending(&self) -> Option<&PendingMatchDay> {
        self.pending.as_ref()
    }

    /// Whether a human manager watches and manages this club's matches
    /// live (and so makes their own substitutions).
    pub fn watched_live(&self, club: ClubId) -> bool {
        self.clubs[club].manager.is_some_and(|m| {
            self.managers[m].control == Control::Human && self.managers[m].watch_matches
        })
    }

    /// Whether the AI runs this club's squad (contracts, releases,
    /// signings): every AI club, plus human clubs that delegate it.
    pub fn ai_runs_squad(&self, club: ClubId) -> bool {
        self.clubs[club].manager.is_none_or(|m| {
            self.managers[m].control == Control::Ai || self.managers[m].delegate_squad
        })
    }

    /// A stable hash of the entire world, equal on every platform for equal
    /// worlds.
    pub fn state_hash(&self) -> StateHash {
        state_hash(self)
    }

    /// Checks cross-reference invariants. Returns a description of the
    /// first problem found.
    pub fn check_invariants(&self) -> Result<(), String> {
        if !self.players.columns_aligned() {
            return Err("player columns have different lengths".into());
        }
        let mut seen = vec![false; self.players.len()];
        for (cid, club) in self.clubs.iter() {
            if self.nations.get(club.nation).is_none() {
                return Err(format!("{cid} has unknown nation"));
            }
            let mut numbers = BTreeSet::new();
            for &pid in &club.squad {
                if !self.players.contains(pid) {
                    return Err(format!("{cid} squad lists unknown {pid}"));
                }
                if self.players.club(pid) != Some(cid) {
                    return Err(format!("{pid} is in {cid} squad but not at the club"));
                }
                if std::mem::replace(&mut seen[pid.index()], true) {
                    return Err(format!("{pid} appears in more than one squad"));
                }
                if self.players.status(pid) != Status::Active {
                    return Err(format!("{pid} is retired but in {cid} squad"));
                }
                if self.players.contract(pid).is_none() {
                    return Err(format!("{pid} is at {cid} without a contract"));
                }
                if let Some(n) = self.players.squad_number(pid)
                    && !numbers.insert(n)
                {
                    return Err(format!("{cid} has duplicate squad number {n}"));
                }
            }
            if let Some(&p) = club.individual.keys().find(|p| !club.squad.contains(p)) {
                return Err(format!("{cid} trains {p}, who is not in its squad"));
            }
            if let Some(p) = club.captain
                && !club.squad.contains(&p)
            {
                return Err(format!("{cid} captain {p} is not in its squad"));
            }
            if let Some(m) = club.manager
                && self.managers.get(m).and_then(|m| m.club) != Some(cid)
            {
                return Err(format!("{cid} manager {m} does not point back"));
            }
            if let Some(l) = club.league
                && !self.leagues.get(l).is_some_and(|l| l.clubs.contains(&cid))
            {
                return Err(format!("{cid} league {l} does not list it"));
            }
        }
        for (lid, league) in self.leagues.iter() {
            for &c in &league.clubs {
                if self.clubs.get(c).and_then(|c| c.league) != Some(lid) {
                    return Err(format!("{lid} lists {c}, which is not in it"));
                }
            }
            if let Some(season) = &league.season {
                if season.rows.len() != league.clubs.len() {
                    return Err(format!(
                        "{lid} table has {} rows for {} clubs",
                        season.rows.len(),
                        league.clubs.len()
                    ));
                }
            }
        }
        // Nobody plays twice on one day.
        let mut days: BTreeSet<(ClubId, GameDate)> = BTreeSet::new();
        let mut book = |c: ClubId, d: GameDate| days.insert((c, d));
        for (lid, league) in self.leagues.iter() {
            for f in league.season.iter().flat_map(|s| &s.fixtures) {
                if !book(f.home, f.date) || !book(f.away, f.date) {
                    return Err(format!("{lid}: a club plays twice on {}", f.date));
                }
            }
        }
        for (cid, cup) in self.cups.iter() {
            for f in cup.edition.iter().flat_map(|e| &e.fixtures) {
                let f = &f.fixture;
                if self.clubs.get(f.home).is_none() || self.clubs.get(f.away).is_none() {
                    return Err(format!("{cid} has a fixture with an unknown club"));
                }
                if !book(f.home, f.date) || !book(f.away, f.date) {
                    return Err(format!("{cid}: a club plays twice on {}", f.date));
                }
            }
        }
        let r = &self.relations;
        let at_club = |p: &PlayerId| self.players.contains(*p) && self.players.club(*p).is_some();
        if let Some(p) = r
            .memories
            .keys()
            .chain(r.requests.keys())
            .chain(r.talks.keys())
            .chain(r.promises.iter().map(|p| &p.player))
            .find(|p| !at_club(p))
        {
            return Err(format!("{p} has relations with a club but is at none"));
        }
        if let Some(p) = r
            .promises
            .iter()
            .find(|p| self.players.club(p.player) != Some(p.club))
        {
            return Err(format!("a promise to {} names another club", p.player));
        }
        for pid in self.players.ids() {
            if self.players.club(pid).is_some() && !seen[pid.index()] {
                return Err(format!("{pid} has a club but is in no squad"));
            }
            if self.players.club(pid).is_none() && self.players.contract(pid).is_some() {
                return Err(format!("{pid} has a contract but no club"));
            }
            if self.nations.get(self.players.bio(pid).nation).is_none() {
                return Err(format!("{pid} has unknown nation"));
            }
        }
        for (cid, club) in self.clubs.iter() {
            for &s in &club.staff {
                let Some(st) = self.staff.get(s) else {
                    return Err(format!("{cid} lists unknown staff {s}"));
                };
                if st.club != Some(cid) {
                    return Err(format!("{s} is on {cid} staff but not at the club"));
                }
                if st.contract.is_none() {
                    return Err(format!("{s} works at {cid} without a contract"));
                }
            }
        }
        for (sid, st) in self.staff.iter() {
            if let Some(c) = st.club
                && !self.clubs.get(c).is_some_and(|c| c.staff.contains(&sid))
            {
                return Err(format!("{sid} has club {c}, which does not list them"));
            }
        }
        for (&p, loan) in self.market.loans() {
            if self.players.club(p) != Some(loan.borrower) {
                return Err(format!("{p} is on loan to {} but not there", loan.borrower));
            }
            if self.players.contract(p).is_none() {
                return Err(format!("{p} is on loan without a contract"));
            }
        }
        for (mid, m) in self.managers.iter() {
            if let Some(c) = m.club
                && self.clubs.get(c).and_then(|c| c.manager) != Some(mid)
            {
                return Err(format!("{mid} club {c} does not point back"));
            }
        }
        Ok(())
    }
}
