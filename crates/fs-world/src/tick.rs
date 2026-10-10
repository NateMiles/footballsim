//! The daily simulation step.
//!
//! Each day, in order: calendar events (season rollover on 1 July,
//! contract renewals on 1 June, expiry warnings on 1 January, transfer
//! windows), the day's matches, recovery, injuries, and on the 1st of the
//! month player development and AI tactic refreshes.
//!
//! Per-player passes run in parallel with per-player RNG streams, so the
//! result does not depend on thread count.

use crate::attributes::{ATTR_MAX, Attr, HiddenAttr};
use crate::command::Event;
use crate::economy;
use crate::ids::ClubId;
use crate::matchday::{play_today, refresh_ai_tactics, sample_injury};
use crate::people::{GrowthFactors, age_years, develop_month, season_year};
use crate::player::{CONDITION_MAX, Condition, Players, Status};
use crate::season::{current_season, rollover};
use crate::squad::{maintain_squads, renew_contracts, warn_expiring};
use crate::staff;
use crate::world::World;
use fs_core::Stream;
use fs_core::par;
use fs_match::TimedInstruction;
use std::collections::BTreeMap;

/// Morale drifts back toward this value.
const MORALE_BASELINE: u16 = CONDITION_MAX / 2;
/// Sharpness never decays below this without match play.
const SHARPNESS_FLOOR: u16 = 2_000;
/// Chance per day of a training injury for an average player.
const TRAINING_INJURY_RATE: f64 = 0.0008;

/// The first part of a day, before any match: calendar business.
/// `world.date` is already the new day.
pub(crate) fn morning(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date;
    let (_, month, day) = today.ymd();
    if world.leagues.is_empty() {
        return;
    }
    if (month, day) == (7, 1) && current_season(world).is_some_and(|y| y < season_year(today)) {
        rollover(world, events);
    }
    match (month, day) {
        (6, 1) => {
            renew_contracts(world, events);
            staff::warn_expiring(world, events);
        }
        (1, 1) => warn_expiring(world, events),
        (8, 20) | (1, 10) => maintain_squads(world, events),
        _ => {}
    }
    crate::squad::emergency_signings(world, events);
}

/// The rest of the day: the matches (with any live instructions), then
/// recovery, injuries and monthly development.
pub(crate) fn afternoon(
    world: &mut World,
    events: &mut Vec<Event>,
    instructions: &BTreeMap<ClubId, Vec<TimedInstruction>>,
) {
    if !world.leagues.is_empty() {
        play_today(world, events, instructions);
    }
    recovery(world);
    injuries(world, events);
    crate::scouting::daily(world, events);
    if !world.leagues.is_empty() {
        crate::market::daily(world, events);
    }
    crate::board::fill_vacancies(world, events);
    if world.date.ymd().2 == 1 {
        economy::monthly_accounts(world);
        if !world.leagues.is_empty() {
            crate::board::monthly(world, events);
        }
        development(world);
        if !world.leagues.is_empty() {
            refresh_ai_tactics(world);
        }
    }
}

fn recovery(world: &mut World) {
    let seed = world.seed;
    let day = u64::from(world.date.days());
    let Players {
        condition,
        attributes,
        hidden,
        status,
        ..
    } = &mut world.players;
    par::for_each_indexed_mut(condition, |i, c| {
        if status[i] != Status::Active {
            return;
        }
        let mut rng = seed.derive(Stream::DailyTick, day, i as u64);
        let natural_fitness = u32::from(attributes[i].get(Attr::NaturalFitness));
        let professionalism = u32::from(hidden[i].get(HiddenAttr::Professionalism));
        recover(c, natural_fitness, professionalism, &mut rng);
    });
}

/// Heals injuries that have run their course and causes new training
/// injuries, more often for injury-prone or overloaded players.
fn injuries(world: &mut World, events: &mut Vec<Event>) {
    let seed = world.seed;
    let today = world.date;
    let day = u64::from(today.days());
    // Medical teams: (prevention, treatment) per club.
    let medical: Vec<(f64, f64)> = world
        .clubs
        .ids()
        .map(|c| {
            let r = staff::ratings(world, c);
            (staff::prevention_factor(&r), staff::treatment_factor(&r))
        })
        .collect();
    let Players {
        injury,
        condition,
        hidden,
        status,
        club,
        ..
    } = &mut world.players;
    par::for_each_indexed_mut(injury, |i, inj| {
        if inj.is_some_and(|x| x.until <= today) {
            *inj = None;
        }
        if inj.is_some() || status[i] != Status::Active {
            return;
        }
        let Some(c) = club[i] else { return };
        let (prevention, treatment) = medical[c.index()];
        let mut rng = seed.derive(Stream::Injury, day, i as u64);
        let proneness = f64::from(hidden[i].get(HiddenAttr::InjuryProneness)) / 1000.0;
        let load = 1.0 + f64::from(condition[i].fatigue) / 5000.0;
        if rng.chance(TRAINING_INJURY_RATE * (0.5 + proneness) * load * prevention) {
            *inj = Some(sample_injury(&mut rng, today, proneness * 0.5, treatment));
        }
    });
    for c in world.clubs.ids().filter(|&c| world.is_human_club(c)) {
        for &p in &world.clubs[c].squad {
            if let Some(inj) = world.players.injury(p)
                && inj.since == today
                && !events
                    .iter()
                    .any(|e| matches!(e, Event::PlayerInjured { player, .. } if *player == p))
            {
                events.push(Event::PlayerInjured {
                    player: p,
                    club: c,
                    until: inj.until,
                });
            }
        }
    }
}

