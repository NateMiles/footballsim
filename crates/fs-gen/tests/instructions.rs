//! Roles, set-piece takers and opposition instructions in the world.

use fs_gen::{GenConfig, Size, generate};
use fs_match::roles::{Duty, Marking, Role, SlotRole};
use fs_match::{Formation, Instructions, Tactic};
use fs_world::matchday::team_sheet;
use fs_world::{Actor, ClubId, Command, CommandError, Event, Game, ManagerId};

fn career(seed: u64, club: u32) -> (Game, ManagerId, ClubId) {
    let mut game = Game::new(generate(&GenConfig {
        human_clubs: vec![club],
        ..GenConfig::preset(Size::Tiny, seed)
    }));
    let m = game.world().human_managers().next().unwrap();
    game.submit(Actor::Manager(m), Command::SetDelegation { squad: true })
        .unwrap();
    game.submit(
        Actor::Manager(m),
        Command::SetTactic {
            tactic: Some(Tactic::balanced(Formation::F442)),
        },
    )
    .unwrap();
    (game, m, ClubId::new(club))
}

/// Roles for a 4-4-2: wingers on attack and a target man.
fn wingers() -> Instructions {
    let mut i = Instructions::default();
    for (k, s) in Formation::F442.slots().iter().enumerate() {
        if Role::Winger.fits(s.position) {
            i.roles[k] = Some(SlotRole {
                role: Role::Winger,
                duty: Duty::Attack,
            });
        }
    }
    i
}

#[test]
fn instructions_reach_the_match_and_marking_lasts_one_match() {
    let (mut game, m, club) = career(81, 3);
    let w = game.world();
    let mine = team_sheet(w, club);
    let taker = mine.starters[10].id;
    let opponent = ClubId::new(4);
    let target = w.clubs()[opponent].squad()[0];
    let mut ins = wingers();
    ins.penalty_taker = Some(taker);
    ins.marking = vec![(target, Marking::Tight)];
    game.submit(
        Actor::Manager(m),
        Command::SetInstructions {
            instructions: Some(ins.clone()),
        },
    )
    .unwrap();
    let sheet = team_sheet(game.world(), club);
    assert_eq!(sheet.instructions, ins);

    // After a match, the roles and taker stay; the marking goes.
    let mut played = false;
    while !played {
        let events = game
            .submit(Actor::System, Command::Advance { days: 1 })
            .unwrap();
        played = events.iter().any(|e| {
            matches!(e, Event::MatchPlayed { home, away, .. } if *home == club || *away == club)
        });
    }
    let sheet = team_sheet(game.world(), club);
    assert_eq!(sheet.instructions.roles, ins.roles);
    assert_eq!(sheet.instructions.penalty_taker, Some(taker));
    assert!(sheet.instructions.marking.is_empty());
}

#[test]
fn roles_only_apply_to_the_formation_they_were_set_for() {
    let (mut game, m, club) = career(82, 5);
    game.submit(
        Actor::Manager(m),
        Command::SetInstructions {
            instructions: Some(wingers()),
        },
    )
    .unwrap();
    game.submit(
        Actor::Manager(m),
        Command::SetTactic {
            tactic: Some(Tactic::balanced(Formation::F433)),
        },
    )
    .unwrap();
    let sheet = team_sheet(game.world(), club);
    assert!(sheet.instructions.roles.iter().all(Option::is_none));
}

#[test]
fn bad_instructions_are_refused() {
    let (mut game, m, club) = career(83, 6);
    let mut i = Instructions::default();
    i.roles[0] = Some(SlotRole {
        role: Role::Poacher,
        duty: Duty::Attack,
    });
    let set = |game: &mut Game, i: Instructions| {
        game.submit(
            Actor::Manager(m),
            Command::SetInstructions {
                instructions: Some(i),
            },
        )
    };
    assert!(matches!(
        set(&mut game, i),
        Err(CommandError::InvalidInstructions(_))
    ));
    let theirs = game.world().clubs()[ClubId::new(1)].squad()[0];
    let i = Instructions {
        corner_taker: Some(theirs),
        ..Instructions::default()
    };
    assert!(matches!(
        set(&mut game, i),
        Err(CommandError::NotYourPlayer(_))
    ));
    let mine = game.world().clubs()[club].squad()[0];
    let i = Instructions {
        marking: vec![(mine, Marking::CloseDown)],
        ..Instructions::default()
    };
    assert!(matches!(
        set(&mut game, i),
        Err(CommandError::InvalidInstructions(_))
    ));
}
