//! Cups across seasons: draws, knockouts, groups, continental
//! qualification and coefficients.

use fs_core::GameDate;
use fs_gen::{GenConfig, Size, generate};
use fs_world::cup::{CupScope, StageDraw};
use fs_world::format::{Source, StageKind};
use fs_world::{Actor, ClubId, Command, Event, FixtureRef, Game, World};
use std::collections::BTreeSet;

fn date(y: i32, m: u32, d: u32) -> GameDate {
    GameDate::from_ymd(y, m, d).unwrap()
}

/// Advances to `until`, returning the events.
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

/// Eight nations of two divisions: one continent, both continental cups
/// with small group stages.
fn continental_world(seed: u64) -> World {
    generate(&GenConfig {
        nations: 8,
        clubs_per_nation: 40,
        free_agents: 400,
        human_clubs: Vec::new(),
        ..GenConfig::preset(Size::Tiny, seed)
    })
}

#[test]
fn a_national_cup_runs_from_august_to_may() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 31)));
    let (cup_id, cup) = game.world().cups().iter().next().unwrap();
    assert!(matches!(cup.scope, CupScope::Nation(_)));
    let e = cup.edition().unwrap();
    assert_eq!(e.entrants[0].len(), 20);
    // Round one brings 20 clubs down to 16: the twelve best seeds get byes.
    match &e.stages[0] {
        StageDraw::Knockout { ties, byes, .. } => {
            assert_eq!(ties.len(), 4);
            assert_eq!(byes.len(), 12);
        }
        StageDraw::Groups { .. } => panic!("a knockout cup"),
    }
    let events = advance_to(&mut game, date(2027, 6, 30));
    let w = game.world();
    assert_eq!(w.check_invariants(), Ok(()));
    let e = w.cups()[cup_id].edition().unwrap();
    assert!(e.finished);
    let winner = e.winner.expect("a winner");
    assert!(e.fixtures.iter().all(|f| f.fixture.result.is_some()));
    // 4 + 8 + 4 + 2 + 1 matches.
    assert_eq!(e.fixtures.len(), 19);
    let last = e.fixtures.last().unwrap();
    assert!(last.fixture.home == winner || last.fixture.away == winner);
    assert_eq!(last.fixture.date.ymd().1, 5, "the final is in May");
    assert!(
        events
            .iter()
            .any(|ev| matches!(ev, Event::CupWon { winner: x, .. } if *x == winner))
    );
    // Every tie has a winner, through extra time and penalties if need be.
    for s in &e.stages {
        if let StageDraw::Knockout { ties, .. } = s {
            assert!(ties.iter().all(|t| t.winner.is_some()));
        }
    }
    // The next season starts a new edition and remembers this one.
    advance_to(&mut game, date(2027, 7, 2));
    let cup = &game.world().cups()[cup_id];
    assert_eq!(cup.history().len(), 1);
    assert_eq!(cup.history()[0].winner, Some(winner));
    assert_eq!(cup.edition().unwrap().year, 2027);
}

#[test]
fn cup_matches_decide_ties_and_never_clash() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 32)));
    advance_to(&mut game, date(2027, 6, 1));
    let w = game.world();
    let e = w.cups().iter().next().unwrap().1.edition().unwrap();
    let mut extra = 0;
    for f in &e.fixtures {
        let r = f.fixture.result.as_ref().unwrap();
        // Knockout matches never end level without penalties.
        if r.goals[0] == r.goals[1] {
            assert!(r.penalties.is_some(), "{f:?}");
            assert!(r.extra_time);
        }
        extra += usize::from(r.extra_time);
    }
    assert!(extra < e.fixtures.len());
    // No club has two matches on one day (also an invariant).
    assert_eq!(w.check_invariants(), Ok(()));
}

#[test]
fn continental_cups_take_the_best_clubs_once() {
    let world = continental_world(33);
    assert_eq!(world.continents().len(), 1);
    let continental: Vec<_> = world
        .cups()
        .iter()
        .filter(|(_, c)| c.is_continental())
        .collect();
    assert_eq!(continental.len(), 2);
    let mut seen: BTreeSet<ClubId> = BTreeSet::new();
    for (_, c) in &continental {
        let e = c.edition().unwrap();
        for club in e.entrants.iter().flatten() {
            assert!(seen.insert(*club), "{club} entered twice");
        }
    }
    // The best nation's four strongest clubs go straight to the groups.
    let ranking = world.continents()[0].ranking();
    let (_, cc) = continental[0];
    let groups_stage = cc
        .format
        .stages
        .iter()
        .position(|s| matches!(s.kind, StageKind::Groups { .. }))
        .unwrap();
    let e = cc.edition().unwrap();
    let best: Vec<ClubId> = e.entrants[groups_stage]
        .iter()
        .copied()
        .filter(|&c| world.clubs()[c].nation == ranking[0])
        .collect();
    assert_eq!(best.len(), 4);
    // Qualifying fills the group stage exactly.
    let direct = e.entrants[groups_stage].len();
    let StageKind::Groups { groups, size, .. } = cc.format.stages[groups_stage].kind else {
        unreachable!()
    };
    assert_eq!(direct, usize::from(groups) * usize::from(size) * 3 / 4);
    assert!(
        cc.format
            .entries
            .iter()
            .all(|en| !matches!(en.source, Source::Levels { .. }))
    );
}

