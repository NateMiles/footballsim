//! The daily simulation step.
//!
//! Each day, in order: calendar events (season rollover on 1 July,
//! contract renewals on 1 June, expiry warnings on 1 January, transfer
//! windows), the day's matches and cup draws, recovery, injuries, the
//! market, then staggered work: each club's players develop once a month
//! on the club's own day, and AI tactics are reviewed on a cycle set by
//! the club's importance ([`crate::importance`]). Money and boards are
//! settled on the 1st of the month.
//!
//! Per-player passes run in parallel with per-player RNG streams, so the
//! result does not depend on thread count. With the `profile` feature,
//! [`crate::profile`] times each stage.

use crate::attributes::{ATTR_MAX, Attr, HiddenAttr};
use crate::command::Event;
use crate::economy;
use crate::ids::ClubId;
use crate::matchday::{play_today, review_ai_tactics, sample_injury};
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
        crate::profile::time("rollover", || rollover(world, events));
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
    crate::profile::time("emergency signings", || {
        crate::squad::emergency_signings(world, events);
    });
    crate::profile::time("press", || crate::press::morning(world, events));
}

/// The rest of the day: the matches (with any live instructions), then
/// recovery, injuries and monthly development.
pub(crate) fn afternoon(
    world: &mut World,
    events: &mut Vec<Event>,
    instructions: &BTreeMap<ClubId, Vec<TimedInstruction>>,
) {
    use crate::profile::time;
    if !world.leagues.is_empty() {
        time("matches", || play_today(world, events, instructions));
        time("cups", || crate::cup::daily(world, events));
    }
    time("training", || crate::training::daily(world));
    time("recovery", || recovery(world));
    time("injuries", || injuries(world, events));
    time("news", || crate::news::daily(world));
    time("scouting", || crate::scouting::daily(world, events));
    if !world.leagues.is_empty() {
        time("market", || crate::market::daily(world, events));
    }
    time("vacancies", || crate::board::fill_vacancies(world, events));
    if !world.leagues.is_empty() {
        time("people", || crate::happiness::daily(world, events));
    }
    // Staggered work: each player develops on their own day of the
    // month, and each club reviews its tactic on its own days.
    time("development", || development(world));
    if !world.leagues.is_empty() {
        time("ai tactics", || review_ai_tactics(world));
    }
    if world.date.ymd().2 == 1 {
        time("monthly accounts", || economy::monthly_accounts(world));
        if !world.leagues.is_empty() {
            time("monthly board", || crate::board::monthly(world, events));
        }
    }
}

fn recovery(world: &mut World) {
    let seed = world.seed;
    let day = u64::from(world.date.days());
    // Each club's training sets how fast its players recover.
    let rates: Vec<u32> = world
        .clubs
        .as_slice()
        .iter()
        .map(|c| c.training.recovery_per_mille())
        .collect();
    let Players {
        condition,
        attributes,
        hidden,
        status,
        club,
        happiness,
        ..
    } = &mut world.players;
    par::for_each_indexed_mut(condition, |i, c| {
        if status[i] != Status::Active {
            return;
        }
        let mut rng = seed.derive(Stream::DailyTick, day, i as u64);
        let natural_fitness = u32::from(attributes[i].get(Attr::NaturalFitness));
        let professionalism = u32::from(hidden[i].get(HiddenAttr::Professionalism));
        let rate = club[i].map_or(1000, |c| rates[c.index()]);
        let baseline = crate::happiness::morale_baseline(happiness[i]);
        recover(
            c,
            natural_fitness,
            professionalism,
            rate,
            baseline,
            &mut rng,
        );
    });
}

