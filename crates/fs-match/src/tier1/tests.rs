//! Tier 1 behaviour tests.

use super::play;
use crate::result::{GoalKind, MatchEvent};
use crate::test_support::input;

#[test]
fn plays_a_full_match() {
    let r = play(&input(600, 600, 1));
    assert!(r.goals[0] < 15 && r.goals[1] < 15);
    let s = &r.stats;
    assert!(s[0].passes > 150 && s[1].passes > 150, "{s:?}");
    assert!((s[0].possession + s[1].possession - 1.0).abs() < 1e-3);
    let goals_from_events = r
        .events
        .iter()
        .filter(|e| matches!(e, MatchEvent::Goal { .. }))
        .count();
    assert_eq!(goals_from_events, usize::from(r.goals[0] + r.goals[1]));
    for side in &r.players {
        let scored: u32 = side.iter().map(|p| u32::from(p.goals)).sum();
        let _ = scored;
        assert!(side.iter().filter(|p| p.started).count() == 11);
    }
}

#[test]
fn same_input_same_result() {
    let i = input(600, 550, 7);
    assert_eq!(play(&i), play(&i));
}

#[test]
fn better_team_wins_more() {
    let mut strong = 0;
    let mut weak = 0;
    for k in 0..200 {
        let r = play(&input(750, 450, k));
        match r.outcome() {
            1 => strong += 1,
            -1 => weak += 1,
            _ => {}
        }
    }
    assert!(strong > weak * 3, "strong {strong} weak {weak}");
}

#[test]
fn substitutions_happen() {
    let r = play(&input(600, 600, 3));
    let subs = r
        .events
        .iter()
        .filter(|e| matches!(e, MatchEvent::Substitution { .. }))
        .count();
    assert!(subs >= 1);
}

#[test]
fn minutes_add_up() {
    for k in 0..100 {
        let r = play(&input(600, 580, k));
        for side in 0..2 {
            let total: u32 = r.players[side].iter().map(|p| u32::from(p.minutes)).sum();
            let reds = u32::from(r.stats[side].reds);
            assert!(total <= 11 * 90 + 5, "{total}");
            assert!(
                total + reds * 90 + 90 >= 11 * 90,
                "{total} with {reds} reds"
            );
            for p in &r.players[side] {
                assert!(p.minutes <= 90);
                assert!((3.0..=10.0).contains(&p.rating));
                if !p.started && p.minutes > 0 {
                    assert!(r.events.iter().any(
                        |e| matches!(e, MatchEvent::Substitution { on, .. } if *on == p.player)
                    ));
                }
            }
        }
    }
}

#[test]
fn attacking_mentality_takes_more_shots() {
    let mut shots = [0u32; 2];
    for k in 0..300 {
        let mut i = input(600, 600, k);
        i.home.tactic.mentality = 2;
        let r = play(&i);
        shots[0] += u32::from(r.stats[0].shots);
        let mut j = input(600, 600, k);
        j.home.tactic.mentality = -2;
        shots[1] += u32::from(play(&j).stats[0].shots);
    }
    assert!(shots[0] > shots[1] * 11 / 10, "{shots:?}");
}

#[test]
fn rare_events_happen_at_sane_rates() {
    let (mut reds, mut injuries, mut pens) = (0, 0, 0);
    let n = 500;
    for k in 0..n {
        let r = play(&input(600, 600, k));
        reds += r.stats[0].reds + r.stats[1].reds;
        for e in &r.events {
            match e {
                MatchEvent::Injury { .. } => injuries += 1,
                MatchEvent::Goal {
                    kind: GoalKind::Penalty,
                    ..
                }
                | MatchEvent::PenaltyMissed { .. } => pens += 1,
                _ => {}
            }
        }
    }
    let per = |x: u32| f64::from(x) / f64::from(n as u32);
    assert!(
        (0.03..0.4).contains(&per(u32::from(reds))),
        "reds {}",
        per(u32::from(reds))
    );
    assert!(
        (0.05..1.0).contains(&per(injuries)),
        "injuries {}",
        per(injuries)
    );
    assert!((0.05..0.6).contains(&per(pens)), "penalties {}", per(pens));
}

#[test]
fn neutral_venue_removes_home_edge() {
    let (mut home, mut away) = (0, 0);
    for k in 0..400 {
        let mut i = input(600, 600, k);
        i.neutral = true;
        match play(&i).outcome() {
            1 => home += 1,
            -1 => away += 1,
            _ => {}
        }
    }
    assert!(
        (home as f64 / away.max(1) as f64 - 1.0).abs() < 0.3,
        "{home} {away}"
    );
}

mod live {
    use crate::instruction::{Instruction, InstructionError, Shout};
    use crate::log::Outcome;
    use crate::result::{MatchEvent, Side};
    use crate::test_support::input;
    use crate::tier1::{LiveMatch, play};
    use crate::{Formation, Tactic};

