//! Match preparation through the protocol.

use fs_api::dto::{Formation, Tactic, WorldSize};
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

fn career(seed: u32) -> Session {
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
    let club = choices.clubs[6].club.id;
    expect!(
        call(
            &mut s,
            Request::StartCareer {
                club,
                first_name: "Kim".into(),
                last_name: "Ode".into(),
            }
        ),
        Overview { overview }
    );
    s
}

fn hash(s: &Session) -> fs_core::StateHash {
    s.game().unwrap().world().state_hash()
}

#[test]
fn the_next_opponent_is_reported() {
    let mut s = career(51);
    let report = expect!(
        call(&mut s, Request::OppositionReport { club: None }),
        OppositionReport { report }
    );
    assert_eq!(report.likely_xi.len(), 11);
    assert_eq!(report.comparison.len(), 5);
    assert!(report.fixture.is_some());
    let xg = report
        .expected_goals
        .expect("expected goals for the next match");
    assert!(xg[0] > 0.2 && xg[1] > 0.2, "{xg:?}");
    assert!(!report.style.is_empty());
    assert!(report.knowledge <= 100);
    assert!(report.key_players.len() >= 2);
}

#[test]
fn predictions_compare_tactics_without_changing_the_world() {
    let mut s = career(52);
    let before = hash(&s);
    let p = expect!(
        call(
            &mut s,
            Request::Predict {
                tactic: None,
                runs: 300
            }
        ),
        Prediction { prediction }
    );
    assert_eq!(p.runs, 300);
    assert!((p.home_win + p.draw + p.away_win - 1.0).abs() < 1e-4);
    assert!(!p.scores.is_empty());
    let again = expect!(
        call(
            &mut s,
            Request::Predict {
                tactic: None,
                runs: 300
            }
        ),
        Prediction { prediction }
    );
    assert_eq!(p, again, "predictions are deterministic");
    let bold = Tactic {
        formation: Formation::F433,
        mentality: 2,
        tempo: 2,
        width: 1,
        pressing: 2,
        defensive_line: 2,
        directness: 0,
    };
    let other = expect!(
        call(
            &mut s,
            Request::Predict {
                tactic: Some(bold),
                runs: 300
            }
        ),
        Prediction { prediction }
    );
    assert_eq!(other.tactic, bold);
    let moved = (other.goals[0] - p.goals[0]).abs() + (other.goals[1] - p.goals[1]).abs();
    assert!(moved > 1e-3, "a different tactic plays differently");
    assert_eq!(hash(&s), before, "forecasts never change the world");
}

#[test]
fn season_odds_add_up() {
    let mut s = career(53);
    let view = expect!(
        call(&mut s, Request::Projections { league: None }),
        Projections { projections }
    );
    assert_eq!(view.rows.len(), 20);
    assert_eq!(view.zones[0].name, "Champions");
    let champions: f32 = view.rows.iter().map(|r| r.zones[0]).sum();
    assert!((champions - 1.0).abs() < 1e-3, "{champions}");
    for r in &view.rows {
        let total: f32 = r.positions.iter().sum();
        assert!((total - 1.0).abs() < 1e-3);
        assert!(
            (10.0..110.0).contains(&r.expected_points),
            "{}",
            r.expected_points
        );
    }
    // The strongest clubs are favourites before a ball is kicked.
    let best = view
        .rows
        .iter()
        .max_by(|a, b| a.zones[0].total_cmp(&b.zones[0]))
        .unwrap();
    assert!(best.zones[0] > 0.1);
}
