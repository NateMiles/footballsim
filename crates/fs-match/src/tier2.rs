//! Tier 2: statistical match simulation.
//!
//! Each side's expected goals come from a log-linear model of the two
//! teams' [`TeamStrength`]s. Goals are drawn from Poisson distributions
//! with a Dixon-Coles adjustment for low-scoring draws. Everything else in
//! the result (shots, passes, cards, scorers, ratings) is drawn so season
//! totals look right, without simulating play.
//!
//! [`Coefficients::default`] holds values fitted to tier 1 output by
//! `fs-cli fit-tier2`.

use crate::input::{MatchInput, PlayerSnapshot, Tactic, TeamSheet};
use crate::result::{
    GoalKind, MatchEvent, MatchResult, PlayerMatchStats, Side, TeamStats, rate_players,
};
use crate::shootout;
use crate::skills::Skills;
use crate::strength::TeamStrength;
use fs_core::attributes::Position;
use fs_core::{Rng, math};
use serde::{Deserialize, Serialize};

/// Model coefficients.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coefficients {
    /// Log expected goals for two equal teams on neutral ground.
    pub intercept: f64,
    /// Added for the home side.
    pub home: f64,
    /// Per unit of (own attack - opponent defence).
    pub attack: f64,
    /// Per unit of (own control - opponent control).
    pub control: f64,
    /// Per unit of (own finishing - opponent keeper).
    pub finishing: f64,
    /// Per step of each of the side's own tactic sliders, in
    /// [`Tactic::sliders`] order.
    pub own_tactic: [f64; 6],
    /// Per step of each of the opponent's sliders.
    pub opponent_tactic: [f64; 6],
    /// Dixon-Coles low-score correlation (negative raises 0-0 and 1-1).
    pub rho: f64,
    /// Average xG per shot, to turn goals into shots.
    pub xg_per_shot: f64,
    pub on_target_rate: f64,
    pub passes_per_match: f64,
    pub pass_completion: f64,
    pub fouls_per_team: f64,
    pub yellow_per_foul: f64,
    pub corners_per_team: f64,
    pub reds_per_team: f64,
    /// Expected in-match injuries per team.
    pub injuries_per_team: f64,
    /// Possession logit for the home side: intercept (home edge).
    pub possession_home: f64,
    /// Possession logit per unit of control difference.
    pub possession_slope: f64,
    /// Possession logit per step of directness difference (home - away).
    pub possession_direct: f64,
    /// Possession logit per step of pressing difference (home - away).
    pub possession_press: f64,
}

impl Default for Coefficients {
    /// Fitted to the calibrated tier 1 engine; see [`crate::fitted`].
    fn default() -> Self {
        crate::fitted::TIER2
    }
}

/// Number of goal-model features.
pub const FEATURES: usize = 17;

/// The regression features for one side: `[1, home, attack diff, control
/// diff, finishing diff, own sliders x6, opponent sliders x6]`.
pub fn features(
    own: &TeamStrength,
    opp: &TeamStrength,
    home: bool,
    own_tactic: Tactic,
    opp_tactic: Tactic,
) -> [f64; FEATURES] {
    let mut f = [0.0; FEATURES];
    f[0] = 1.0;
    f[1] = if home { 1.0 } else { 0.0 };
    f[2] = own.attack - opp.defence;
    f[3] = own.control - opp.control;
    f[4] = own.finishing - opp.keeper;
    for (i, v) in own_tactic.sliders().into_iter().enumerate() {
        f[5 + i] = f64::from(v);
    }
    for (i, v) in opp_tactic.sliders().into_iter().enumerate() {
        f[11 + i] = f64::from(v);
    }
    f
}

impl Coefficients {
    /// The goal-model coefficients as a vector matching [`features`].
    pub fn beta(&self) -> [f64; FEATURES] {
        let mut b = [0.0; FEATURES];
        b[..5].copy_from_slice(&[
            self.intercept,
            self.home,
            self.attack,
            self.control,
            self.finishing,
        ]);
        b[5..11].copy_from_slice(&self.own_tactic);
        b[11..].copy_from_slice(&self.opponent_tactic);
        b
    }

