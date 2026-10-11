//! What tactical choices beyond the sliders are worth: roles, duties and
//! opposition instructions, each played against the standard setup.
//!
//! Every fixture of a calibration season is played three times with the
//! same random stream: as it is, with the choice made by the home side,
//! and with it made by the away side. The points the choosing side gains
//! over the standard setup, scaled to a season, should stay small: a
//! choice worth much more than a few points would be one every manager
//! must make, and tactics would be solved.

use crate::calibrate::Check;
use crate::season::{LEAGUE_SIZE, league_world};
use fs_core::attributes::Position;
use fs_match::roles::{Duty, Marking, Role, SlotRole};
use fs_match::skills::Skills;
use fs_match::{MatchInput, TeamSheet, tier1};
use fs_world::competition::double_round_robin;
use fs_world::{ClubId, matchday};

use Position::*;

/// A choice a side can make, applied to its sheet given the opponent's.
/// Returns false when it does not apply (no slot of that kind).
type Apply = fn(&mut TeamSheet, &TeamSheet) -> bool;

/// Gives every slot of these positions a role and duty.
fn roles(sheet: &mut TeamSheet, positions: &[Position], role: Option<Role>, duty: Duty) -> bool {
    let mut changed = false;
    for (i, s) in sheet.tactic.formation.slots().iter().enumerate() {
        let role = role.unwrap_or(Role::standard(s.position));
        if positions.contains(&s.position) && role.fits(s.position) {
            sheet.instructions.roles[i] = Some(SlotRole { role, duty });
            changed = true;
        }
    }
    changed
}

/// The opponent's starters ranked by a skill, best first.
fn ranked(opponent: &TeamSheet, skill: fn(&Skills) -> f64) -> Vec<fs_core::ids::PlayerId> {
    let slots = opponent.tactic.formation.slots();
    let mut v: Vec<(f64, fs_core::ids::PlayerId)> = opponent
        .starters
        .iter()
        .zip(slots)
        .map(|(p, s)| (skill(&Skills::of(p, s.position)), p.id))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|x| x.1).collect()
}

const WIDE: [Position; 4] = [
    MidfielderLeft,
    MidfielderRight,
    AttackingMidLeft,
    AttackingMidRight,
];
const OUTFIELD: [Position; 13] = [
    DefenderRight,
    DefenderCentre,
    DefenderLeft,
    WingBackRight,
    WingBackLeft,
    DefensiveMidfielder,
    MidfielderRight,
    MidfielderCentre,
    MidfielderLeft,
    AttackingMidRight,
    AttackingMidCentre,
    AttackingMidLeft,
    Striker,
];

/// Every choice the report measures.
pub const CHOICES: &[(&str, Apply)] = &[
    ("Sweeper keeper", |s, _| {
        roles(s, &[Goalkeeper], Some(Role::SweeperKeeper), Duty::Support)
    }),
    ("Ball-playing defenders", |s, _| {
        roles(
            s,
            &[DefenderCentre],
            Some(Role::BallPlayingDefender),
            Duty::Support,
        )
    }),
    ("Full-backs as wing-backs", |s, _| {
        roles(
            s,
            &[DefenderLeft, DefenderRight],
            Some(Role::WingBack),
            Duty::Support,
        )
    }),
    ("Full-backs on attack", |s, _| {
        roles(
            s,
            &[DefenderLeft, DefenderRight, WingBackLeft, WingBackRight],
            None,
            Duty::Attack,
        )
    }),
    ("Full-backs on defend", |s, _| {
        roles(
            s,
            &[DefenderLeft, DefenderRight, WingBackLeft, WingBackRight],
            None,
            Duty::Defend,
        )
    }),
    ("Ball-winning midfielders", |s, _| {
        roles(
            s,
            &[DefensiveMidfielder, MidfielderCentre],
            Some(Role::BallWinner),
            Duty::Support,
        )
    }),
    ("Deep-lying playmaker", |s, _| {
        roles(
            s,
            &[DefensiveMidfielder],
            Some(Role::DeepPlaymaker),
            Duty::Support,
        )
    }),
    ("Box-to-box midfielders", |s, _| {
        roles(s, &[MidfielderCentre], Some(Role::BoxToBox), Duty::Support)
    }),
    ("Central playmakers", |s, _| {
        roles(
            s,
            &[MidfielderCentre, AttackingMidCentre],
            Some(Role::Playmaker),
            Duty::Support,
        )
    }),
    ("Central midfielders on attack", |s, _| {
        roles(s, &[MidfielderCentre], None, Duty::Attack)
    }),
    ("Central midfielders on defend", |s, _| {
        roles(s, &[MidfielderCentre], None, Duty::Defend)
    }),
    ("Wingers", |s, _| {
        roles(s, &WIDE, Some(Role::Winger), Duty::Support)
    }),
    ("Inside forwards", |s, _| {
        roles(s, &WIDE, Some(Role::InsideForward), Duty::Attack)
    }),
    ("Wide players on defend", |s, _| {
        roles(s, &WIDE, None, Duty::Defend)
    }),
    ("Shadow striker", |s, _| {
        roles(
            s,
            &[AttackingMidCentre],
            Some(Role::ShadowStriker),
            Duty::Attack,
        )
    }),
    ("Target man", |s, _| {
        roles(s, &[Striker], Some(Role::TargetMan), Duty::Support)
    }),
    ("Poacher", |s, _| {
        roles(s, &[Striker], Some(Role::Poacher), Duty::Attack)
    }),
    ("False nine", |s, _| {
        roles(s, &[Striker], Some(Role::FalseNine), Duty::Support)
    }),
    ("Strikers on attack", |s, _| {
        roles(s, &[Striker], None, Duty::Attack)
    }),
    ("Everyone on attack", |s, _| {
        roles(s, &OUTFIELD, None, Duty::Attack)
    }),
    ("Everyone on defend", |s, _| {
        roles(s, &OUTFIELD, None, Duty::Defend)
    }),
    ("Tight marking on their best finisher", |s, o| {
        let target = ranked(o, |k| k.shoot)[0];
        s.instructions.marking = vec![(target, Marking::Tight)];
        true
    }),
    ("Closing down their best passer", |s, o| {
        let target = ranked(o, |k| k.pass_short)[0];
        s.instructions.marking = vec![(target, Marking::CloseDown)];
        true
    }),
    ("Tight marking on their three best players", |s, o| {
        s.instructions.marking = ranked(o, |k| k.overall)
            .into_iter()
            .take(3)
            .map(|p| (p, Marking::Tight))
            .collect();
        true
    }),
    ("Closing down their three best players", |s, o| {
        s.instructions.marking = ranked(o, |k| k.overall)
            .into_iter()
            .take(3)
            .map(|p| (p, Marking::CloseDown))
            .collect();
        true
    }),
];

