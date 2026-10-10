//! Calibration report: simulated statistics against real-football targets
//! (docs/MATCH_ENGINE.md, section 5).

use crate::season::SeasonOutcome;
use core::fmt::Write;

/// One statistic and its acceptable range.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub name: &'static str,
    pub value: f64,
    pub low: f64,
    pub high: f64,
    /// Decimal places when printing.
    pub decimals: usize,
}

impl Check {
    /// Whether the value is in range.
    pub fn ok(&self) -> bool {
        (self.low..=self.high).contains(&self.value)
    }
}

/// A full calibration report.
#[derive(Debug, Clone)]
pub struct Report {
    pub matches: usize,
    pub seasons: usize,
    pub checks: Vec<Check>,
}

impl Report {
    /// Whether every check passed.
    pub fn all_ok(&self) -> bool {
        self.checks.iter().all(Check::ok)
    }

    /// The report as a Markdown table.
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(
            s,
            "{} matches over {} seasons.\n",
            self.matches, self.seasons
        );
        let _ = writeln!(s, "| Statistic | Simulated | Target | |");
        let _ = writeln!(s, "| --- | ---: | ---: | --- |");
        for c in &self.checks {
            let d = c.decimals;
            let _ = writeln!(
                s,
                "| {} | {:.d$} | {:.d$} to {:.d$} | {} |",
                c.name,
                c.value,
                c.low,
                c.high,
                if c.ok() { "ok" } else { "**out of range**" }
            );
        }
        s
    }
}

/// Builds the calibration report for a set of simulated seasons.
pub fn report(seasons: &[SeasonOutcome]) -> Report {
    let mut n = 0.0;
    let (mut goals, mut home_w, mut draws, mut away_w) = (0.0, 0.0, 0.0, 0.0);
    let (mut shots, mut on_target, mut passes, mut completed) = (0.0, 0.0, 0.0, 0.0);
    let (mut fouls, mut yellows, mut reds, mut corners, mut xg) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut possessions: Vec<f64> = Vec::new();
    let mut zero_zero = 0.0;
    let mut injuries = 0.0;
    for season in seasons {
        for m in &season.matches {
            let r = &m.result;
            n += 1.0;
            goals += f64::from(r.goals[0]) + f64::from(r.goals[1]);
            match r.outcome() {
                1 => home_w += 1.0,
                0 => draws += 1.0,
                _ => away_w += 1.0,
            }
            if r.goals == [0, 0] {
                zero_zero += 1.0;
            }
            for side in &r.players {
                injuries += side.iter().filter(|p| p.injured).count() as f64;
            }
            for t in &r.stats {
                shots += f64::from(t.shots);
                on_target += f64::from(t.shots_on_target);
                passes += f64::from(t.passes);
                completed += f64::from(t.passes_completed);
                fouls += f64::from(t.fouls);
                yellows += f64::from(t.yellows);
                reds += f64::from(t.reds);
                corners += f64::from(t.corners);
                xg += f64::from(t.xg);
                possessions.push(f64::from(t.possession));
            }
        }
    }
    let n = f64::max(n, 1.0);
    possessions.sort_by(f64::total_cmp);
    let p95 = possessions
        .get(possessions.len() * 95 / 100)
        .copied()
        .unwrap_or(0.5);
    let (mut champion, mut bottom, mut top_scorer) = (0.0, 0.0, 0.0);
    for s in seasons {
        champion += f64::from(*s.points.iter().max().unwrap_or(&0));
        bottom += f64::from(*s.points.iter().min().unwrap_or(&0));
        let mut goals: std::collections::BTreeMap<fs_core::ids::PlayerId, u32> =
            std::collections::BTreeMap::new();
        for m in &s.matches {
            for side in &m.result.players {
                for p in side {
                    *goals.entry(p.player).or_default() += u32::from(p.goals);
                }
            }
        }
        top_scorer += f64::from(goals.values().copied().max().unwrap_or(0));
    }
    let sn = f64::max(seasons.len() as f64, 1.0);
    let check = |name, value, low, high, decimals| Check {
        name,
        value,
        low,
        high,
        decimals,
    };
    Report {
        matches: n as usize,
        seasons: seasons.len(),
        checks: vec![
            check("Goals per match", goals / n, 2.5, 3.0, 2),
            check("Home win %", 100.0 * home_w / n, 42.0, 48.0, 1),
            check("Draw %", 100.0 * draws / n, 22.0, 28.0, 1),
            check("Away win %", 100.0 * away_w / n, 27.0, 33.0, 1),
            check("0-0 %", 100.0 * zero_zero / n, 5.0, 10.0, 1),
            check("Shots per team", shots / (2.0 * n), 11.0, 14.0, 1),
            check(
                "Shots on target %",
                100.0 * on_target / shots.max(1.0),
                30.0,
                36.0,
                1,
            ),
            check("xG per shot", xg / shots.max(1.0), 0.09, 0.12, 3),
            check("Goals per xG", goals / xg.max(1e-9), 0.92, 1.08, 2),
            check("Passes per team", passes / (2.0 * n), 400.0, 520.0, 0),
            check(
                "Pass completion %",
                100.0 * completed / passes.max(1.0),
                78.0,
                85.0,
                1,
            ),
            check("Fouls per match", fouls / n, 20.0, 26.0, 1),
            check("Yellow cards per match", yellows / n, 3.0, 5.0, 2),
            check("Red cards per match", reds / n, 0.05, 0.2, 3),
            check("Corners per team", corners / (2.0 * n), 4.0, 6.0, 1),
            check("Injuries per match", injuries / n, 0.4, 0.9, 2),
            check("95th percentile possession %", 100.0 * p95, 62.0, 72.0, 1),
            check("Champion points", champion / sn, 80.0, 95.0, 1),
            check("Bottom club points", bottom / sn, 15.0, 32.0, 1),
            check("Top scorer goals", top_scorer / sn, 22.0, 36.0, 1),
        ]
        .into_iter()
        .chain(tactic_balance(seasons))
        .collect(),
    }
}

