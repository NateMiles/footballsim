//! The protocol end to end, as a UI would use it.

use fs_api::dto::{self, Instruction, Shout, StopReason, WorldSize};
use fs_api::{ErrorCode, Request, Response, Session};

/// Sends a request through JSON, as every host does.
fn call(s: &mut Session, r: Request) -> Response {
    let json = serde_json::to_string(&r).unwrap();
    serde_json::from_str(&s.call_json(&json)).unwrap()
}

macro_rules! expect {
    ($response:expr, $variant:ident { $field:ident }) => {
        match $response {
            Response::$variant { $field } => $field,
            other => panic!("expected {}, got {other:?}", stringify!($variant)),
        }
    };
}

/// A new career at the third-best club.
fn career(seed: u32) -> (Session, dto::Overview) {
    let mut s = Session::new();
    let choices = expect!(
        call(
            &mut s,
            Request::NewWorld {
                seed,
                size: WorldSize::Tiny
            }
        ),
        ClubChoices { choices }
    );
    assert_eq!(choices.clubs.len(), 20);
    assert_eq!(choices.clubs[0].stars, 5);
    let club = choices.clubs[2].club.id;
    let overview = expect!(
        call(
            &mut s,
            Request::StartCareer {
                club,
                first_name: "Sam".into(),
                last_name: "Reed".into(),
            }
        ),
        Overview { overview }
    );
    assert_eq!(overview.club.as_ref().unwrap().id, club);
    assert_eq!(overview.manager.name, "Sam Reed");
    assert!(overview.watch_matches);
    (s, overview)
}

/// Continues until a match is waiting.
fn to_match_day(s: &mut Session) -> dto::MatchDay {
    for _ in 0..30 {
        let c = expect!(call(s, Request::Continue), Continued { continued });
        if c.stopped == StopReason::MatchDay {
            return c.match_day.unwrap();
        }
    }
    panic!("no match day");
}

#[test]
fn hello_and_errors() {
    let mut s = Session::new();
    let hello = call(&mut s, Request::Hello);
    assert!(matches!(
        hello,
        Response::Hello {
            in_game: false,
            protocol: fs_api::PROTOCOL_VERSION,
            ..
        }
    ));
    let err = expect!(call(&mut s, Request::Overview), Error { error });
    assert_eq!(err.code, ErrorCode::NoGame);
    let bad: Response = serde_json::from_str(&s.call_json("{\"type\":\"nope\"}")).unwrap();
    assert_eq!(expect!(bad, Error { error }).code, ErrorCode::BadRequest);
}

#[test]
fn the_screens_have_what_they_need() {
    let (mut s, overview) = career(1);
    let inbox = expect!(call(&mut s, Request::Inbox), Inbox { inbox });
    assert_eq!(inbox.unread, 1, "a welcome message");

    let squad = expect!(call(&mut s, Request::Squad { club: None }), Squad { squad });
    assert!(squad.players.len() >= 20);
    let starters = squad
        .players
        .iter()
        .filter(|p| p.selection == dto::Selection::Starter)
        .count();
    assert_eq!(starters, 11);

    let player = expect!(
        call(
            &mut s,
            Request::Player {
                player: squad.players[0].id
            }
        ),
        Player { player }
    );
    assert!(player.own);
    assert!(player.attributes.len() >= 3);
    assert_eq!(player.position_ratings.len(), 14);

    let tactics = expect!(call(&mut s, Request::Tactics), Tactics { tactics });
    assert_eq!(tactics.starters.len(), 11);
    assert_eq!(tactics.formations.len(), 6);
    assert!(!tactics.own_tactic && !tactics.own_lineup);

    let table = expect!(
        call(&mut s, Request::Table { league: None }),
        Table { table }
    );
    assert_eq!(table.rows.len(), 20);
    assert_eq!(table.rows[0].zone, Some(dto::TableZone::Champion));

    let fixtures = expect!(
        call(
            &mut s,
            Request::Fixtures {
                league: None,
                club: None
            }
        ),
        Fixtures { fixtures }
    );
    assert_eq!(fixtures.fixtures.len(), 38);
    assert_eq!(overview.next_fixture.as_ref(), fixtures.fixtures.first());
}

