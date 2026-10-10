//! Human managers in the world: watched match days, lineups and
//! appointments.

use fs_gen::{GenConfig, Size, generate};
use fs_match::view::LiveView;
use fs_match::{Instruction, Shout, Side};
use fs_world::matchday::{fixture_input, team_sheet};
use fs_world::{
    Actor, ClubId, Command, CommandError, Event, Game, Lineup, ManagerId, WatchedFixture, replay,
};

/// A tiny world whose human manager watches matches.
fn watching_game(seed: u64) -> (Game, ManagerId, ClubId) {
    let mut game = Game::new(generate(&GenConfig::preset(Size::Tiny, seed)));
    let m = game.world().human_managers().next().unwrap();
    let club = game.world().managers()[m].club.unwrap();
    game.submit(Actor::Manager(m), Command::SetWatchMatches { watch: true })
        .unwrap();
    (game, m, club)
}

/// Advances until the next watched match day and returns its fixture.
fn advance_to_match_day(game: &mut Game) -> WatchedFixture {
    for _ in 0..10 {
        let events = game
            .submit(Actor::System, Command::Advance { days: 60 })
            .unwrap();
        if let Some(p) = game.world().pending() {
            assert!(matches!(events.last(), Some(Event::MatchDayReady { .. })));
            return p.awaiting[0];
        }
    }
    panic!("no match day in 600 days");
}

/// Watches a match to the end, making a substitution at 60 minutes and a
/// shout at 70. Returns the view's instructions and final score.
fn watch(input: &fs_match::MatchInput, side: Side) -> (Vec<fs_match::TimedInstruction>, [u8; 2]) {
    let mut view = LiveView::new(input);
    view.play_until(60.0 * 60.0);
    let lineup = view.lineup(side);
    let off = lineup.on_pitch[10].unwrap().id;
    let on = lineup.bench[0];
    view.instruct(side, Instruction::Substitute { off, on })
        .unwrap();
    view.play_until(70.0 * 60.0);
    view.instruct(side, Instruction::Shout(Shout::PushForward))
        .unwrap();
    let mut score = [0, 0];
    while !view.engine().is_finished() {
        score = view.play_until(view.engine().elapsed() + 600.0).score;
    }
    (view.instructions().to_vec(), score)
}

#[test]
fn a_watched_match_day_waits_for_the_manager() {
    let (mut game, m, club) = watching_game(21);
    let watched = advance_to_match_day(&mut game);
    assert_eq!(watched.manager, m);
    assert_eq!(watched.club, club);
    let w = game.world();
    let fixture = &w.leagues()[watched.league].season().unwrap().fixtures[watched.index];
    assert_eq!(fixture.date, w.date());
    assert!(fixture.result.is_none(), "the match waits");

    // Nobody else's matches were played either.
    let today = w.date();
    let played_today = w.leagues()[watched.league]
        .season()
        .unwrap()
        .fixtures
        .iter()
        .filter(|f| f.date == today && f.result.is_some())
        .count();
    assert_eq!(played_today, 0);

    // Only the watching manager can report the match; the setting cannot
    // change mid-day.
    assert_eq!(
        game.submit(
            Actor::System,
            Command::PlayMatch {
                instructions: vec![]
            }
        ),
        Err(CommandError::NotPermitted)
    );
    assert_eq!(
        game.submit(Actor::Manager(m), Command::SetWatchMatches { watch: false }),
        Err(CommandError::MatchDayInProgress)
    );
    // The manager may still change their team before kickoff.
    game.submit(Actor::Manager(m), Command::SetTactic { tactic: None })
        .unwrap();
}

#[test]
fn the_world_plays_the_match_the_manager_watched() {
    let (mut game, m, club) = watching_game(22);
    let watched = advance_to_match_day(&mut game);
    let input = fixture_input(game.world(), watched.league, watched.index);
    let side = if input
        .home
        .starters
        .iter()
        .any(|p| game.world().players().club(p.id) == Some(club))
    {
        Side::Home
    } else {
        Side::Away
    };
    assert!(!if side == Side::Home {
        input.home.auto_subs
    } else {
        input.away.auto_subs
    });
    let (instructions, score) = watch(&input, side);
    assert_eq!(instructions.len(), 2);

    // Instructions for the other side are refused.
    let mut wrong = instructions.clone();
    wrong[0].side = side.other();
    assert_eq!(
        game.submit(
            Actor::Manager(m),
            Command::PlayMatch {
                instructions: wrong
            }
        ),
        Err(CommandError::WrongSide)
    );

    let date = game.world().date();
    let events = game
        .submit(Actor::Manager(m), Command::PlayMatch { instructions })
        .unwrap();
    assert!(game.world().pending().is_none());
    assert_eq!(
        game.world().date(),
        date,
        "the day finishes without advancing"
    );
    let fixture = &game.world().leagues()[watched.league]
        .season()
        .unwrap()
        .fixtures[watched.index];
    assert_eq!(fixture.result.as_ref().unwrap().goals, score);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::MatchPlayed { goals, .. } if *goals == score))
    );

    // The log replays to the same world.
    let initial = generate(&GenConfig::preset(Size::Tiny, 22));
    let replayed = replay(initial, game.log()).unwrap();
    assert_eq!(replayed.world().state_hash(), game.world().state_hash());
}