/// Heals injuries that have run their course and causes new training
/// injuries, more often for injury-prone or overloaded players.
fn injuries(world: &mut World, events: &mut Vec<Event>) {
    let seed = world.seed;
    let today = world.date;
    let day = u64::from(today.days());
    // Medical teams and training load: (prevention, treatment) per club.
    let medical: Vec<(f64, f64)> = world
        .clubs
        .ids()
        .map(|c| {
            let r = staff::ratings(world, c);
            (
                staff::prevention_factor(&r) * world.clubs[c].training.injury_risk(),
                staff::treatment_factor(&r),
            )
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

/// Days of the month over which development is spread.
const DEVELOPMENT_DAYS: u32 = 28;

/// Monthly growth, plateau and decline for every active player. Each
/// club's players develop on the club's own day of the month (free
/// agents on their own), so the work is spread over the first 28 days.
fn development(world: &mut World) {
    let seed = world.seed;
    let today = world.date;
    let day = today.ymd().2;
    if day > DEVELOPMENT_DAYS {
        return;
    }
    let due = |i: usize| (i as u32) % DEVELOPMENT_DAYS + 1 == day;
    let month = u64::from(today.days());
    // Matches each club has played this season, for playing-time shares.
    let mut played = crate::cup::matches_played(world);
    for (_, league) in world.leagues.iter() {
        if let Some(s) = league.season() {
            for row in &s.rows {
                played[row.club.index()] += u32::from(row.played);
            }
        }
    }
    // Training: growth and attribute emphasis per club developing today,
    // and players' own training.
    let schedules: Vec<(f64, Option<[f64; crate::attributes::ATTR_COUNT]>)> = world
        .clubs
        .as_slice()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if !due(i) {
                return (1.0, None);
            }
            let t = c.training;
            let focus = (t.focus != crate::training::Focus::Balanced).then(|| t.focus.weights());
            (t.growth(), focus)
        })
        .collect();
    let individual: BTreeMap<crate::ids::PlayerId, crate::training::Individual> = world
        .clubs
        .as_slice()
        .iter()
        .enumerate()
        .filter(|(i, _)| due(*i))
        .flat_map(|(_, c)| c.individual.iter().map(|(&p, &t)| (p, t)))
        .collect();
    // Coaching: (seniors, under-21s), for the clubs developing today.
    let coaching: Vec<(f64, f64)> = world
        .clubs
        .ids()
        .map(|c| {
            if !due(c.index()) {
                return (1.0, 1.0);
            }
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
        if status[i] != Status::Active || !due(club[i].map_or(i, |c| c.index())) {
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
        let (training, focus) = club[i].map_or((1.0, None), |c| schedules[c.index()]);
        let squad_weights = focus.unwrap_or(crate::people::EVEN);
        let own = individual
            .get(&crate::ids::PlayerId::new(i as u32))
            .map(|t| t.weights(squad_weights));
        develop_month(
            attrs,
            positions[i].best(),
            potential[i],
            age,
            GrowthFactors {
                professionalism,
                playing_time: playing,
                coaching: coached,
                training,
            },
            own.as_ref().unwrap_or(&squad_weights),
            &mut rng,
        );
    });
    // Position training: a point of familiarity a month, two for
    // adaptable players.
    for (&p, &t) in &individual {
        if let crate::training::Individual::Position(pos) = t {
            let i = p.index();
            let mut rng = seed.derive(Stream::Development, month, i as u64 | (1 << 41));
            let adaptability = f64::from(hidden[i].get(HiddenAttr::Adaptability)) / 1000.0;
            let gain = 1 + u8::from(rng.chance(adaptability));
            let now = positions[i].get(pos);
            positions[i].set(pos, now.saturating_add(gain));
        }
    }
}

/// One day of recovery for one player. Attribute inputs are 0-1000.
fn recover(
    c: &mut Condition,
    natural_fitness: u32,
    professionalism: u32,
    rate_per_mille: u32,
    morale_baseline: u16,
    rng: &mut fs_core::Rng,
) {
    let max = u32::from(CONDITION_MAX);
    let attr_max = u32::from(ATTR_MAX);

    // Fitness closes 15% to 30% of the gap to full each day, faster with
    // better natural fitness, so a tired player is fresh within a week.
    let fitness = u32::from(c.fitness);
    let rate = (150 + natural_fitness * 150 / attr_max) * rate_per_mille / 1000; // per mille
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

    // Morale reverts 2% toward the player's baseline each day (neutral
    // for a player with no complaints), plus small noise.
    let morale = i32::from(c.morale);
    let baseline = i32::from(morale_baseline);
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
            recover(
                &mut c,
                800,
                500,
                1000,
                crate::happiness::morale_baseline(0),
                &mut rng,
            );
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
                recover(
                    &mut c,
                    nf,
                    nf,
                    1000,
                    crate::happiness::morale_baseline(0),
                    &mut rng,
                );
                assert!(c.fitness <= CONDITION_MAX);
                assert!(c.morale <= CONDITION_MAX);
                assert!(c.sharpness >= SHARPNESS_FLOOR);
            }
        }
    }
}
