//! The home screen and new-game choices.

use super::competition::next_and_last;
use super::refs::*;
use crate::dto;
use fs_world::matchday::team_sheet;
use fs_world::{ClubId, ManagerId, World};

/// Every club a new manager could take, best first.
pub fn club_choices(world: &World) -> dto::ClubChoices {
    let mut clubs: Vec<(ClubId, u16)> = world
        .clubs()
        .iter()
        .filter(|(_, c)| c.league().is_some() && c.squad().len() >= 11)
        .map(|(id, c)| (id, c.reputation))
        .collect();
    clubs.sort_by_key(|&(id, rep)| (std::cmp::Reverse(rep), id));
    let n = clubs.len().max(1);
    let choices = clubs
        .iter()
        .enumerate()
        .map(|(rank, &(id, _))| {
            let sheet = team_sheet(world, id);
            let ability: u32 = sheet
                .starters
                .iter()
                .map(|p| u32::from(world.players().ability(p.id)))
                .sum::<u32>()
                / sheet.starters.len().max(1) as u32;
            dto::ClubChoice {
                club: club_ref(world, id),
                league: league_ref(world, world.clubs()[id].league().expect("filtered")),
                stars: 5 - (rank * 5 / n) as u8,
                squad_ability: percent_of_1000(ability as u16),
            }
        })
        .collect();
    dto::ClubChoices { clubs: choices }
}

/// The match `manager` can watch today, if any.
pub fn match_day(world: &World, manager: ManagerId) -> Option<dto::MatchDay> {
    let w = world
        .pending()?
        .awaiting
        .iter()
        .find(|w| w.manager == manager)?;
    let f = w.fixture.get(world)?;
    Some(dto::MatchDay {
        fixture: fixture(world, w.fixture),
        side: if f.home == w.club {
            dto::Side::Home
        } else {
            dto::Side::Away
        },
    })
}

pub fn overview(
    world: &World,
    manager: ManagerId,
    club: Option<ClubId>,
    unread: u32,
) -> dto::Overview {
    let m = &world.managers()[manager];
    let league = club.and_then(|c| world.clubs()[c].league());
    let position = club.zip(league).and_then(|(club, l)| {
        let season = world.leagues()[l].season()?;
        season
            .table()
            .iter()
            .position(|r| r.club == club)
            .map(|p| (p + 1) as u8)
    });
    let (next_fixture, last_fixture) = club.map_or((None, None), |c| next_and_last(world, c));
    let today = world.date();
    let finances = club.map(|c| world.clubs()[c].finances());
    dto::Overview {
        date: date(today),
        weekday: format!("{:?}", today.weekday()),
        season: season_label(fs_world::people::season_year(today)),
        manager: dto::ManagerInfo {
            id: manager.raw(),
            name: format!(
                "{} {}",
                world.names().get(m.first_name),
                world.names().get(m.last_name)
            ),
            look: crate::query::refs::look(fs_world::looks::manager(world, manager)),
        },
        club: club.map(|c| club_ref(world, c)),
        league: league.map(|l| league_ref(world, l)),
        position,
        next_fixture,
        last_fixture,
        form: club.map(|c| form(world, c, 5)).unwrap_or_default(),
        unread,
        match_day: match_day(world, manager),
        watch_matches: m.watch_matches,
        delegate_squad: m.delegate_squad,
        board: club.map(|c| super::management::board(world, c)),
        balance: finances.map(|f| f.balance),
        transfer_budget: finances.map(|f| f.transfer_budget),
        window_open: fs_world::transfer::window_open(today),
        cups: club
            .map(|c| super::cup::statuses(world, c))
            .unwrap_or_default(),
        press_waiting: super::media::waiting(world, manager),
        headlines: super::media::headlines(world, club, 4),
    }
}
