//! Finances, board, staff and jobs.

use super::refs::*;
use crate::dto;
use fs_world::economy::{player_wage_bill, staff_wage_bill};
use fs_world::objectives::{self, Aim, Ethos, Importance, Objective, Progress};
use fs_world::people::age_years;
use fs_world::staff::{self, StaffRole};
use fs_world::{ClubId, ManagerId, StaffId, World};

/// The board's objective, in words.
pub fn objective(world: &World, club: ClubId, target: u8) -> String {
    let Some(league) = world.clubs()[club].league() else {
        return "No league season".into();
    };
    let l = &world.leagues()[league];
    let n = l.clubs().len() as u8;
    let swaps = l.swaps;
    let lower = world
        .leagues()
        .iter()
        .any(|(_, o)| o.nation == l.nation && o.level == l.level + 1);
    match target {
        0 => "No target".into(),
        1 => "Win the league".into(),
        t if l.level > 1 && t <= swaps => "Win promotion".into(),
        t if t <= n / 4 => format!("Finish in the top {t}"),
        t if t <= n / 2 => "Finish in the top half".into(),
        t if lower && t <= n.saturating_sub(swaps) => "Avoid relegation".into(),
        t => format!("Finish {} or better", ordinal(t)),
    }
}

pub fn board(world: &World, club: ClubId) -> dto::BoardView {
    let b = world.clubs()[club].board();
    dto::BoardView {
        target: b.target,
        objective: objective(world, club, b.target),
        confidence: b.confidence,
        objectives: world.clubs()[club]
            .objectives()
            .iter()
            .map(|o| objective_view(world, club, o))
            .collect(),
        ethos: match objectives::ethos(world, club) {
            Ethos::Youth => "Brings through young players",
            Ethos::Attacking => "Plays attacking football",
            Ethos::Prudent => "Lives within its means",
            Ethos::Ambitious => "Wants results above all",
        }
        .into(),
    }
}

fn objective_view(world: &World, club: ClubId, o: &Objective) -> dto::ObjectiveView {
    let s = objectives::standing(world, club, o);
    let (text, detail) = match o.aim {
        Aim::Cup { cup, stage } => {
            let c = &world.cups()[cup];
            let stages = &c.format.stages;
            let name = &c.format.name;
            let text = if usize::from(stage) + 1 >= stages.len() {
                format!("Reach the final of the {name}")
            } else {
                let st = stages
                    .get(usize::from(stage))
                    .map_or("later rounds".to_owned(), |x| x.name.to_lowercase());
                format!("Reach the {st} of the {name}")
            };
            let detail = match s.progress {
                Progress::Met => "Done".to_owned(),
                Progress::Failed => "Knocked out".to_owned(),
                _ => "Still in it".to_owned(),
            };
            (text, detail)
        }
        Aim::Youth { share } => (
            format!("Give players under 22 at least {share}% of the minutes"),
            format!("{:.0}% so far", s.value),
        ),
        Aim::Goals { tenths } => (
            format!("Score {:.1} league goals a game", f64::from(tenths) / 10.0),
            format!("{:.2} so far", s.value),
        ),
        Aim::Profit => (
            "End the season without a loss".to_owned(),
            if s.value >= 0.0 {
                format!("{} up so far", crate::inbox::money(s.value as i64))
            } else {
                format!("{} down so far", crate::inbox::money(-(s.value as i64)))
            },
        ),
    };
    dto::ObjectiveView {
        text,
        detail,
        status: match s.progress {
            Progress::Met => "met",
            Progress::Failed => "failed",
            Progress::OnTrack => "onTrack",
            Progress::Behind => "behind",
        }
        .into(),
        important: o.importance == Importance::High,
    }
}

pub fn finances(world: &World, club: ClubId) -> dto::FinancesView {
    let c = &world.clubs()[club];
    let f = c.finances();
    dto::FinancesView {
        club: club_ref(world, club),
        balance: f.balance,
        transfer_budget: f.transfer_budget,
        wage_budget: f.wage_budget,
        wage_bill: player_wage_bill(world, club),
        staff_bill: staff_wage_bill(world, club),
        season: (&f.season).into(),
        history: f
            .history
            .iter()
            .rev()
            .map(|a| dto::SeasonAccounts {
                season: season_label(a.year),
                ledger: (&a.ledger).into(),
                closing_balance: a.closing_balance,
            })
            .collect(),
        facilities: dto::Facilities {
            capacity: c.facilities.capacity,
            training: c.facilities.training,
            youth: c.facilities.youth,
        },
    }
}

