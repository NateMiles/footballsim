//! Calibration smoke tests and tier agreement.

use fs_core::Seed;
use fs_lab::{league_world, predict, report, simulate_seasons};
use fs_match::Tier;
use fs_world::{ClubId, matchday};

/// Every check within its range, widened by half its width (a quick run
/// has more noise than the full report in docs/CALIBRATION.md).
fn assert_roughly_calibrated(tier: Tier, seasons: u32) {
    let r = report(&simulate_seasons(7000, seasons, tier));
    for c in &r.checks {
        let margin = (c.high - c.low) * 0.5;
        assert!(
            (c.low - margin..=c.high + margin).contains(&c.value),
            "{tier:?} {}: {} not near {}..{}",
            c.name,
            c.value,
            c.low,
            c.high
        );
    }
}

#[test]
fn tier1_is_roughly_calibrated() {
    assert_roughly_calibrated(Tier::Event, 6);
}

#[test]
fn tier2_is_roughly_calibrated() {
    assert_roughly_calibrated(Tier::Statistical, 10);
}

/// Tier 2 must track tier 1 fixture by fixture. Tier 2 summarises teams
/// in a few numbers, so individual fixtures can drift (see
/// docs/CALIBRATION.md); these bounds catch it getting worse.
#[test]
fn tiers_agree_on_outcomes() {
    let rows = fs_lab::compare::compare_tiers(2000);
    let (mean, worst) = fs_lab::compare::outcome_error(&rows);
    assert!(mean < 0.05, "mean outcome-probability error {mean:.3}");
    assert!(worst < 0.2, "worst outcome-probability error {worst:.3}");
}

#[test]
fn predictions_are_deterministic() {
    let world = league_world(43);
    let home = matchday::team_sheet(&world, ClubId::new(0));
    let away = matchday::team_sheet(&world, ClubId::new(1));
    let a = predict(&home, &away, Seed(9), 500, Tier::Event);
    let b = predict(&home, &away, Seed(9), 500, Tier::Event);
    assert_eq!(a, b);
    assert!((a.home_win + a.draw + a.away_win - 1.0).abs() < 1e-9);
}
