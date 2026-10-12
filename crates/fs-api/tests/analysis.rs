//! Post-match analysis and season statistics through the protocol.

use fs_api::dto::{FixtureKey, WorldSize};
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

fn career() -> Session {
    let mut s = Session::new();
    let choices = expect!(
        call(
            &mut s,
            Request::NewWorld {
                seed: 71,
                size: WorldSize::Tiny
            }
        ),
        ClubChoices { choices }
    );
    let club = choices.clubs[4].club.id;
    expect!(
        call(
            &mut s,
            Request::StartCareer {
                club,
                first_name: "Lee".into(),
                last_name: "Ash".into(),
            }
        ),
        Overview { overview }
    );
    s
}

#[test]
fn matches_can_be_analysed() {
    let mut s = career();
    // Play into the season, letting the assistant manage match days.
    let mut played = 0;
    for _ in 0..12 {
        let c = expect!(call(&mut s, Request::Continue), Continued { continued });
        if c.match_day.is_some() {
            expect!(call(&mut s, Request::SkipMatch), Overview { overview });
            played += 1;
        }
    }
    assert!(played >= 2, "{played}");
    let stats = expect!(
        call(&mut s, Request::SeasonStats { club: None }),
        SeasonStats { stats }
    );
    let all = &stats.competitions[0];
    assert!(all.competition.is_none());
    assert!(all.played >= 2);
    assert_eq!(all.won + all.drawn + all.lost, all.played);
    assert!(all.players.len() >= 11);
    assert!(stats.competitions.len() >= 2);
    let by_competition: u32 = stats.competitions[1..].iter().map(|c| c.played).sum();
    assert_eq!(by_competition, all.played);

    let f = &all.matches[0];
    let a = expect!(
        call(
            &mut s,
            Request::MatchAnalysis {
                fixture: FixtureKey {
                    kind: f.competition.kind,
                    id: f.competition.id,
                    index: f.index,
                }
            }
        ),
        MatchAnalysis { analysis }
    );
    assert_eq!(Some(a.goals), f.result.as_ref().map(|r| r.goals));
    let shots: u16 = a.stats.iter().map(|t| t.shots).sum();
    assert_eq!(a.shots.len(), usize::from(shots));
    assert_eq!(a.xg.len(), a.shots.len() + 1);
    assert!(
        a.sides
            .iter()
            .all(|s| s.touches.len() == 6 && !s.links.is_empty())
    );
    assert!(!a.possession.is_empty());
}
