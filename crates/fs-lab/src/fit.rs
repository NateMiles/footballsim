//! Fitting tier 2 to tier 1.
//!
//! Goals per side are modelled as Poisson with a log-linear mean in the
//! features of [`fs_match::tier2::features`]. The coefficients are found
//! by iteratively reweighted least squares (Newton's method for the
//! Poisson likelihood). The remaining tier 2 constants are averages of
//! tier 1 output.

use crate::season::SeasonOutcome;
use fs_core::math;
use fs_match::tier2::{Coefficients, features, possession_features};
use fs_match::{MatchInput, TeamSheet};

const K: usize = fs_match::tier2::FEATURES;

/// Fits tier 2 coefficients to simulated tier 1 seasons.
pub fn fit_tier2(seasons: &[SeasonOutcome]) -> Coefficients {
    let mut rows: Vec<([f64; K], f64)> = Vec::new();
    let mut pairs: Vec<([f64; 2], [u8; 2])> = Vec::new();
    let mut c = Coefficients::default();
    let (mut shots, mut xg, mut on_target, mut passes, mut completed) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut fouls, mut yellows, mut corners, mut reds, mut teams) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut poss_rows: Vec<([f64; 4], f64)> = Vec::new();
    for s in seasons {
        for m in &s.matches {
            let [h, a] = m.strength;
            let g = m.result.goals;
            let fh = features(&h, &a, true, m.tactics[0], m.tactics[1]);
            let fa = features(&a, &h, false, m.tactics[1], m.tactics[0]);
            rows.push((fh, f64::from(g[0])));
            rows.push((fa, f64::from(g[1])));
            for t in &m.result.stats {
                teams += 1.0;
                shots += f64::from(t.shots);
                xg += f64::from(t.xg);
                on_target += f64::from(t.shots_on_target);
                passes += f64::from(t.passes);
                completed += f64::from(t.passes_completed);
                fouls += f64::from(t.fouls);
                yellows += f64::from(t.yellows);
                corners += f64::from(t.corners);
                reds += f64::from(t.reds);
            }
            let ph = f64::from(m.result.stats[0].possession).clamp(0.05, 0.95);
            let input = tactics_only_input(m.tactics);
            poss_rows.push((
                possession_features(&h, &a, &input),
                math::ln(ph / (1.0 - ph)),
            ));
            pairs.push(([fh, fa].map(|_| 0.0), g));
        }
    }
    let beta = poisson_regression(&rows);
    c.set_beta(beta);

    // Expected goals per match under the fitted model, for rho.
    let mut idx = 0;
    for p in &mut pairs {
        let dot = |f: &[f64; K]| f.iter().zip(beta).map(|(x, b)| x * b).sum::<f64>();
        p.0 = [
            math::exp(dot(&rows[idx].0)),
            math::exp(dot(&rows[idx + 1].0)),
        ];
        idx += 2;
    }
    c.rho = fit_rho(&pairs);

    let teams = f64::max(teams, 1.0);
    c.xg_per_shot = xg / shots.max(1.0);
    c.on_target_rate = on_target / shots.max(1.0);
    c.passes_per_match = 2.0 * passes / teams;
    c.pass_completion = completed / passes.max(1.0);
    c.fouls_per_team = fouls / teams;
    c.yellow_per_foul = yellows / fouls.max(1.0);
    c.corners_per_team = corners / teams;
    c.reds_per_team = reds / teams;
    let [ph, ps, pd, pp] = least_squares(&poss_rows);
    c.possession_home = ph;
    c.possession_slope = ps;
    c.possession_direct = pd;
    c.possession_press = pp;
    c
}

/// A match input carrying only the two tactics, for computing possession
/// features from a record.
fn tactics_only_input(tactics: [fs_match::Tactic; 2]) -> MatchInput {
    let sheet = |tactic| TeamSheet {
        tactic,
        starters: Vec::new(),
        bench: Vec::new(),
    };
    MatchInput {
        home: sheet(tactics[0]),
        away: sheet(tactics[1]),
        neutral: false,
        seed: fs_core::Seed(0),
        key: 0,
    }
}

/// Ordinary least squares with four regressors (include a constant column
/// for an intercept).
fn least_squares(rows: &[([f64; 4], f64)]) -> [f64; 4] {
    let mut xtx = [[0.0; K]; K];
    let mut xty = [0.0; K];
    for (x, y) in rows {
        for i in 0..4 {
            xty[i] += x[i] * y;
            for j in 0..4 {
                xtx[i][j] += x[i] * x[j];
            }
        }
    }
    // Pad the unused dimensions so the shared solver stays well-posed.
    for (i, row) in xtx.iter_mut().enumerate().skip(4) {
        row[i] = 1.0;
    }
    let b = solve(xtx, xty);
    [b[0], b[1], b[2], b[3]]
}