#[test]
fn tactics_and_lineups_can_be_chosen() {
    let (mut s, _) = career(2);
    let tactics = expect!(call(&mut s, Request::Tactics), Tactics { tactics });
    let mut tactic = tactics.tactic;
    tactic.formation = dto::Formation::F433;
    tactic.pressing = 2;
    let after = expect!(
        call(
            &mut s,
            Request::SetTactic {
                tactic: Some(tactic)
            }
        ),
        Tactics { tactics }
    );
    assert_eq!(after.tactic, tactic);
    assert!(after.own_tactic);

    let mut starters = after.starters.clone();
    let mut bench = after.bench.clone();
    std::mem::swap(&mut starters[10], &mut bench[0]);
    let after = expect!(
        call(
            &mut s,
            Request::SetLineup {
                lineup: Some(dto::Lineup {
                    starters: starters.clone(),
                    bench,
                })
            }
        ),
        Tactics { tactics }
    );
    assert_eq!(after.starters, starters);
    assert!(after.own_lineup);

    let err = expect!(
        call(
            &mut s,
            Request::SetLineup {
                lineup: Some(dto::Lineup {
                    starters: vec![starters[0]; 11],
                    bench: vec![],
                })
            }
        ),
        Error { error }
    );
    assert_eq!(err.code, ErrorCode::Rejected);
}

#[test]
fn a_watched_match_is_the_match_the_world_records() {
    let (mut s, overview) = career(3);
    let day = to_match_day(&mut s);
    assert_eq!(
        day.fixture.home.id == overview.club.as_ref().unwrap().id,
        day.side == dto::Side::Home
    );
    let start = expect!(call(&mut s, Request::StartMatch), LiveStart { start });
    assert_eq!(start.side, day.side);
    assert_eq!(start.lineups[0].on_pitch.len(), 11);
    assert!(start.players.len() >= 22);

    assert_eq!(start.team_talks.len(), 6);
    // A team talk before kick-off: every player reacts.
    let talked = expect!(
        call(
            &mut s,
            Request::LiveInstruct {
                instruction: Instruction::TeamTalk {
                    talk: dto::TeamTalk::Motivate
                }
            }
        ),
        LiveChunk { chunk }
    );
    assert!(talked.at_break);
    assert!(talked.reactions.len() >= 11);

    // Watch in 30-second steps, with a substitution and a shout, and a
    // half-time team talk when play stops for the break.
    let mut half_time_talk = false;
    let mut t = 0.0;
    let mut last;
    let mut frames = 0;
    let mut lines = 0;
    let side = if start.side == dto::Side::Home { 0 } else { 1 };
    loop {
        t += 30.0;
        let chunk = expect!(
            call(&mut s, Request::LiveTick { until: t }),
            LiveChunk { chunk }
        );
        frames += chunk.frames.len();
        lines += chunk.commentary.len();
        if chunk.at_break && !chunk.finished && !half_time_talk {
            assert!(chunk.elapsed >= 45.0 * 60.0 && chunk.elapsed < t);
            let after = expect!(
                call(
                    &mut s,
                    Request::LiveInstruct {
                        instruction: Instruction::TeamTalk {
                            talk: dto::TeamTalk::Demand
                        }
                    }
                ),
                LiveChunk { chunk }
            );
            assert!(!after.reactions.is_empty());
            half_time_talk = true;
        }
        if (3600.0..3630.0).contains(&t) {
            let l = &chunk.lineups[side];
            let off = l.on_pitch[9].as_ref().unwrap().id;
            let on = l.bench[0];
            let left = l.substitutions_left;
            let after = expect!(
                call(
                    &mut s,
                    Request::LiveInstruct {
                        instruction: Instruction::Substitute { off, on }
                    }
                ),
                LiveChunk { chunk }
            );
            assert_eq!(after.lineups[side].substitutions_left, left - 1);
            call(
                &mut s,
                Request::LiveInstruct {
                    instruction: Instruction::Shout {
                        shout: Shout::Encourage,
                    },
                },
            );
            // The team cannot change off the pitch mid-match.
            let err = expect!(
                call(&mut s, Request::SetTactic { tactic: None }),
                Error { error }
            );
            assert_eq!(err.code, ErrorCode::Rejected);
        }
        let done = chunk.finished;
        last = Some(chunk);
        if done {
            break;
        }
    }
    assert!(half_time_talk);
    let last = last.unwrap();
    assert!(frames > 5 * 90 * 60 / 2, "{frames} frames");
    assert!(lines > 20, "{lines} commentary lines");

    let report = expect!(call(&mut s, Request::FinishMatch), MatchReport { report });
    let result = report.fixture.result.unwrap();
    assert_eq!(result.goals, last.score);
    assert!(
        report
            .events
            .iter()
            .any(|e| matches!(e, dto::MatchEvent::Substitution { off, .. } if *off != 0))
    );

    let overview = expect!(call(&mut s, Request::Overview), Overview { overview });
    assert!(overview.match_day.is_none());
    assert_eq!(
        overview.last_fixture.unwrap().result.unwrap().goals,
        last.score
    );
    let inbox = expect!(call(&mut s, Request::Inbox), Inbox { inbox });
    assert!(
        inbox
            .messages
            .iter()
            .any(|m| m.category == dto::Category::Match)
    );
}