#[test]
fn continental_seasons_finish_and_move_coefficients() {
    let mut game = Game::new(continental_world(34));
    let before: Vec<u32> = {
        let c = &game.world().continents()[0];
        c.nations.iter().map(|&n| c.coefficient(n)).collect()
    };
    advance_to(&mut game, date(2027, 6, 30));
    let w = game.world();
    for (_, cup) in w.cups().iter() {
        let e = cup.edition().unwrap();
        assert!(e.finished, "{} unfinished", cup.format.name);
        assert!(e.winner.is_some(), "{} has no winner", cup.format.name);
        assert!(e.fixtures.iter().all(|f| f.fixture.result.is_some()));
        if cup.is_continental() {
            // Group stages played home and away, then knockouts.
            assert!(
                e.stages
                    .iter()
                    .any(|s| matches!(s, StageDraw::Groups { groups, .. } if groups.iter().all(|g| g.rows.iter().all(|r| r.played == 6))))
            );
            // The final was at a neutral ground.
            let last = e.fixtures.len() - 1;
            let r = FixtureRef::Cup {
                cup: w
                    .cups()
                    .ids()
                    .find(|&id| std::ptr::eq(&w.cups()[id], cup))
                    .unwrap(),
                index: last as u32,
            };
            assert!(fs_world::matchday::fixture_input(w, r).neutral);
        }
    }
    advance_to(&mut game, date(2027, 7, 2));
    let w = game.world();
    let c = &w.continents()[0];
    let after: Vec<u32> = c.nations.iter().map(|&n| c.coefficient(n)).collect();
    assert_ne!(before, after);
    assert_eq!(c.seasons().len(), fs_world::cup::COEFFICIENT_SEASONS);
    // Last season's champions of the top nation play in the main cup.
    let top = c.ranking()[0];
    let league = w
        .leagues()
        .iter()
        .find(|(_, l)| l.nation == top && l.level == 1)
        .unwrap()
        .1;
    let champion = league.history().last().unwrap().champion().unwrap();
    let (_, cc) = w.cups().iter().find(|(_, c)| c.is_continental()).unwrap();
    assert!(cc.edition().unwrap().involves(champion));
    assert_eq!(w.check_invariants(), Ok(()));
}

#[test]
fn human_managers_hear_about_their_draws() {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, 35)));
    let events = advance_to(&mut game, date(2027, 6, 30));
    let human = game.world().human_managers().next().unwrap();
    let club = game.world().managers()[human].club;
    let Some(club) = club else { return };
    let drawn = events
        .iter()
        .filter(|e| matches!(e, Event::CupDrawn { club: c, .. } if *c == club))
        .count();
    assert!(drawn >= 1);
    let played = events
        .iter()
        .filter(|e| matches!(e, Event::MatchPlayed { fixture: FixtureRef::Cup { .. }, home, away, .. } if *home == club || *away == club))
        .count();
    assert!(played >= 1);
}

#[test]
fn a_written_format_builds_the_same_world() {
    let cfg = GenConfig {
        nations: 6,
        clubs_per_nation: 40,
        free_agents: 100,
        ..GenConfig::preset(Size::Tiny, 36)
    };
    let format = fs_gen::format_for(&cfg);
    let json = serde_json::to_string(&format).unwrap();
    let back: fs_world::format::WorldFormat = serde_json::from_str(&json).unwrap();
    assert_eq!(back, format);
    let from_data = generate(&GenConfig {
        format: Some(back),
        ..cfg.clone()
    });
    assert_eq!(from_data.state_hash(), generate(&cfg).state_hash());
}

#[test]
fn custom_formats_change_the_structure() {
    let cfg = GenConfig::preset(Size::Tiny, 37);
    let mut format = fs_gen::format_for(&cfg);
    // Split the one division in two, and play the cup's semi-finals over
    // two legs.
    format.nations[0].leagues[0].clubs = 12;
    format.nations[0]
        .leagues
        .push(fs_world::format::LeagueFormat {
            name: "Second Division".into(),
            clubs: 8,
            swaps: 2,
        });
    let cup = &mut format.nations[0].cups[0];
    let semis = cup.stages.len() - 2;
    if let StageKind::Knockout { legs, .. } = &mut cup.stages[semis].kind {
        *legs = 2;
    }
    let first = cup.stages[semis].dates[0];
    cup.stages[semis].dates = vec![first, fs_world::format::DateRule::new(4, 28, first.weekday)];
    assert_eq!(format.validate(), Ok(()));
    let mut game = Game::new(generate(&GenConfig {
        format: Some(format),
        ..cfg
    }));
    assert_eq!(game.world().leagues().len(), 2);
    advance_to(&mut game, date(2027, 6, 30));
    let w = game.world();
    let e = w.cups().iter().next().unwrap().1.edition().unwrap();
    assert!(e.winner.is_some());
    match &e.stages[semis] {
        StageDraw::Knockout { ties, .. } => assert!(ties.iter().all(|t| t.legs.len() == 2)),
        StageDraw::Groups { .. } => panic!("semi-finals are a knockout"),
    }
    assert_eq!(w.check_invariants(), Ok(()));
}