    /// Sets the goal-model coefficients from a vector matching
    /// [`features`].
    pub fn set_beta(&mut self, b: [f64; FEATURES]) {
        [
            self.intercept,
            self.home,
            self.attack,
            self.control,
            self.finishing,
        ] = [b[0], b[1], b[2], b[3], b[4]];
        self.own_tactic.copy_from_slice(&b[5..11]);
        self.opponent_tactic.copy_from_slice(&b[11..]);
    }

    /// Rust source for these coefficients, for the generated
    /// `fitted.rs`.
    pub fn to_rust(&self) -> String {
        let arr = |a: &[f64; 6]| {
            let parts: Vec<String> = a.iter().map(|v| format!("{v:.4}")).collect();
            format!("[{}]", parts.join(", "))
        };
        format!(
            "Coefficients {{\n    intercept: {:.4},\n    home: {:.4},\n    attack: {:.4},\n    control: {:.4},\n    finishing: {:.4},\n    own_tactic: {},\n    opponent_tactic: {},\n    rho: {:.3},\n    xg_per_shot: {:.4},\n    on_target_rate: {:.4},\n    passes_per_match: {:.1},\n    pass_completion: {:.4},\n    fouls_per_team: {:.3},\n    yellow_per_foul: {:.4},\n    corners_per_team: {:.3},\n    reds_per_team: {:.4},\n    injuries_per_team: {:.4},\n    possession_home: {:.4},\n    possession_slope: {:.4},\n    possession_direct: {:.4},\n    possession_press: {:.4},\n}}",
            self.intercept,
            self.home,
            self.attack,
            self.control,
            self.finishing,
            arr(&self.own_tactic),
            arr(&self.opponent_tactic),
            self.rho,
            self.xg_per_shot,
            self.on_target_rate,
            self.passes_per_match,
            self.pass_completion,
            self.fouls_per_team,
            self.yellow_per_foul,
            self.corners_per_team,
            self.reds_per_team,
            self.injuries_per_team,
            self.possession_home,
            self.possession_slope,
            self.possession_direct,
            self.possession_press,
        )
    }
}

/// Expected goals for each side.
pub fn expected_goals(input: &MatchInput, c: &Coefficients) -> [f64; 2] {
    let hs = TeamStrength::of(&input.home);
    let aw = TeamStrength::of(&input.away);
    expected_goals_from(&hs, &aw, input, c)
}

fn expected_goals_from(
    hs: &TeamStrength,
    aw: &TeamStrength,
    input: &MatchInput,
    c: &Coefficients,
) -> [f64; 2] {
    let beta = c.beta();
    let dot = |f: [f64; FEATURES]| f.iter().zip(beta).map(|(x, b)| x * b).sum::<f64>();
    let (th, ta) = (input.home.tactic, input.away.tactic);
    let fh = features(hs, aw, !input.neutral, th, ta);
    let fa = features(aw, hs, false, ta, th);
    [math::exp(dot(fh)), math::exp(dot(fa))]
}

/// Possession features for the home side: `[1 (home edge), control
/// diff, directness diff, pressing diff]`.
pub fn possession_features(hs: &TeamStrength, aw: &TeamStrength, input: &MatchInput) -> [f64; 4] {
    let (h, a) = (input.home.tactic.clamped(), input.away.tactic.clamped());
    [
        if input.neutral { 0.0 } else { 1.0 },
        hs.control - aw.control,
        f64::from(h.directness - a.directness),
        f64::from(h.pressing - a.pressing),
    ]
}

fn possession_home(
    hs: &TeamStrength,
    aw: &TeamStrength,
    input: &MatchInput,
    c: &Coefficients,
) -> f64 {
    let f = possession_features(hs, aw, input);
    let b = [
        c.possession_home,
        c.possession_slope,
        c.possession_direct,
        c.possession_press,
    ];
    math::logistic(f.iter().zip(b).map(|(x, y)| x * y).sum::<f64>()).clamp(0.2, 0.8)
}

/// Plays a match with the fitted coefficients.
pub fn play(input: &MatchInput) -> MatchResult {
    play_with(input, &Coefficients::default())
}