/// Monthly growth, plateau and decline for every active player.
fn development(world: &mut World) {
    let seed = world.seed;
    let today = world.date;
    let month = u64::from(today.days());
    // Matches each club has played this season, for playing-time shares.
    let mut played = vec![0u32; world.clubs.len()];
    for (_, league) in world.leagues.iter() {
        if let Some(s) = league.season() {
            for row in &s.rows {
                played[row.club.index()] = u32::from(row.played);
            }
        }
    }
    // Coaching: (seniors, under-21s) per club.
    let coaching: Vec<(f64, f64)> = world
        .clubs
        .ids()
        .map(|c| {
            staff::coaching_factors(
                &staff::ratings(world, c),
                world.clubs[c].facilities.training,
            )
        })
        .collect();
    let Players {
        attributes,
        positions,
        potential,
        bio,
        hidden,
        status,
        club,
        season,
        ..
    } = &mut world.players;
    par::for_each_indexed_mut(attributes, |i, attrs| {
        if status[i] != Status::Active {
            return;
        }
        let mut rng = seed.derive(Stream::Development, month, i as u64);
        let age = age_years(bio[i].birth_date, today);
        let professionalism = f64::from(hidden[i].get(HiddenAttr::Professionalism)) / 1000.0;
        let playing = match club[i] {
            None => 0.2,
            Some(c) if played[c.index()] == 0 => 0.5,
            Some(c) => f64::from(season[i].minutes) / (90.0 * f64::from(played[c.index()])),
        };
        // Free agents train alone.
        let coached = club[i].map_or(0.85, |c| {
            let (senior, youth) = coaching[c.index()];
            if age < 21.0 { youth } else { senior }
        });
        develop_month(
            attrs,
            positions[i].best(),
            potential[i],
            age,
            GrowthFactors {
                professionalism,
                playing_time: playing,
                coaching: coached,
            },
            &mut rng,
        );
    });
}

/// One day of recovery for one player. Attribute inputs are 0-1000.
fn recover(c: &mut Condition, natural_fitness: u32, professionalism: u32, rng: &mut fs_core::Rng) {
    let max = u32::from(CONDITION_MAX);
    let attr_max = u32::from(ATTR_MAX);

    // Fitness closes 15% to 30% of the gap to full each day, faster with
    // better natural fitness, so a tired player is fresh within a week.
    let fitness = u32::from(c.fitness);
    let rate = 150 + natural_fitness * 150 / attr_max; // per mille
    let gain = (max - fitness) * rate / 1000 + rng.below(20);
    c.fitness = (fitness + gain).min(max) as u16;

    // Fatigue sheds 5% to 9% a day.
    let fatigue = u32::from(c.fatigue);
    let shed = fatigue * (50 + natural_fitness * 40 / attr_max) / 1000 + 1;
    c.fatigue = fatigue.saturating_sub(shed) as u16;

    // Sharpness fades slowly without games; professional players keep it
    // better.
    let sharpness = u32::from(c.sharpness);
    if sharpness > u32::from(SHARPNESS_FLOOR) {
        let loss = sharpness * (6 - professionalism * 3 / attr_max) / 1000;
        c.sharpness = sharpness
            .saturating_sub(loss)
            .max(u32::from(SHARPNESS_FLOOR)) as u16;
    }

    // Morale reverts 2% toward neutral each day, plus small noise.
    let morale = i32::from(c.morale);
    let baseline = i32::from(MORALE_BASELINE);
    let pull = (baseline - morale) * 2 / 100;
    let noise = rng.range_i32(-30, 30);
    c.morale = (morale + pull + noise).clamp(0, max as i32) as u16;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_core::Rng;

    #[test]
    fn tired_player_recovers() {
        let mut c = Condition {
            fitness: 6_000,
            sharpness: 9_000,
            fatigue: 5_000,
            morale: 5_000,
        };
        let mut rng = Rng::from_u64(1);
        for _ in 0..14 {
            recover(&mut c, 800, 500, &mut rng);
        }
        assert!(c.fitness > 9_400, "{c:?}");
        assert!(c.fatigue < 2_000, "{c:?}");
        assert!(
            c.sharpness < 9_000 && c.sharpness >= SHARPNESS_FLOOR,
            "{c:?}"
        );
        assert!((4_000..=6_000).contains(&c.morale), "{c:?}");
    }

    #[test]
    fn values_stay_in_range() {
        let mut rng = Rng::from_u64(2);
        for nf in [0, 500, 1000] {
            let mut c = Condition {
                fitness: CONDITION_MAX,
                sharpness: SHARPNESS_FLOOR,
                fatigue: 0,
                morale: CONDITION_MAX,
            };
            for _ in 0..1000 {
                recover(&mut c, nf, nf, &mut rng);
                assert!(c.fitness <= CONDITION_MAX);
                assert!(c.morale <= CONDITION_MAX);
                assert!(c.sharpness >= SHARPNESS_FLOOR);
            }
        }
    }
}
