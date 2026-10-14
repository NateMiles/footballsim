//! The news: what the football world is talking about.
//!
//! Stories are facts with a weight (how newsworthy they are), reported
//! from where things happen: results, transfers, sackings, trophies,
//! injuries, retirements, youth intakes, unrest and rumours. Only stories
//! that matter beyond their own town are kept, plus everything about human
//! managers' clubs. The words are written by whoever presents them (the
//! API), so a story stays small and could be told in any language.
//!
//! Nothing here is random and nothing here changes the world: news only
//! reports it.

use crate::fixture::FixtureRef;
use crate::ids::{ClubId, CupId, LeagueId, ManagerId, PlayerId};
use crate::world::World;
use fs_core::GameDate;
use serde::{Deserialize, Serialize};

/// Why a result made the news.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Angle {
    /// The much smaller club won.
    Upset,
    /// Won by four or more.
    Thrashing,
    /// The top two met.
    Summit,
    /// Seven goals or more.
    GoalFest,
    /// A human manager's club played; no other angle.
    Report,
}

/// What a story is about.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryKind {
    Result {
        fixture: FixtureRef,
        home: ClubId,
        away: ClubId,
        goals: [u8; 2],
        angle: Angle,
    },
    HatTrick {
        player: PlayerId,
        club: ClubId,
        against: ClubId,
        goals: u8,
    },
    Transfer {
        player: PlayerId,
        from: Option<ClubId>,
        to: ClubId,
        fee: i64,
        loan: bool,
        /// The highest fee the world has seen.
        record: bool,
    },
    ManagerSacked {
        manager: ManagerId,
        club: ClubId,
    },
    ManagerAppointed {
        manager: ManagerId,
        club: ClubId,
    },
    /// The board's patience is running out.
    UnderPressure {
        manager: ManagerId,
        club: ClubId,
    },
    CupWon {
        cup: CupId,
        club: ClubId,
    },
    Champions {
        league: LeagueId,
        club: ClubId,
    },
    Promoted {
        club: ClubId,
        to: LeagueId,
    },
    Relegated {
        club: ClubId,
        to: LeagueId,
    },
    LongInjury {
        player: PlayerId,
        club: ClubId,
        days: u16,
    },
    TransferRequest {
        player: PlayerId,
        club: ClubId,
    },
    Retired {
        player: PlayerId,
        club: Option<ClubId>,
    },
    /// An outstanding youngster from a youth intake.
    Wonderkid {
        player: PlayerId,
        club: ClubId,
    },
    /// A club has made an offer for a player.
    Rumour {
        player: PlayerId,
        club: ClubId,
        owner: ClubId,
    },
    /// A manager said something at a press conference.
    Quote {
        manager: ManagerId,
        club: ClubId,
        quote: crate::press::Quote,
    },
}

/// A story, as filed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Story {
    pub id: u32,
    pub date: GameDate,
    pub kind: StoryKind,
    /// How newsworthy it is, 0-100.
    pub weight: u8,
}

/// Every story filed in the last few months.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Newsroom {
    /// Oldest first.
    stories: Vec<Story>,
    next: u32,
    /// The highest transfer fee so far.
    record_fee: i64,
}

impl Newsroom {
    /// Stories, oldest first.
    pub fn stories(&self) -> &[Story] {
        &self.stories
    }

    pub fn get(&self, id: u32) -> Option<&Story> {
        self.stories
            .binary_search_by_key(&id, |s| s.id)
            .ok()
            .map(|i| &self.stories[i])
    }
}

/// Days a story is kept.
pub const KEEP_DAYS: i64 = 120;
/// Stories below this weight are not kept, unless they concern a human
/// manager's club.
pub const MIN_WEIGHT: u32 = 45;

/// How much a competition matters to the news, 0-60.
fn league_weight(world: &World, league: LeagueId) -> u32 {
    match world.leagues()[league].level {
        1 => 50,
        2 => 30,
        3 => 18,
        _ => 10,
    }
}

fn cup_weight(world: &World, cup: CupId) -> u32 {
    u32::from(world.cups()[cup].format.prestige) / 2 + 10
}

fn fixture_weight(world: &World, f: FixtureRef) -> u32 {
    match f {
        FixtureRef::League { league, .. } => league_weight(world, league),
        FixtureRef::Cup { cup, .. } => cup_weight(world, cup),
    }
}

