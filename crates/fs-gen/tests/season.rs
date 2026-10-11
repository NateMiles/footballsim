//! The season loop: fixtures, tables, promotion and relegation, injuries,
//! suspensions, contracts, youth, retirement and long-run world health.

use fs_core::GameDate;
use fs_gen::{GenConfig, Size, generate};
use fs_world::matchday::team_sheet;
use fs_world::people::{age_years, wage_demand};
use fs_world::{
    Actor, ClubId, Command, CommandError, Event, Game, LeagueId, ManagerId, Rules, Status, World,
};

fn date(y: i32, m: u32, d: u32) -> GameDate {
    GameDate::from_ymd(y, m, d).unwrap()
}

/// Advances the game to `target`, returning every event.
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

/// A preset whose human manager cannot be sacked, for tests that follow
/// one manager for a long time.
fn unsackable(size: Size, seed: u64) -> GenConfig {
    GenConfig {
        rules: Rules { sack_humans: false },
        ..GenConfig::preset(size, seed)
    }
}

fn human(world: &World) -> (ManagerId, ClubId) {
    let m = world.human_managers().next().unwrap();
    (m, world.managers()[m].club.unwrap())
}

#[test]
fn a_full_season_is_played_and_archived() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 11)));
    let league = LeagueId::new(0);
    let fixtures = game.world().leagues()[league]
        .season()
        .unwrap()
        .fixtures
        .len();
    assert_eq!(fixtures, 380);

    // Mid-season the table agrees with the fixtures played so far.
    advance_to(&mut game, date(2027, 1, 15));
    let season = game.world().leagues()[league].season().unwrap();
    let played: u16 = season.table().iter().map(|r| r.played).sum();
    let done = season
        .fixtures
        .iter()
        .filter(|f| f.result.is_some())
        .count();
    assert_eq!(usize::from(played), 2 * done);
    assert!(done > 100 && done < 380, "{done}");

    let events = advance_to(&mut game, date(2027, 7, 2));
    let w = game.world();
    let history = w.leagues()[league].history();
    assert_eq!(history.len(), 1);
    let table = &history[0].table;
    assert!(table.iter().all(|r| r.played == 38));
    let wins: u16 = table.iter().map(|r| r.won).sum();
    let losses: u16 = table.iter().map(|r| r.lost).sum();
    assert_eq!(wins, losses);
    let scored: u16 = table.iter().map(|r| r.scored).sum();
    let conceded: u16 = table.iter().map(|r| r.conceded).sum();
    assert_eq!(scored, conceded);
    assert!(history[0].top_scorer.is_some());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::SeasonEnded { .. }))
    );
    // A new season has started, with fresh fixtures.
    let next = w.leagues()[league].season().unwrap();
    assert_eq!(next.year, 2027);
    assert!(next.fixtures.iter().all(|f| f.result.is_none()));
    assert_eq!(w.check_invariants(), Ok(()));
}

#[test]
fn promotion_and_relegation_swap_three_clubs() {
    let cfg = GenConfig {
        nations: 1,
        clubs_per_nation: 40,
        free_agents: 100,
        ..GenConfig::preset(Size::Tiny, 12)
    };
    let mut game = Game::new(generate(&cfg));
    assert_eq!(game.world().leagues().len(), 2);
    advance_to(&mut game, date(2027, 7, 2));
    let w = game.world();
    let (top, second) = (LeagueId::new(0), LeagueId::new(1));
    let relegated = &w.leagues()[top].history()[0].relegated;
    let promoted = &w.leagues()[second].history()[0].promoted;
    assert_eq!(relegated.len(), 3);
    assert_eq!(promoted.len(), 3);
    for &c in relegated {
        assert_eq!(w.clubs()[c].league(), Some(second));
    }
    for &c in promoted {
        assert_eq!(w.clubs()[c].league(), Some(top));
    }
    // The relegated clubs were the bottom three.
    let table = &w.leagues()[top].history()[0].table;
    let bottom: Vec<ClubId> = table.iter().rev().take(3).map(|r| r.club).collect();
    assert_eq!(relegated, &bottom);
    assert_eq!(w.leagues()[top].clubs().len(), 20);
    assert_eq!(w.check_invariants(), Ok(()));
}

