//! The transfer and loan market.

use fs_core::GameDate;
use fs_gen::{GenConfig, Size, generate};
use fs_world::market::asking_price;
use fs_world::transfer::{Listing, OfferKind, OfferStatus, Refusal, market_value};
use fs_world::{
    Actor, ClubId, Command, CommandError, Event, Game, ManagerId, OfferResponse, PlayerId, World,
};

fn date(y: i32, m: u32, d: u32) -> GameDate {
    GameDate::from_ymd(y, m, d).unwrap()
}

fn advance_to(game: &mut Game, target: GameDate) -> Vec<Event> {
    let mut events = Vec::new();
    while game.world().date() < target {
        let days = target.days_since(game.world().date()).min(366) as u16;
        events.extend(
            game.submit(Actor::System, Command::Advance { days })
                .unwrap(),
        );
    }
    events
}

fn human(world: &World) -> (ManagerId, ClubId) {
    let m = world.human_managers().next().unwrap();
    (m, world.managers()[m].club.unwrap())
}

/// A squad player (not a starter) at an AI club the human could afford.
fn target(world: &World, buyer: ClubId) -> (ClubId, PlayerId) {
    let budget = world.clubs()[buyer].finances().transfer_budget;
    for (club, c) in world.clubs().iter().filter(|(c, _)| *c != buyer) {
        let mut squad: Vec<PlayerId> = c.squad().to_vec();
        squad.sort_by_key(|&p| std::cmp::Reverse(world.players().ability(p)));
        for &p in squad.iter().skip(12).take(8) {
            if asking_price(world, club, p).is_some_and(|a| a * 2 <= budget) {
                return (club, p);
            }
        }
    }
    panic!("nobody affordable");
}

#[test]
fn the_ai_market_moves_players_and_money_balances() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Small, 41)));
    advance_to(&mut game, date(2026, 9, 2));
    let w = game.world();
    w.check_invariants().unwrap();
    let h = w.market().history();
    let transfers = h.iter().filter(|r| !r.loan).count();
    let loans = h.iter().filter(|r| r.loan).count();
    assert!(transfers > 300, "{transfers} transfers");
    assert!(loans > 100, "{loans} loans");
    let sales: i64 = w
        .clubs()
        .iter()
        .map(|(_, c)| c.finances().season.player_sales)
        .sum();
    let buys: i64 = w
        .clubs()
        .iter()
        .map(|(_, c)| c.finances().season.player_purchases)
        .sum();
    assert_eq!(sales, buys, "fees move between clubs");
    assert!(w.market().loans().len() >= loans / 2);
    // Loans end at the rollover.
    advance_to(&mut game, date(2027, 7, 2));
    let w = game.world();
    w.check_invariants().unwrap();
    assert!(
        w.market()
            .loans()
            .values()
            .all(|l| l.until > date(2027, 7, 1)),
        "last season's loans are over"
    );
}

#[test]
fn a_human_buys_a_player() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 42)));
    let (m, club) = human(game.world());
    advance_to(&mut game, date(2026, 7, 3));
    let w = game.world();
    let (seller, player) = target(w, club);
    let ask = asking_price(w, seller, player).unwrap();
    let wage = w.players().contract(player).unwrap().wage * 2;
    let budget = w.clubs()[club].finances().transfer_budget;
    let balance = w.clubs()[club].finances().balance;

    // Too low: refused or countered at the asking price.
    game.submit(
        Actor::Manager(m),
        Command::MakeOffer {
            player,
            kind: OfferKind::Transfer { fee: ask / 3 },
            wage,
            years: 3,
        },
    )
    .unwrap();
    assert_eq!(
        game.submit(
            Actor::Manager(m),
            Command::MakeOffer {
                player,
                kind: OfferKind::Transfer { fee: ask },
                wage,
                years: 3
            }
        ),
        Err(CommandError::AlreadyOffered(player))
    );
    let events = game
        .submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    let id = game.world().market().offers().last().unwrap().id;
    assert!(events.iter().any(|e| matches!(
        e,
        Event::OfferUpdated {
            status: OfferStatus::Refused(Refusal::TooLow),
            ..
        }
    )));

    // At the asking price: agreed, then completed the next day.
    game.submit(
        Actor::Manager(m),
        Command::MakeOffer {
            player,
            kind: OfferKind::Transfer { fee: ask },
            wage,
            years: 3,
        },
    )
    .unwrap();
    assert!(game.world().market().offer(id + 1).is_some());
    game.submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    assert_eq!(
        game.world().market().offer(id + 1).unwrap().status,
        OfferStatus::Agreed
    );
    let events = game
        .submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TransferCompleted { player: p, .. } if *p == player))
    );
    let w = game.world();
    assert_eq!(w.players().club(player), Some(club));
    assert_eq!(w.players().contract(player).unwrap().wage, wage);
    assert_eq!(w.clubs()[club].finances().transfer_budget, budget - ask);
    assert!(w.clubs()[club].finances().balance <= balance - ask);
    w.check_invariants().unwrap();
}

