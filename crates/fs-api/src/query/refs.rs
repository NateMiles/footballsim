//! Building blocks shared by the views.

use crate::dto;
use fs_core::GameDate;
use fs_core::attributes::Position;
use fs_world::cup::{CupScope, StageDraw};
use fs_world::people::age_years;
use fs_world::{ClubId, CupId, FixtureRef, LeagueId, PlayerId, World};

/// A date as `YYYY-MM-DD`.
pub fn date(d: GameDate) -> String {
    d.to_string()
}

/// A season label such as `"2026/27"`.
pub fn season_label(year: i32) -> String {
    format!("{year}/{:02}", (year + 1).rem_euclid(100))
}

/// A 0-1000 value as 0-100.
pub fn percent_of_1000(v: u16) -> u8 {
    (v / 10).min(100) as u8
}

/// A 0-10000 value as 0-100.
pub fn percent_of_10000(v: u16) -> u8 {
    (v / 100).min(100) as u8
}

/// Kit colours: a stable pick from a palette, by club name.
pub fn colors(name: &str) -> dto::Colors {
    const PALETTE: [(&str, &str); 16] = [
        ("#c8102e", "#ffffff"),
        ("#034694", "#ffffff"),
        ("#6cabdd", "#1c2c5b"),
        ("#fdb913", "#231f20"),
        ("#132257", "#ffffff"),
        ("#7a263a", "#1bb1e7"),
        ("#ef0107", "#063672"),
        ("#003399", "#ffcd00"),
        ("#00a650", "#ffffff"),
        ("#241f20", "#ffffff"),
        ("#ffffff", "#1d1d1b"),
        ("#e03a3e", "#000000"),
        ("#f58220", "#1d1d1b"),
        ("#5c2d91", "#f5c518"),
        ("#0057b8", "#e8e8e8"),
        ("#1b4d3e", "#d4af37"),
    ];
    // FNV-1a: stable everywhere, unlike the standard hasher.
    let mut h: u32 = 0x811c_9dc5;
    for b in name.bytes() {
        h = (h ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }
    let (primary, secondary) = PALETTE[(h % PALETTE.len() as u32) as usize];
    dto::Colors {
        primary: primary.into(),
        secondary: secondary.into(),
    }
}

pub fn club_ref(world: &World, id: ClubId) -> dto::ClubRef {
    let c = &world.clubs()[id];
    dto::ClubRef {
        id: id.raw(),
        name: c.name.clone(),
        short_name: c.short_name.clone(),
        colors: colors(&c.name),
    }
}

pub fn league_ref(world: &World, id: LeagueId) -> dto::LeagueRef {
    let l = &world.leagues()[id];
    dto::LeagueRef {
        id: id.raw(),
        name: l.name.clone(),
        nation: world.nations()[l.nation].name.clone(),
        level: l.level,
    }
}

/// First name and last name.
pub fn player_names(world: &World, id: PlayerId) -> (String, String) {
    let bio = world.players().bio(id);
    (
        world.names().get(bio.first_name).to_owned(),
        world.names().get(bio.last_name).to_owned(),
    )
}

pub fn player_ref(world: &World, id: PlayerId) -> dto::PlayerRef {
    let (first, last) = player_names(world, id);
    let initial = first
        .chars()
        .next()
        .map(|c| format!("{c}. "))
        .unwrap_or_default();
    dto::PlayerRef {
        id: id.raw(),
        name: format!("{first} {last}"),
        short_name: format!("{initial}{last}"),
        number: world.players().squad_number(id),
        position: world.players().positions(id).best().code().into(),
    }
}

pub fn age(world: &World, id: PlayerId) -> u8 {
    age_years(world.players().bio(id).birth_date, world.date()) as u8
}

/// Positions a player plays well (familiarity 15+), best first.
pub fn good_positions(world: &World, id: PlayerId) -> Vec<String> {
    let ratings = world.players().positions(id);
    let mut good: Vec<Position> = Position::ALL
        .into_iter()
        .filter(|&p| ratings.get(p) >= 15)
        .collect();
    good.sort_by_key(|&p| std::cmp::Reverse(ratings.get(p)));
    good.into_iter().map(|p| p.code().to_owned()).collect()
}

pub fn injury(world: &World, id: PlayerId) -> Option<dto::Injury> {
    world.players().injury(id).map(|i| dto::Injury {
        severity: format!("{:?}", i.severity).to_lowercase(),
        until: date(i.until),
    })
}

pub fn contract(world: &World, id: PlayerId) -> Option<dto::Contract> {
    world.players().contract(id).map(|c| dto::Contract {
        wage: c.wage,
        expires: date(c.expires),
        appearance_bonus: c.appearance_bonus,
        goal_bonus: c.goal_bonus,
        release_clause: c.release_clause,
    })
}

/// A cup as other views refer to it.
pub fn cup_ref(world: &World, id: CupId) -> dto::CupRef {
    let c = &world.cups()[id];
    let region = match c.scope {
        CupScope::Nation(n) => world.nations()[n].name.clone(),
        CupScope::Continent(k) => world
            .continents()
            .get(usize::from(k))
            .map_or_else(String::new, |x| x.name.clone()),
    };
    dto::CupRef {
        id: id.raw(),
        name: c.format.name.clone(),
        short_name: c.format.short_name.clone(),
        region,
        continental: c.is_continental(),
    }
}

/// The competition a fixture belongs to.
pub fn competition_ref(world: &World, r: FixtureRef) -> dto::CompetitionRef {
    match r {
        FixtureRef::League { league, .. } => {
            let l = &world.leagues()[league];
            dto::CompetitionRef {
                kind: dto::CompetitionKind::League,
                id: league.raw(),
                name: l.name.clone(),
                short_name: if l.level == 1 {
                    "League".into()
                } else {
                    format!("Div {}", l.level)
                },
            }
        }
        FixtureRef::Cup { cup, .. } => {
            let c = &world.cups()[cup];
            dto::CompetitionRef {
                kind: dto::CompetitionKind::Cup,
                id: cup.raw(),
                name: c.format.name.clone(),
                short_name: c.format.short_name.clone(),
            }
        }
    }
}

/// A cup fixture's round and leg, such as `"Semi-finals, 2nd leg"`.
fn round(world: &World, cup: CupId, index: usize) -> Option<String> {
    let c = &world.cups()[cup];
    let e = c.edition()?;
    let f = e.fixtures.get(index)?;
    let stage = e.stages.get(usize::from(f.stage))?;
    let name = &c.format.stages[stage.format_index()].name;
    Some(match stage {
        StageDraw::Groups { .. } => format!("{name}, {}", group_name(usize::from(f.tie))),
        StageDraw::Knockout { ties, .. } => {
            if ties
                .get(usize::from(f.tie))
                .is_some_and(|t| t.legs.len() > 1)
            {
                format!("{name}, {} leg", ordinal(f.leg + 1))
            } else {
                name.clone()
            }
        }
    })
}

/// `"Group A"`, `"Group B"`...
pub fn group_name(k: usize) -> String {
    let letter = char::from(b'A' + (k % 26) as u8);
    format!("Group {letter}")
}

/// A fixture in any competition.
pub fn fixture(world: &World, r: FixtureRef) -> dto::Fixture {
    let f = r.get(world).expect("fixture exists");
    let (index, round, neutral) = match r {
        FixtureRef::League { index, .. } => (index, None, false),
        FixtureRef::Cup { cup, index } => (
            index,
            round(world, cup, index as usize),
            fs_world::cup::match_rules(world, cup, index as usize).0,
        ),
    };
    dto::Fixture {
        competition: competition_ref(world, r),
        index,
        date: date(f.date),
        home: club_ref(world, f.home),
        away: club_ref(world, f.away),
        result: f.result.as_ref().map(|r| dto::FixtureResult {
            goals: r.goals,
            xg: r.xg,
            scorers: r
                .scorers
                .iter()
                .map(|&(side, player)| dto::Scorer {
                    side: if side == 0 {
                        dto::Side::Home
                    } else {
                        dto::Side::Away
                    },
                    player: player_ref(world, player),
                })
                .collect(),
            extra_time: r.extra_time,
            penalties: r.penalties,
        }),
        round,
        neutral,
    }
}

/// A league fixture.
pub fn league_fixture(world: &World, league: LeagueId, index: usize) -> dto::Fixture {
    fixture(
        world,
        FixtureRef::League {
            league,
            index: index as u32,
        },
    )
}

/// A club's league results so far this season, oldest first: `"W"`,
/// `"D"` or `"L"`.
pub fn results(world: &World, club: ClubId) -> Vec<String> {
    let Some(league) = world.clubs()[club].league() else {
        return Vec::new();
    };
    let Some(season) = world.leagues()[league].season() else {
        return Vec::new();
    };
    season
        .fixtures
        .iter()
        .filter(|f| f.home == club || f.away == club)
        .filter_map(|f| {
            let r = f.result.as_ref()?;
            let (us, them) = if f.home == club {
                (r.goals[0], r.goals[1])
            } else {
                (r.goals[1], r.goals[0])
            };
            Some(
                match us.cmp(&them) {
                    std::cmp::Ordering::Greater => "W",
                    std::cmp::Ordering::Equal => "D",
                    std::cmp::Ordering::Less => "L",
                }
                .to_owned(),
            )
        })
        .collect()
}

/// The last `n` of a club's results.
pub fn form(world: &World, club: ClubId, n: usize) -> Vec<String> {
    let all = results(world, club);
    all[all.len().saturating_sub(n)..].to_vec()
}

/// 1st, 2nd, 3rd, 4th...
pub fn ordinal(n: u8) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}