#[test]
fn advancing_past_a_watched_match_lets_the_assistant_manage_it() {
    let (mut game, _, _) = watching_game(23);
    let watched = advance_to_match_day(&mut game);
    let date = game.world().date();
    // The assistant makes the substitutions in a match nobody watched.
    let mut input = fixture_input(game.world(), watched.league, watched.index);
    input.home.auto_subs = true;
    input.away.auto_subs = true;
    let expected = fs_match::play(&input, fs_match::Tier::Event);
    game.submit(Actor::System, Command::Advance { days: 1 })
        .unwrap();
    assert_eq!(game.world().date(), date.add_days(1));
    assert!(game.world().pending().is_none());
    let fixture = &game.world().leagues()[watched.league]
        .season()
        .unwrap()
        .fixtures[watched.index];
    assert_eq!(fixture.result.as_ref().unwrap().goals, expected.goals);
}

#[test]
fn a_skipped_match_is_managed_by_the_assistant() {
    let (mut game, m, _) = watching_game(23);
    let watched = advance_to_match_day(&mut game);
    let date = game.world().date();
    let mut input = fixture_input(game.world(), watched.league, watched.index);
    input.home.auto_subs = true;
    input.away.auto_subs = true;
    let expected = fs_match::play(&input, fs_match::Tier::Event);
    game.submit(Actor::Manager(m), Command::SkipMatch).unwrap();
    assert_eq!(game.world().date(), date);
    assert!(game.world().pending().is_none());
    let fixture = &game.world().leagues()[watched.league]
        .season()
        .unwrap()
        .fixtures[watched.index];
    assert_eq!(fixture.result.as_ref().unwrap().goals, expected.goals);
    assert_eq!(
        game.submit(Actor::Manager(m), Command::SkipMatch),
        Err(CommandError::NoMatchWaiting(m))
    );
}

#[test]
fn a_chosen_lineup_is_used_and_repaired() {
    let (mut game, m, club) = watching_game(24);
    let auto = team_sheet(game.world(), club);
    // Swap the first substitute into the last starting slot.
    let mut starters: Vec<_> = auto.starters.iter().map(|p| p.id).collect();
    let mut bench: Vec<_> = auto.bench.iter().map(|p| p.id).collect();
    std::mem::swap(&mut starters[10], &mut bench[0]);
    let lineup = Lineup {
        starters: starters.clone(),
        bench: bench.clone(),
    };
    game.submit(
        Actor::Manager(m),
        Command::SetLineup {
            lineup: Some(lineup.clone()),
        },
    )
    .unwrap();
    let sheet = team_sheet(game.world(), club);
    let ids: Vec<_> = sheet.starters.iter().map(|p| p.id).collect();
    assert_eq!(ids, starters);

    // A starter who leaves is replaced; the sheet stays complete.
    game.submit(
        Actor::Manager(m),
        Command::ReleasePlayer {
            player: starters[3],
        },
    )
    .unwrap();
    let sheet = team_sheet(game.world(), club);
    assert_eq!(sheet.starters.len(), 11);
    assert!(sheet.starters.iter().all(|p| p.id != starters[3]));
    assert_eq!(sheet.starters[0].id, starters[0]);

    // Lineups must be valid and the manager's own players.
    let mut twice = lineup.clone();
    twice.bench.push(twice.starters[0]);
    assert!(matches!(
        game.submit(
            Actor::Manager(m),
            Command::SetLineup {
                lineup: Some(twice)
            }
        ),
        Err(CommandError::InvalidLineup(_))
    ));
    let other = team_sheet(game.world(), ClubId::new(5));
    let mut stranger = lineup;
    stranger.bench = vec![other.starters[0].id];
    assert!(matches!(
        game.submit(
            Actor::Manager(m),
            Command::SetLineup {
                lineup: Some(stranger)
            }
        ),
        Err(CommandError::InvalidLineup(_))
    ));
    game.submit(Actor::Manager(m), Command::SetLineup { lineup: None })
        .unwrap();
}

#[test]
fn a_human_can_be_appointed_to_any_ai_club() {
    let mut cfg = GenConfig::preset(Size::Tiny, 25);
    cfg.human_clubs.clear();
    let mut game = Game::new(generate(&cfg));
    let club = ClubId::new(7);
    let old = game.world().clubs()[club].manager.unwrap();
    let appoint = Command::AppointHuman {
        club,
        first_name: "Alex".into(),
        last_name: "Morgan".into(),
    };
    let events = game.submit(Actor::System, appoint.clone()).unwrap();
    let Some(&Event::ManagerAppointed { manager, club: c }) = events.first() else {
        panic!("{events:?}")
    };
    assert_eq!(c, club);
    let w = game.world();
    assert_eq!(w.human_club(manager), Some(club));
    assert_eq!(w.managers()[old].club, None);
    assert!(w.watched_live(club));
    assert_eq!(w.names().get(w.managers()[manager].first_name), "Alex");
    w.check_invariants().unwrap();

    assert_eq!(
        game.submit(Actor::System, appoint),
        Err(CommandError::ClubTaken(club))
    );
    assert_eq!(
        game.submit(
            Actor::Manager(manager),
            Command::AppointHuman {
                club: ClubId::new(8),
                first_name: "A".into(),
                last_name: "B".into(),
            }
        ),
        Err(CommandError::NotPermitted)
    );
    assert_eq!(
        game.submit(
            Actor::System,
            Command::AppointHuman {
                club: ClubId::new(8),
                first_name: " ".into(),
                last_name: "B".into(),
            }
        ),
        Err(CommandError::InvalidName)
    );

    // The new manager's matches wait for them.
    let watched = advance_to_match_day(&mut game);
    assert_eq!(watched.manager, manager);
}
