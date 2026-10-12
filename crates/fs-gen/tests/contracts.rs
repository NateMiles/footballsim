//! Contract negotiation, bonuses, release clauses and sell-on clauses.

use fs_gen::{GenConfig, Size, generate};
use fs_world::contract::{Outcome, Terms, demands};
use fs_world::happiness::SquadStatus;
use fs_world::transfer::OfferStatus;
use fs_world::{Actor, ClubId, Command, CommandError, Event, Game, ManagerId, PlayerId, World};

fn game(seed: u64, humans: Vec<u32>) -> Game {
    Game::new(generate(&GenConfig {
        human_clubs: humans,
        ..GenConfig::preset(Size::Tiny, seed)
    }))
}

fn manager_of(world: &World, club: ClubId) -> ManagerId {
    world.clubs()[club].manager.unwrap()
}

fn talk(
    game: &mut Game,
    m: ManagerId,
    player: PlayerId,
    terms: Terms,
) -> Result<Outcome, CommandError> {
    let events = game.submit(
        Actor::Manager(m),
        Command::NegotiateContract { player, terms },
    )?;
    Ok(events
        .iter()
        .find_map(|e| match e {
            Event::ContractTalks { outcome, .. } => Some(*outcome),
            _ => None,
        })
        .expect("talks reported"))
}

/// A senior player at the club who is happy enough to talk.
fn senior(world: &World, club: ClubId) -> PlayerId {
    let ps = world.players();
    world.clubs()[club]
        .squad()
        .iter()
        .copied()
        .find(|&p| ps.squad_status(p) == SquadStatus::Regular)
        .unwrap()
}

#[test]
fn an_agent_counters_and_a_fair_offer_is_signed() {
    let mut g = game(91, vec![2]);
    let club = ClubId::new(2);
    let m = manager_of(g.world(), club);
    let p = senior(g.world(), club);
    let d = demands(g.world(), p, club);
    // A little short: the agent names their price.
    let low = Terms {
        wage: (d.wage * 0.85) as u32,
        release_clause: d.terms.release_clause,
        ..d.terms
    };
    let counter = match talk(&mut g, m, p, low).unwrap() {
        Outcome::Countered(c) => c,
        other => panic!("{other:?}"),
    };
    assert!(counter.wage > low.wage);
    let before = g.world().clubs()[club].finances().season;
    assert_eq!(talk(&mut g, m, p, counter).unwrap(), Outcome::Agreed);
    let w = g.world();
    let c = w.players().contract(p).unwrap();
    assert_eq!(c.wage, counter.wage);
    assert_eq!(c.release_clause, counter.release_clause);
    let after = w.clubs()[club].finances().season;
    assert!(after.agent_fees > before.agent_fees);
    assert_eq!(
        after.signing_fees - before.signing_fees,
        counter.signing_fee
    );
}

#[test]
fn an_insulting_offer_ends_talks_for_two_weeks() {
    let mut g = game(92, vec![3]);
    let club = ClubId::new(3);
    let m = manager_of(g.world(), club);
    let p = senior(g.world(), club);
    let d = demands(g.world(), p, club);
    let insult = Terms {
        wage: (d.wage * 0.4) as u32,
        ..d.terms
    };
    assert!(matches!(
        talk(&mut g, m, p, insult).unwrap(),
        Outcome::BrokenOff(_)
    ));
    assert!(matches!(
        talk(&mut g, m, p, d.terms),
        Err(CommandError::TalksClosed(..))
    ));
    // Agents run out of patience with offers that stay short too.
    let q = g.world().clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&q| q != p && g.world().players().squad_status(q) == SquadStatus::Regular)
        .nth(1)
        .unwrap();
    let d = demands(g.world(), q, club);
    let short = Terms {
        wage: (d.wage * 0.75) as u32,
        release_clause: d.terms.release_clause,
        signing_fee: 0,
        ..d.terms
    };
    let mut rounds = 0;
    loop {
        rounds += 1;
        match talk(&mut g, m, q, short).unwrap() {
            Outcome::Countered(_) => assert!(rounds < 5),
            Outcome::BrokenOff(_) => break,
            Outcome::Agreed => panic!("signed for too little"),
        }
    }
    assert!((2..=4).contains(&rounds), "{rounds}");
}

