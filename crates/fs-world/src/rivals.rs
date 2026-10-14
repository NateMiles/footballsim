//! Rival managers: what AI managers say about human managers' clubs
//! between matches, and the feuds that grow out of it.
//!
//! An AI manager speaks when there is a reason to: to answer something a
//! human manager said about them, the day after the two clubs met, when
//! the clubs are close at the top of the table, or to rub it in when the
//! human's club is on a bad run. What is said makes the news and gives the
//! press something to ask about. Each pair of managers has a heat, from
//! respect (negative) to a feud: barbs raise it, respect cools it, and it
//! fades by itself over weeks. A heated rivalry makes comments likelier
//! and sharper, and turns the next meeting into a grudge match.
//!
//! No AI manager talks to another; this is about human managers' clubs.

use crate::command::Event;
use crate::ids::{ClubId, ManagerId};
use crate::news::{self, StoryKind};
use crate::press::{Quote, Tone, Topic};
use crate::world::World;
use fs_core::{Rng, Stream};
use serde::{Deserialize, Serialize};

/// Why a rival manager spoke.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Angle {
    /// Answering what the human manager said.
    Reply,
    /// The day after the clubs met: the rival's goals first.
    AfterMeeting { goals: [u8; 2] },
    /// Close at the top of the table.
    Race,
    /// The human manager's club is losing.
    Slump,
}

/// Heat at which a rivalry is a feud.
pub const FEUD: i16 = 40;

/// The heat between a human manager and a rival manager.
pub fn heat(world: &World, human: ManagerId, rival: ManagerId) -> i16 {
    world.press.heat.get(&(human, rival)).copied().unwrap_or(0)
}

/// Changes the heat between a human manager and the manager of `club`.
pub(crate) fn warm(world: &mut World, human: ManagerId, club: ClubId, by: i16) {
    let Some(rival) = world.clubs()[club].manager else {
        return;
    };
    if rival == human || by == 0 {
        return;
    }
    let h = world.press.heat.entry((human, rival)).or_insert(0);
    *h = (*h + by).clamp(-50, 100);
    if *h == 0 {
        world.press.heat.remove(&(human, rival));
    }
}