    #[test]
    fn live_play_matches_batch_play() {
        for k in 0..20 {
            let i = input(620, 580, k);
            let mut live = LiveMatch::new(&i);
            while live.advance() {}
            assert_eq!(live.finish(), play(&i), "match {k}");
        }
    }

    #[test]
    fn action_log_is_coherent() {
        let i = input(600, 600, 3);
        let mut live = LiveMatch::new(&i);
        let mut actions = Vec::new();
        while live.advance() {
            actions.extend(live.take_actions());
        }
        assert!(actions.len() > 800, "{} actions", actions.len());
        assert!(
            actions.windows(2).all(|w| w[0].at <= w[1].at + 1e-3),
            "times go backwards"
        );
        let goals = actions
            .iter()
            .filter(|a| a.outcome == Outcome::Goal)
            .count();
        let score = live.score();
        assert_eq!(goals, usize::from(score[0] + score[1]));
        assert!(live.elapsed() > 90.0 * 60.0);
    }

    #[test]
    fn replaying_instructions_reproduces_the_live_match() {
        for k in 0..10 {
            let i = input(600, 600, 100 + k);
            let mut live = LiveMatch::new(&i);
            live.advance_until(30.0 * 60.0);
            live.instruct(Side::Home, Instruction::Shout(Shout::PushForward))
                .unwrap();
            live.advance_until(55.0 * 60.0);
            let lineup = live.lineup(Side::Home);
            let off = lineup.on_pitch[9].unwrap().id;
            let on = lineup.bench[0];
            live.instruct(Side::Home, Instruction::Substitute { off, on })
                .unwrap();
            let mut t = Tactic::balanced(Formation::F433);
            t.pressing = 2;
            live.instruct(Side::Away, Instruction::ChangeTactic(t))
                .unwrap();
            live.advance_until(70.0 * 60.0);
            live.instruct(Side::Away, Instruction::Shout(Shout::CalmDown))
                .unwrap();
            let mut replay = i.clone();
            replay.instructions = live.instructions().to_vec();
            assert_eq!(replay.instructions.len(), 4);
            let watched = live.finish();
            assert_eq!(play(&replay), watched, "match {k}");
            assert!(
                watched
                    .events
                    .iter()
                    .any(|e| matches!(e, MatchEvent::Substitution { on: p, .. } if *p == on))
            );
        }
    }

    #[test]
    fn instructions_change_the_match() {
        let i = input(600, 600, 7);
        let mut live = LiveMatch::new(&i);
        live.advance_until(10.0 * 60.0);
        live.instruct(Side::Home, Instruction::Shout(Shout::DemandMore))
            .unwrap();
        assert_ne!(live.finish(), play(&i));
    }

    #[test]
    fn invalid_instructions_are_rejected() {
        let i = input(600, 600, 8);
        let mut live = LiveMatch::new(&i);
        live.advance_until(60.0);
        let bench = live.lineup(Side::Home).bench;
        let starter = live.lineup(Side::Home).on_pitch[5].unwrap().id;
        assert_eq!(
            live.instruct(
                Side::Home,
                Instruction::Substitute {
                    off: bench[0],
                    on: bench[1]
                }
            ),
            Err(InstructionError::NotOnPitch(bench[0]))
        );
        assert_eq!(
            live.instruct(
                Side::Home,
                Instruction::Substitute {
                    off: starter,
                    on: starter
                }
            ),
            Err(InstructionError::NotOnBench(starter))
        );
        // Five substitutions, then no more.
        for k in 0..5 {
            let l = live.lineup(Side::Home);
            let off = l.on_pitch[1 + k].unwrap().id;
            live.instruct(
                Side::Home,
                Instruction::Substitute {
                    off,
                    on: l.bench[0],
                },
            )
            .unwrap();
        }
        let l = live.lineup(Side::Home);
        assert_eq!(l.substitutions_left, 0);
        assert_eq!(
            live.instruct(
                Side::Home,
                Instruction::Substitute {
                    off: l.on_pitch[8].unwrap().id,
                    on: l.bench[0]
                }
            ),
            Err(InstructionError::NoSubstitutionsLeft)
        );
        while live.advance() {}
        assert_eq!(
            live.instruct(Side::Home, Instruction::Shout(Shout::Encourage)),
            Err(InstructionError::MatchOver)
        );
    }

    #[test]
    fn shouts_wear_off() {
        let i = input(600, 600, 9);
        let mut live = LiveMatch::new(&i);
        live.advance_until(20.0 * 60.0);
        live.instruct(Side::Home, Instruction::Shout(Shout::TightenUp))
            .unwrap();
        assert_eq!(live.lineup(Side::Home).shout, Some(Shout::TightenUp));
        live.advance_until(40.0 * 60.0);
        assert_eq!(live.lineup(Side::Home).shout, None);
    }