/// Maximum-likelihood Poisson regression by IRLS.
pub fn poisson_regression(rows: &[([f64; K], f64)]) -> [f64; K] {
    let mean_y = rows.iter().map(|r| r.1).sum::<f64>() / rows.len().max(1) as f64;
    let mut beta = [0.0; K];
    beta[0] = math::ln(mean_y.max(0.01));
    for _ in 0..50 {
        let mut xtwx = [[0.0; K]; K];
        let mut grad = [0.0; K];
        for (x, y) in rows {
            let eta: f64 = x.iter().zip(beta).map(|(a, b)| a * b).sum();
            let mu = math::exp(eta.clamp(-10.0, 5.0));
            for i in 0..K {
                grad[i] += x[i] * (y - mu);
                for j in 0..K {
                    xtwx[i][j] += x[i] * x[j] * mu;
                }
            }
        }
        // Tiny ridge keeps an unused feature (e.g. constant mentality)
        // from making the system singular.
        for (i, row) in xtwx.iter_mut().enumerate() {
            row[i] += 1e-6;
        }
        let step = solve(xtwx, grad);
        let mut max_step: f64 = 0.0;
        for i in 0..K {
            beta[i] += step[i];
            max_step = max_step.max(step[i].abs());
        }
        if max_step < 1e-10 {
            break;
        }
    }
    beta
}

/// Solves `a x = b` by Gaussian elimination with partial pivoting.
fn solve(mut a: [[f64; K]; K], mut b: [f64; K]) -> [f64; K] {
    for col in 0..K {
        let pivot = (col..K)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap_or(col);
        a.swap(col, pivot);
        b.swap(col, pivot);
        let d = a[col][col];
        if d.abs() < 1e-15 {
            continue;
        }
        let pivot_row = a[col];
        for row in col + 1..K {
            let f = a[row][col] / d;
            for (x, p) in a[row][col..].iter_mut().zip(&pivot_row[col..]) {
                *x -= f * p;
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = [0.0; K];
    for row in (0..K).rev() {
        let s: f64 = (row + 1..K).map(|k| a[row][k] * x[k]).sum();
        x[row] = if a[row][row].abs() < 1e-15 {
            0.0
        } else {
            (b[row] - s) / a[row][row]
        };
    }
    x
}

/// Dixon-Coles rho by grid search on the low-score likelihood.
fn fit_rho(pairs: &[([f64; 2], [u8; 2])]) -> f64 {
    let mut best = (f64::NEG_INFINITY, 0.0);
    for step in 0..=60 {
        let rho = -0.2 + 0.005 * f64::from(step);
        let ll: f64 = pairs
            .iter()
            .map(|&([lh, la], [x, y])| {
                let tau = match (x, y) {
                    (0, 0) => 1.0 - lh * la * rho,
                    (0, 1) => 1.0 + lh * rho,
                    (1, 0) => 1.0 + la * rho,
                    (1, 1) => 1.0 - rho,
                    _ => 1.0,
                };
                math::ln(tau.max(1e-9))
            })
            .sum();
        if ll > best.0 {
            best = (ll, rho);
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_core::Rng;
    use fs_match::tier2::poisson;

    #[test]
    fn regression_recovers_known_coefficients() {
        let mut truth = [0.0; K];
        truth[..5].copy_from_slice(&[0.2, 0.25, 2.0, 0.0, 1.0]);
        truth[5] = 0.05;
        truth[11] = 0.1;
        let mut rng = Rng::from_u64(1);
        let rows: Vec<([f64; K], f64)> = (0..60_000)
            .map(|_| {
                let mut x = [0.0; K];
                x[0] = 1.0;
                x[1] = f64::from(u8::from(rng.chance(0.5)));
                for v in &mut x[2..5] {
                    *v = rng.range_f64(-0.3, 0.3);
                }
                for v in &mut x[5..] {
                    *v = f64::from(rng.range_i32(-2, 2));
                }
                let eta: f64 = x.iter().zip(truth).map(|(a, b)| a * b).sum();
                (x, f64::from(poisson(&mut rng, math::exp(eta))))
            })
            .collect();
        let beta = poisson_regression(&rows);
        for (b, t) in beta.iter().zip(truth) {
            assert!((b - t).abs() < 0.08, "{beta:?}");
        }
    }
}
