//! Private conversations between a manager and a player, and the promises
//! that come out of them.
//!
//! How a player takes a talk depends on what is true (their form, their
//! playing time, their contract), on their personality, and on whether the
//! manager has broken promises to them before. A promise is checked when it
//! falls due; kept promises build goodwill and broken ones are remembered
//! for months.

use crate::command::{CommandError, Event};
use crate::happiness::{self, Feeling, SquadStatus};
use crate::ids::{ClubId, PlayerId};
use crate::personality::trait_of;
use crate::player::CONDITION_MAX;
use crate::world::World;
use fs_core::attributes::HiddenAttr;
use fs_core::{GameDate, Stream};
use serde::{Deserialize, Serialize};

/// What the manager says.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Talk {
    /// Praise their form.
    Praise,
    /// Criticise their form.
    Criticise,
    /// Tell them they are an important part of the plans.
    Reassure,
    /// Promise more first-team football.
    PromiseFootball,
    /// Promise an improved contract.
    PromiseContract,
}

impl Talk {
    pub const ALL: [Talk; 5] = [
        Talk::Praise,
        Talk::Criticise,
        Talk::Reassure,
        Talk::PromiseFootball,
        Talk::PromiseContract,
    ];
}

/// How the player took it.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reaction {
    Delighted,
    Pleased,
    /// Took criticism as a challenge.
    Determined,
    Indifferent,
    /// Did not believe it.
    Sceptical,
    Annoyed,
    Angry,
}

/// What was promised.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromiseKind {
    /// At least this share of the club's minutes (0-10000), give or take.
    Football { share: u16 },
    /// A better contract than this one.
    Contract { expires: GameDate, wage: u32 },
}

/// A promise made to a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Promise {
    pub player: PlayerId,
    pub club: ClubId,
    pub kind: PromiseKind,
    pub made: GameDate,
    pub due: GameDate,
}

/// Days between talks with the same player.
pub const TALK_INTERVAL: i64 = 7;
/// Days a manager has to keep a promise.
pub const PROMISE_DAYS: u32 = 60;

/// The player's form this season.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Good,
    Ordinary,
    Poor,
}

fn form(world: &World, player: PlayerId) -> Form {
    let s = world.players().season_stats(player);
    match s.average_rating() {
        Some(r) if s.appearances >= 3 && r >= 7.0 => Form::Good,
        Some(r) if s.appearances >= 3 && r < 6.5 => Form::Poor,
        _ => Form::Ordinary,
    }
}

fn underplayed(world: &World, player: PlayerId) -> bool {
    let ps = world.players();
    u32::from(ps.playing_time(player)) * 10 < u32::from(ps.squad_status(player).expected()) * 8
}

fn shift_morale(world: &mut World, player: PlayerId, by: i32) {
    let c = &mut world.players.condition[player.index()];
    c.morale = (i32::from(c.morale) + by).clamp(0, i32::from(CONDITION_MAX)) as u16;
}

