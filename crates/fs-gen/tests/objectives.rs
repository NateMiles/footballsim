//! Board objectives beyond the league table.

use fs_core::GameDate;
use fs_gen::{GenConfig, Size, generate};
use fs_world::objectives::{Aim, Progress, standing};
use fs_world::{Actor, ClubId, Command, Event, Game};

#[test]
fn boards_set_cup_targets_by_standing_and_an_aim_by_ethos() {
    let game = Game::new(generate(&GenConfig::preset(Size::Tiny, 101)));
    let w = game.world();
    let cup_target = |c: ClubId| {
        w.clubs()[c].objectives().iter().find_map(|o| match o.aim {
            Aim::Cup { stage, .. } => Some(stage),
            _ => None,
        })
    };
    let mut by_rep: Vec<ClubId> = w.clubs().ids().collect();
    by_rep.sort_by_key(|&c| std::cmp::Reverse(w.clubs()[c].reputation));
    let (best, worst) = (by_rep[0], by_rep[by_rep.len() - 1]);
    assert!(cup_target(best).unwrap() > cup_target(worst).unwrap());
    let aims: Vec<Aim> = w
        .clubs()
        .ids()
        .flat_map(|c| w.clubs()[c].objectives().iter().map(|o| o.aim))
        .collect();
    assert!(aims.iter().any(|a| matches!(a, Aim::Youth { .. })));
    assert!(aims.iter().any(|a| matches!(a, Aim::Goals { .. })));
    assert!(aims.iter().any(|a| matches!(a, Aim::Profit)));
}

#[test]
fn objectives_are_judged_at_the_end_of_the_season() {
    let mut cfg = GenConfig::preset(Size::Tiny, 102);
    cfg.rules.sack_humans = false;
    let mut game = Game::new(generate(&cfg));
    let human = game.world().human_managers().next().unwrap();
    let club = game.world().managers()[human].club.unwrap();
    game.submit(
        Actor::Manager(human),
        Command::SetWatchMatches { watch: false },
    )
    .unwrap();
    let end = GameDate::from_ymd(2027, 6, 30).unwrap();
    while game.world().date() < end {
        let days = end.days_since(game.world().date()).min(200) as u16;
        game.submit(Actor::System, Command::Advance { days })
            .unwrap();
    }
    // Before the rollover: every cup objective is settled by now.
    let w = game.world();
    for o in w.clubs()[club].objectives() {
        if let Aim::Cup { .. } = o.aim {
            let p = standing(w, club, o).progress;
            assert!(matches!(p, Progress::Met | Progress::Failed), "{p:?}");
        }
    }
    let events = game
        .submit(Actor::System, Command::Advance { days: 2 })
        .unwrap();
    let judged = events.iter().find_map(|e| match e {
        Event::ObjectivesJudged {
            club: c,
            met,
            total,
        } if *c == club => Some((*met, *total)),
        _ => None,
    });
    let (met, total) = judged.unwrap_or_else(|| {
        panic!(
            "not judged: manager at {:?}, date {}, objectives {:?}, events {:?}",
            game.world().managers()[human].club,
            game.world().date(),
            game.world().clubs()[club].objectives(),
            events
                .iter()
                .filter(|e| !matches!(e, Event::DayAdvanced { .. }))
                .take(5)
                .collect::<Vec<_>>()
        )
    });
    assert!(met <= total && total >= 1);
    // New objectives for the new season.
    assert!(!game.world().clubs()[club].objectives().is_empty());
}
