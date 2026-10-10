//! Fixed scripted scenarios.
//!
//! The determinism check runs [`determinism`] on native and on WebAssembly
//! and compares the final hashes. Change it only deliberately: any change
//! alters the expected hash.

use crate::{GenConfig, Size, generate};
use fs_world::{Actor, Command, Game};

/// Generates a world, then issues a mix of human and system commands,
/// including one that must be rejected. Covers generation, the daily
/// tick, command validation and the command log.
///
/// # Panics
/// Panics if a command that should succeed is rejected, or the invalid one
/// is accepted.
pub fn determinism(seed: u64, size: Size, days: u16) -> Game {
    let world = generate(&GenConfig::preset(size, seed));
    let human = world.human_managers().next().expect("preset has a human");
    let club = world.managers()[human].club.expect("human has a club");
    let squad = world.clubs()[club].squad().to_vec();
    let mut game = Game::new(world);

    game.submit(Actor::Manager(human), Command::Advance { days: 7 })
        .expect("advance");
    game.submit(
        Actor::Manager(human),
        Command::SetSquadNumber {
            player: squad[squad.len() - 1],
            number: Some(99),
        },
    )
    .expect("set number");
    let rejected = game.submit(
        Actor::Manager(human),
        Command::SetSquadNumber {
            player: squad[0],
            number: Some(99),
        },
    );
    assert!(rejected.is_err(), "duplicate number must be rejected");

    let mut remaining = days;
    while remaining > 0 {
        let step = remaining.min(fs_world::command::MAX_ADVANCE_DAYS);
        game.submit(Actor::System, Command::Advance { days: step })
            .expect("advance");
        remaining -= step;
    }
    game
}
