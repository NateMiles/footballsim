//! Competition formats as data: how many divisions each nation has, which
//! cups it plays, and which competitions its clubs enter abroad.
//!
//! A [`WorldFormat`] is plain data (it serialises to JSON), so a generator
//! or a data pack can describe any structure and the world interprets it.
//! [`WorldFormat::validate`] checks one before a world is built from it.
//! Enums are externally tagged (`{"levels": {"levels": [3, 5]}}`) so
//! the same types also go into compact binary saves.

use crate::finance::Money;
use fs_core::GameDate;
use fs_core::date::Weekday;
use serde::{Deserialize, Serialize};

/// Every competition in a world.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldFormat {
    /// One entry per nation, in nation order.
    pub nations: Vec<NationFormat>,
    /// Groups of nations whose clubs meet in continental competitions.
    #[serde(default)]
    pub continents: Vec<ContinentFormat>,
}

/// A nation's domestic game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NationFormat {
    /// Divisions, top first.
    pub leagues: Vec<LeagueFormat>,
    /// Domestic cups. The first is the nation's main cup, whose winner
    /// qualifies for continental competition.
    #[serde(default)]
    pub cups: Vec<CupFormat>,
}

/// One division.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeagueFormat {
    /// Name, after the nation's name ("First Division").
    pub name: String,
    pub clubs: u16,
    /// Clubs exchanged with the division below.
    pub swaps: u8,
}

/// Nations whose clubs play each other in continental cups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinentFormat {
    pub name: String,
    /// Member nations, by index into [`WorldFormat::nations`].
    pub nations: Vec<u16>,
    /// Most prestigious first: a club enters at most one per season, the
    /// first it qualifies for.
    pub cups: Vec<CupFormat>,
}

/// A cup: who enters, and the stages it is played in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CupFormat {
    pub name: String,
    /// Short name for fixture lists ("Cup", "CC").
    pub short_name: String,
    /// 1-100: how much winning it matters to boards and reputations.
    pub prestige: u8,
    pub entries: Vec<Entry>,
    pub stages: Vec<Stage>,
    /// Paid to the winner on top of the stage money.
    #[serde(default)]
    pub winner_prize: Money,
}

/// Clubs that enter a cup, and the stage they start in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Index into [`CupFormat::stages`].
    pub stage: u8,
    pub source: Source,
}

/// Where a cup's entrants come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Source {
    /// Every club in the cup's nation in divisions `levels[0]` to
    /// `levels[1]` (1 is the top). Domestic cups only.
    Levels { levels: [u8; 2] },
    /// For each member nation ranked `ranks[0]` to `ranks[1]` (1 is the
    /// best by coefficient), the clubs that finished `places[0]` to
    /// `places[1]` in its top division last season. Continental cups only.
    LeaguePlaces { ranks: [u16; 2], places: [u8; 2] },
    /// For each member nation ranked `ranks[0]` to `ranks[1]`, last
    /// season's winner of its main cup. Continental cups only.
    CupWinners { ranks: [u16; 2] },
}

/// One stage of a cup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub name: String,
    pub kind: StageKind,
    /// Match days: one per leg for a knockout round, one per round of
    /// matches for groups.
    pub dates: Vec<DateRule>,
    /// Paid to every club that plays in this stage.
    #[serde(default)]
    pub prize: Money,
}

/// How a stage is played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StageKind {
    /// One knockout round.
    Knockout {
        /// 1 or 2.
        legs: u8,
        /// Play extra time before penalties when the tie is level.
        extra_time: bool,
        /// Played at a neutral ground (finals).
        neutral: bool,
        /// Clubs that go through. `None` halves the field, or brings it
        /// down to a power of two first; either way the best-seeded clubs
        /// get any byes.
        winners: Option<u16>,
        /// Strongest half drawn against weakest half (group winners
        /// against runners-up after a group stage), otherwise an open
        /// draw.
        seeded: bool,
    },
    /// A group stage: round robins, the top clubs of each group going
    /// through.
    Groups {
        groups: u8,
        size: u8,
        advance: u8,
        /// Times each pair meets (2: home and away).
        meetings: u8,
    },
}

impl StageKind {
    /// Match days this stage needs.
    pub fn match_days(&self) -> usize {
        match *self {
            StageKind::Knockout { legs, .. } => usize::from(legs),
            StageKind::Groups { size, meetings, .. } => {
                let size = usize::from(size);
                (size - 1 + size % 2) * usize::from(meetings)
            }
        }
    }

    /// Clubs a stage starts with, when it has a fixed size.
    pub fn size(&self) -> Option<u16> {
        match *self {
            StageKind::Knockout { .. } => None,
            StageKind::Groups { groups, size, .. } => Some(u16::from(groups) * u16::from(size)),
        }
    }
}