fn member(world: &World, id: StaffId, viewer: Option<ClubId>) -> dto::StaffMember {
    let s = &world.staff()[id];
    let a = s.attrs;
    dto::StaffMember {
        id: id.raw(),
        name: format!(
            "{} {}",
            world.names().get(s.first_name),
            world.names().get(s.last_name)
        ),
        role: s.role.into(),
        age: age_years(s.birth_date, world.date()) as u8,
        nation: world.nations()[s.nation].code.clone(),
        attributes: dto::StaffAttributes {
            coaching: a.coaching,
            youth_coaching: a.youth_coaching,
            judging_ability: a.judging_ability,
            judging_potential: a.judging_potential,
            physiotherapy: a.physiotherapy,
            management: a.management,
        },
        rating: a.key(s.role),
        wage: s.contract().map(|c| c.wage),
        expires: s.contract().map(|c| date(c.expires)),
        demand: s.wage_demand(),
        would_join: viewer.is_some_and(|c| s.would_join(world.clubs()[c].reputation)),
    }
}

/// Free staff shown on the staff screen.
const AVAILABLE_SHOWN: usize = 120;

pub fn staff(world: &World, club: ClubId) -> dto::StaffView {
    let today = world.date();
    let mut current: Vec<StaffId> = world.clubs()[club].staff().to_vec();
    current.sort_by_key(|&s| {
        (
            world.staff()[s].role,
            std::cmp::Reverse(world.staff()[s].attrs.key(world.staff()[s].role)),
        )
    });
    let mut free: Vec<StaffId> = world
        .staff()
        .ids()
        .filter(|&s| staff::available(world, s, today))
        .collect();
    free.sort_by(|&a, &b| {
        world.staff()[b]
            .quality()
            .total_cmp(&world.staff()[a].quality())
            .then(a.cmp(&b))
    });
    free.truncate(AVAILABLE_SHOWN);
    let r = staff::ratings(world, club);
    dto::StaffView {
        club: club_ref(world, club),
        staff: current
            .iter()
            .map(|&s| member(world, s, Some(club)))
            .collect(),
        ratings: dto::StaffRatings {
            coaching: r.coaching as f32,
            youth_coaching: r.youth_coaching as f32,
            scouting_ability: r.scouting_ability as f32,
            scouting_potential: r.scouting_potential as f32,
            physiotherapy: r.physiotherapy as f32,
            management: r.management as f32,
        },
        available: free.iter().map(|&s| member(world, s, Some(club))).collect(),
        limits: StaffRole::ALL.iter().map(|r| r.wanted().1 as u8).collect(),
    }
}

pub fn jobs(world: &World, manager: ManagerId) -> dto::JobsView {
    let m = &world.managers()[manager];
    let mut clubs = fs_world::board::vacancies(world);
    clubs.sort_by_key(|&c| (std::cmp::Reverse(world.clubs()[c].reputation), c));
    let n = world.clubs().len().max(1);
    let mut all: Vec<u16> = world.clubs().iter().map(|(_, c)| c.reputation).collect();
    all.sort_unstable_by(|a, b| b.cmp(a));
    let stars = |rep: u16| {
        let rank = all.iter().position(|&r| r <= rep).unwrap_or(n - 1);
        5 - (rank * 5 / n) as u8
    };
    dto::JobsView {
        reputation: (m.reputation / 100).min(100) as u8,
        employed: m.club.is_some(),
        jobs: clubs
            .into_iter()
            .map(|c| dto::Job {
                club: club_ref(world, c),
                league: league_ref(
                    world,
                    world.clubs()[c].league().expect("vacancies have leagues"),
                ),
                stars: stars(world.clubs()[c].reputation),
                would_hire: fs_world::board::would_hire(world, c, m.reputation),
            })
            .collect(),
    }
}
