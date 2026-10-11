//! Milestone 4 through the protocol: money, staff, scouting, the market
//! and jobs.

use fs_api::dto::{self, OfferKind, PlayerSearch, WorldSize};
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

fn career(seed: u32, pick: usize) -> (Session, dto::Overview) {
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
    let club = choices.clubs[pick].club.id;
    let overview = expect!(
        call(
            &mut s,
            Request::StartCareer {
                club,
                first_name: "Kim".into(),
                last_name: "Lane".into(),
            }
        ),
        Overview { overview }
    );
    (s, overview)
}

#[test]
fn the_overview_shows_the_board_and_money() {
    let (_, o) = career(61, 5);
    let board = o.board.unwrap();
    assert!(board.target >= 1 && board.confidence > 0);
    assert!(!board.objective.is_empty());
    assert!(o.balance.unwrap() > 0);
    assert!(o.window_open, "new games start in the summer window");
}

#[test]
fn finances_and_staff() {
    let (mut s, _) = career(62, 5);
    let f = expect!(call(&mut s, Request::Finances), Finances { finances });
    assert!(f.balance > 0 && f.wage_bill > 0 && f.staff_bill > 0);
    assert!(f.facilities.capacity > 0);

    let staff = expect!(call(&mut s, Request::Staff), Staff { staff });
    assert!(staff.staff.len() >= 8);
    assert_eq!(staff.limits.len(), 4);
    let scouts = staff
        .staff
        .iter()
        .filter(|m| m.role == dto::StaffRole::Scout)
        .count();
    let hire = staff
        .available
        .iter()
        .find(|m| m.would_join && m.role == dto::StaffRole::Scout)
        .expect("a scout who would join");
    let after = expect!(
        call(
            &mut s,
            Request::HireStaff {
                staff: hire.id,
                wage: hire.demand,
                years: 2
            }
        ),
        Staff { staff }
    );
    assert_eq!(
        after
            .staff
            .iter()
            .filter(|m| m.role == dto::StaffRole::Scout)
            .count(),
        scouts + 1
    );
    let after = expect!(
        call(&mut s, Request::ReleaseStaff { staff: hire.id }),
        Staff { staff }
    );
    assert!(after.staff.iter().all(|m| m.id != hire.id));
}

#[test]
fn searching_scouting_and_bidding() {
    let (mut s, o) = career(63, 3);
    let me = o.club.unwrap().id;
    let found = expect!(
        call(
            &mut s,
            Request::SearchPlayers {
                search: PlayerSearch {
                    positions: vec!["ST".into()],
                    max_age: Some(27),
                    ..PlayerSearch::default()
                }
            }
        ),
        Search { search }
    );
    assert!(!found.rows.is_empty());
    assert!(
        found
            .rows
            .iter()
            .all(|r| r.player.position == "ST" && r.age <= 27)
    );
    assert!(
        found
            .rows
            .iter()
            .all(|r| r.club.as_ref().is_none_or(|c| c.id != me))
    );
    assert!(found.rows.windows(2).all(|w| w[0].ability >= w[1].ability));

    // A rival's player is an estimate; scouting narrows it.
    let row = found.rows.iter().find(|r| r.club.is_some()).unwrap();
    let id = row.player.id;
    let p = expect!(
        call(&mut s, Request::Player { player: id }),
        Player { player }
    );
    assert!(p.knowledge < 100);
    assert!(p.ability_low <= p.ability && p.ability <= p.ability_high);
    assert!(p.attributes[0].attributes.iter().any(|a| a.low < a.high));
    let p = expect!(
        call(&mut s, Request::ScoutPlayer { player: id }),
        Player { player }
    );
    assert!(p.scouting);
    let t = expect!(call(&mut s, Request::Transfers), Transfers { transfers });
    assert_eq!(t.scouting.len(), 1);
    assert!(t.window_open);

    // Bid the asking price; the seller answers the next day.
    let fee = p.asking_price.unwrap_or(p.value);
    let budget = t.transfer_budget;
    let result = call(
        &mut s,
        Request::MakeOffer {
            player: id,
            kind: OfferKind::Transfer { fee },
            wage: 1_000_000,
            years: 2,
        },
    );
    if fee > budget {
        let e = expect!(result, Error { error });
        assert_eq!(e.code, ErrorCode::Rejected);
        return;
    }
    let t = expect!(result, Transfers { transfers });
    assert_eq!(t.outgoing.len(), 1);
    assert_eq!(t.outgoing[0].status, dto::OfferStatus::Pending);
    call(&mut s, Request::Continue);
    let t = expect!(call(&mut s, Request::Transfers), Transfers { transfers });
    assert_ne!(t.outgoing[0].status, dto::OfferStatus::Pending, "answered");
    let inbox = expect!(call(&mut s, Request::Inbox), Inbox { inbox });
    assert!(
        inbox
            .messages
            .iter()
            .any(|m| m.category == dto::Category::Transfer)
    );
}

#[test]
fn resigning_and_finding_a_new_job() {
    let (mut s, o) = career(64, 10);
    let old = o.club.unwrap();
    let o = expect!(call(&mut s, Request::Resign), Overview { overview });
    assert!(o.club.is_none() && o.board.is_none());
    let e = expect!(call(&mut s, Request::Squad { club: None }), Error { error });
    assert_eq!(e.code, ErrorCode::NoGame);
    let jobs = expect!(call(&mut s, Request::Jobs), Jobs { jobs });
    assert!(!jobs.employed);
    let job = jobs.jobs.iter().find(|j| j.club.id == old.id).unwrap();
    assert!(job.would_hire);
    let o = expect!(
        call(&mut s, Request::ApplyForJob { club: old.id }),
        Overview { overview }
    );
    assert_eq!(o.club.unwrap().id, old.id);
    // Saves keep an unemployed or employed manager alike.
    let save = expect!(call(&mut s, Request::Save), Saved { save });
    let mut loaded = Session::new();
    expect!(
        call(&mut loaded, Request::Load { data: save.data }),
        Overview { overview }
    );
}