/// A match day in the season: the first `weekday` on or after `month`
/// and `day`. July to December fall in the year the season starts, the
/// rest in the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRule {
    pub month: u8,
    pub day: u8,
    pub weekday: Weekday,
}

impl DateRule {
    pub const fn new(month: u8, day: u8, weekday: Weekday) -> Self {
        Self {
            month,
            day,
            weekday,
        }
    }

    /// The date in the season starting in `year`.
    ///
    /// # Panics
    /// Panics if the rule is not a valid day of the year (checked by
    /// [`WorldFormat::validate`]).
    pub fn resolve(&self, year: i32) -> GameDate {
        let y = if self.month >= 7 { year } else { year + 1 };
        let mut d = GameDate::from_ymd(y, u32::from(self.month), u32::from(self.day))
            .or_else(|| GameDate::from_ymd(y, u32::from(self.month), 28))
            .expect("valid date rule");
        while d.weekday() != self.weekday {
            d = d.add_days(1);
        }
        d
    }

    /// Position in the season, for ordering (July first).
    fn season_order(&self) -> (u8, u8) {
        ((self.month + 5) % 12, self.day)
    }
}

impl WorldFormat {
    /// Clubs in a nation's leagues.
    pub fn clubs_in(&self, nation: usize) -> u32 {
        self.nations[nation]
            .leagues
            .iter()
            .map(|l| u32::from(l.clubs))
            .sum()
    }

