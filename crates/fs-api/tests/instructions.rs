//! Roles, set-piece takers and opposition instructions through the protocol.

use fs_api::dto::{Duty, Marking, MarkingChoice, PlayerRole, RoleChoice, WorldSize};
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
fn instructions_can_be_given() {
    let mut s = career();
    let t = expect!(call(&mut s, Request::Tactics), Tactics { tactics });
    let ins = &t.instructions;
    assert_eq!(ins.roles.len(), 11);
    assert!(ins.roles.iter().all(|r| r.role == r.options[0].role));
    assert!(!ins.roles[0].has_duty);

    // Everyone on attack, a striker as target man, a penalty taker and one
    // opponent marked.
    let mut roles: Vec<RoleChoice> = ins
        .roles
        .iter()
        .map(|r| RoleChoice {
            role: r.role,
            duty: Duty::Attack,
        })
        .collect();
    let striker = ins.roles.iter().position(|r| r.position == "ST").unwrap();
    roles[striker].role = PlayerRole::TargetMan;
    let taker = t.starters[striker];
    let report = expect!(
        call(&mut s, Request::OppositionReport { club: None }),
        OppositionReport { report }
    );
    let target = report.likely_xi[0].player.id;
    let t = expect!(
        call(
            &mut s,
            Request::SetInstructions {
                roles,
                penalty_taker: Some(taker),
                free_kick_taker: None,
                corner_taker: None,
                marking: vec![MarkingChoice {
                    player: target,
                    marking: Marking::Tight
                }],
            }
        ),
        Tactics { tactics }
    );
    let ins = &t.instructions;
    assert_eq!(ins.roles[striker].role, PlayerRole::TargetMan);
    assert!(ins.roles[1..].iter().all(|r| r.duty == Duty::Attack));
    assert_eq!(ins.penalty_taker, Some(taker));
    assert_eq!(ins.marking.len(), 1);
    assert_eq!(ins.marking[0].player.id, target);

    // A role that does not fit is refused.
    let mut bad: Vec<RoleChoice> = ins
        .roles
        .iter()
        .map(|r| RoleChoice {
            role: r.role,
            duty: r.duty,
        })
        .collect();
    bad[0].role = PlayerRole::Poacher;
    let err = expect!(
        call(
            &mut s,
            Request::SetInstructions {
                roles: bad,
                penalty_taker: None,
                free_kick_taker: None,
                corner_taker: None,
                marking: vec![],
            }
        ),
        Error { error }
    );
    assert_eq!(err.code, ErrorCode::Rejected);
}
