//! Leagues: membership, fixtures, tables and history.

use crate::ids::{ClubId, NationId, PlayerId};
use fs_core::GameDate;
use fs_match::Tier;
use serde::{Deserialize, Serialize};

/// A league competition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct League {
    pub name: String,
    pub nation: NationId,
    /// 1 for the top division, 2 for the one below, and so on.
    pub level: u8,
    /// Clubs promoted from the bottom of the league above / relegated to
    /// the league below.
    pub swaps: u8,
    /// Current members. Maintained by [`crate::World`].
    pub(crate) clubs: Vec<ClubId>,
    pub(crate) season: Option<LeagueSeason>,
    pub(crate) history: Vec<SeasonSummary>,
}

impl League {
    /// A league with no clubs.
    pub fn new(name: String, nation: NationId, level: u8, swaps: u8) -> Self {
        Self {
            name,
            nation,
            level,
            swaps,
            clubs: Vec::new(),
            season: None,
            history: Vec::new(),
        }
    }

    /// Current members.
    pub fn clubs(&self) -> &[ClubId] {
        &self.clubs
    }

    /// The season in progress, if any.
    pub fn season(&self) -> Option<&LeagueSeason> {
        self.season.as_ref()
    }

    /// Finished seasons, oldest first.
    pub fn history(&self) -> &[SeasonSummary] {
        &self.history
    }
}

/// One season of a league.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeagueSeason {
    /// The year the season started.
    pub year: i32,
    /// Every fixture, in date order.
    pub fixtures: Vec<Fixture>,
    /// Index of the first fixture not yet played.
    pub(crate) next: usize,
    /// One row per club, in membership order (unsorted).
    pub(crate) rows: Vec<TableRow>,
}

impl LeagueSeason {
    /// The table, sorted: points, goal difference, goals scored, then
    /// club ID.
    pub fn table(&self) -> Vec<TableRow> {
        let mut t = self.rows.clone();
        sort_table(&mut t);
        t
    }

    /// Whether every fixture has been played.
    pub fn finished(&self) -> bool {
        self.next >= self.fixtures.len()
    }
}

/// Sorts table rows into standings order.
pub fn sort_table(rows: &mut [TableRow]) {
    rows.sort_by(|a, b| {
        b.points()
            .cmp(&a.points())
            .then(b.goal_difference().cmp(&a.goal_difference()))
            .then(b.scored.cmp(&a.scored))
            .then(a.club.cmp(&b.club))
    });
}

/// A scheduled match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    pub date: GameDate,
    pub home: ClubId,
    pub away: ClubId,
    pub result: Option<FixtureResult>,
}

/// The summary of a played match that the league keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureResult {
    pub goals: [u8; 2],
    pub xg: [f32; 2],
    pub tier: Tier,
    /// Scorers in order, with the side (0 home, 1 away).
    pub scorers: Vec<(u8, PlayerId)>,
}

/// A league table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    pub club: ClubId,
    pub played: u16,
    pub won: u16,
    pub drawn: u16,
    pub lost: u16,
    pub scored: u16,
    pub conceded: u16,
}

impl TableRow {
    /// An empty row.
    pub fn new(club: ClubId) -> Self {
        Self {
            club,
            played: 0,
            won: 0,
            drawn: 0,
            lost: 0,
            scored: 0,
            conceded: 0,
        }
    }

    /// Three for a win, one for a draw.
    pub fn points(&self) -> u16 {
        3 * self.won + self.drawn
    }

    /// Goals scored minus conceded.
    pub fn goal_difference(&self) -> i32 {
        i32::from(self.scored) - i32::from(self.conceded)
    }

    /// Records a result from this club's point of view.
    pub fn record(&mut self, scored: u8, conceded: u8) {
        self.played += 1;
        self.scored += u16::from(scored);
        self.conceded += u16::from(conceded);
        match scored.cmp(&conceded) {
            core::cmp::Ordering::Greater => self.won += 1,
            core::cmp::Ordering::Equal => self.drawn += 1,
            core::cmp::Ordering::Less => self.lost += 1,
        }
    }
}

/// What the league remembers about a finished season.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonSummary {
    pub year: i32,
    /// Final standings.
    pub table: Vec<TableRow>,
    pub promoted: Vec<ClubId>,
    pub relegated: Vec<ClubId>,
    /// Top scorer and their league goals.
    pub top_scorer: Option<(PlayerId, u16)>,
}

impl SeasonSummary {
    /// The champion, if the season had any clubs.
    pub fn champion(&self) -> Option<ClubId> {
        self.table.first().map(|r| r.club)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_sorts_by_points_then_goal_difference() {
        let mut a = TableRow::new(ClubId::new(0));
        let mut b = TableRow::new(ClubId::new(1));
        let mut c = TableRow::new(ClubId::new(2));
        a.record(1, 0);
        b.record(3, 0);
        c.record(0, 0);
        let mut t = vec![a, c, b];
        sort_table(&mut t);
        assert_eq!(
            t.iter().map(|r| r.club.raw()).collect::<Vec<_>>(),
            vec![1, 0, 2]
        );
        assert_eq!(t[0].points(), 3);
        assert_eq!(t[2].drawn, 1);
    }
}