fn points(goals: [u8; 2], side: usize) -> f64 {
    match goals[side].cmp(&goals[1 - side]) {
        core::cmp::Ordering::Greater => 3.0,
        core::cmp::Ordering::Equal => 1.0,
        core::cmp::Ordering::Less => 0.0,
    }
}

/// Season points each choice gains over the standard setup, over
/// `seasons` calibration seasons from `first_seed`.
pub fn choice_balance(first_seed: u64, seasons: u32) -> Vec<Check> {
    let mut gain = vec![(0.0f64, 0usize); CHOICES.len()];
    for s in 0..u64::from(seasons) {
        let world = league_world(first_seed + s);
        let n = world.clubs().len().min(LEAGUE_SIZE);
        let sheets: Vec<TeamSheet> = (0..n)
            .map(|c| matchday::team_sheet(&world, ClubId::new(c as u32)))
            .collect();
        let fixtures: Vec<(usize, usize)> = double_round_robin(n).into_iter().flatten().collect();
        // Per fixture and choice: points gained as home and as away side,
        // or None where the choice does not apply.
        let mut out: Vec<Vec<[Option<f64>; 2]>> = vec![Vec::new(); fixtures.len()];
        fs_core::par::for_each_heavy(&mut out, |i, slot| {
            let (h, a) = fixtures[i];
            let play = |home: &TeamSheet, away: &TeamSheet| {
                let input = MatchInput::new(home.clone(), away.clone(), world.seed(), i as u64);
                tier1::play(&input).goals
            };
            let base = play(&sheets[h], &sheets[a]);
            *slot = CHOICES
                .iter()
                .map(|&(_, apply)| {
                    let mut home = sheets[h].clone();
                    let mut away = sheets[a].clone();
                    let as_home = apply(&mut home, &sheets[a])
                        .then(|| points(play(&home, &sheets[a]), 0) - points(base, 0));
                    let as_away = apply(&mut away, &sheets[h])
                        .then(|| points(play(&sheets[h], &away), 1) - points(base, 1));
                    [as_home, as_away]
                })
                .collect();
        });
        for per_fixture in out {
            for (k, sides) in per_fixture.into_iter().enumerate() {
                for d in sides.into_iter().flatten() {
                    gain[k].0 += d;
                    gain[k].1 += 1;
                }
            }
        }
    }
    let per_season = 2.0 * (LEAGUE_SIZE as f64 - 1.0);
    CHOICES
        .iter()
        .zip(gain)
        .map(|(&(name, _), (sum, count))| Check {
            name,
            value: per_season * sum / count.max(1) as f64,
            low: -4.0,
            high: 4.0,
            decimals: 1,
        })
        .collect()
}