/// Plays a match with custom coefficients.
pub fn play_with(input: &MatchInput, c: &Coefficients) -> MatchResult {
    let mut rng = input.rng();
    let (hs, home_skills) = TeamStrength::with_skills(&input.home);
    let (aw, away_skills) = TeamStrength::with_skills(&input.away);
    let lambda = expected_goals_from(&hs, &aw, input, c);
    let full_time = sample_score(&mut rng, lambda, c.rho);
    // Extra time draws from its own stream, so the first 90 minutes are
    // the same whether or not the match is a knockout.
    let mut decider_rng = input.decider_rng();
    let extra_time = input
        .decider
        .is_some_and(|d| d.extra_time && d.level(full_time));
    let mut extra_goals = [0u8; 2];
    if extra_time {
        for side in 0..2 {
            extra_goals[side] =
                poisson(&mut decider_rng, lambda[side] * EXTRA_TIME_RATE).min(10) as u8;
        }
    }
    let goals = [full_time[0] + extra_goals[0], full_time[1] + extra_goals[1]];

    let strengths = [hs, aw];
    let sheets = [&input.home, &input.away];
    let mut stats = [TeamStats::default(); 2];
    let pos_home = possession_home(&hs, &aw, input, c);
    let total_passes = rng.normal(c.passes_per_match, 70.0).max(400.0);
    for side in 0..2 {
        let opp = 1 - side;
        let st = &mut stats[side];
        let poss = if side == 0 { pos_home } else { 1.0 - pos_home };
        st.possession = poss as f32;
        let g = u16::from(goals[side]);
        let extra = (lambda[side] / c.xg_per_shot - lambda[side]).max(1.0);
        st.shots = g + poisson(&mut rng, extra) as u16;
        // Goals are always on target; pick the rate for the other shots so
        // the overall rate matches.
        let expected_shots = lambda[side] / c.xg_per_shot;
        let rest = ((c.on_target_rate * expected_shots - lambda[side])
            / (expected_shots - lambda[side]).max(0.1))
        .clamp(0.05, 0.9);
        st.shots_on_target = g + binomial(&mut rng, st.shots - g, rest);
        st.xg = (lambda[side] * (0.75 + 0.5 * rng.unit_f64())) as f32;
        st.passes = (total_passes * poss).round() as u16;
        let completion = (c.pass_completion
            + 0.6 * (strengths[side].control - strengths[opp].control))
            .clamp(0.5, 0.95);
        st.passes_completed = binomial(&mut rng, st.passes, completion);
        st.fouls = poisson(
            &mut rng,
            c.fouls_per_team * (0.5 + strengths[side].discipline),
        ) as u16;
        st.yellows = binomial(&mut rng, st.fouls, c.yellow_per_foul);
        st.reds = u16::from(rng.chance(c.reds_per_team));
        st.corners = poisson(&mut rng, c.corners_per_team * (0.6 + 0.8 * poss)) as u16;
        st.offsides = poisson(&mut rng, 1.8) as u16;
        st.crosses = poisson(&mut rng, 16.0) as u16;
        st.crosses_completed = binomial(&mut rng, st.crosses, 0.25);
        st.dribbles = poisson(&mut rng, 18.0) as u16;
        st.dribbles_won = binomial(&mut rng, st.dribbles, 0.55);
        st.tackles = poisson(&mut rng, 16.0) as u16;
        st.interceptions = poisson(&mut rng, 10.0) as u16;
    }
    for side in 0..2 {
        let opp_on = stats[1 - side].shots_on_target;
        let opp_goals = u16::from(goals[1 - side]);
        stats[side].saves = opp_on.saturating_sub(opp_goals);
    }

    let mut events = Vec::new();
    let mut players = [Vec::new(), Vec::new()];
    let mut defensive = [Vec::new(), Vec::new()];
    let skills = [home_skills, away_skills];
    for side in 0..2 {
        let (ps, def) = side_players(
            &mut rng,
            sheets[side],
            &skills[side],
            c.injuries_per_team,
            side,
            goals,
            extra_goals[side],
            extra_time,
            &stats[side],
            &mut events,
        );
        players[side] = ps;
        defensive[side] = def;
    }
    events.sort_by_key(MatchEvent::minute);
    let full = if extra_time { 120 } else { 90 };
    let shootout = input.decider.filter(|d| d.level(goals)).map(|_| {
        let lineups = [0, 1].map(|s| on_pitch_at_end(sheets[s], &players[s], full));
        let first = if decider_rng.chance(0.5) {
            Side::Home
        } else {
            Side::Away
        };
        shootout::play(&mut decider_rng, [&lineups[0], &lineups[1]], first)
    });
    let mut result = MatchResult {
        goals,
        stats,
        players,
        events,
        extra_time,
        shootout,
    };
    rate_players(&mut result, &defensive);
    result
}

