//! Contract negotiation.

use super::refs::*;
use crate::dto;
use fs_world::contract::{self, Outcome, Terms};
use fs_world::{ClubId, PlayerId, World};

impl From<Terms> for dto::ContractTerms {
    fn from(t: Terms) -> Self {
        dto::ContractTerms {
            wage: t.wage,
            years: t.years,
            status: t.status.into(),
            appearance_bonus: t.appearance_bonus,
            goal_bonus: t.goal_bonus,
            signing_fee: t.signing_fee,
            release_clause: t.release_clause,
        }
    }
}

impl From<dto::ContractTerms> for Terms {
    fn from(t: dto::ContractTerms) -> Self {
        Terms {
            wage: t.wage,
            years: t.years,
            status: t.status.into(),
            appearance_bonus: t.appearance_bonus,
            goal_bonus: t.goal_bonus,
            signing_fee: t.signing_fee,
            release_clause: t.release_clause,
        }
    }
}

fn agent_label(greed: f64) -> &'static str {
    match greed {
        g if g < 0.33 => "Reasonable",
        g if g < 0.66 => "Demanding",
        _ => "Hard bargainer",
    }
}

/// Where talks between `club` and a player stand.
pub fn negotiation(world: &World, club: ClubId, player: PlayerId) -> dto::NegotiationView {
    let d = contract::demands(world, player, club);
    let agent = contract::agent(world, player);
    let talks = world
        .relations()
        .negotiation(player)
        .filter(|t| t.club == club && world.date().days_since(t.last) <= contract::STALE_DAYS);
    let rounds = talks.map_or(0, |t| t.rounds);
    let (closed_until, wants_out) = match contract::open(world, player, club) {
        Ok(()) => (None, false),
        Err(contract::Closed::Until(d)) => (Some(date(d)), false),
        Err(contract::Closed::WantsOut) => (None, true),
    };
    let fin = world.clubs()[club].finances();
    dto::NegotiationView {
        player: player_ref(world, player),
        club: world.players().club(player).map(|c| club_ref(world, c)),
        renewal: world.players().club(player) == Some(club),
        ask: d.terms.into(),
        wants_clause: d.wants_clause,
        agent: agent_label(agent.greed).into(),
        agent_fee_percent: (3.0 + 5.0 * agent.greed).round() as u8,
        offers_left: agent.patience().saturating_sub(rounds),
        closed_until,
        wants_out,
        wage_bill: fs_world::economy::player_wage_bill(world, club),
        wage_budget: fin.wage_budget,
        statuses: super::people::statuses(),
    }
}

/// The outcome as the UI shows it.
pub fn outcome(o: Outcome) -> (dto::TalksOutcome, &'static str) {
    match o {
        Outcome::Agreed => (dto::TalksOutcome::Agreed, "Agreed. The contract is signed."),
        Outcome::Countered(c) => (
            dto::TalksOutcome::Countered { counter: c.into() },
            "The agent comes back with a counter-offer.",
        ),
        Outcome::BrokenOff(until) => (
            dto::TalksOutcome::BrokenOff { until: date(until) },
            "The agent has walked away from talks for now.",
        ),
    }
}

/// A sell-on clause on a player, if any.
pub fn sell_on(world: &World, player: PlayerId) -> Option<dto::SellOnView> {
    world.market().sell_on(player).map(|s| dto::SellOnView {
        club: club_ref(world, s.club),
        percent: s.percent,
    })
}
