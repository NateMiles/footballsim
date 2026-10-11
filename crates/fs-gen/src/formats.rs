//! The default competition structure for generated worlds: divisions of
//! equal size, a national cup for every nation, and two continental cups
//! for every continent of four or more nations.
//!
//! The result is ordinary [`WorldFormat`] data; a data pack can replace
//! it entirely.

use fs_core::date::Weekday;
use fs_world::economy::revenue_potential;
use fs_world::format::{
    ContinentFormat, CupFormat, DateRule, Entry, LeagueFormat, NationFormat, Source, Stage,
    StageKind, WorldFormat,
};
use fs_world::{Money, Nation};

/// Most nations in one continent.
pub const MAX_CONTINENT: usize = 30;

const ORDINALS: [&str; 10] = [
    "First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth", "Ninth", "Tenth",
];

const CONTINENTS: [&str; 6] = [
    "Northern", "Southern", "Eastern", "Western", "Central", "Island",
];

/// The default structure for nations of `clubs_per_nation` clubs in
/// divisions of `clubs_per_league`. Prize money is set from the nations'
/// reputations.
pub fn default_format(
    nations: &[Nation],
    clubs_per_nation: u16,
    clubs_per_league: u16,
) -> WorldFormat {
    let per = clubs_per_league.max(2);
    let nation_formats: Vec<NationFormat> = nations
        .iter()
        .map(|n| {
            let mut leagues = Vec::new();
            let mut left = clubs_per_nation;
            while left > 0 {
                let clubs = left.min(per);
                let level = leagues.len();
                leagues.push(LeagueFormat {
                    name: format!(
                        "{} Division",
                        ORDINALS.get(level).copied().unwrap_or("Lower")
                    ),
                    clubs,
                    swaps: 3u8.min((clubs / 2) as u8),
                });
                left -= clubs;
            }
            // A last division too small to play is folded into the one
            // above.
            if leagues.len() > 1 && leagues.last().is_some_and(|l| l.clubs < 2) {
                let last = leagues.pop().expect("non-empty");
                leagues.last_mut().expect("non-empty").clubs += last.clubs;
            }
            let sizes: Vec<u16> = leagues.iter().map(|l| l.clubs).collect();
            NationFormat {
                cups: vec![national_cup(&sizes, n.reputation)],
                leagues,
            }
        })
        .collect();
    let continents = continents(nations, &nation_formats);
    WorldFormat {
        nations: nation_formats,
        continents,
    }
}

/// Knockout round names, counted from the final.
fn round_name(k: usize, rounds: usize) -> String {
    match rounds - k {
        1 => "Final".into(),
        2 => "Semi-finals".into(),
        3 => "Quarter-finals".into(),
        _ => format!("{} round", ORDINALS.get(k).copied().unwrap_or("Next")),
    }
}

fn knockout(legs: u8, neutral: bool, winners: Option<u16>, seeded: bool) -> StageKind {
    StageKind::Knockout {
        legs,
        extra_time: true,
        neutral,
        winners,
        seeded,
    }
}

/// Clubs left after a default knockout round.
fn halve(n: usize) -> usize {
    if n <= 1 {
        n
    } else if n.is_power_of_two() {
        n / 2
    } else {
        1 << (usize::BITS - 1 - n.leading_zeros())
    }
}