/// Season points gained per step of each tactic slider. Sliders are
/// assigned independently of squad quality, so a plain regression slope
/// is fair. A large slope means one style is simply better, which would
/// make tactics a solved problem.
fn tactic_balance(seasons: &[SeasonOutcome]) -> Vec<Check> {
    let names = [
        "Points per mentality step",
        "Points per tempo step",
        "Points per width step",
        "Points per pressing step",
        "Points per defensive line step",
        "Points per directness step",
    ];
    let mut rows: Vec<([f64; 6], f64)> = Vec::new();
    for s in seasons {
        let mut tactic = vec![None; s.points.len()];
        for m in &s.matches {
            tactic[m.home] = Some(m.tactics[0]);
            tactic[m.away] = Some(m.tactics[1]);
        }
        for (team, t) in tactic.iter().enumerate() {
            if let Some(t) = t {
                let x = [
                    t.mentality,
                    t.tempo,
                    t.width,
                    t.pressing,
                    t.defensive_line,
                    t.directness,
                ]
                .map(f64::from);
                rows.push((x, f64::from(s.points[team])));
            }
        }
    }
    let n = rows.len().max(1) as f64;
    let my = rows.iter().map(|r| r.1).sum::<f64>() / n;
    (0..6)
        .map(|k| {
            let mx = rows.iter().map(|r| r.0[k]).sum::<f64>() / n;
            let sxy: f64 = rows.iter().map(|r| (r.0[k] - mx) * (r.1 - my)).sum();
            let sxx: f64 = rows.iter().map(|r| (r.0[k] - mx) * (r.0[k] - mx)).sum();
            Check {
                name: names[k],
                value: if sxx > 0.0 { sxy / sxx } else { 0.0 },
                low: -4.0,
                high: 4.0,
                decimals: 1,
            }
        })
        .collect()
}
