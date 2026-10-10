//! Tier 1 against tier 2, fixture by fixture.

use crate::predict::{Prediction, predict};
use crate::season::league_world;
use core::fmt::Write;
use fs_core::Seed;
use fs_match::Tier;
use fs_world::{ClubId, matchday};

/// Fixtures compared: a spread from top-v-bottom to evenly matched, in
/// the seed-42 calibration league.
pub const PAIRS: [(u32, u32); 10] = [
    (0, 19),
    (19, 0),
    (3, 4),
    (10, 9),
    (5, 15),
    (1, 2),
    (12, 7),
    (16, 11),
    (8, 18),
    (14, 6),
];

/// One fixture's predictions from both tiers.
#[derive(Debug, Clone)]
pub struct Comparison {
    pub home: u32,
    pub away: u32,
    pub event: Prediction,
    pub statistical: Prediction,
}

/// Predicts every pair in [`PAIRS`] with both tiers.
pub fn compare_tiers(runs: u32) -> Vec<Comparison> {
    let world = league_world(42);
    PAIRS
        .iter()
        .map(|&(h, a)| {
            let home = matchday::team_sheet(&world, ClubId::new(h));
            let away = matchday::team_sheet(&world, ClubId::new(a));
            Comparison {
                home: h,
                away: a,
                event: predict(&home, &away, Seed(1), runs, Tier::Event),
                statistical: predict(&home, &away, Seed(1), runs, Tier::Statistical),
            }
        })
        .collect()
}

/// Mean and worst absolute difference in outcome probabilities.
pub fn outcome_error(rows: &[Comparison]) -> (f64, f64) {
    let mut total = 0.0;
    let mut worst: f64 = 0.0;
    let mut n = 0.0;
    for r in rows {
        let (e, s) = (&r.event, &r.statistical);
        for d in [
            e.home_win - s.home_win,
            e.draw - s.draw,
            e.away_win - s.away_win,
        ] {
            total += d.abs();
            worst = worst.max(d.abs());
            n += 1.0;
        }
    }
    (total / f64::max(n, 1.0), worst)
}

/// The comparison as a Markdown table.
pub fn to_markdown(rows: &[Comparison]) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "| Fixture | Tier 1 H / D / A % | Tier 2 H / D / A % | Tier 1 goals | Tier 2 goals |"
    );
    let _ = writeln!(s, "| --- | --- | --- | --- | --- |");
    for r in rows {
        let (e, t) = (&r.event, &r.statistical);
        let _ = writeln!(
            s,
            "| {} v {} | {:.0} / {:.0} / {:.0} | {:.0} / {:.0} / {:.0} | {:.2} - {:.2} | {:.2} - {:.2} |",
            r.home,
            r.away,
            100.0 * e.home_win,
            100.0 * e.draw,
            100.0 * e.away_win,
            100.0 * t.home_win,
            100.0 * t.draw,
            100.0 * t.away_win,
            e.home_goals,
            e.away_goals,
            t.home_goals,
            t.away_goals
        );
    }
    let (mean, worst) = outcome_error(rows);
    let _ = writeln!(
        s,
        "\nMean outcome-probability difference {:.1} points; worst {:.1}.",
        100.0 * mean,
        100.0 * worst
    );
    s
}