    #[test]
    fn manual_sides_get_no_tiredness_substitutions() {
        for k in 0..20 {
            let mut i = input(600, 600, 200 + k);
            i.home.auto_subs = false;
            let r = play(&i);
            let injuries: Vec<_> = r
                .events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        MatchEvent::Injury {
                            side: Side::Home,
                            ..
                        }
                    )
                })
                .collect();
            let subs = r
                .events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        MatchEvent::Substitution {
                            side: Side::Home,
                            ..
                        }
                    )
                })
                .count();
            assert!(
                subs <= injuries.len(),
                "match {k}: {subs} subs, {} injuries",
                injuries.len()
            );
        }
    }
}

mod knockouts {
    use crate::input::Decider;
    use crate::result::{MatchEvent, MatchResult, Side};
    use crate::test_support::input;
    use crate::tier1::{LiveMatch, play};

    fn knockout(key: u64, carried: [u8; 2], extra_time: bool) -> crate::MatchInput {
        let mut i = input(600, 600, key);
        i.decider = Some(Decider {
            carried,
            extra_time,
        });
        i
    }

    fn check_consistent(r: &MatchResult) {
        let full = r.length();
        for side in 0..2 {
            let goals: u32 = r.players[side].iter().map(|p| u32::from(p.goals)).sum();
            assert_eq!(goals, u32::from(r.goals[side]));
            for p in &r.players[side] {
                assert!(p.minutes <= full, "{} minutes of {full}", p.minutes);
            }
        }
        assert!(r.events.iter().all(|e| e.minute() <= full));
    }

    #[test]
    fn knockouts_always_have_a_winner() {
        let (mut extra, mut shootouts) = (0, 0);
        for key in 0..300 {
            let r = play(&knockout(key, [0, 0], true));
            check_consistent(&r);
            assert!(r.tie_winner([0, 0]).is_some());
            if r.extra_time {
                extra += 1;
                assert!(
                    r.players[0].iter().any(|p| p.minutes > 90),
                    "nobody played extra time"
                );
            }
            if let Some(s) = &r.shootout {
                shootouts += 1;
                assert!(r.extra_time);
                assert_eq!(r.goals[0], r.goals[1]);
                assert_ne!(s.score[0], s.score[1]);
            }
        }
        // About a quarter of even matches are level after 90 minutes, and
        // most of those after 120.
        assert!((40..120).contains(&extra), "{extra} went to extra time");
        assert!(shootouts > 15 && shootouts < extra, "{shootouts} shootouts");
    }

    #[test]
    fn without_extra_time_level_ties_go_straight_to_penalties() {
        for key in 0..100 {
            let r = play(&knockout(key, [0, 0], false));
            assert!(!r.extra_time);
            assert_eq!(r.shootout.is_some(), r.goals[0] == r.goals[1]);
        }
    }

    #[test]
    fn decided_ties_play_like_ordinary_matches() {
        for key in 0..60 {
            let plain = play(&input(600, 600, key));
            let ko = play(&knockout(key, [0, 0], true));
            if plain.goals[0] != plain.goals[1] {
                assert_eq!(plain, ko);
            }
        }
    }

    #[test]
    fn second_legs_count_the_first() {
        // Two goals down from the first leg: the home side must score
        // three to win, and level aggregates still need a decider.
        let mut away_through = 0;
        for key in 0..200 {
            let r = play(&knockout(key, [0, 2], true));
            check_consistent(&r);
            let w = r.tie_winner([0, 2]).expect("a winner");
            away_through += usize::from(w == Side::Away);
            if r.shootout.is_some() {
                assert_eq!(r.goals[0], r.goals[1] + 2);
            }
        }
        assert!(away_through > 140, "{away_through}");
    }

    #[test]
    fn live_knockouts_match_batch_play() {
        for key in 0..40 {
            let i = knockout(key, [0, 0], true);
            let batch = play(&i);
            if !batch.extra_time {
                continue;
            }
            let mut live = LiveMatch::new(&i);
            let mut halves = 0;
            while live.advance() {
                halves = halves.max(live.half());
            }
            assert_eq!(halves, 3);
            assert_eq!(live.half_starts().len(), 3);
            assert!(live.minute() >= 120);
            assert_eq!(live.shootout().cloned(), batch.shootout);
            assert_eq!(live.finish(), batch);
            return;
        }
        panic!("no match went to extra time");
    }

    #[test]
    fn extra_time_allows_another_substitution() {
        for key in 0..60 {
            let r = play(&knockout(key, [0, 0], true));
            if r.extra_time {
                let subs = r
                    .events
                    .iter()
                    .filter(|e| {
                        matches!(
                            e,
                            MatchEvent::Substitution {
                                side: Side::Home,
                                ..
                            }
                        )
                    })
                    .count();
                assert!(subs <= 6);
            }
        }
    }
}
