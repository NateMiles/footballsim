//! Training through the protocol.

use fs_api::dto::{IndividualTraining, TrainingFocus, TrainingIntensity, WorldSize};
use fs_api::{ErrorCode, Request, Response, Session};

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
fn training_can_be_planned() {
    let mut s = career();
    let t = expect!(call(&mut s, Request::Training), Training { training });
    assert_eq!(t.focus, TrainingFocus::Balanced);
    assert_eq!((t.growth, t.recovery, t.injury_risk), (0, 0, 0));
    assert_eq!(t.familiarity.iter().filter(|f| f.current).count(), 1);
    assert!(
        t.familiarity
            .iter()
            .any(|f| f.current && f.familiarity == 100)
    );
    let t = expect!(
        call(
            &mut s,
            Request::SetTraining {
                focus: TrainingFocus::Attacking,
                intensity: TrainingIntensity::Hard,
            }
        ),
        Training { training }
    );
    assert_eq!(t.intensity, TrainingIntensity::Hard);
    assert!(t.growth > 0 && t.injury_risk > 0 && t.recovery < 0);
    let player = t.players[3].player.id;
    let t = expect!(
        call(
            &mut s,
            Request::SetIndividualTraining {
                player,
                training: Some(IndividualTraining::Position {
                    position: "WBR".into()
                }),
            }
        ),
        Training { training }
    );
    let row = t.players.iter().find(|p| p.player.id == player).unwrap();
    assert!(row.progress.is_some());
    let err = expect!(
        call(
            &mut s,
            Request::SetIndividualTraining {
                player,
                training: Some(IndividualTraining::Attribute {
                    attribute: "Juggling".into()
                }),
            }
        ),
        Error { error }
    );
    assert_eq!(err.code, ErrorCode::BadRequest);
}