#[test]
fn a_match_can_be_skipped() {
    let (mut s, _) = career(4);
    to_match_day(&mut s);
    let overview = expect!(call(&mut s, Request::SkipMatch), Overview { overview });
    assert!(overview.match_day.is_none());
    assert!(overview.last_fixture.unwrap().result.is_some());
    let err = expect!(
        call(&mut s, Request::LiveTick { until: 10.0 }),
        Error { error }
    );
    assert_eq!(err.code, ErrorCode::NoMatch);
}

#[test]
fn saves_load_to_the_same_game() {
    let (mut s, _) = career(5);
    to_match_day(&mut s);
    call(&mut s, Request::SkipMatch);
    call(&mut s, Request::MarkRead { id: None });
    let save = expect!(call(&mut s, Request::Save), Saved { save });
    assert_eq!(save.summary.manager, "Sam Reed");

    let mut loaded = Session::new();
    let overview = expect!(
        call(
            &mut loaded,
            Request::Load {
                data: save.data.clone()
            }
        ),
        Overview { overview }
    );
    assert_eq!(overview.unread, 0);
    assert_eq!(
        loaded.game().unwrap().world().state_hash(),
        s.game().unwrap().world().state_hash()
    );
    // Both continue identically.
    let a = expect!(call(&mut s, Request::Continue), Continued { continued });
    let b = expect!(
        call(&mut loaded, Request::Continue),
        Continued { continued }
    );
    assert_eq!(a, b);
    assert_eq!(
        loaded.game().unwrap().world().state_hash(),
        s.game().unwrap().world().state_hash()
    );

    let err = expect!(
        call(
            &mut loaded,
            Request::Load {
                data: "AAAA".into()
            }
        ),
        Error { error }
    );
    assert_eq!(err.code, ErrorCode::BadSave);
}

#[test]
fn a_performance_test_runs_beside_the_game() {
    let (mut s, overview) = career(9);
    let status = expect!(
        call(
            &mut s,
            Request::BenchmarkStart {
                size: WorldSize::Tiny
            }
        ),
        Benchmark { benchmark }
    );
    assert!(status.active);
    assert_eq!(status.clubs, 20);
    assert!(status.threads >= 1);
    let status = expect!(
        call(&mut s, Request::BenchmarkRun { days: 30 }),
        Benchmark { benchmark }
    );
    assert_eq!(status.date, "2026-07-31");
    let status = expect!(
        call(&mut s, Request::BenchmarkRun { days: 30 }),
        Benchmark { benchmark }
    );
    assert!(status.matches > 0, "August has matches");
    let status = expect!(call(&mut s, Request::BenchmarkEnd), Benchmark { benchmark });
    assert!(!status.active);
    // The game itself did not move.
    let now = expect!(call(&mut s, Request::Overview), Overview { overview });
    assert_eq!(now.date, overview.date);
}
