//! Training: schedules, individual training and tactical familiarity.

use fs_core::GameDate;
use fs_gen::{GenConfig, Size, generate};
use fs_match::{Formation, Tactic};
use fs_world::attributes::{Attr, Position};
use fs_world::matchday::team_sheet;
use fs_world::people::age_years;
use fs_world::training::{Focus, Individual, Intensity, Schedule};
use fs_world::{Actor, ClubId, Command, Event, Game, ManagerId, World};

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

/// A world where every club is human-run (with the assistant handling
/// contracts), so tests can set any club's training.
fn all_human(seed: u64) -> Game {
    let mut game = Game::new(generate(&GenConfig {
        human_clubs: (0..20).collect(),
        ..GenConfig::preset(Size::Tiny, seed)
    }));
    let managers: Vec<ManagerId> = game.world().human_managers().collect();
    for m in managers {
        game.submit(Actor::Manager(m), Command::SetDelegation { squad: true })
            .unwrap();
    }
    game
}

fn manager_of(world: &World, club: ClubId) -> ManagerId {
    world.clubs()[club].manager.unwrap()
}

#[test]
fn a_new_shape_needs_drilling() {
    let mut game = all_human(61);
    let club = ClubId::new(3);
    let m = manager_of(game.world(), club);
    let current = game.world().clubs()[club].tactic().unwrap().formation;
    assert_eq!(team_sheet(game.world(), club).starters[0].cohesion, 10_000);
    let other = Formation::ALL.into_iter().find(|&f| f != current).unwrap();
    game.submit(
        Actor::Manager(m),
        Command::SetTactic {
            tactic: Some(Tactic::balanced(other)),
        },
    )
    .unwrap();
    let fresh = team_sheet(game.world(), club).starters[0].cohesion;
    assert!(fresh < 5_000, "{fresh}");
    // Tactics training drills it in about a month.
    game.submit(
        Actor::Manager(m),
        Command::SetTraining {
            schedule: Schedule {
                focus: Focus::Tactics,
                intensity: Intensity::Normal,
            },
        },
    )
    .unwrap();
    advance_to(&mut game, date(2026, 8, 5));
    let w = game.world();
    assert_eq!(team_sheet(w, club).starters[0].cohesion, 10_000);
    // The old shape fades.
    assert!(w.clubs()[club].familiarity().get(current) < 1_000);
}

/// Mean growth of two attributes among a club's young players.
fn growth(start: &World, end: &World, club: ClubId, a: Attr) -> f64 {
    let ps = start.players();
    let young: Vec<_> = start.clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&p| {
            age_years(ps.bio(p).birth_date, start.date()) < 22.0
                && end.players().club(p) == Some(club)
        })
        .collect();
    let total: f64 = young
        .iter()
        .map(|&p| {
            f64::from(end.players().attributes(p).get(a)) - f64::from(ps.attributes(p).get(a))
        })
        .sum();
    total / young.len().max(1) as f64
}

#[test]
fn a_focus_moves_growth_between_attributes() {
    let mut game = all_human(62);
    let start = game.world().clone();
    let w = game.world();
    let clubs: Vec<ClubId> = w.clubs().ids().collect();
    for (k, &c) in clubs.iter().enumerate() {
        let focus = if k % 2 == 0 {
            Focus::Attacking
        } else {
            Focus::Defending
        };
        let m = manager_of(game.world(), c);
        game.submit(
            Actor::Manager(m),
            Command::SetTraining {
                schedule: Schedule {
                    focus,
                    intensity: Intensity::Normal,
                },
            },
        )
        .unwrap();
    }
    advance_to(&mut game, date(2027, 5, 1));
    let end = game.world();
    let (mut attacking, mut defending) = (0.0, 0.0);
    for (k, &c) in clubs.iter().enumerate() {
        let edge = growth(&start, end, c, Attr::Finishing) - growth(&start, end, c, Attr::Tackling);
        if k % 2 == 0 {
            attacking += edge;
        } else {
            defending += edge;
        }
    }
    assert!(
        attacking > defending + 5.0,
        "finishing over tackling: attacking clubs {attacking:.1}, defending clubs {defending:.1}"
    );
}

#[test]
fn hard_training_injures_more_than_light() {
    let mut game = all_human(63);
    let clubs: Vec<ClubId> = game.world().clubs().ids().collect();
    for (k, &c) in clubs.iter().enumerate() {
        let intensity = if k % 2 == 0 {
            Intensity::Hard
        } else {
            Intensity::Light
        };
        let m = manager_of(game.world(), c);
        game.submit(
            Actor::Manager(m),
            Command::SetTraining {
                schedule: Schedule {
                    focus: Focus::Balanced,
                    intensity,
                },
            },
        )
        .unwrap();
    }
    // Injuries on days without matches come from training.
    let (mut hard, mut light) = (0, 0);
    for _ in 0..240 {
        let events = game
            .submit(Actor::System, Command::Advance { days: 1 })
            .unwrap();
        if events
            .iter()
            .any(|e| matches!(e, Event::MatchPlayed { .. }))
        {
            continue;
        }
        for e in &events {
            if let Event::PlayerInjured { club, .. } = e {
                if club.index() % 2 == 0 {
                    hard += 1;
                } else {
                    light += 1;
                }
            }
        }
    }
    assert!(hard > light * 3 / 2, "hard {hard}, light {light}");
}

#[test]
fn players_can_learn_a_position() {
    let mut game = all_human(64);
    let club = ClubId::new(5);
    let m = manager_of(game.world(), club);
    let w = game.world();
    let player = w.clubs()[club]
        .squad()
        .iter()
        .copied()
        .find(|&p| w.players().positions(p).get(Position::WingBackLeft) < 10)
        .expect("someone new to wing-back");
    let before = w.players().positions(player).get(Position::WingBackLeft);
    game.submit(
        Actor::Manager(m),
        Command::SetIndividualTraining {
            player,
            training: Some(Individual::Position(Position::WingBackLeft)),
        },
    )
    .unwrap();
    advance_to(&mut game, date(2026, 11, 1));
    let after = game
        .world()
        .players()
        .positions(player)
        .get(Position::WingBackLeft);
    assert!(after >= before + 4, "{before} -> {after}");
    assert_eq!(game.world().check_invariants(), Ok(()));
}

#[test]
fn only_your_players_train_with_you() {
    let mut game = all_human(65);
    let (a, b) = (ClubId::new(1), ClubId::new(2));
    let m = manager_of(game.world(), a);
    let theirs = game.world().clubs()[b].squad()[0];
    assert!(
        game.submit(
            Actor::Manager(m),
            Command::SetIndividualTraining {
                player: theirs,
                training: Some(Individual::Attribute(Attr::Finishing)),
            },
        )
        .is_err()
    );
}