#[test]
fn unavailable_players_are_left_out() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 13)));
    advance_to(&mut game, date(2027, 2, 1));
    let w = game.world();
    let today = w.date();
    let mut checked = 0;
    for (club, c) in w.clubs().iter() {
        let out: Vec<_> = c
            .squad()
            .iter()
            .copied()
            .filter(|&p| !w.players().available(p, today))
            .collect();
        if out.is_empty() {
            continue;
        }
        let sheet = team_sheet(w, club);
        for p in &out {
            assert!(
                !sheet
                    .starters
                    .iter()
                    .chain(&sheet.bench)
                    .any(|s| s.id == *p),
                "{p} picked while unavailable"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "expected some injured or suspended players by February"
    );
}

#[test]
fn human_contracts_must_be_renewed_or_the_player_leaves() {
    let mut game = Game::new(generate(&unsackable(Size::Tiny, 14)));
    let (manager, club) = human(game.world());
    let ending = date(2027, 6, 30);
    let expiring: Vec<_> = game.world().clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&p| {
            game.world()
                .players()
                .contract(p)
                .is_some_and(|c| c.expires == ending)
        })
        .collect();
    assert!(expiring.len() >= 2, "need two expiring contracts");
    let (keep, lose) = (expiring[0], expiring[1]);

    let events = advance_to(&mut game, date(2027, 6, 2));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ContractExpiring { player, .. } if *player == keep))
    );
    let w = game.world();
    let age = age_years(w.players().bio(keep).birth_date, w.date()) as u32;
    let demand = wage_demand(w.players().ability(keep), w.players().potential(keep), age);
    let lowball = game.submit(
        Actor::Manager(manager),
        Command::OfferContract {
            player: keep,
            wage: demand / 2,
            years: 2,
        },
    );
    assert_eq!(lowball, Err(CommandError::ContractRejected { demand }));
    game.submit(
        Actor::Manager(manager),
        Command::OfferContract {
            player: keep,
            wage: demand,
            years: 2,
        },
    )
    .unwrap();

    let events = advance_to(&mut game, date(2027, 7, 2));
    let w = game.world();
    assert_eq!(w.players().club(keep), Some(club));
    assert_eq!(
        w.players().contract(keep).unwrap().expires,
        date(2029, 6, 30)
    );
    assert_ne!(w.players().club(lose), Some(club));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::PlayerLeft { player, .. } if *player == lose))
            || w.players().status(lose) == Status::Retired
    );
}

#[test]
fn humans_can_sign_free_agents_and_set_tactics() {
    let mut game = Game::new(generate(&unsackable(Size::Tiny, 15)));
    let (manager, club) = human(game.world());
    let w = game.world();
    let free = w
        .players()
        .ids()
        .find(|&p| w.players().club(p).is_none() && w.players().status(p) == Status::Active)
        .unwrap();
    let age = age_years(w.players().bio(free).birth_date, w.date()) as u32;
    let demand = wage_demand(w.players().ability(free), w.players().potential(free), age);
    let events = game
        .submit(
            Actor::Manager(manager),
            Command::OfferContract {
                player: free,
                wage: demand,
                years: 3,
            },
        )
        .unwrap();
    assert!(events.contains(&Event::PlayerSigned { player: free, club }));
    assert_eq!(game.world().players().club(free), Some(club));
    assert!(game.world().players().squad_number(free).is_some());

    let mut tactic = fs_match::Tactic::balanced(fs_match::Formation::F352);
    tactic.pressing = 2;
    game.submit(
        Actor::Manager(manager),
        Command::SetTactic {
            tactic: Some(tactic),
        },
    )
    .unwrap();
    assert_eq!(team_sheet(game.world(), club).tactic, tactic);
    game.submit(
        Actor::Manager(manager),
        Command::ReleasePlayer { player: free },
    )
    .unwrap();
    assert_eq!(game.world().players().club(free), None);
    assert_eq!(game.world().check_invariants(), Ok(()));
}