/// The rival managers a human manager has history with, hottest first.
pub fn rivalries(world: &World, human: ManagerId) -> Vec<(ManagerId, i16)> {
    let mut v: Vec<(ManagerId, i16)> = world
        .press
        .heat
        .iter()
        .filter(|((h, _), _)| *h == human)
        .map(|(&(_, r), &heat)| (r, heat))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// Every morning: heat fades a little every third day, and AI managers
/// may have something to say about human managers' clubs.
pub(crate) fn daily(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    if today.days() % 3 == 0 {
        world.press.heat.retain(|_, h| {
            *h -= h.signum();
            *h != 0
        });
    }
    let humans: Vec<(ClubId, ManagerId)> = world
        .clubs()
        .iter()
        .filter_map(|(c, club)| club.manager.map(|m| (c, m)))
        .filter(|&(c, _)| world.is_human_club(c))
        .collect();
    for (club, manager) in humans {
        let mut rng = world.seed().derive(
            Stream::Media,
            u64::from(today.days()),
            u64::from(club.raw()) | 1 << 33,
        );
        let said = reply(world, &mut rng, club, manager)
            .or_else(|| after_meeting(world, &mut rng, club, manager))
            .or_else(|| race(world, &mut rng, club, manager))
            .or_else(|| slump(world, &mut rng, club, manager));
        if let Some((rival_club, angle, tone)) = said {
            say(world, club, manager, rival_club, angle, tone, events);
        }
    }
}

fn rival_manager(world: &World, club: ClubId) -> Option<ManagerId> {
    let m = world.clubs()[club].manager?;
    (!world.is_human_club(club)).then_some(m)
}

fn standing(world: &World, club: ClubId) -> i32 {
    i32::from(world.clubs()[club].reputation)
}

/// Answering a dig the human manager made yesterday.
fn reply(
    world: &World,
    rng: &mut Rng,
    club: ClubId,
    manager: ManagerId,
) -> Option<(ClubId, Angle, Tone)> {
    let target = world.news().stories().iter().rev().find_map(|s| {
        if world.date().days_since(s.date) != 1 {
            return None;
        }
        let StoryKind::Quote {
            manager: m, quote, ..
        } = s.kind
        else {
            return None;
        };
        if m != manager || quote.tone != Tone::Dismissive {
            return None;
        }
        match quote.topic {
            Topic::Preview { opponent }
            | Topic::RivalManager { opponent, .. }
            | Topic::Grudge { opponent }
            | Topic::RivalComment {
                rival: opponent, ..
            } => Some(opponent),
            _ => None,
        }
    })?;
    let rival = rival_manager(world, target)?;
    if !rng.chance(0.75) {
        return None;
    }
    let tone =
        if heat(world, manager, rival) >= 30 || standing(world, target) > standing(world, club) {
            Tone::Dismissive
        } else {
            Tone::Defiant
        };
    Some((target, Angle::Reply, tone))
}

/// The day after the clubs met.
fn after_meeting(
    world: &World,
    rng: &mut Rng,
    club: ClubId,
    manager: ManagerId,
) -> Option<(ClubId, Angle, Tone)> {
    let (opponent, ours, theirs) = world.news().stories().iter().rev().find_map(|s| {
        if world.date().days_since(s.date) != 1 {
            return None;
        }
        match s.kind {
            StoryKind::Result {
                home, away, goals, ..
            } if home == club => Some((away, goals[0], goals[1])),
            StoryKind::Result {
                home, away, goals, ..
            } if away == club => Some((home, goals[1], goals[0])),
            _ => None,
        }
    })?;
    let rival = rival_manager(world, opponent)?;
    let h = heat(world, manager, rival);
    if !rng.chance(0.35 + f64::from(h.max(0)) / 200.0) {
        return None;
    }
    let tone = if theirs > ours {
        if h >= 30 {
            Tone::Dismissive
        } else {
            Tone::Confident
        }
    } else if theirs == ours {
        Tone::Confident
    } else if ours - theirs == 1 {
        Tone::Defiant
    } else {
        Tone::Praise
    };
    Some((
        opponent,
        Angle::AfterMeeting {
            goals: [theirs, ours],
        },
        tone,
    ))
}

/// Close at the top: on Mondays, the nearest challenger has a word.
fn race(
    world: &World,
    rng: &mut Rng,
    club: ClubId,
    _manager: ManagerId,
) -> Option<(ClubId, Angle, Tone)> {
    if world.date().weekday() != fs_core::date::Weekday::Monday {
        return None;
    }
    let league = world.clubs()[club].league()?;
    let table = world.leagues()[league].season()?.table();
    let pos = table.iter().position(|r| r.club == club)?;
    if pos > 2 || table[pos].played < 8 {
        return None;
    }
    let (other, above) = table
        .iter()
        .enumerate()
        .take(4)
        .filter(|(i, r)| *i != pos && rival_manager(world, r.club).is_some())
        .min_by_key(|(i, _)| i.abs_diff(pos))
        .map(|(i, r)| (r.club, i < pos))?;
    if !rng.chance(0.35) {
        return None;
    }
    let tone = if above {
        Tone::Confident
    } else {
        Tone::Dismissive
    };
    Some((other, Angle::Race, tone))
}

/// Three defeats in four: a rival with a grudge rubs it in.
fn slump(
    world: &World,
    rng: &mut Rng,
    club: ClubId,
    manager: ManagerId,
) -> Option<(ClubId, Angle, Tone)> {
    if world.date().weekday() != fs_core::date::Weekday::Thursday {
        return None;
    }
    if losses_in_last(world, club, 4) < 3 {
        return None;
    }
    let (rival, h) = rivalries(world, manager).into_iter().next()?;
    if h < 20 {
        return None;
    }
    let rival_club = world.managers()[rival].club?;
    if world.is_human_club(rival_club) || !rng.chance(0.5) {
        return None;
    }
    Some((rival_club, Angle::Slump, Tone::Dismissive))
}

/// League defeats in the club's last `n` league matches.
fn losses_in_last(world: &World, club: ClubId, n: usize) -> usize {
    let Some(season) = world.clubs()[club]
        .league()
        .and_then(|l| world.leagues()[l].season())
    else {
        return 0;
    };
    season
        .fixtures
        .iter()
        .rev()
        .filter_map(|f| {
            let r = f.result.as_ref()?;
            if f.home == club {
                Some(r.goals[0] < r.goals[1])
            } else if f.away == club {
                Some(r.goals[1] < r.goals[0])
            } else {
                None
            }
        })
        .take(n)
        .filter(|&lost| lost)
        .count()
}

/// Files what the rival said, and what it does: a barb fires the human
/// manager's players up and heats the rivalry; respect cools it.
fn say(
    world: &mut World,
    club: ClubId,
    manager: ManagerId,
    rival_club: ClubId,
    angle: Angle,
    tone: Tone,
    events: &mut Vec<Event>,
) {
    let Some(rival) = rival_manager(world, rival_club) else {
        return;
    };
    let Some(story) = news::report(
        world,
        StoryKind::Quote {
            manager: rival,
            club: rival_club,
            quote: Quote {
                topic: Topic::AboutRival { rival: club, angle },
                tone,
            },
        },
    ) else {
        return;
    };
    let by = match tone {
        Tone::Dismissive => 10,
        Tone::Confident | Tone::Defiant => 4,
        Tone::Praise | Tone::Humble => -5,
        _ => 0,
    };
    warm(world, manager, rival_club, by);
    if tone == Tone::Dismissive {
        crate::press::squad_morale(world, club, 10);
    }
    events.push(Event::RivalSpoke {
        manager,
        club,
        rival,
        story,
    });
}

/// A rival's words about a club in the last few days, newest first: what
/// the press will ask about.
pub fn recent_comment(world: &World, club: ClubId, days: i64) -> Option<(ClubId, Tone, Angle)> {
    let today = world.date();
    world.news().stories().iter().rev().find_map(|s| {
        if today.days_since(s.date) > days {
            return None;
        }
        match s.kind {
            StoryKind::Quote {
                club: rival_club,
                quote:
                    Quote {
                        topic: Topic::AboutRival { rival, angle },
                        tone,
                    },
                ..
            } if rival == club => Some((rival_club, tone, angle)),
            _ => None,
        }
    })
}