#[test]
fn bonuses_can_stand_in_for_wages_and_are_paid() {
    let mut g = game(93, vec![4]);
    let club = ClubId::new(4);
    let m = manager_of(g.world(), club);
    let w = g.world();
    let p = w.clubs()[club]
        .squad()
        .iter()
        .copied()
        .find(|&p| w.players().squad_status(p) == SquadStatus::Key)
        .unwrap();
    let d = demands(w, p, club);
    let wage = (d.wage * 0.75) as u32;
    // 42 appearances discounted by 15% replace the missing quarter, and more.
    let bonus = (d.wage * 0.25 * 52.0 / (42.0 * 0.85) * 1.15) as u32;
    let terms = Terms {
        wage,
        appearance_bonus: bonus,
        release_clause: d.terms.release_clause,
        ..d.terms
    };
    assert_eq!(talk(&mut g, m, p, terms).unwrap(), Outcome::Agreed);
    let mut paid = 0;
    while paid == 0 {
        g.submit(Actor::System, Command::Advance { days: 7 })
            .unwrap();
        paid = g.world().clubs()[club].finances().season.bonuses;
    }
    assert!(paid >= i64::from(bonus));
}

#[test]
fn a_release_clause_cannot_be_refused_and_a_sell_on_pays_out() {
    // Three human clubs, so every decision is ours.
    let mut g = game(94, vec![1, 5, 9]);
    let (a, b, c) = (ClubId::new(1), ClubId::new(5), ClubId::new(9));
    let (ma, mb, mc) = (
        manager_of(g.world(), a),
        manager_of(g.world(), b),
        manager_of(g.world(), c),
    );
    let p = senior(g.world(), a);
    let d = demands(g.world(), p, a);
    let clause = 2_000_000;
    let renewal = Terms {
        release_clause: Some(clause),
        wage: d.terms.wage * 2,
        ..d.terms
    };
    assert_eq!(talk(&mut g, ma, p, renewal).unwrap(), Outcome::Agreed);

    // B meets the clause with generous terms and a 20% sell-on: no answer
    // needed from A.
    let db = demands(g.world(), p, b);
    let terms = Terms {
        wage: db.terms.wage * 2,
        release_clause: db.terms.release_clause,
        ..db.terms
    };
    g.submit(
        Actor::Manager(mb),
        Command::MakeTransferOffer {
            player: p,
            fee: clause,
            sell_on: 20,
            terms,
        },
    )
    .unwrap();
    let o = *g.world().market().open_offer(b, p).expect("offer");
    assert_eq!(o.status, OfferStatus::Agreed);
    g.submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    assert_eq!(g.world().players().club(p), Some(b));
    assert_eq!(g.world().market().sell_on(p).unwrap().club, a);

    // C buys from B; A gets a fifth of the fee.
    let fee = 3_000_000;
    let dc = demands(g.world(), p, c);
    let terms = Terms {
        wage: dc.terms.wage * 2,
        release_clause: dc.terms.release_clause,
        ..dc.terms
    };
    g.submit(
        Actor::Manager(mc),
        Command::MakeTransferOffer {
            player: p,
            fee,
            sell_on: 0,
            terms,
        },
    )
    .unwrap();
    let id = g.world().market().open_offer(c, p).unwrap().id;
    let before_a = g.world().clubs()[a].finances().season.player_sales;
    let before_b = g.world().clubs()[b].finances().season.player_sales;
    g.submit(
        Actor::Manager(mb),
        Command::RespondToOffer {
            offer: id,
            response: fs_world::OfferResponse::Accept,
        },
    )
    .unwrap();
    g.submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    let w = g.world();
    assert_eq!(w.players().club(p), Some(c));
    assert_eq!(
        w.clubs()[a].finances().season.player_sales - before_a,
        fee / 5
    );
    assert_eq!(
        w.clubs()[b].finances().season.player_sales - before_b,
        fee - fee / 5
    );
    assert!(w.market().sell_on(p).is_none());
    assert_eq!(w.check_invariants(), Ok(()));
}
