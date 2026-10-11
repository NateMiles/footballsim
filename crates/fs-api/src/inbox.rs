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
    /// `club` is the manager's club now, if any.
    pub fn record(
        &mut self,
        world: &World,
        manager: ManagerId,
        club: Option<ClubId>,
        events: &[Event],
    ) {
        // Without a club, only events naming the manager match.
        let club = club.unwrap_or(ClubId::new(u32::MAX));
        let my_league = world.clubs().get(club).and_then(|c| c.league());
        let club_name = |c: ClubId| world.clubs()[c].name.clone();
        let player = |p| player_ref(world, p).name;
        let staff_name = |s: fs_world::StaffId| {
            let st = &world.staff()[s];
            format!(
                "{} {}",
                world.names().get(st.first_name),
                world.names().get(st.last_name)
            )
        };
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
                Event::OfferReceived {
                    offer,
                    player: p,
                    buyer,
                    seller,
                } if *seller == club => {
                    let what =
                        world
                            .market()
                            .offer(*offer)
                            .map_or(String::new(), |o| match o.kind {
                                fs_world::transfer::OfferKind::Transfer { fee } => {
                                    format!("a fee of {}", money(fee))
                                }
                                fs_world::transfer::OfferKind::Loan { wage_share } => {
                                    format!("a loan, paying {wage_share}% of the wages")
                                }
                            });
                    self.push(
                        world,
                        Category::Transfer,
                        format!("{} bid for {}", club_name(*buyer), player(*p)),
                        format!(
                            "{} have offered {what}. Answer within three days or the offer lapses.",
                            club_name(*buyer)
                        ),
                        Some(Link::Transfers),
                    );
                }
                Event::OfferUpdated {
                    player: p,
                    buyer,
                    seller,
                    status,
                    ..
                } if *buyer == club => {
                    use fs_world::transfer::OfferStatus as S;
                    let (title, body) = match status {
                        S::Countered { fee } => (
                            format!("{} want more for {}", club_name(*seller), player(*p)),
                            format!("They would sell for {}.", money(*fee)),
                        ),
                        S::Agreed => (
                            format!("Deal agreed for {}", player(*p)),
                            "The move completes tomorrow.".to_owned(),
                        ),
                        S::Refused(_) => {
                            let reason = dto::OfferStatus::from(*status);
                            let why = match reason {
                                dto::OfferStatus::Refused { reason } => reason,
                                _ => String::new(),
                            };
                            (format!("Bid for {} failed", player(*p)), format!("{why}."))
                        }
                        _ => continue,
                    };
                    self.push(
                        world,
                        Category::Transfer,
                        title,
                        body,
                        Some(Link::Transfers),
                    );
                }
                Event::TransferCompleted {
                    player: p,
                    from,
                    to,
                    fee,
                    loan,
                } if *from == club || *to == club => {
                    let title = if *to == club {
                        format!("{} joins", player(*p))
                    } else {
                        format!("{} leaves", player(*p))
                    };
                    let how = if *loan {
                        "on loan".to_owned()
                    } else {
                        format!("for {}", money(*fee))
                    };
                    self.push(
                        world,
                        Category::Transfer,
                        title,
                        format!(
                            "{} moves from {} to {} {how}.",
                            player(*p),
                            club_name(*from),
                            club_name(*to)
                        ),
                        Some(Link::Player { player: p.raw() }),
                    );
                }
                Event::LoanEnded {
                    player: p,
                    from,
                    to,
                } if *from == club || *to == club => {
                    self.push(
                        world,
                        Category::Transfer,
                        format!("{}'s loan is over", player(*p)),
                        format!("{} returns to {}.", player(*p), club_name(*to)),
                        Some(Link::Player { player: p.raw() }),
                    );
                }
                Event::ScoutReport { club: c, player: p } if *c == club => self.push(
                    world,
                    Category::Scouting,
                    format!("Scout report: {}", player(*p)),
                    "The scouts have finished assessing the player.".to_owned(),
                    Some(Link::Player { player: p.raw() }),
                ),
                Event::StaffLeft { staff, club: c } if *c == club => self.push(
                    world,
                    Category::Staff,
                    format!("{} has left the staff", staff_name(*staff)),
                    "Hire a replacement from the staff screen.".to_owned(),
                    Some(Link::Staff),
                ),
                Event::StaffContractExpiring {
                    staff,
                    club: c,
                    expires,
                } if *c == club => self.push(
                    world,
                    Category::Staff,
                    format!("{}'s contract is ending", staff_name(*staff)),
                    format!(
                        "The contract runs out on {}. Renew it from the staff screen.",
                        date(*expires)
                    ),
                    Some(Link::Staff),
                ),
                Event::BoardWarning { club: c, .. } if *c == club => self.push(
                    world,
                    Category::Board,
                    "The board is concerned".to_owned(),
                    "Results are below what the board expects. Things need to improve.".to_owned(),
                    None,
                ),
                Event::ManagerSacked {
                    manager: m,
                    club: c,
                } if *m == manager => self.push(
                    world,
                    Category::Board,
                    format!("Sacked by {}", club_name(*c)),
                    "The board has lost confidence in you. Look for a new job.".to_owned(),
                    Some(Link::Jobs),
                ),
                Event::ManagerSacked { club: c, .. }
                    if my_league.is_some() && world.clubs()[*c].league() == my_league =>
                {
                    self.push(
                        world,
                        Category::Club,
                        format!("{} sack their manager", club_name(*c)),
                        format!("{} are looking for a new manager.", club_name(*c)),
                        None,
                    );
                }
                Event::ManagerLeft {
                    manager: m,
                    club: c,
                } if *m == manager => self.push(
                    world,
                    Category::Board,
                    format!("You have left {}", club_name(*c)),
                    "Look for a new job.".to_owned(),
                    Some(Link::Jobs),
                ),
                _ => {}
            }
        }
    }
}

/// Money for people: `£1.2m`, `£450k`, `£800`.
pub fn money(m: i64) -> String {
    let a = m.abs() as f64;
    let sign = if m < 0 { "-" } else { "" };
    if a >= 1e6 {
        format!("{sign}£{:.1}m", a / 1e6)
    } else if a >= 1e3 {
        format!("{sign}£{:.0}k", a / 1e3)
    } else {
        format!("{sign}£{a:.0}")
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