/// A nation's cup: everyone enters; with three or more divisions the top
/// two join once the lower clubs have played down to half their number.
/// Single-match rounds on Wednesdays, the final on a Sunday after the
/// league ends.
fn national_cup(levels: &[u16], reputation: u16) -> CupFormat {
    let total: usize = levels.iter().map(|&c| usize::from(c)).sum();
    let (top, early) = if levels.len() >= 3 {
        let top = usize::from(levels[0] + levels[1]);
        (top, total - top)
    } else {
        (0, total)
    };
    // Rounds before and after the big clubs join.
    let mut field = early;
    let mut rounds = 0;
    let mut join = 0;
    if top > 0 {
        while field > top / 2 && field > 1 {
            field = halve(field);
            rounds += 1;
        }
        join = rounds;
        field += top;
    }
    while field > 1 {
        field = halve(field);
        rounds += 1;
    }
    let pool = [
        (8, 19),
        (9, 23),
        (10, 28),
        (11, 25),
        (1, 6),
        (1, 27),
        (2, 24),
        (3, 17),
        (4, 14),
    ];
    let before_final = rounds - 1;
    let winner_prize = (revenue_potential(reputation) as f64 * 0.01) as Money;
    let stages = (0..rounds)
        .map(|k| {
            let final_round = k + 1 == rounds;
            let date = if final_round {
                DateRule::new(5, 25, Weekday::Sunday)
            } else {
                // Spread over the season, the last before the final in
                // April.
                let i = if before_final <= 1 {
                    pool.len() - 1
                } else {
                    k * (pool.len() - 1) / (before_final - 1)
                };
                let (m, d) = pool[i.min(pool.len() - 1)];
                DateRule::new(m, d, Weekday::Wednesday)
            };
            // The final's money doubles each round from the first.
            let prize = winner_prize >> (rounds - k).min(16);
            Stage {
                name: round_name(k, rounds),
                kind: knockout(1, final_round, None, false),
                dates: vec![date],
                prize,
            }
        })
        .collect();
    let mut entries = Vec::new();
    let n = levels.len() as u8;
    if top > 0 {
        entries.push(Entry {
            stage: join as u8,
            source: Source::Levels { levels: [1, 2] },
        });
        entries.push(Entry {
            stage: 0,
            source: Source::Levels { levels: [3, n] },
        });
    } else {
        entries.push(Entry {
            stage: 0,
            source: Source::Levels { levels: [1, n] },
        });
    }
    CupFormat {
        name: "Cup".into(),
        short_name: "Cup".into(),
        prestige: 40,
        entries,
        stages,
        winner_prize,
    }
}

/// Nations shared out among continents of at most [`MAX_CONTINENT`], the
/// strongest spread evenly. Continents of fewer than four nations play no
/// continental cups and are left out.
fn continents(nations: &[Nation], formats: &[NationFormat]) -> Vec<ContinentFormat> {
    let n = nations.len();
    if n < 4 {
        return Vec::new();
    }
    let count = n.div_ceil(MAX_CONTINENT);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(nations[i].reputation), i));
    let mut members: Vec<Vec<u16>> = vec![Vec::new(); count];
    for (k, &i) in order.iter().enumerate() {
        members[k % count].push(i as u16);
    }
    members
        .into_iter()
        .enumerate()
        .filter(|(_, m)| m.len() >= 4)
        .map(|(k, m)| {
            let top_rep = nations[usize::from(m[0])].reputation;
            let smallest_top = m
                .iter()
                .map(|&i| formats[usize::from(i)].leagues[0].clubs)
                .min()
                .unwrap_or(2);
            let name = CONTINENTS.get(k).copied().unwrap_or("Far");
            ContinentFormat {
                name: format!("{name} Confederation"),
                cups: continental_cups(m.len(), smallest_top, top_rep),
                nations: m,
            }
        })
        .collect()
}

/// League places per nation rank in the main and second competitions.
fn main_places(rank: usize) -> u8 {
    match rank {
        1..=4 => 4,
        5..=6 => 3,
        7..=10 => 2,
        _ => 1,
    }
}

fn second_places(rank: usize) -> u8 {
    if rank <= 6 { 2 } else { 1 }
}

/// One nation-rank's entrants: league places (first to last), and whether
/// its cup winner comes too.
#[derive(Debug, Clone, Copy)]
struct Unit {
    rank: usize,
    places: [u8; 2],
    cup_winner: bool,
}

impl Unit {
    fn clubs(&self) -> usize {
        usize::from(self.places[1] + 1 - self.places[0])
            * usize::from(self.places[1] >= self.places[0])
            + usize::from(self.cup_winner)
    }
}