#[test]
fn counter_offers_can_be_accepted() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 43)));
    let (m, club) = human(game.world());
    advance_to(&mut game, date(2026, 7, 3));
    let (seller, player) = target(game.world(), club);
    let ask = asking_price(game.world(), seller, player).unwrap();
    let wage = game.world().players().contract(player).unwrap().wage * 2;
    game.submit(
        Actor::Manager(m),
        Command::MakeOffer {
            player,
            kind: OfferKind::Transfer { fee: ask * 8 / 10 },
            wage,
            years: 2,
        },
    )
    .unwrap();
    let id = game.world().market().offers().last().unwrap().id;
    game.submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    let status = game.world().market().offer(id).unwrap().status;
    assert!(
        matches!(status, OfferStatus::Countered { .. }),
        "{status:?}"
    );
    game.submit(Actor::Manager(m), Command::AcceptCounter { offer: id })
        .unwrap();
    assert_eq!(
        game.world().market().offer(id).unwrap().status,
        OfferStatus::Agreed
    );
    game.submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    assert_eq!(game.world().players().club(player), Some(club));
}

#[test]
fn offers_need_an_open_window() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 44)));
    let (m, club) = human(game.world());
    advance_to(&mut game, date(2026, 9, 5));
    let (_, player) = target(game.world(), club);
    assert_eq!(
        game.submit(
            Actor::Manager(m),
            Command::MakeOffer {
                player,
                kind: OfferKind::Transfer { fee: 1 },
                wage: 1,
                years: 1
            }
        ),
        Err(CommandError::WindowClosed)
    );
}

#[test]
fn a_human_sells_a_listed_player() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Small, 45)));
    let (m, club) = human(game.world());
    // List the whole bench for sale: AI clubs will bid.
    let w = game.world();
    let mut squad: Vec<PlayerId> = w.clubs()[club].squad().to_vec();
    squad.sort_by_key(|&p| std::cmp::Reverse(w.players().ability(p)));
    for &p in squad.iter().skip(14).take(10) {
        game.submit(
            Actor::Manager(m),
            Command::SetListing {
                player: p,
                listing: Some(Listing::Transfer),
            },
        )
        .unwrap();
    }
    let mut received = None;
    for _ in 0..40 {
        let events = game
            .submit(Actor::System, Command::Advance { days: 1 })
            .unwrap();
        if let Some(Event::OfferReceived { offer, .. }) = events
            .iter()
            .find(|e| matches!(e, Event::OfferReceived { .. }))
        {
            received = Some(*offer);
            break;
        }
    }
    let offer = received.expect("an AI club bid within 40 days");
    let o = *game.world().market().offer(offer).unwrap();
    let value = market_value(game.world(), o.player);
    assert!(value > 0);
    game.submit(
        Actor::Manager(m),
        Command::RespondToOffer {
            offer,
            response: OfferResponse::Accept,
        },
    )
    .unwrap();
    let status = game.world().market().offer(offer).unwrap().status;
    if status == OfferStatus::Agreed {
        game.submit(Actor::System, Command::Advance { days: 1 })
            .unwrap();
        assert_eq!(game.world().players().club(o.player), Some(o.buyer));
        assert!(game.world().clubs()[club].finances().season.player_sales > 0);
    } else {
        assert_eq!(status, OfferStatus::Refused(Refusal::PlayerRefused));
    }
    game.world().check_invariants().unwrap();
}

#[test]
fn a_human_borrows_a_player_who_goes_back_in_summer() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Small, 46)));
    let (m, club) = human(game.world());
    advance_to(&mut game, date(2026, 7, 2));
    let w = game.world();
    let rep = w.clubs()[club].reputation;
    let (&player, _) = w
        .market()
        .listed()
        .iter()
        .find(|(p, l)| {
            **l == Listing::Loan
                && w.players()
                    .club(**p)
                    .is_some_and(|c| c != club && w.clubs()[c].reputation <= rep)
        })
        .expect("a youngster listed for loan");
    let parent = w.players().club(player).unwrap();
    game.submit(
        Actor::Manager(m),
        Command::MakeOffer {
            player,
            kind: OfferKind::Loan { wage_share: 100 },
            wage: 0,
            years: 0,
        },
    )
    .unwrap();
    advance_to(&mut game, date(2026, 7, 4));
    let w = game.world();
    assert_eq!(w.players().club(player), Some(club), "on loan");
    assert_eq!(w.market().loan(player).unwrap().parent, parent);
    assert_eq!(
        game.submit(Actor::Manager(m), Command::ReleasePlayer { player }),
        Err(CommandError::OnLoan(player))
    );
    advance_to(&mut game, date(2027, 7, 2));
    let w = game.world();
    let back = w.players().club(player);
    assert!(back != Some(club), "the loan ended");
    assert!(w.market().loan(player).is_none_or(|l| l.borrower != club));
    w.check_invariants().unwrap();
}