/// Validates and applies a talk. The caller has checked the player is at
/// `club`.
pub(crate) fn talk(
    world: &mut World,
    club: ClubId,
    player: PlayerId,
    talk: Talk,
) -> Result<Vec<Event>, CommandError> {
    let today = world.date();
    if let Some(last) = world.relations.last_talk(player)
        && today.days_since(last) < TALK_INTERVAL
    {
        return Err(CommandError::TalkedRecently(player));
    }
    let promised = |w: &World, football: bool| {
        w.relations.promises.iter().any(|p| {
            p.player == player && matches!(p.kind, PromiseKind::Football { .. }) == football
        })
    };
    match talk {
        Talk::PromiseFootball if promised(world, true) => {
            return Err(CommandError::AlreadyPromised(player));
        }
        Talk::PromiseContract if promised(world, false) => {
            return Err(CommandError::AlreadyPromised(player));
        }
        _ => {}
    }

    let mut rng = world.seed().derive(
        Stream::People,
        u64::from(today.days()),
        u64::from(player.raw()) | 1 << 40,
    );
    let professionalism = trait_of(world, player, HiddenAttr::Professionalism);
    let temperament = trait_of(world, player, HiddenAttr::Temperament);
    let volatile = temperament < 0.3;
    let distrust = world.relations.remembers(player, Feeling::PromiseBroken);
    let form = form(world, player);

    let reaction = match talk {
        Talk::Praise => {
            if world.relations.remembers(player, Feeling::Praised) {
                Reaction::Indifferent
            } else {
                match form {
                    Form::Good => {
                        let delighted = rng.chance(0.3 + 0.4 * (1.0 - temperament));
                        if delighted {
                            happiness::remember(world, player, Feeling::Praised, 10, 45);
                            shift_morale(world, player, 1_000);
                            Reaction::Delighted
                        } else {
                            happiness::remember(world, player, Feeling::Praised, 6, 45);
                            shift_morale(world, player, 600);
                            Reaction::Pleased
                        }
                    }
                    Form::Ordinary if rng.chance(0.5) => {
                        happiness::remember(world, player, Feeling::Praised, 4, 30);
                        shift_morale(world, player, 300);
                        Reaction::Pleased
                    }
                    Form::Ordinary => Reaction::Indifferent,
                    Form::Poor => Reaction::Sceptical,
                }
            }
        }
        Talk::Criticise => {
            let well = match form {
                Form::Good => 0.0,
                Form::Poor => 0.25 + 0.55 * professionalism + 0.2 * temperament,
                Form::Ordinary => 0.1 + 0.5 * professionalism,
            };
            if form == Form::Good {
                let hurt = if volatile { -20 } else { -15 };
                happiness::remember(world, player, Feeling::UnfairCriticism, hurt, 90);
                shift_morale(world, player, -1_200);
                Reaction::Angry
            } else if rng.chance(well.min(0.95)) {
                happiness::remember(world, player, Feeling::FiredUp, 5, 60);
                shift_morale(world, player, 300);
                Reaction::Determined
            } else {
                let hurt = if form == Form::Poor { -8 } else { -6 };
                happiness::remember(world, player, Feeling::Criticised, hurt, 60);
                shift_morale(world, player, -600);
                Reaction::Annoyed
            }
        }
        Talk::Reassure => {
            if distrust {
                Reaction::Sceptical
            } else if underplayed(world, player) {
                happiness::remember(world, player, Feeling::Doubted, -4, 60);
                Reaction::Sceptical
            } else {
                let important = matches!(
                    world.players().squad_status(player),
                    SquadStatus::Key | SquadStatus::Regular
                );
                happiness::remember(
                    world,
                    player,
                    Feeling::Reassured,
                    if important { 5 } else { 3 },
                    60,
                );
                shift_morale(world, player, 300);
                Reaction::Pleased
            }
        }
        Talk::PromiseFootball => {
            let status = world.players().squad_status(player);
            let target = if underplayed(world, player) {
                status
            } else {
                status.above()
            };
            if distrust {
                Reaction::Sceptical
            } else if !underplayed(world, player) && status == SquadStatus::Key {
                Reaction::Indifferent
            } else {
                promise(
                    world,
                    club,
                    player,
                    PromiseKind::Football {
                        share: target.expected(),
                    },
                    today.add_days(PROMISE_DAYS),
                );
                shift_morale(world, player, 500);
                Reaction::Pleased
            }
        }
        Talk::PromiseContract => {
            let ps = world.players();
            let contract = *ps
                .contract(player)
                .expect("players at clubs have contracts");
            let age = crate::people::age_years(ps.bio(player).birth_date, today) as u32;
            let demand = crate::people::wage_demand(ps.ability(player), ps.potential(player), age);
            let content =
                contract.expires.days_since(today) > 730 && contract.wage >= demand * 95 / 100;
            if distrust {
                Reaction::Sceptical
            } else if content {
                Reaction::Indifferent
            } else {
                let due = today
                    .add_days(PROMISE_DAYS)
                    .min(contract.expires.max(today.add_days(14)));
                promise(
                    world,
                    club,
                    player,
                    PromiseKind::Contract {
                        expires: contract.expires,
                        wage: contract.wage,
                    },
                    due,
                );
                shift_morale(world, player, 500);
                Reaction::Pleased
            }
        }
    };
    world.relations.talks.insert(player, today);
    happiness::refresh(world, player);
    Ok(vec![Event::PlayerReacted {
        player,
        club,
        talk,
        reaction,
    }])
}

fn promise(world: &mut World, club: ClubId, player: PlayerId, kind: PromiseKind, due: GameDate) {
    let today = world.date();
    world.relations.promises.push(Promise {
        player,
        club,
        kind,
        made: today,
        due,
    });
    let days = due.days_since(today).max(1) as u32;
    happiness::remember(world, player, Feeling::PromiseMade, 8, days);
}

/// Whether a promise has been kept, as of today.
pub fn kept(world: &World, p: &Promise) -> bool {
    let ps = world.players();
    match p.kind {
        PromiseKind::Football { share } => {
            u32::from(ps.playing_time(p.player)) * 100 >= u32::from(share) * 85
        }
        PromiseKind::Contract { expires, wage } => ps
            .contract(p.player)
            .is_some_and(|c| c.expires > expires || c.wage > wage),
    }
}

/// Settles promises that fall due today. A contract promise is kept as
/// soon as the new contract is signed. A player injured when a football
/// promise falls due gets another month once fit.
pub(crate) fn settle(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let mut i = 0;
    while i < world.relations.promises.len() {
        let p = world.relations.promises[i];
        let early = matches!(p.kind, PromiseKind::Contract { .. }) && kept(world, &p);
        if p.due > today && !early {
            i += 1;
            continue;
        }
        if let PromiseKind::Football { .. } = p.kind
            && let Some(injury) = world.players().injury(p.player)
            && !kept(world, &p)
        {
            world.relations.promises[i].due = injury.until.add_days(30);
            i += 1;
            continue;
        }
        world.relations.promises.remove(i);
        let kept = kept(world, &p);
        happiness::forget(world, p.player, Feeling::PromiseMade);
        if kept {
            happiness::remember(world, p.player, Feeling::PromiseKept, 10, 120);
        } else {
            let volatile = trait_of(world, p.player, HiddenAttr::Temperament) < 0.3;
            let hurt = if volatile { -30 } else { -25 };
            happiness::remember(world, p.player, Feeling::PromiseBroken, hurt, 240);
        }
        happiness::refresh(world, p.player);
        if world.is_human_club(p.club) {
            events.push(Event::PromiseSettled {
                player: p.player,
                club: p.club,
                kept,
            });
        }
    }
}
