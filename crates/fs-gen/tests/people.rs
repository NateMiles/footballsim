//! Players as people: statuses, moods, talks, promises, transfer requests
//! and captains.

use fs_core::GameDate;
use fs_gen::{GenConfig, Size, generate};
use fs_match::pick_team;
use fs_world::happiness::{self, Feeling, Reason, SquadStatus};
use fs_world::matchday::{snapshot, team_sheet};
use fs_world::people::{age_years, wage_demand};
use fs_world::talk::{Reaction, Talk};
use fs_world::transfer::Listing;
use fs_world::{
    Actor, ClubId, Command, CommandError, Event, Game, Lineup, ManagerId, PlayerId, World,
};

fn date(y: i32, m: u32, d: u32) -> GameDate {
    GameDate::from_ymd(y, m, d).unwrap()
}

fn advance_to(game: &mut Game, until: GameDate) -> Vec<Event> {
    let mut events = Vec::new();
    while game.world().date() < until {
        let days = until.days_since(game.world().date()).min(300) as u16;
        events.extend(
            game.submit(Actor::System, Command::Advance { days })
                .unwrap(),
        );
    }
    events
}

/// A world where one club is human-run, with the assistant handling
/// contracts.
fn one_human(seed: u64, club: u32) -> (Game, ManagerId, ClubId) {
    let mut game = Game::new(generate(&GenConfig {
        human_clubs: vec![club],
        ..GenConfig::preset(Size::Tiny, seed)
    }));
    let m = game.world().human_managers().next().unwrap();
    game.submit(Actor::Manager(m), Command::SetDelegation { squad: true })
        .unwrap();
    (game, m, ClubId::new(club))
}

fn statuses(world: &World, club: ClubId, s: SquadStatus) -> Vec<PlayerId> {
    world.clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&p| world.players().squad_status(p) == s)
        .collect()
}

/// Advances a day at a time, keeping `out` out of the team (a lineup with
/// an injured player is otherwise filled from the whole squad).
fn advance_without(
    game: &mut Game,
    m: ManagerId,
    club: ClubId,
    out: &[PlayerId],
    until: GameDate,
) -> Vec<Event> {
    let mut events = Vec::new();
    while game.world().date() < until {
        let out: Vec<PlayerId> = out
            .iter()
            .copied()
            .filter(|&p| game.world().players().club(p) == Some(club))
            .collect();
        leave_out(game, m, club, &out);
        let next = game.world().date().add_days(1);
        events.extend(advance_to(game, next));
    }
    events
}

/// Keeps `out` out of the team: the best eleven and bench from everyone
/// else.
fn leave_out(game: &mut Game, m: ManagerId, club: ClubId, out: &[PlayerId]) {
    let w = game.world();
    let squad: Vec<_> = w.clubs()[club]
        .squad()
        .iter()
        .filter(|&&p| !out.contains(&p) && w.players().available(p, w.date()))
        .map(|&p| snapshot(w, p))
        .collect();
    let sheet = pick_team(&squad, team_sheet(w, club).tactic);
    let lineup = Lineup {
        starters: sheet.starters.iter().map(|s| s.id).collect(),
        bench: sheet.bench.iter().map(|s| s.id).collect(),
    };
    game.submit(
        Actor::Manager(m),
        Command::SetLineup {
            lineup: Some(lineup),
        },
    )
    .unwrap();
}

#[test]
fn every_club_has_a_pecking_order_and_a_captain() {
    let game = Game::new(generate(&GenConfig::preset(Size::Tiny, 71)));
    let w = game.world();
    for (c, club) in w.clubs().iter() {
        assert_eq!(statuses(w, c, SquadStatus::Key).len(), 5, "{c}");
        assert_eq!(statuses(w, c, SquadStatus::Regular).len(), 6, "{c}");
        let captain = club.captain().expect("a captain");
        assert!(club.squad().contains(&captain));
    }
    assert_eq!(w.check_invariants(), Ok(()));
}

#[test]
fn a_benched_star_wants_football_and_a_broken_promise_lingers() {
    let (mut game, m, club) = one_human(72, 4);
    let star = statuses(game.world(), club, SquadStatus::Key)[0];
    advance_without(&mut game, m, club, &[star], date(2026, 10, 1));
    let w = game.world();
    assert!(w.players().playing_time(star) < 2_000);
    let a = happiness::assess(w, star).unwrap();
    let pt = a
        .factors
        .iter()
        .find(|f| f.reason == Reason::PlayingTime)
        .expect("a playing-time reason");
    assert!(pt.effect <= -20, "{a:?}");

    // Promise football, then keep him out.
    let events = game
        .submit(
            Actor::Manager(m),
            Command::TalkToPlayer {
                player: star,
                talk: Talk::PromiseFootball,
            },
        )
        .unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::PlayerReacted {
            reaction: Reaction::Pleased,
            ..
        }
    )));
    assert_eq!(
        game.submit(
            Actor::Manager(m),
            Command::TalkToPlayer {
                player: star,
                talk: Talk::Reassure,
            },
        ),
        Err(CommandError::TalkedRecently(star))
    );
    let events = advance_without(&mut game, m, club, &[star], date(2027, 1, 15));
    let settled = events.iter().find_map(|e| match e {
        Event::PromiseSettled { player, kept, .. } if *player == star => Some(*kept),
        _ => None,
    });
    let w = game.world();
    if w.players().club(star) != Some(club) {
        return; // sold after a transfer request; nothing left to check
    }
    assert_eq!(settled, Some(false));
    assert!(w.relations().remembers(star, Feeling::PromiseBroken));
    // Now he does not believe a word.
    let events = game
        .submit(
            Actor::Manager(m),
            Command::TalkToPlayer {
                player: star,
                talk: Talk::Reassure,
            },
        )
        .unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::PlayerReacted {
            reaction: Reaction::Sceptical,
            ..
        }
    )));
}

