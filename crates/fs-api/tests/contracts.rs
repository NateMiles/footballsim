//! Contract negotiation through the protocol.

use fs_api::dto::{ContractTerms, TalksOutcome, WorldSize};
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
fn a_contract_can_be_negotiated() {
    let mut s = career();
    let squad = expect!(call(&mut s, Request::Squad { club: None }), Squad { squad });
    let player = squad.players.iter().find(|p| !p.academy).unwrap().id;
    let n = expect!(
        call(&mut s, Request::ContractTalks { player }),
        Negotiation { negotiation }
    );
    assert!(n.renewal && !n.wants_out && n.closed_until.is_none());
    assert!(n.offers_left >= 2);
    assert!((3..=8).contains(&n.agent_fee_percent));

    // Half the ask: the agent walks away or counters.
    let low = ContractTerms {
        wage: n.ask.wage / 2,
        ..n.ask
    };
    let r = expect!(
        call(&mut s, Request::NegotiateContract { player, terms: low }),
        Talks { result }
    );
    assert!(!matches!(r.outcome, TalksOutcome::Agreed));
    assert!(!r.message.is_empty());

    // Another player, offered exactly what their agent asks: signed.
    let other = squad
        .players
        .iter()
        .filter(|p| !p.academy && p.id != player)
        .nth(2)
        .unwrap()
        .id;
    let n = expect!(
        call(&mut s, Request::ContractTalks { player: other }),
        Negotiation { negotiation }
    );
    let r = expect!(
        call(
            &mut s,
            Request::NegotiateContract {
                player: other,
                terms: n.ask
            }
        ),
        Talks { result }
    );
    assert_eq!(r.outcome, TalksOutcome::Agreed);
    let v = expect!(
        call(&mut s, Request::Player { player: other }),
        Player { player }
    );
    let c = v.contract.unwrap();
    assert_eq!(c.wage, n.ask.wage);
    assert_eq!(c.release_clause, n.ask.release_clause);
}
