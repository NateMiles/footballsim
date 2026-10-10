//! Integration tests over generated worlds: commands, replay, snapshots and
//! determinism.

use fs_gen::{GenConfig, Size, generate, scenario};
use fs_world::snapshot::{FORMAT_VERSION, MAGIC};
use fs_world::{
    Actor, ClubId, Command, CommandError, Control, Game, ReplayError, SnapshotError, World, replay,
};

fn tiny(seed: u64) -> World {
    generate(&GenConfig::preset(Size::Tiny, seed))
}

fn human_and_club(world: &World) -> (fs_world::ManagerId, ClubId) {
    let m = world.human_managers().next().unwrap();
    (m, world.managers()[m].club.unwrap())
}

#[test]
fn human_can_number_own_players_only() {
    let world = tiny(1);
    let (human, club) = human_and_club(&world);
    let own = world.clubs()[club].squad()[3];
    let other = world.clubs()[ClubId::new(5)].squad()[3];
    let mut game = Game::new(world);

    let ok = game.submit(
        Actor::Manager(human),
        Command::SetSquadNumber {
            player: own,
            number: Some(77),
        },
    );
    assert!(ok.is_ok());
    assert_eq!(game.world().players().squad_number(own), Some(77));

    let err = game
        .submit(
            Actor::Manager(human),
            Command::SetSquadNumber {
                player: other,
                number: Some(78),
            },
        )
        .unwrap_err();
    assert_eq!(err, CommandError::NotYourPlayer(human));
    assert_eq!(game.log().len(), 1, "rejected commands are not logged");
}

#[test]
fn squad_number_rules() {
    let world = tiny(2);
    let (human, club) = human_and_club(&world);
    let squad = world.clubs()[club].squad().to_vec();
    let holder_of_1 = squad[0];
    let mut game = Game::new(world);
    let set = |p, n| Command::SetSquadNumber {
        player: p,
        number: n,
    };

    assert_eq!(
        game.submit(Actor::Manager(human), set(squad[1], Some(1))),
        Err(CommandError::SquadNumberTaken {
            number: 1,
            holder: holder_of_1
        })
    );
    assert_eq!(
        game.submit(Actor::Manager(human), set(squad[1], Some(0))),
        Err(CommandError::InvalidSquadNumber(0))
    );
    assert_eq!(
        game.submit(Actor::Manager(human), set(squad[1], Some(100))),
        Err(CommandError::InvalidSquadNumber(100))
    );
    // Freeing a number lets someone else take it.
    game.submit(Actor::Manager(human), set(holder_of_1, None))
        .unwrap();
    game.submit(Actor::Manager(human), set(squad[1], Some(1)))
        .unwrap();
    assert_eq!(game.world().check_invariants(), Ok(()));
}

#[test]
fn only_system_or_humans_advance_time() {
    let world = tiny(3);
    let ai = world
        .managers()
        .iter()
        .find(|(_, m)| m.control == Control::Ai)
        .map(|(id, _)| id)
        .unwrap();
    let (human, _) = human_and_club(&world);
    let start = world.date();
    let mut game = Game::new(world);

    assert_eq!(
        game.submit(Actor::Manager(ai), Command::Advance { days: 1 }),
        Err(CommandError::NotPermitted)
    );
    assert_eq!(
        game.submit(Actor::System, Command::Advance { days: 0 }),
        Err(CommandError::InvalidDays(0))
    );
    let events = game
        .submit(Actor::Manager(human), Command::Advance { days: 3 })
        .unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(game.world().date(), start.add_days(3));
}

#[test]
fn replay_reproduces_the_game_exactly() {
    // The scenario starts from the same generated world as `tiny(4)`.
    let game = scenario::determinism(4, Size::Tiny, 60);
    let replayed = replay(tiny(4), game.log()).unwrap();
    assert_eq!(replayed.world().state_hash(), game.world().state_hash());
    assert_eq!(replayed.log(), game.log());
}

#[test]
fn replay_detects_tampering() {
    let game = scenario::determinism(5, Size::Tiny, 10);
    let mut log = game.log().to_vec();
    log.swap(0, 1);
    assert!(matches!(
        replay(tiny(5), &log),
        Err(ReplayError::BadSequence { .. })
    ));

    let mut log = game.log().to_vec();
    log[1].date = log[1].date.add_days(1);
    assert!(matches!(
        replay(tiny(5), &log),
        Err(ReplayError::Diverged { seq: 1, .. })
    ));
}

#[test]
fn snapshot_round_trip() {
    let game = scenario::determinism(6, Size::Tiny, 30);
    let bytes = game.world().to_snapshot();
    let loaded = World::from_snapshot(&bytes).unwrap();
    assert_eq!(&loaded, game.world());
    assert_eq!(loaded.state_hash(), game.world().state_hash());

    // Continuing from the loaded copy matches continuing from the original.
    let mut a = Game::new(game.world().clone());
    let mut b = Game::new(loaded);
    a.submit(Actor::System, Command::Advance { days: 20 })
        .unwrap();
    b.submit(Actor::System, Command::Advance { days: 20 })
        .unwrap();
    assert_eq!(a.world().state_hash(), b.world().state_hash());
}

#[test]
fn snapshot_rejects_bad_input() {
    let bytes = tiny(7).to_snapshot();
    assert_eq!(&bytes[..4], &MAGIC);

    assert_eq!(World::from_snapshot(b"nope"), Err(SnapshotError::BadMagic));

    let mut wrong_version = bytes.clone();
    wrong_version[4..6].copy_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
    assert_eq!(
        World::from_snapshot(&wrong_version),
        Err(SnapshotError::UnsupportedVersion(FORMAT_VERSION + 1))
    );

    let truncated = &bytes[..bytes.len() / 2];
    assert!(matches!(
        World::from_snapshot(truncated),
        Err(SnapshotError::Decode(_))
    ));
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn thread_count_does_not_change_results() {
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                scenario::determinism(8, Size::Tiny, 120)
                    .world()
                    .state_hash()
            })
    };
    let one = run(1);
    assert_eq!(one, run(4));
    assert_eq!(one, run(16));
}

#[test]
fn year_long_run_stays_healthy() {
    let game = scenario::determinism(9, Size::Tiny, 365);
    let world = game.world();
    assert_eq!(world.check_invariants(), Ok(()));
    for p in world.players().ids() {
        let c = world.players().condition(p);
        assert!(c.fitness <= 10_000 && c.morale <= 10_000 && c.fatigue <= 10_000);
    }
}
