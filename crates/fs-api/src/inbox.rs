//! The manager's inbox: world events turned into messages.

use crate::dto::{self, Category, Link};
use crate::query::refs::{club_ref, date, fixture, player_ref, season_label};
use fs_world::{ClubId, Event, LeagueId, ManagerId, World};
use serde::{Deserialize, Serialize};

/// Messages for one manager, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Inbox {
    messages: Vec<dto::Message>,
}

impl Inbox {
    /// Messages, newest first.
    pub fn view(&self) -> dto::InboxView {
        dto::InboxView {
            messages: self.messages.iter().rev().cloned().collect(),
            unread: self.unread(),
        }
    }

    pub fn unread(&self) -> u32 {
        self.messages.iter().filter(|m| !m.read).count() as u32
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Marks one message read, or all of them.
    pub fn mark_read(&mut self, id: Option<u32>) {
        for m in &mut self.messages {
            if id.is_none_or(|id| m.id == id) {
                m.read = true;
            }
        }
    }

    fn push(
        &mut self,
        world: &World,
        category: Category,
        title: String,
        body: String,
        link: Option<Link>,
    ) {
        self.messages.push(dto::Message {
            id: self.messages.len() as u32,
            date: date(world.date()),
            category,
            title,
            body,
            read: false,
            link,
        });
    }

    /// Adds messages for the events that concern `manager` and `club`.
    pub fn record(&mut self, world: &World, manager: ManagerId, club: ClubId, events: &[Event]) {
        let club_name = |c: ClubId| world.clubs()[c].name.clone();
        let player = |p| player_ref(world, p).name;
        for e in events {
            match e {
                Event::MatchPlayed {
                    league,
                    home,
                    away,
                    goals,
                } if *home == club || *away == club => {
                    let (us, them) = if *home == club {
                        (goals[0], goals[1])
                    } else {
                        (goals[1], goals[0])
                    };
                    let verdict = match us.cmp(&them) {
                        std::cmp::Ordering::Greater => "Win",
                        std::cmp::Ordering::Equal => "Draw",
                        std::cmp::Ordering::Less => "Defeat",
                    };
                    let link =
                        played_today(world, *league, *home, *away).map(|index| Link::Fixture {
                            league: league.raw(),
                            index: index as u32,
                        });
                    let body = match link {
                        Some(Link::Fixture { index, .. }) => {
                            scorers_line(world, *league, index as usize)
                        }
                        _ => String::new(),
                    };
                    self.push(
                        world,
                        Category::Match,
                        format!(
                            "{verdict}: {} {}-{} {}",
                            club_name(*home),
                            goals[0],
                            goals[1],
                            club_name(*away)
                        ),
                        body,
                        link,
                    );
                }
                Event::PlayerInjured {
                    player: p,
                    club: c,
                    until,
                } if *c == club => self.push(
                    world,
                    Category::Injury,
                    format!("{} injured", player(*p)),
                    format!("Expected to be out until {}.", date(*until)),
                    Some(Link::Player { player: p.raw() }),
                ),
                Event::ContractExpiring {
                    player: p,
                    club: c,
                    expires,
                } if *c == club => self.push(
                    world,
                    Category::Contract,
                    format!("{}'s contract is ending", player(*p)),
                    format!(
                        "The contract runs out on {}. Offer a new one or the player will leave.",
                        date(*expires)
                    ),
                    Some(Link::Player { player: p.raw() }),
                ),
                Event::PlayerLeft { player: p, club: c } if *c == club => self.push(
                    world,
                    Category::Transfer,
                    format!("{} has left", player(*p)),
                    format!("{} is no longer at {}.", player(*p), club_name(club)),
                    Some(Link::Player { player: p.raw() }),
                ),
                Event::PlayerSigned { player: p, club: c } if *c == club => self.push(
                    world,
                    Category::Transfer,
                    format!("{} signs", player(*p)),
                    format!("{} has joined {}.", player(*p), club_name(club)),
                    Some(Link::Player { player: p.raw() }),
                ),
                Event::PlayerRetired { player: p, club: c } if *c == club => self.push(
                    world,
                    Category::Transfer,
                    format!("{} retires", player(*p)),
                    format!("{} has hung up their boots.", player(*p)),
                    Some(Link::Player { player: p.raw() }),
                ),
                Event::YouthIntake { club: c, players } if *c == club => self.push(
                    world,
                    Category::Youth,
                    "Youth intake".to_owned(),
                    format!(
                        "{} youngsters join the academy: {}.",
                        players.len(),
                        players
                            .iter()
                            .map(|&p| player(p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    Some(Link::Squad),
                ),
                Event::SeasonEnded {
                    league,
                    year,
                    champion,
                } if world.clubs()[club].league() == Some(*league) || *champion == club => {
                    self.push(
                        world,
                        Category::Season,
                        format!("{} are champions", club_name(*champion)),
                        format!(
                            "{} win the {} {}.",
                            club_name(*champion),
                            world.leagues()[*league].name,
                            season_label(*year)
                        ),
                        Some(Link::Table {
                            league: league.raw(),
                        }),
                    );
                }
                Event::Promoted { club: c, to, .. } if *c == club => self.push(
                    world,
                    Category::Season,
                    "Promoted!".to_owned(),
                    format!(
                        "{} will play in the {} next season.",
                        club_name(club),
                        world.leagues()[*to].name
                    ),
                    Some(Link::Table { league: to.raw() }),
                ),
                Event::Relegated { club: c, to, .. } if *c == club => self.push(
                    world,
                    Category::Season,
                    "Relegated".to_owned(),
                    format!(
                        "{} will play in the {} next season.",
                        club_name(club),
                        world.leagues()[*to].name
                    ),
                    Some(Link::Table { league: to.raw() }),
                ),
                Event::ManagerAppointed {
                    manager: m,
                    club: c,
                } if *m == manager => {
                    let r = club_ref(world, *c);
                    self.push(
                        world,
                        Category::Club,
                        format!("Welcome to {}", r.name),
                        "The board welcomes you. Check the squad, set your tactics and lineup, then press Continue. Matches stop for you to watch them live; you can change that on the home screen.".to_owned(),
                        Some(Link::Squad),
                    );
                }
                _ => {}
            }
        }
    }
}

/// The index of today's played fixture between two clubs.
fn played_today(world: &World, league: LeagueId, home: ClubId, away: ClubId) -> Option<usize> {
    let season = world.leagues()[league].season()?;
    season.fixtures.iter().position(|f| {
        f.home == home && f.away == away && f.result.is_some() && f.date == world.date()
    })
}

fn scorers_line(world: &World, league: LeagueId, index: usize) -> String {
    let f = fixture(world, league, index);
    let Some(r) = f.result else {
        return String::new();
    };
    if r.scorers.is_empty() {
        return "No goals.".to_owned();
    }
    let names = r
        .scorers
        .iter()
        .map(|s| s.player.short_name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    format!("Scorers: {names}.")
}