/// Goal rate in extra time per minute, relative to normal time: tired
/// legs and cautious teams.
const EXTRA_TIME_RATE: f64 = 0.3;

/// The players on the pitch at the final whistle, keeper first, for a
/// shootout. Everyone is assumed a little tired.
fn on_pitch_at_end(sheet: &TeamSheet, ps: &[PlayerMatchStats], full: u8) -> shootout::Lineup {
    let starters = sheet.starters.len();
    let on: Vec<(&PlayerSnapshot, f64)> = sheet
        .starters
        .iter()
        .chain(&sheet.bench)
        .zip(ps)
        .enumerate()
        .filter(|(i, (_, st))| {
            if *i < starters {
                st.minutes == full
            } else {
                st.minutes > 0
            }
        })
        .map(|(_, (p, _))| (p, 0.9))
        .collect();
    shootout::Lineup::new(&on)
}

/// Draws a score from two Poissons with the Dixon-Coles low-score
/// adjustment, by rejection sampling.
pub fn sample_score(rng: &mut Rng, lambda: [f64; 2], rho: f64) -> [u8; 2] {
    let (lh, la) = (lambda[0], lambda[1]);
    let tau = |x: u32, y: u32| match (x, y) {
        (0, 0) => 1.0 - lh * la * rho,
        (0, 1) => 1.0 + lh * rho,
        (1, 0) => 1.0 + la * rho,
        (1, 1) => 1.0 - rho,
        _ => 1.0,
    };
    let max_tau = [tau(0, 0), tau(0, 1), tau(1, 0), tau(1, 1), 1.0]
        .into_iter()
        .fold(0.0, f64::max);
    loop {
        let x = poisson(rng, lh);
        let y = poisson(rng, la);
        if rng.unit_f64() * max_tau < tau(x, y).max(0.0) {
            return [x.min(20) as u8, y.min(20) as u8];
        }
    }
}

/// Poisson sample by inversion (fine for the small means used here).
pub fn poisson(rng: &mut Rng, mean: f64) -> u32 {
    if mean <= 0.0 {
        return 0;
    }
    let limit = math::exp(-mean);
    let mut k = 0;
    let mut p = rng.unit_f64();
    while p > limit && k < 200 {
        k += 1;
        p *= rng.unit_f64();
    }
    k
}

/// Binomial sample: exact for small `n`, normal approximation for large
/// `n` (pass counts), where the exact method would cost hundreds of
/// random draws.
fn binomial(rng: &mut Rng, n: u16, p: f64) -> u16 {
    if n <= 40 {
        return (0..n).filter(|_| rng.chance(p)).count() as u16;
    }
    let n_f = f64::from(n);
    let sd = (n_f * p * (1.0 - p)).max(0.0);
    rng.normal(n_f * p, math::sqrt(sd)).round().clamp(0.0, n_f) as u16
}