#[test]
fn ten_seasons_keep_the_world_healthy() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 16)));
    let (manager, _) = human(game.world());
    game.submit(
        Actor::Manager(manager),
        Command::SetDelegation { squad: true },
    )
    .unwrap();
    let best_xi = |w: &World| {
        let mut all = Vec::new();
        for &c in w.leagues()[LeagueId::new(0)].clubs() {
            let mut a: Vec<f64> = w.clubs()[c]
                .squad()
                .iter()
                .map(|&p| f64::from(w.players().ability(p)))
                .collect();
            a.sort_by(|x, y| y.total_cmp(x));
            all.extend(a.into_iter().take(11));
        }
        all.iter().sum::<f64>() / all.len() as f64
    };
    let start = best_xi(game.world());
    for season in 0..10 {
        advance_to(&mut game, date(2027 + season, 7, 2));
        let w = game.world();
        assert_eq!(w.check_invariants(), Ok(()), "season {season}");
        let today = w.date();
        let mut ages = Vec::new();
        for (_, c) in w.clubs().iter() {
            let first_team: Vec<_> = c
                .squad()
                .iter()
                .copied()
                .filter(|&p| age_years(w.players().bio(p).birth_date, today) >= 18.0)
                .collect();
            assert!(
                (18..=34).contains(&first_team.len()),
                "first team of {}",
                first_team.len()
            );
            assert!(
                first_team
                    .iter()
                    .any(|&p| w.players().positions(p).best() == fs_world::Position::Goalkeeper),
                "a club has no senior goalkeeper"
            );
            ages.extend(
                first_team
                    .iter()
                    .map(|&p| age_years(w.players().bio(p).birth_date, today)),
            );
        }
        let mean_age = ages.iter().sum::<f64>() / ages.len() as f64;
        assert!(
            (24.5..=28.0).contains(&mean_age),
            "season {season}: mean first-team age {mean_age:.1}"
        );
        let now = best_xi(w);
        assert!(
            (now / start - 1.0).abs() < 0.05,
            "season {season}: top-division ability {now:.0} vs {start:.0}"
        );
        // Money: wages a sensible share of income, few clubs in debt,
        // and nobody hoarding or sinking without limit.
        let e = fs_world::economy::health(w);
        assert!(
            (0.35..0.8).contains(&e.wage_ratio[0]),
            "season {season}: wages/income {:.2}",
            e.wage_ratio[0]
        );
        assert!(
            e.in_debt < 0.2,
            "season {season}: {:.0}% in debt",
            e.in_debt * 100.0
        );
        assert!(
            (-1.0..4.0).contains(&e.balance_ratio[0]),
            "season {season}: balance/potential {:.2}",
            e.balance_ratio[0]
        );
        // Every club has a manager, or will within the week.
        for (id, c) in w.clubs().iter() {
            assert!(
                c.manager.is_some() || c.board().vacant_since.is_some(),
                "season {season}: {id} has no manager and is not looking"
            );
        }
    }
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn a_season_with_rollover_is_independent_of_thread_count() {
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 17)));
                advance_to(&mut game, date(2027, 8, 1));
                game.world().state_hash()
            })
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn snapshot_survives_a_season() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 18)));
    advance_to(&mut game, date(2027, 7, 15));
    let bytes = game.world().to_snapshot();
    let loaded = World::from_snapshot(&bytes).unwrap();
    assert_eq!(loaded.state_hash(), game.world().state_hash());
    let mut a = Game::new(game.world().clone());
    let mut b = Game::new(loaded);
    advance_to(&mut a, date(2027, 10, 1));
    advance_to(&mut b, date(2027, 10, 1));
    assert_eq!(a.world().state_hash(), b.world().state_hash());
}
