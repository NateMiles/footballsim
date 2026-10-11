//! Cups through the protocol: views, schedules, messages and a watched
//! cup tie.

use fs_api::dto::{self, CompetitionKind, StopReason, WorldSize};
use fs_api::{Request, Response, Session};

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

fn career(seed: u32, size: WorldSize, pick: usize) -> Session {
    let mut s = Session::new();
    let choices = expect!(
        call(&mut s, Request::NewWorld { seed, size }),
        ClubChoices { choices }
    );
    let club = choices.clubs[pick].club.id;
    expect!(
        call(
            &mut s,
            Request::StartCareer {
                club,
                first_name: "Alex".into(),
                last_name: "Moss".into(),
            }
        ),
        Overview { overview }
    );
    s
}

#[test]
fn cup_views_follow_the_draw() {
    let mut s = career(41, WorldSize::Tiny, 10);
    let cup = expect!(call(&mut s, Request::Cup { cup: None }), Cup { cup });
    assert!(!cup.cup.continental);
    assert_eq!(cup.season, "2026/27");
    assert_eq!(cup.stages.len(), 1);
    assert_eq!(cup.stages[0].ties.len() * 2 + cup.stages[0].byes.len(), 20);
    assert_eq!(cup.upcoming.last().map(String::as_str), Some("Final"));
    // The club's schedule holds league and cup matches in date order.
    let schedule = expect!(
        call(&mut s, Request::Schedule { club: None }),
        Schedule { schedule }
    );
    assert!(schedule.fixtures.windows(2).all(|w| w[0].date <= w[1].date));
    assert!(
        schedule
            .fixtures
            .iter()
            .any(|f| f.competition.kind == CompetitionKind::League)
    );
    let overview = expect!(call(&mut s, Request::Overview), Overview { overview });
    assert_eq!(overview.cups.len(), 1);
    assert!(overview.cups[0].alive);
    // Through to the spring: later rounds are drawn and messages arrive.
    expect!(
        call(&mut s, Request::SetWatchMatches { watch: false }),
        Overview { overview }
    );
    for _ in 0..40 {
        expect!(call(&mut s, Request::Continue), Continued { continued });
    }
    let cup = expect!(call(&mut s, Request::Cup { cup: None }), Cup { cup });
    assert!(cup.stages.len() >= 3, "{} stages drawn", cup.stages.len());
    for stage in &cup.stages[..cup.stages.len() - 1] {
        for tie in &stage.ties {
            assert!(tie.winner.is_some());
            let leg = &tie.legs[0];
            let r = leg.result.as_ref().unwrap();
            if r.goals[0] == r.goals[1] {
                assert!(r.penalties.is_some());
            }
            assert!(leg.round.is_some());
        }
    }
    let inbox = expect!(call(&mut s, Request::Inbox), Inbox { inbox });
    assert!(
        inbox
            .messages
            .iter()
            .any(|m| m.category == dto::Category::Cup)
    );
    let competitions = expect!(
        call(&mut s, Request::Competitions),
        Competitions { competitions }
    );
    assert_eq!(competitions.nations.len(), 1);
    assert_eq!(competitions.nations[0].cups.len(), 1);
    assert!(competitions.continents.is_empty());
}

#[test]
fn a_cup_tie_can_be_watched_to_the_end() {
    let mut s = career(42, WorldSize::Tiny, 12);
    for _ in 0..120 {
        let c = expect!(call(&mut s, Request::Continue), Continued { continued });
        if c.stopped != StopReason::MatchDay {
            continue;
        }
        let md = c.match_day.unwrap();
        if md.fixture.competition.kind != CompetitionKind::Cup {
            expect!(call(&mut s, Request::SkipMatch), Overview { overview });
            continue;
        }
        assert!(md.fixture.round.is_some());
        let start = expect!(call(&mut s, Request::StartMatch), LiveStart { start });
        assert_eq!(start.fixture.competition.kind, CompetitionKind::Cup);
        let mut t = 0.0;
        let last = loop {
            t += 600.0;
            let chunk = expect!(
                call(&mut s, Request::LiveTick { until: t }),
                LiveChunk { chunk }
            );
            if chunk.finished {
                break chunk;
            }
        };
        let report = expect!(call(&mut s, Request::FinishMatch), MatchReport { report });
        let r = report.fixture.result.as_ref().unwrap();
        assert_eq!(r.goals, last.score);
        // A knockout match always has a winner.
        assert!(r.goals[0] != r.goals[1] || r.penalties.is_some());
        assert_eq!(r.penalties.is_some(), report.shootout.is_some());
        assert_eq!(last.shootout.is_some(), report.shootout.is_some());
        if r.extra_time {
            assert!(last.clock.starts_with("12") || last.clock.starts_with("120"));
        }
        return;
    }
    panic!("no cup match to watch");
}
