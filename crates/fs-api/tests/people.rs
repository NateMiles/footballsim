//! The dressing room through the protocol.

use fs_api::dto::{SquadStatus, Talk, WorldSize};
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
fn the_dressing_room_can_be_managed() {
    let mut s = career();
    let d = expect!(call(&mut s, Request::Dynamics), Dynamics { dynamics });
    assert!(d.captain.is_some());
    assert_eq!(d.leaders.len(), 3);
    assert!(d.rows.len() > 20);
    assert_eq!(d.talks.len(), 5);
    assert!(d.rows.windows(2).all(|w| w[0].happiness <= w[1].happiness));

    // Own players show their mood and character; the captain can change.
    let row = d.rows.iter().find(|r| !r.captain && r.age >= 18).unwrap();
    let player = row.player.id;
    let v = expect!(call(&mut s, Request::Player { player }), Player { player });
    let people = v.people.expect("own player");
    assert!(v.personality.is_some());
    assert!(people.can_talk);
    let d = expect!(
        call(&mut s, Request::SetCaptain { player }),
        Dynamics { dynamics }
    );
    assert_eq!(d.captain.unwrap().id, player);

    // A talk gets an answer, then the player needs a week.
    let r = expect!(
        call(
            &mut s,
            Request::TalkToPlayer {
                player,
                talk: Talk::Reassure
            }
        ),
        Talk { result }
    );
    assert!(!r.message.is_empty());
    assert!(!r.player.people.unwrap().can_talk);
    let err = expect!(
        call(
            &mut s,
            Request::TalkToPlayer {
                player,
                talk: Talk::Praise
            }
        ),
        Error { error }
    );
    assert_eq!(err.code, ErrorCode::Rejected);

    // A demotion shows up as a reason.
    let d = expect!(
        call(
            &mut s,
            Request::SetSquadStatus {
                player,
                status: SquadStatus::Backup
            }
        ),
        Dynamics { dynamics }
    );
    let row = d.rows.iter().find(|r| r.player.id == player).unwrap();
    assert_eq!(row.status, SquadStatus::Backup);
    let v = expect!(call(&mut s, Request::Player { player }), Player { player });
    assert!(
        v.people
            .unwrap()
            .mood
            .reasons
            .iter()
            .any(|r| r.text.contains("smaller role"))
    );
    let err = expect!(
        call(
            &mut s,
            Request::AnswerTransferRequest {
                player,
                accept: false
            }
        ),
        Error { error }
    );
    assert_eq!(err.code, ErrorCode::Rejected);
}

#[test]
fn rivals_characters_need_scouting() {
    let mut s = career();
    let mine = expect!(call(&mut s, Request::Dynamics), Dynamics { dynamics });
    let mine: Vec<_> = mine.rows.iter().map(|r| r.player.id).collect();
    let other = (0..200).find(|p| !mine.contains(p)).unwrap();
    let v = expect!(
        call(&mut s, Request::Player { player: other }),
        Player { player }
    );
    assert!(v.people.is_none());
    assert_eq!(v.personality.is_some(), v.knowledge >= 60);
}