#[test]
fn criticising_good_form_angers() {
    let (mut game, m, club) = one_human(73, 2);
    advance_to(&mut game, date(2026, 10, 15));
    let w = game.world();
    let player = w.clubs()[club]
        .squad()
        .iter()
        .copied()
        .find(|&p| {
            let s = w.players().season_stats(p);
            s.appearances >= 3 && s.average_rating().is_some_and(|r| r >= 7.0)
        })
        .expect("someone in form");
    let before = happiness::assess(w, player).unwrap().happiness;
    let events = game
        .submit(
            Actor::Manager(m),
            Command::TalkToPlayer {
                player,
                talk: Talk::Criticise,
            },
        )
        .unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::PlayerReacted {
            reaction: Reaction::Angry,
            ..
        }
    )));
    let w = game.world();
    assert!(w.relations().remembers(player, Feeling::UnfairCriticism));
    assert!(happiness::assess(w, player).unwrap().happiness < before);
    assert_eq!(
        w.players().happiness(player),
        happiness::assess(w, player).unwrap().happiness
    );
}

#[test]
fn a_contract_promise_is_kept_by_a_new_contract() {
    let (mut game, m, club) = one_human(74, 6);
    // Take contracts back from the assistant.
    game.submit(Actor::Manager(m), Command::SetDelegation { squad: false })
        .unwrap();
    let w = game.world();
    let today = w.date();
    let player = w.clubs()[club]
        .squad()
        .iter()
        .copied()
        .find(|&p| {
            let c = w.players().contract(p).unwrap();
            c.expires.days_since(today) < 700
                && age_years(w.players().bio(p).birth_date, today) >= 20.0
        })
        .expect("a contract ending within two years");
    game.submit(
        Actor::Manager(m),
        Command::TalkToPlayer {
            player,
            talk: Talk::PromiseContract,
        },
    )
    .unwrap();
    assert_eq!(game.world().relations().promises().len(), 1);
    let w = game.world();
    let ps = w.players();
    let age = age_years(ps.bio(player).birth_date, today) as u32;
    let wage = wage_demand(ps.ability(player), ps.potential(player), age) * 11 / 10;
    game.submit(
        Actor::Manager(m),
        Command::OfferContract {
            player,
            wage,
            years: 4,
        },
    )
    .unwrap();
    let events = advance_to(&mut game, date(2026, 7, 3));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::PromiseSettled { kept: true, .. }))
    );
    assert!(
        game.world()
            .relations()
            .remembers(player, Feeling::PromiseKept)
    );
    assert!(game.world().relations().promises().is_empty());
}

#[test]
fn unsettled_players_ask_to_leave_and_can_be_sold() {
    let (mut game, m, club) = one_human(75, 3);
    let w = game.world();
    // Leave out three key players.
    let mut out = statuses(w, club, SquadStatus::Key);
    out.truncate(3);
    let not_asked = statuses(game.world(), club, SquadStatus::Backup)[0];
    assert_eq!(
        game.submit(
            Actor::Manager(m),
            Command::AnswerTransferRequest {
                player: not_asked,
                accept: true,
            },
        ),
        Err(CommandError::NoTransferRequest(not_asked))
    );
    let events = advance_without(&mut game, m, club, &out, date(2027, 3, 1));
    let asked: Vec<PlayerId> = events
        .iter()
        .filter_map(|e| match e {
            Event::TransferRequested { player, .. } => Some(*player),
            _ => None,
        })
        .collect();
    assert!(!asked.is_empty(), "nobody asked to leave");
    let p = *asked
        .iter()
        .find(|&&p| game.world().players().club(p) == Some(club))
        .expect("still here");
    game.submit(
        Actor::Manager(m),
        Command::AnswerTransferRequest {
            player: p,
            accept: true,
        },
    )
    .unwrap();
    let w = game.world();
    assert_eq!(w.market().listing(p), Some(Listing::Transfer));
    assert!(w.relations().request(p).unwrap().accepted);
    assert_eq!(w.check_invariants(), Ok(()));
}

#[test]
fn a_new_captain_upsets_the_old_one() {
    let (mut game, m, club) = one_human(76, 1);
    let w = game.world();
    let old = w.clubs()[club].captain().unwrap();
    let new = w.clubs()[club]
        .squad()
        .iter()
        .copied()
        .find(|&p| p != old)
        .unwrap();
    game.submit(Actor::Manager(m), Command::SetCaptain { player: new })
        .unwrap();
    let w = game.world();
    assert_eq!(w.clubs()[club].captain(), Some(new));
    assert!(w.relations().remembers(old, Feeling::LostCaptaincy));
    let a = happiness::assess(w, new).unwrap();
    assert!(a.factors.iter().any(|f| f.reason == Reason::Captain));
}

#[test]
fn ai_clubs_grant_requests_and_nothing_dangles() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 77)));
    advance_to(&mut game, date(2027, 8, 1));
    let w = game.world();
    assert!(w.relations().requests_made() > 0);
    for (&p, r) in w.relations().requests() {
        assert!(
            r.accepted || w.is_human_club(r.club),
            "AI clubs answer at once"
        );
        assert_eq!(w.players().club(p), Some(r.club));
    }
    assert_eq!(w.check_invariants(), Ok(()));
}