/// Builds player stats for one side: minutes and substitutions, scorers,
/// assisters and shot takers, cards.
#[allow(clippy::too_many_arguments)] // private helper; grouping would add a one-use struct
fn side_players(
    rng: &mut Rng,
    sheet: &TeamSheet,
    starter_skills: &[Skills],
    injuries_per_team: f64,
    side: usize,
    goals: [u8; 2],
    extra_goals: u8,
    extra_time: bool,
    stats: &TeamStats,
    events: &mut Vec<MatchEvent>,
) -> (Vec<PlayerMatchStats>, Vec<bool>) {
    let full: u8 = if extra_time { 120 } else { 90 };
    let slots = sheet.tactic.formation.slots();
    let n = sheet.starters.len() + sheet.bench.len();
    let mut ps: Vec<PlayerMatchStats> = Vec::with_capacity(n);
    let mut skills: Vec<Skills> = Vec::with_capacity(n);
    let mut xs: Vec<f64> = Vec::with_capacity(n);
    let mut defensive = Vec::with_capacity(n);
    for (i, p) in sheet.starters.iter().enumerate() {
        let mut s = PlayerMatchStats::new(p.id, true);
        s.minutes = full;
        ps.push(s);
        skills.push(starter_skills[i]);
        xs.push(slots[i].x);
        defensive.push(matches!(
            slots[i].position,
            Position::Goalkeeper
                | Position::DefenderCentre
                | Position::DefenderLeft
                | Position::DefenderRight
                | Position::WingBackLeft
                | Position::WingBackRight
        ));
    }
    for p in &sheet.bench {
        ps.push(PlayerMatchStats::new(p.id, false));
        skills.push(Skills::default());
        xs.push(0.0);
        defensive.push(false);
    }

    // Three substitutions between 55' and 85' for the least durable
    // outfielders.
    let mut outfield: Vec<usize> = (1..sheet.starters.len()).collect();
    outfield.sort_by(|&a, &b| {
        skills[a]
            .stamina
            .total_cmp(&skills[b].stamina)
            .then(a.cmp(&b))
    });
    let subs = sheet.bench.len().min(3);
    for (bench_used, &off) in outfield.iter().take(subs).enumerate() {
        let pos = slots[off].position;
        let on_idx = sheet.starters.len() + bench_used;
        if on_idx >= ps.len() {
            break;
        }
        // Prefer a bench player who suits the position.
        let minute = 55 + rng.below(31) as u8;
        ps[off].minutes = minute;
        ps[on_idx].minutes = full - minute;
        skills[on_idx] = Skills::of(&sheet.bench[bench_used], pos);
        xs[on_idx] = xs[off];
        defensive[on_idx] = defensive[off];
        events.push(MatchEvent::Substitution {
            minute,
            side: Side::from_index(side),
            off: ps[off].player,
            on: ps[on_idx].player,
        });
    }

    // Goals, assists and shots, weighted by position and skill.
    let played: Vec<usize> = (0..ps.len()).filter(|&i| ps[i].minutes > 0).collect();
    let scorer_w: Vec<f64> = played
        .iter()
        .map(|&i| xs[i] * xs[i] * xs[i] * (0.3 + skills[i].shoot) * f64::from(ps[i].minutes))
        .collect();
    let assist_w: Vec<f64> = played
        .iter()
        .map(|&i| {
            let s = &skills[i];
            xs[i] * xs[i] * (0.3 + s.pass_short + s.through + s.cross) * f64::from(ps[i].minutes)
        })
        .collect();
    let own_goals = goals[side];
    for g in 0..own_goals {
        let Some(k) = rng.weighted_index(&scorer_w) else {
            break;
        };
        let scorer = played[k];
        ps[scorer].goals += 1;
        ps[scorer].shots += 1;
        ps[scorer].shots_on_target += 1;
        let mut assist = None;
        if rng.chance(0.72) {
            let mut w = assist_w.clone();
            w[k] = 0.0;
            if let Some(a) = rng.weighted_index(&w) {
                ps[played[a]].assists += 1;
                ps[played[a]].key_passes += 1;
                assist = Some(ps[played[a]].player);
            }
        }
        let kind = if rng.chance(0.15) {
            GoalKind::Header
        } else {
            GoalKind::OpenPlay
        };
        let minute = if g < own_goals - extra_goals {
            1 + rng.below(90) as u8
        } else {
            91 + rng.below(30) as u8
        };
        events.push(MatchEvent::Goal {
            minute,
            side: Side::from_index(side),
            scorer: ps[scorer].player,
            assist,
            kind,
            xg: 0.0,
        });
    }
    let remaining = stats.shots.saturating_sub(u16::from(own_goals));
    for _ in 0..remaining {
        if let Some(k) = rng.weighted_index(&scorer_w) {
            ps[played[k]].shots = ps[played[k]].shots.saturating_add(1);
        }
    }
    // Spread passes and defensive actions by minutes.
    let total_minutes: f64 = played
        .iter()
        .map(|&i| f64::from(ps[i].minutes))
        .sum::<f64>()
        .max(1.0);
    for &i in &played {
        let share = f64::from(ps[i].minutes) / total_minutes;
        ps[i].passes = (f64::from(stats.passes) * share).round() as u16;
        ps[i].passes_completed = (f64::from(stats.passes_completed) * share).round() as u16;
        ps[i].tackles =
            (f64::from(stats.tackles) * share * if defensive[i] { 1.5 } else { 0.7 }).round() as u8;
        ps[i].goals_conceded = goals[1 - side];
    }
    ps[0].saves = stats.saves.min(255) as u8;
    // Injuries, more likely the longer a player was on.
    for &i in &played {
        let share = f64::from(ps[i].minutes) / total_minutes;
        if rng.chance(injuries_per_team * share) {
            ps[i].injured = true;
            events.push(MatchEvent::Injury {
                minute: 1 + rng.below(u32::from(ps[i].minutes.max(1))) as u8,
                side: Side::from_index(side),
                player: ps[i].player,
            });
        }
    }
    // Cards to random outfielders.
    for _ in 0..stats.yellows {
        let pick = played[1 + rng.index(played.len() - 1)];
        ps[pick].yellow = true;
        events.push(MatchEvent::Card {
            minute: 1 + rng.below(90) as u8,
            side: Side::from_index(side),
            player: ps[pick].player,
            red: false,
        });
    }
    for _ in 0..stats.reds {
        let pick = played[1 + rng.index(played.len() - 1)];
        ps[pick].red = true;
        events.push(MatchEvent::Card {
            minute: 1 + rng.below(90) as u8,
            side: Side::from_index(side),
            player: ps[pick].player,
            red: true,
        });
    }
    (ps, defensive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::input;

    #[test]
    fn poisson_mean() {
        let mut rng = Rng::from_u64(1);
        let n = 50_000;
        let total: u32 = (0..n).map(|_| poisson(&mut rng, 1.4)).sum();
        let mean = f64::from(total) / f64::from(n);
        assert!((mean - 1.4).abs() < 0.03, "{mean}");
    }

    #[test]
    fn result_is_consistent() {
        let r = play(&input(600, 500, 3));
        for side in 0..2 {
            let goals: u32 = r.players[side].iter().map(|p| u32::from(p.goals)).sum();
            assert_eq!(goals, u32::from(r.goals[side]));
            assert!(r.stats[side].shots >= u16::from(r.goals[side]));
            assert!(r.stats[side].shots_on_target <= r.stats[side].shots);
        }
        assert_eq!(play(&input(600, 500, 3)), r);
    }

    #[test]
    fn stronger_team_expects_more_goals() {
        let i = input(750, 450, 0);
        let [h, a] = expected_goals(&i, &Coefficients::default());
        assert!(h > a * 1.5, "{h} {a}");
    }
}

#[cfg(test)]
mod knockout_tests {
    use super::*;
    use crate::input::Decider;
    use crate::test_support::input;

    #[test]
    fn knockouts_have_winners_and_keep_normal_time() {
        let mut extra = 0;
        for key in 0..400 {
            let plain = play(&input(600, 600, key));
            let mut i = input(600, 600, key);
            i.decider = Some(Decider {
                carried: [0, 0],
                extra_time: true,
            });
            let r = play(&i);
            assert!(r.tie_winner([0, 0]).is_some());
            if plain.goals[0] != plain.goals[1] {
                assert_eq!(plain, r);
                continue;
            }
            assert!(r.extra_time);
            extra += 1;
            let full = 120;
            for side in 0..2 {
                let goals: u32 = r.players[side].iter().map(|p| u32::from(p.goals)).sum();
                assert_eq!(goals, u32::from(r.goals[side]));
                assert!(r.players[side].iter().all(|p| p.minutes <= full));
                assert_ne!(
                    r.players[side][1].minutes, 90,
                    "a starter's minutes ignore extra time"
                );
            }
            assert!(r.events.iter().all(|e| e.minute() <= full));
            assert_eq!(r.shootout.is_some(), r.goals[0] == r.goals[1]);
        }
        assert!((50..160).contains(&extra), "{extra}");
    }
}
