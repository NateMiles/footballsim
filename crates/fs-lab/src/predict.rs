//! Match prediction by Monte Carlo: play a fixture many times.

use fs_core::Seed;
use fs_match::{MatchInput, TeamSheet, Tier, play};
use std::collections::BTreeMap;

/// Outcome distribution of a fixture.
#[derive(Debug, Clone, PartialEq)]
pub struct Prediction {
    pub runs: u32,
    pub home_win: f64,
    pub draw: f64,
    pub away_win: f64,
    pub home_goals: f64,
    pub away_goals: f64,
    pub home_xg: f64,
    pub away_xg: f64,
    /// Most likely scores with their probabilities, most likely first.
    pub scores: Vec<([u8; 2], f64)>,
}

/// Plays `home` against `away` `runs` times and summarises the results.
/// Runs in parallel on native targets; deterministic for a given seed.
pub fn predict(
    home: &TeamSheet,
    away: &TeamSheet,
    seed: Seed,
    runs: u32,
    tier: Tier,
) -> Prediction {
    let mut results = vec![([0u8; 2], [0f32; 2]); runs as usize];
    fs_core::par::for_each_heavy(&mut results, |i, slot| {
        let input = MatchInput {
            home: home.clone(),
            away: away.clone(),
            neutral: false,
            seed,
            key: i as u64,
        };
        let r = play(&input, tier);
        *slot = (r.goals, [r.stats[0].xg, r.stats[1].xg]);
    });
    let n = f64::from(runs.max(1));
    let mut p = Prediction {
        runs,
        home_win: 0.0,
        draw: 0.0,
        away_win: 0.0,
        home_goals: 0.0,
        away_goals: 0.0,
        home_xg: 0.0,
        away_xg: 0.0,
        scores: Vec::new(),
    };
    let mut scores: BTreeMap<[u8; 2], u32> = BTreeMap::new();
    for (g, xg) in &results {
        match g[0].cmp(&g[1]) {
            core::cmp::Ordering::Greater => p.home_win += 1.0,
            core::cmp::Ordering::Equal => p.draw += 1.0,
            core::cmp::Ordering::Less => p.away_win += 1.0,
        }
        p.home_goals += f64::from(g[0]);
        p.away_goals += f64::from(g[1]);
        p.home_xg += f64::from(xg[0]);
        p.away_xg += f64::from(xg[1]);
        *scores.entry(*g).or_default() += 1;
    }
    for v in [
        &mut p.home_win,
        &mut p.draw,
        &mut p.away_win,
        &mut p.home_goals,
        &mut p.away_goals,
        &mut p.home_xg,
        &mut p.away_xg,
    ] {
        *v /= n;
    }
    let mut s: Vec<([u8; 2], f64)> = scores
        .into_iter()
        .map(|(k, v)| (k, f64::from(v) / n))
        .collect();
    s.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    s.truncate(10);
    p.scores = s;
    p
}