    /// Checks the format can be played. Returns the first problem.
    pub fn validate(&self) -> Result<(), String> {
        if self.nations.is_empty() {
            return Err("no nations".into());
        }
        for (n, nation) in self.nations.iter().enumerate() {
            if nation.leagues.is_empty() {
                return Err(format!("nation {n} has no leagues"));
            }
            for (l, league) in nation.leagues.iter().enumerate() {
                if league.clubs < 2 {
                    return Err(format!("nation {n} league {l} has fewer than 2 clubs"));
                }
                if usize::from(league.swaps) * 2 > usize::from(league.clubs) {
                    return Err(format!("nation {n} league {l} swaps too many clubs"));
                }
            }
            for cup in &nation.cups {
                cup.validate(false, nation.leagues.len())
                    .map_err(|e| format!("nation {n} {}: {e}", cup.name))?;
            }
        }
        let mut member = vec![false; self.nations.len()];
        for c in &self.continents {
            for &n in &c.nations {
                let slot = member
                    .get_mut(usize::from(n))
                    .ok_or_else(|| format!("{} lists unknown nation {n}", c.name))?;
                if std::mem::replace(slot, true) {
                    return Err(format!("nation {n} is in two continents"));
                }
            }
            for cup in &c.cups {
                cup.validate(true, 0)
                    .map_err(|e| format!("{} {}: {e}", c.name, cup.name))?;
                for e in &cup.entries {
                    if let Source::LeaguePlaces { places, .. } = e.source {
                        for &n in &c.nations {
                            let top = self.nations[usize::from(n)].leagues[0].clubs;
                            if u16::from(places[1]) > top {
                                return Err(format!(
                                    "{} takes place {} from a {top}-club league",
                                    cup.name, places[1]
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl CupFormat {
    fn validate(&self, continental: bool, levels: usize) -> Result<(), String> {
        if self.stages.is_empty() {
            return Err("no stages".into());
        }
        if self.entries.is_empty() {
            return Err("no entries".into());
        }
        for e in &self.entries {
            if usize::from(e.stage) >= self.stages.len() {
                return Err(format!("entry to unknown stage {}", e.stage));
            }
            let range_ok = |r: [u16; 2]| r[0] >= 1 && r[0] <= r[1];
            match e.source {
                Source::Levels { levels: l } => {
                    if continental {
                        return Err("continental cups take league places, not levels".into());
                    }
                    if l[0] < 1 || l[0] > l[1] || usize::from(l[1]) > levels {
                        return Err(format!("bad levels {l:?}"));
                    }
                }
                Source::LeaguePlaces { ranks, places } => {
                    if !continental {
                        return Err("domestic cups take levels".into());
                    }
                    if !range_ok(ranks) || places[0] < 1 || places[0] > places[1] {
                        return Err(format!("bad ranks {ranks:?} or places {places:?}"));
                    }
                }
                Source::CupWinners { ranks } => {
                    if !continental {
                        return Err("domestic cups take levels".into());
                    }
                    if !range_ok(ranks) {
                        return Err(format!("bad ranks {ranks:?}"));
                    }
                }
            }
        }
        let mut last: Option<(u8, u8)> = None;
        for (i, stage) in self.stages.iter().enumerate() {
            if stage.dates.len() != stage.kind.match_days() {
                return Err(format!(
                    "stage {} has {} dates for {} match days",
                    stage.name,
                    stage.dates.len(),
                    stage.kind.match_days()
                ));
            }
            for d in &stage.dates {
                if !(1..=12).contains(&d.month)
                    || d.day == 0
                    || u32::from(d.day) > fs_core::date::days_in_month(2001, u32::from(d.month))
                {
                    return Err(format!("stage {} has an invalid date", stage.name));
                }
                let order = d.season_order();
                if last.is_some_and(|l| l >= order) {
                    return Err(format!("stage {} dates are out of order", stage.name));
                }
                last = Some(order);
            }
            match stage.kind {
                StageKind::Knockout { legs, winners, .. } => {
                    if !(1..=2).contains(&legs) {
                        return Err(format!("stage {} has {legs} legs", stage.name));
                    }
                    if winners == Some(0) {
                        return Err(format!("stage {} lets nobody through", stage.name));
                    }
                }
                StageKind::Groups {
                    groups,
                    size,
                    advance,
                    meetings,
                } => {
                    if groups == 0 || size < 2 || advance == 0 || advance >= size || meetings == 0 {
                        return Err(format!("stage {} has a bad group shape", stage.name));
                    }
                    if i + 1 == self.stages.len() {
                        return Err("a cup cannot end with a group stage".into());
                    }
                }
            }
        }
        let last_stage = self.stages.last().expect("checked");
        if !matches!(
            last_stage.kind,
            StageKind::Knockout {
                winners: None | Some(1),
                ..
            }
        ) {
            return Err("the last stage must be a knockout to one winner".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knockout(legs: u8) -> StageKind {
        StageKind::Knockout {
            legs,
            extra_time: true,
            neutral: false,
            winners: None,
            seeded: false,
        }
    }

    fn cup() -> CupFormat {
        CupFormat {
            name: "Cup".into(),
            short_name: "Cup".into(),
            prestige: 50,
            entries: vec![Entry {
                stage: 0,
                source: Source::Levels { levels: [1, 1] },
            }],
            stages: vec![
                Stage {
                    name: "Semi-finals".into(),
                    kind: knockout(2),
                    dates: vec![
                        DateRule::new(3, 1, Weekday::Tuesday),
                        DateRule::new(3, 8, Weekday::Tuesday),
                    ],
                    prize: 0,
                },
                Stage {
                    name: "Final".into(),
                    kind: knockout(1),
                    dates: vec![DateRule::new(5, 20, Weekday::Sunday)],
                    prize: 0,
                },
            ],
            winner_prize: 0,
        }
    }

    fn world(cup: CupFormat) -> WorldFormat {
        WorldFormat {
            nations: vec![NationFormat {
                leagues: vec![LeagueFormat {
                    name: "First Division".into(),
                    clubs: 20,
                    swaps: 3,
                }],
                cups: vec![cup],
            }],
            continents: Vec::new(),
        }
    }

    #[test]
    fn a_simple_format_is_valid() {
        assert_eq!(world(cup()).validate(), Ok(()));
    }

    #[test]
    fn broken_formats_are_rejected() {
        let mut c = cup();
        c.stages[0].dates.pop();
        assert!(world(c).validate().is_err());
        let mut c = cup();
        c.stages.swap(0, 1);
        assert!(world(c).validate().is_err(), "dates out of order");
        let mut c = cup();
        c.entries[0].source = Source::Levels { levels: [1, 2] };
        assert!(world(c).validate().is_err(), "no second division");
        let mut c = cup();
        c.entries[0].source = Source::CupWinners { ranks: [1, 1] };
        assert!(world(c).validate().is_err(), "domestic cup with winners");
    }

    #[test]
    fn dates_resolve_to_the_right_weekday_and_year() {
        let d = DateRule::new(8, 20, Weekday::Tuesday).resolve(2026);
        assert_eq!(d.weekday(), Weekday::Tuesday);
        assert_eq!(d.ymd().0, 2026);
        assert!(d.ymd().2 >= 20 || d.ymd().1 == 9);
        let d = DateRule::new(5, 25, Weekday::Sunday).resolve(2026);
        assert_eq!(d.ymd().0, 2027);
        assert_eq!(d.weekday(), Weekday::Sunday);
    }

    #[test]
    fn round_robins_need_enough_dates() {
        let g = StageKind::Groups {
            groups: 8,
            size: 4,
            advance: 2,
            meetings: 2,
        };
        assert_eq!(g.match_days(), 6);
        assert_eq!(g.size(), Some(32));
        let odd = StageKind::Groups {
            groups: 1,
            size: 5,
            advance: 2,
            meetings: 1,
        };
        assert_eq!(odd.match_days(), 5);
    }

    #[test]
    fn formats_round_trip_through_json() {
        let w = world(cup());
        let text = serde_json::to_string_pretty(&w).expect("serialises");
        assert!(text.contains(r#""levels": {"#), "{text}");
        assert!(text.contains(r#""weekday": "Sunday""#), "{text}");
        let back: WorldFormat = serde_json::from_str(&text).expect("parses");
        assert_eq!(back, w);
    }
}