/// Entries for a competition: the strongest nations' units go straight
/// to the group stage (`group_stage`) until `direct` places are filled;
/// the rest start in qualifying (stage 0).
fn entries(units: &[Unit], direct: usize, group_stage: u8) -> (Vec<Entry>, usize) {
    let mut out = Vec::new();
    let mut left = direct;
    let mut qualifiers = 0;
    for u in units {
        let league = usize::from(u.places[1] + 1).saturating_sub(usize::from(u.places[0]));
        let cup = usize::from(u.cup_winner);
        // The cup winner goes first, then the league places in order.
        let cup_direct = cup.min(left);
        left -= cup_direct;
        let league_direct = league.min(left);
        left -= league_direct;
        let ranks = [u.rank as u16, u.rank as u16];
        let mut push = |stage: u8, source: Source| out.push(Entry { stage, source });
        if cup == 1 {
            let stage = if cup_direct == 1 { group_stage } else { 0 };
            qualifiers += usize::from(stage == 0);
            push(stage, Source::CupWinners { ranks });
        }
        if league_direct > 0 {
            push(
                group_stage,
                Source::LeaguePlaces {
                    ranks,
                    places: [u.places[0], u.places[0] + league_direct as u8 - 1],
                },
            );
        }
        if league > league_direct {
            qualifiers += league - league_direct;
            push(
                0,
                Source::LeaguePlaces {
                    ranks,
                    places: [u.places[0] + league_direct as u8, u.places[1]],
                },
            );
        }
    }
    (out, qualifiers)
}

/// Clubs through each qualifying round, from `field` down to `needed`.
fn qualifying(mut field: usize, needed: usize) -> Vec<u16> {
    let mut out = Vec::new();
    while field > needed {
        let w = needed.max(field.div_ceil(2));
        out.push(w as u16);
        field = w;
    }
    out
}

/// The two continental cups for a continent of `nations` nations.
fn continental_cups(nations: usize, top_division: u16, top_rep: u16) -> Vec<CupFormat> {
    let cap = |p: u8| p.min((top_division / 2) as u8).max(1);
    let main: Vec<Unit> = (1..=nations)
        .map(|r| Unit {
            rank: r,
            places: [1, cap(main_places(r))],
            cup_winner: false,
        })
        .collect();
    let second: Vec<Unit> = (1..=nations)
        .map(|r| {
            let first = cap(main_places(r)) + 1;
            Unit {
                rank: r,
                places: [first, first + second_places(r) - 1],
                cup_winner: true,
            }
        })
        .collect();
    let money = revenue_potential(top_rep) as f64;
    vec![
        continental_cup(
            "Champions Cup",
            "CC",
            100,
            &main,
            Weekday::Tuesday,
            DateRule::new(6, 1, Weekday::Saturday),
            money * 0.01,
        ),
        continental_cup(
            "Continental Shield",
            "CS",
            60,
            &second,
            Weekday::Thursday,
            DateRule::new(5, 19, Weekday::Wednesday),
            money * 0.004,
        ),
    ]
}