/// A club's standing, 0-50.
fn club_weight(world: &World, club: ClubId) -> u32 {
    u32::from(world.clubs()[club].reputation) / 200
}

/// A player's standing, 0-50.
fn player_weight(world: &World, player: PlayerId) -> u32 {
    let ability = u32::from(world.players().ability(player));
    ability.saturating_sub(450) / 9
}

/// The clubs a story is about.
pub fn clubs_of(kind: &StoryKind) -> [Option<ClubId>; 2] {
    match *kind {
        StoryKind::Result { home, away, .. } => [Some(home), Some(away)],
        StoryKind::HatTrick { club, against, .. } => [Some(club), Some(against)],
        StoryKind::Transfer { from, to, .. } => [Some(to), from],
        StoryKind::Rumour { club, owner, .. } => [Some(owner), Some(club)],
        StoryKind::Retired { club, .. } => [club, None],
        StoryKind::ManagerSacked { club, .. }
        | StoryKind::ManagerAppointed { club, .. }
        | StoryKind::UnderPressure { club, .. }
        | StoryKind::CupWon { club, .. }
        | StoryKind::Champions { club, .. }
        | StoryKind::Promoted { club, .. }
        | StoryKind::Relegated { club, .. }
        | StoryKind::LongInjury { club, .. }
        | StoryKind::TransferRequest { club, .. }
        | StoryKind::Wonderkid { club, .. }
        | StoryKind::Quote { club, .. } => [Some(club), None],
    }
}

/// How newsworthy a story is, 0-100.
fn weigh(world: &World, kind: &StoryKind) -> u32 {
    let clubs = clubs_of(kind)
        .iter()
        .flatten()
        .map(|&c| club_weight(world, c))
        .max()
        .unwrap_or(0);
    let w = match *kind {
        StoryKind::Result { fixture, angle, .. } => {
            fixture_weight(world, fixture) / 2
                + clubs / 2
                + match angle {
                    Angle::Upset => 22,
                    Angle::Summit => 20,
                    Angle::Thrashing => 8,
                    Angle::GoalFest => 6,
                    Angle::Report => 0,
                }
        }
        StoryKind::HatTrick {
            player, against, ..
        } => {
            let league = world.clubs()[against]
                .league()
                .map_or(10, |l| league_weight(world, l));
            6 + league / 2 + player_weight(world, player) / 2
        }
        StoryKind::Transfer {
            player,
            fee,
            loan,
            record,
            ..
        } => {
            let money = (fee / 333_333).clamp(0, 60) as u32;
            let base = if loan { money / 3 } else { money };
            base + clubs / 2 + player_weight(world, player) / 3 + if record { 40 } else { 0 }
        }
        StoryKind::Rumour { player, .. } => 5 + clubs / 2 + player_weight(world, player) / 2,
        StoryKind::ManagerSacked { .. } => 25 + clubs,
        StoryKind::ManagerAppointed { .. } => 18 + clubs,
        StoryKind::UnderPressure { .. } => 15 + clubs,
        StoryKind::CupWon { cup, .. } => 30 + cup_weight(world, cup),
        StoryKind::Champions { league, .. } => 35 + league_weight(world, league),
        StoryKind::Promoted { to, .. } | StoryKind::Relegated { to, .. } => {
            15 + league_weight(world, to) / 2 + clubs / 2
        }
        StoryKind::LongInjury { player, days, .. } => {
            (u32::from(days / 30) + player_weight(world, player) + clubs / 3).saturating_sub(5)
        }
        StoryKind::TransferRequest { player, .. } => 10 + player_weight(world, player) + clubs / 3,
        StoryKind::Retired { player, .. } => 10 + player_weight(world, player),
        StoryKind::Wonderkid { player, .. } => {
            let potential = u32::from(world.players().potential(player));
            potential.saturating_sub(700) / 3 + clubs / 3
        }
        StoryKind::Quote { .. } => 30 + clubs,
    };
    w.min(100)
}

fn involves_human(world: &World, kind: &StoryKind) -> bool {
    clubs_of(kind)
        .iter()
        .flatten()
        .any(|&c| world.is_human_club(c))
}

/// Files a story if it is newsworthy enough. Returns its id if kept.
pub fn report(world: &mut World, kind: StoryKind) -> Option<u32> {
    let weight = weigh(world, &kind);
    if weight < MIN_WEIGHT && !involves_human(world, &kind) {
        return None;
    }
    let today = world.date();
    let room = &mut world.news;
    let id = room.next;
    room.next += 1;
    room.stories.push(Story {
        id,
        date: today,
        kind,
        weight: weight as u8,
    });
    Some(id)
}

