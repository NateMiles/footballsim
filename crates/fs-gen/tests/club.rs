//! Club management systems: money and staff.

use fs_core::GameDate;
use fs_gen::{GenConfig, Size, generate};
use fs_world::staff::{self, StaffRole};
use fs_world::{Actor, ClubId, Command, CommandError, Event, Game, ManagerId, World};

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

#[test]
fn every_club_starts_with_staff_money_and_facilities() {
    let w = generate(&GenConfig::preset(Size::Tiny, 31));
    for (id, c) in w.clubs().iter() {
        for role in StaffRole::ALL {
            let n = c
                .staff()
                .iter()
                .filter(|&&s| w.staff()[s].role == role)
                .count();
            assert!(n >= role.wanted().0, "{id} has {n} {role:?}");
        }
        assert!(c.finances().balance > 0);
        assert!(c.finances().wage_budget > 0);
        assert!(c.facilities.capacity >= 1_000);
    }
    let free = w
        .staff()
        .ids()
        .filter(|&s| staff::available(&w, s, w.date()))
        .count();
    assert!(free >= 20);
    // Big clubs have better staff.
    let ratings = |c: u32| staff::ratings(&w, ClubId::new(c)).coaching;
    assert!(ratings(0) > 8.0);
}

#[test]
fn money_moves_through_a_season() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 32)));
    let start: i64 = game
        .world()
        .clubs()
        .iter()
        .map(|(_, c)| c.finances().balance)
        .sum();
    advance_to(&mut game, date(2027, 7, 2));
    let w = game.world();
    for (id, c) in w.clubs().iter() {
        let f = c.finances();
        let last = f
            .history
            .last()
            .unwrap_or_else(|| panic!("{id} has no accounts"));
        assert!(last.ledger.matchday > 0, "{id} took no gate money");
        assert!(last.ledger.broadcast > 0 && last.ledger.commercial > 0);
        assert!(last.ledger.prize_money > 0);
        assert!(last.ledger.player_wages > 0 && last.ledger.staff_wages > 0);
        assert!(f.transfer_budget >= 0);
    }
    let end: i64 = w.clubs().iter().map(|(_, c)| c.finances().balance).sum();
    assert_ne!(start, end);
    let health = fs_world::economy::health(w);
    assert!(
        (0.35..0.8).contains(&health.wage_ratio[0]),
        "median wage ratio {}",
        health.wage_ratio[0]
    );
}

#[test]
fn humans_hire_and_release_staff() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 33)));
    let (m, club) = human(game.world());
    let w = game.world();
    let free: Vec<_> = w
        .staff()
        .ids()
        .filter(|&s| staff::available(w, s, w.date()) && w.staff()[s].role == StaffRole::Scout)
        .filter(|&s| w.staff()[s].would_join(w.clubs()[club].reputation))
        .collect();
    let scout = free[0];
    let demand = w.staff()[scout].wage_demand();

    assert_eq!(
        game.submit(
            Actor::Manager(m),
            Command::HireStaff {
                staff: scout,
                wage: demand / 2,
                years: 2
            }
        ),
        Err(CommandError::StaffWageTooLow { demand })
    );
    game.submit(
        Actor::Manager(m),
        Command::HireStaff {
            staff: scout,
            wage: demand,
            years: 2,
        },
    )
    .unwrap();
    assert!(game.world().clubs()[club].staff().contains(&scout));
    game.world().check_invariants().unwrap();

    let balance = game.world().clubs()[club].finances().balance;
    game.submit(Actor::Manager(m), Command::ReleaseStaff { staff: scout })
        .unwrap();
    assert!(!game.world().clubs()[club].staff().contains(&scout));
    assert!(
        game.world().clubs()[club].finances().balance < balance,
        "severance paid"
    );
    game.world().check_invariants().unwrap();
}

#[test]
fn ai_clubs_keep_their_departments_staffed() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 34)));
    advance_to(&mut game, date(2029, 7, 2));
    let w = game.world();
    w.check_invariants().unwrap();
    for (id, c) in w.clubs().iter().filter(|(id, _)| w.ai_runs_squad(*id)) {
        for role in StaffRole::ALL {
            let n = c
                .staff()
                .iter()
                .filter(|&&s| w.staff()[s].role == role)
                .count();
            assert!(n >= role.wanted().0, "{id} has {n} {role:?}");
        }
    }
}

#[test]
fn scouting_narrows_estimates_and_reports() {
    use fs_world::scouting::{self, FULL};
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 35)));
    let (m, club) = human(game.world());
    let w = game.world();
    let free_agent = w
        .players()
        .ids()
        .find(|&p| w.players().club(p).is_none())
        .unwrap();
    let rival = w.clubs()[ClubId::new(if club == ClubId::new(1) { 2 } else { 1 })].squad()[0];
    let own = w.clubs()[club].squad()[0];
    assert_eq!(scouting::knowledge(w, club, own), FULL);
    assert!(scouting::knowledge(w, club, rival) > scouting::knowledge(w, club, free_agent));
    let before = scouting::estimate(w, club, free_agent);
    let width = |e: &scouting::Estimate| e.ability.high - e.ability.low;
    assert!(width(&before) > 50, "{before:?}");
    // The estimate is stable from day to day.
    assert_eq!(before, scouting::estimate(w, club, free_agent));

    assert_eq!(
        game.submit(Actor::Manager(m), Command::ScoutPlayer { player: own }),
        Err(CommandError::OwnPlayer(own))
    );
    game.submit(
        Actor::Manager(m),
        Command::ScoutPlayer { player: free_agent },
    )
    .unwrap();
    let mut reported = false;
    for _ in 0..60 {
        let events = game
            .submit(Actor::System, Command::Advance { days: 1 })
            .unwrap();
        if events
            .iter()
            .any(|e| matches!(e, Event::ScoutReport { player, .. } if *player == free_agent))
        {
            reported = true;
            break;
        }
    }
    assert!(reported, "no scout report within 60 days");
    let w = game.world();
    let after = scouting::estimate(w, club, free_agent);
    assert!(after.knowledge >= scouting::REPORT_AT);
    assert!(width(&after) < width(&before) / 4, "{after:?}");
    let truth = w.players().ability(free_agent);
    assert!(after.ability.low <= truth && truth <= after.ability.high);
    assert!(w.scouting().assignments(club).is_empty());
}