/// A knockout round's name and its two legs' dates (month, day).
type KnockoutRound = (&'static str, (u8, u8), (u8, u8));

/// A continental cup: qualifying rounds in summer, a group stage in the
/// autumn, two-legged knockout rounds in spring and a one-match final.
fn continental_cup(
    name: &str,
    short: &str,
    prestige: u8,
    units: &[Unit],
    day: Weekday,
    final_date: DateRule,
    stage_money: f64,
) -> CupFormat {
    let total: usize = units.iter().map(Unit::clubs).sum();
    let (groups, slots) = if total >= 40 {
        (8u8, 32usize)
    } else if total >= 20 {
        (4, 16)
    } else {
        (0, 0)
    };
    let date = |(m, d): (u8, u8)| DateRule::new(m, d, day);
    let money = |share: f64| (stage_money * share) as Money;
    let mut stages = Vec::new();
    let entries = if groups > 0 {
        let direct = slots * 3 / 4;
        let (_, qualifiers) = entries(units, direct, 0);
        let rounds = qualifying(qualifiers, slots - direct);
        // Weekly from early July: each round's legs a week apart.
        let weeks: Vec<(u8, u8)> = (0..10u8)
            .map(|w| {
                let day = 7 + 7 * w;
                if day <= 31 { (7, day) } else { (8, day - 31) }
            })
            .collect();
        for (k, &w) in rounds.iter().enumerate().take(weeks.len() / 2) {
            stages.push(Stage {
                name: format!("Qualifying round {}", k + 1),
                kind: knockout(2, false, Some(w), true),
                dates: vec![date(weeks[2 * k]), date(weeks[2 * k + 1])],
                prize: money(0.1),
            });
        }
        let group_stage = stages.len() as u8;
        let (entries, _) = entries(units, direct, group_stage);
        stages.push(Stage {
            name: "Group stage".into(),
            kind: StageKind::Groups {
                groups,
                size: 4,
                advance: 2,
                meetings: 2,
            },
            dates: [(9, 15), (9, 29), (10, 20), (11, 3), (11, 24), (12, 8)]
                .into_iter()
                .map(date)
                .collect(),
            prize: money(1.0),
        });
        let knockouts: &[KnockoutRound] = if slots == 32 {
            &[
                ("Round of 16", (2, 16), (3, 9)),
                ("Quarter-finals", (4, 6), (4, 13)),
                ("Semi-finals", (4, 27), (5, 4)),
            ]
        } else {
            &[
                ("Quarter-finals", (4, 6), (4, 13)),
                ("Semi-finals", (4, 27), (5, 4)),
            ]
        };
        for (k, &(name, a, b)) in knockouts.iter().enumerate() {
            stages.push(Stage {
                name: name.into(),
                kind: knockout(2, false, None, k == 0),
                dates: vec![date(a), date(b)],
                prize: money(0.5 * (k + 1) as f64),
            });
        }
        entries
    } else {
        // Too few clubs for groups: two-legged knockout rounds.
        let (entries, _) = entries(units, total, 0);
        let mut field = total;
        let mut rounds = 0;
        while field > 2 {
            field = halve(field);
            rounds += 1;
        }
        let pool = [
            ((9, 15), (9, 29)),
            ((10, 20), (11, 3)),
            ((11, 24), (12, 8)),
            ((2, 16), (3, 9)),
            ((4, 6), (4, 13)),
            ((4, 27), (5, 4)),
        ];
        for k in 0..rounds {
            let i = (pool.len().saturating_sub(rounds) + k).min(pool.len() - 1);
            let (a, b) = pool[i];
            stages.push(Stage {
                name: round_name(k, rounds + 1),
                kind: knockout(2, false, None, false),
                dates: vec![date(a), date(b)],
                prize: money(0.5),
            });
        }
        entries
    };
    stages.push(Stage {
        name: "Final".into(),
        kind: knockout(1, true, None, false),
        dates: vec![final_date],
        prize: money(1.5),
    });
    CupFormat {
        name: name.into(),
        short_name: short.into(),
        prestige,
        entries,
        stages,
        winner_prize: money(2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nations(n: usize) -> Vec<Nation> {
        (0..n)
            .map(|i| Nation {
                name: format!("N{i}"),
                code: "NNN".into(),
                reputation: 9_000usize.saturating_sub(i * 60).max(1_000) as u16,
                first_names: Vec::new(),
                last_names: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn default_formats_are_valid() {
        for (n, clubs) in [
            (1, 20),
            (1, 40),
            (1, 100),
            (24, 100),
            (60, 100),
            (100, 200),
            (6, 60),
        ] {
            let f = default_format(&nations(n), clubs, 20);
            assert_eq!(f.validate(), Ok(()), "{n} nations of {clubs}");
            assert_eq!(f.clubs_in(0), u32::from(clubs));
        }
    }

    #[test]
    fn national_cups_bring_the_big_clubs_in_later() {
        let f = default_format(&nations(1), 100, 20);
        let cup = &f.nations[0].cups[0];
        assert_eq!(cup.stages.len(), 8);
        assert_eq!(cup.stages.last().unwrap().name, "Final");
        let late = cup
            .entries
            .iter()
            .find(|e| e.source == Source::Levels { levels: [1, 2] })
            .unwrap();
        assert_eq!(late.stage, 2);
        let tiny = default_format(&nations(1), 20, 20);
        assert_eq!(tiny.nations[0].cups[0].stages.len(), 5);
    }

    #[test]
    fn continents_split_and_fill_their_group_stages() {
        let f = default_format(&nations(60), 100, 20);
        assert_eq!(f.continents.len(), 2);
        assert!(f.continents.iter().all(|c| c.nations.len() == 30));
        let small = default_format(&nations(24), 100, 20);
        assert_eq!(small.continents.len(), 1);
        let cc = &small.continents[0].cups[0];
        assert!(cc.stages.iter().any(|s| s.name == "Group stage"));
        assert!(default_format(&nations(3), 100, 20).continents.is_empty());
    }

    #[test]
    fn qualifying_narrows_to_the_places_left() {
        assert_eq!(qualifying(20, 8), vec![10, 8]);
        assert_eq!(qualifying(26, 8), vec![13, 8]);
        assert_eq!(qualifying(8, 8), Vec::<u16>::new());
        assert_eq!(qualifying(12, 8), vec![8]);
    }
}