/// A match result, reported if it has an angle (or concerns a human
/// manager's club), and any hat-tricks in it.
pub(crate) fn result(
    world: &mut World,
    fixture: FixtureRef,
    home: ClubId,
    away: ClubId,
    goals: [u8; 2],
    scorers: &[(u8, PlayerId)],
) {
    let rep = |c: ClubId| i32::from(world.clubs()[c].reputation);
    let (winner, loser) = match goals[0].cmp(&goals[1]) {
        std::cmp::Ordering::Greater => (Some(home), Some(away)),
        std::cmp::Ordering::Less => (Some(away), Some(home)),
        std::cmp::Ordering::Equal => (None, None),
    };
    let margin = goals[0].abs_diff(goals[1]);
    let summit = match fixture {
        FixtureRef::League { league, .. } => world.leagues()[league].season().is_some_and(|s| {
            let table = s.table();
            table.len() > 2
                && table.iter().take(2).any(|r| r.club == home)
                && table.iter().take(2).any(|r| r.club == away)
                && table[0].played >= 5
        }),
        FixtureRef::Cup { .. } => false,
    };
    let angle = if let (Some(w), Some(l)) = (winner, loser)
        && rep(l) - rep(w) >= 1_800
    {
        Some(Angle::Upset)
    } else if summit {
        Some(Angle::Summit)
    } else if margin >= 4 {
        Some(Angle::Thrashing)
    } else if goals[0] + goals[1] >= 7 {
        Some(Angle::GoalFest)
    } else if world.is_human_club(home) || world.is_human_club(away) {
        Some(Angle::Report)
    } else {
        None
    };
    if let Some(angle) = angle {
        report(
            world,
            StoryKind::Result {
                fixture,
                home,
                away,
                goals,
                angle,
            },
        );
    }
    // Hat-tricks: scorers are listed one goal at a time.
    let mut counted: Vec<(u8, PlayerId, u8)> = Vec::new();
    for &(side, p) in scorers {
        match counted.iter_mut().find(|c| c.1 == p) {
            Some(c) => c.2 += 1,
            None => counted.push((side, p, 1)),
        }
    }
    for (side, player, n) in counted {
        if n >= 3 {
            let (club, against) = if side == 0 {
                (home, away)
            } else {
                (away, home)
            };
            report(
                world,
                StoryKind::HatTrick {
                    player,
                    club,
                    against,
                    goals: n,
                },
            );
        }
    }
}

/// A completed transfer or loan.
pub(crate) fn transfer(
    world: &mut World,
    player: PlayerId,
    from: Option<ClubId>,
    to: ClubId,
    fee: i64,
    loan: bool,
) {
    let record = !loan && fee > world.news.record_fee && world.news.record_fee > 0;
    if !loan && fee > world.news.record_fee {
        world.news.record_fee = fee;
    }
    report(
        world,
        StoryKind::Transfer {
            player,
            from,
            to,
            fee,
            loan,
            record,
        },
    );
}

/// Every afternoon: old stories go, and today's long injuries to players
/// worth a story are reported.
pub(crate) fn daily(world: &mut World) {
    let today = world.date();
    let room = &mut world.news;
    let cut = room
        .stories
        .partition_point(|s| today.days_since(s.date) > KEEP_DAYS);
    room.stories.drain(..cut);
    let mut found = Vec::new();
    for (i, inj) in world.players.injury.iter().enumerate() {
        let Some(inj) = inj else { continue };
        if inj.since != today {
            continue;
        }
        let days = inj.until.days_since(today);
        if days < 42 {
            continue;
        }
        let p = PlayerId::new(i as u32);
        if let Some(club) = world.players().club(p) {
            found.push((p, club, days.min(i64::from(u16::MAX)) as u16));
        }
    }
    for (player, club, days) in found {
        report(world, StoryKind::LongInjury { player, club, days });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clubs_of_names_both_sides_of_a_transfer() {
        let kind = StoryKind::Transfer {
            player: PlayerId::new(1),
            from: Some(ClubId::new(2)),
            to: ClubId::new(3),
            fee: 1,
            loan: false,
            record: false,
        };
        assert_eq!(
            clubs_of(&kind),
            [Some(ClubId::new(3)), Some(ClubId::new(2))]
        );
    }
}
