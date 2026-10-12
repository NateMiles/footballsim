//! Contract negotiation: what a player (and their agent) wants, how they
//! value an offer, and how talks go.
//!
//! An offer is a package: wage, years, squad status, appearance and goal
//! bonuses, a signing fee and perhaps a release clause. The player turns
//! it into a weekly value (bonuses at the appearances and goals they
//! expect, discounted for the risk; the signing fee spread over the
//! contract) and weighs the rest: a smaller role than they expect costs a
//! lot, years away from what they want cost a little, and ambitious
//! players at clubs below their level want a release clause. They accept
//! when the package is worth enough against their demand. Otherwise the
//! agent counters with the wage that would do, until their patience runs
//! out and talks break down for two weeks.
//!
//! Agents differ in greed, which raises the demand, shortens patience
//! and sets their fee (a share of the contract's wages, paid by the club
//! when it is signed, with the signing fee).
//!
//! AI clubs keep agreeing plain contracts at demand (`squad`, `market`);
//! negotiation is how human managers sign and renew players.

use crate::finance::Money;
use crate::happiness::{self, Feeling, SquadStatus};
use crate::ids::{ClubId, PlayerId};
use crate::people::{age_years, wage_demand};
use crate::personality::trait_of;
use crate::player::Contract;
use crate::squad::contract_expiry;
use crate::transfer::round_money;
use crate::world::World;
use fs_core::attributes::{HiddenAttr, Position};
use fs_core::{GameDate, Stream};
use serde::{Deserialize, Serialize};

/// A contract offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Terms {
    /// Weekly wage.
    pub wage: u32,
    /// Seasons, counting the current one when offered before January.
    pub years: u8,
    /// The role promised.
    pub status: SquadStatus,
    pub appearance_bonus: u32,
    pub goal_bonus: u32,
    /// Paid once, on signing.
    pub signing_fee: Money,
    pub release_clause: Option<Money>,
}

impl Terms {
    /// A plain offer: wage, years and the role the player expects.
    pub fn plain(wage: u32, years: u8, status: SquadStatus) -> Self {
        Terms {
            wage,
            years,
            status,
            appearance_bonus: 0,
            goal_bonus: 0,
            signing_fee: 0,
            release_clause: None,
        }
    }
}

/// How talks went.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Signed.
    Agreed,
    /// The agent's counter-offer.
    Countered(Terms),
    /// Talks broke down until the given day.
    BrokenOff(GameDate),
}

/// Talks under way with one player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Talks {
    pub club: ClubId,
    /// Offers made so far.
    pub rounds: u8,
    pub last: GameDate,
    /// No more talks before this day.
    pub closed_until: Option<GameDate>,
}

/// Days talks stay closed after breaking down.
pub const CLOSED_DAYS: u32 = 14;
/// Talks with no offer for this long start afresh.
pub const STALE_DAYS: i64 = 30;
/// Share of the demand an offer must be worth.
const ACCEPT: f64 = 0.9;
/// An offer worth less than this share ends talks at once.
const INSULT: f64 = 0.6;

/// A player's agent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Agent {
    /// 0-1: higher demands, less patience, a bigger fee.
    pub greed: f64,
}

impl Agent {
    /// Offers the agent will hear before walking away.
    pub fn patience(self) -> u8 {
        2 + ((1.0 - self.greed) * 2.99) as u8
    }

    /// The agent's fee: a share of the contract's total wages.
    pub fn fee(self, terms: &Terms) -> Money {
        let total = Money::from(terms.wage) * 52 * Money::from(terms.years);
        round_money(total as f64 * (0.03 + 0.05 * self.greed))
    }
}

/// Every player's agent, the same for the whole career.
pub fn agent(world: &World, player: PlayerId) -> Agent {
    let mut rng = world
        .seed()
        .derive(Stream::People, u64::from(player.raw()), 0xA6E7);
    Agent {
        greed: rng.unit_f64(),
    }
}

/// What the player wants, and what matters to them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Demands {
    /// The agent's opening ask.
    pub terms: Terms,
    /// The wage the package is measured against (before the agent's
    /// mark-up).
    pub wage: f64,
    /// Whether they insist on a release clause.
    pub wants_clause: bool,
}

/// Appearances a season a player in this role expects.
fn appearances(status: SquadStatus) -> f64 {
    match status {
        SquadStatus::Key => 42.0,
        SquadStatus::Regular => 34.0,
        SquadStatus::Rotation => 22.0,
        SquadStatus::Backup => 8.0,
        SquadStatus::Prospect => 3.0,
    }
}

/// Goals per appearance a player expects by position.
fn scoring(position: Position) -> f64 {
    use Position::*;
    match position {
        Striker => 0.45,
        AttackingMidCentre | AttackingMidLeft | AttackingMidRight => 0.25,
        MidfielderLeft | MidfielderRight => 0.15,
        MidfielderCentre => 0.1,
        DefensiveMidfielder => 0.05,
        Goalkeeper => 0.0,
        _ => 0.03,
    }
}

fn preferred_years(age: f64) -> u8 {
    match age {
        a if a < 24.0 => 3,
        a if a < 30.0 => 4,
        a if a < 33.0 => 2,
        _ => 1,
    }
}

/// What `player` wants to sign for or stay at `club`.
pub fn demands(world: &World, player: PlayerId, club: ClubId) -> Demands {
    let ps = world.players();
    let age = age_years(ps.bio(player).birth_date, world.date());
    let base = f64::from(wage_demand(
        ps.ability(player),
        ps.potential(player),
        age as u32,
    ));
    let to = f64::from(world.clubs()[club].reputation);
    let from = ps
        .club(player)
        .map_or(0.0, |c| f64::from(world.clubs()[c].reputation));
    let step_down = ((from - to) / 10_000.0).max(0.0) * 1.5;
    // Mood: unhappy players want more to stay; happy ones less.
    let mood = if ps.club(player) == Some(club) {
        1.0 - f64::from(ps.happiness(player)) / 400.0
    } else {
        1.0
    };
    let wage = base * (1.0 + step_down) * mood;
    let a = agent(world, player);
    let status = if ps.club(player) == Some(club) {
        ps.squad_status(player)
    } else {
        happiness::newcomer_status(world, club, player)
    };
    // Ambitious players at clubs below their level want a way out.
    let bar = club_level(world, club);
    let wants_clause = trait_of(world, player, HiddenAttr::Ambition) > 0.6
        && f64::from(ps.ability(player)) > bar - 20.0;
    let value = crate::transfer::market_value(world, player);
    let ask = (wage * (1.0 + 0.15 * a.greed) / 50.0).ceil() as u32 * 50;
    Demands {
        terms: Terms {
            wage: ask.max(100),
            years: preferred_years(age),
            status,
            appearance_bonus: 0,
            goal_bonus: 0,
            signing_fee: round_money(wage * 4.0 * (0.5 + a.greed)),
            release_clause: wants_clause.then(|| round_money(value as f64 * 2.5).max(100_000)),
        },
        wage: wage.max(100.0),
        wants_clause,
    }
}

/// Mean ability of a club's best eleven.
fn club_level(world: &World, club: ClubId) -> f64 {
    let ps = world.players();
    let mut a: Vec<u16> = world.clubs()[club]
        .squad()
        .iter()
        .map(|&p| ps.ability(p))
        .collect();
    a.sort_unstable_by(|x, y| y.cmp(x));
    let top = &a[..a.len().min(11)];
    top.iter().map(|&x| f64::from(x)).sum::<f64>() / top.len().max(1) as f64
}

/// The weekly value of a package's money: wage, bonuses at what the
/// player expects to play and score (discounted for the risk), and the
/// signing fee spread over the contract.
fn weekly_value(world: &World, player: PlayerId, terms: &Terms) -> f64 {
    let apps = appearances(terms.status);
    let goals = apps * scoring(world.players().positions(player).best());
    let bonuses = (apps * f64::from(terms.appearance_bonus) + goals * f64::from(terms.goal_bonus))
        / 52.0
        * 0.85;
    let fee = terms.signing_fee.max(0) as f64 / (52.0 * f64::from(terms.years.max(1)));
    f64::from(terms.wage) + bonuses + fee
}

/// Everything but the money, as a multiplier.
fn conditions(demands: &Demands, terms: &Terms) -> f64 {
    // Ranks: 0 is a key player.
    let rank = |s: SquadStatus| s as i32;
    let steps = rank(terms.status) - rank(demands.terms.status);
    let role = match steps {
        s if s > 0 => (0..s).fold(1.0, |a, _| a * 0.85),
        s if s < 0 => 1.05,
        _ => 1.0,
    };
    let years = 1.0 - 0.03 * f64::from(terms.years.abs_diff(demands.terms.years));
    let clause = match (demands.wants_clause, terms.release_clause) {
        (true, None) => 0.92,
        (true, Some(c)) => {
            let ask = demands.terms.release_clause.unwrap_or(c).max(1) as f64;
            // A clause well above the ask is worth less to them.
            (1.0 - 0.08 * ((c as f64 / ask - 1.0).max(0.0))).max(0.92)
        }
        (false, Some(_)) => 1.02,
        (false, None) => 1.0,
    };
    role * years * clause
}

/// What an offer is worth to the player, as a share of their demand.
pub fn appeal(world: &World, player: PlayerId, club: ClubId, terms: &Terms) -> f64 {
    let d = demands(world, player, club);
    weekly_value(world, player, terms) / d.wage * conditions(&d, terms)
}

/// Whether the player would sign these terms at `club` today (no
/// counter-offers: used for transfers).
pub fn accepts(world: &World, player: PlayerId, club: ClubId, terms: &Terms) -> bool {
    let ps = world.players();
    let to = world.clubs()[club].reputation;
    let from = ps.club(player).map_or(0, |c| world.clubs()[c].reputation);
    let surplus = world.market().listing(player).is_some();
    if !surplus && u32::from(to) + 2_500 < u32::from(from) {
        return false;
    }
    appeal(world, player, club, terms) >= ACCEPT
}

/// The agent's counter: the offer with the wage (and, if needed, the role
/// and a release clause) that would be accepted.
fn counter(world: &World, player: PlayerId, club: ClubId, terms: &Terms) -> Terms {
    let d = demands(world, player, club);
    let mut c = *terms;
    if c.release_clause.is_none() && d.wants_clause {
        c.release_clause = d.terms.release_clause;
    }
    if (c.status as i32) - (d.terms.status as i32) >= 2 {
        c.status = SquadStatus::ALL[d.terms.status as usize + 1];
    }
    let others = weekly_value(world, player, &Terms { wage: 0, ..c });
    let needed = 0.95 * d.wage / conditions(&d, &c) - others;
    c.wage = ((needed / 50.0).ceil() as u32 * 50)
        .max(terms.wage)
        .max(100);
    c
}

/// Why talks cannot happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closed {
    /// Talks broke down; they reopen on this day.
    Until(GameDate),
    /// The player wants to leave.
    WantsOut,
}

/// Whether the player will talk to `club` today.
pub fn open(world: &World, player: PlayerId, club: ClubId) -> Result<(), Closed> {
    if let Some(t) = world.relations.negotiations.get(&player)
        && t.club == club
        && let Some(until) = t.closed_until
        && until > world.date()
    {
        return Err(Closed::Until(until));
    }
    let ours = world.players().club(player) == Some(club);
    if ours && world.relations.request(player).is_some() {
        return Err(Closed::WantsOut);
    }
    Ok(())
}

/// One round of talks. The caller has checked the club may make the
/// offer (its own player, or a free agent with room in the squad) and that
/// talks are [`open`].
pub(crate) fn negotiate(
    world: &mut World,
    club: ClubId,
    player: PlayerId,
    terms: &Terms,
) -> Outcome {
    let today = world.date();
    let fresh = world
        .relations
        .negotiations
        .get(&player)
        .is_none_or(|t| t.club != club || today.days_since(t.last) > STALE_DAYS);
    let rounds = if fresh {
        1
    } else {
        world.relations.negotiations[&player].rounds + 1
    };
    let value = appeal(world, player, club, terms);
    if value >= ACCEPT {
        sign(world, club, player, terms);
        world.relations.negotiations.remove(&player);
        return Outcome::Agreed;
    }
    let outcome = if value < INSULT || rounds >= agent(world, player).patience() {
        let until = today.add_days(CLOSED_DAYS);
        if world.players().club(player) == Some(club) {
            happiness::remember(world, player, Feeling::TalksFailed, -8, 60);
            happiness::refresh(world, player);
        }
        Outcome::BrokenOff(until)
    } else {
        Outcome::Countered(counter(world, player, club, terms))
    };
    world.relations.negotiations.insert(
        player,
        Talks {
            club,
            rounds,
            last: today,
            closed_until: match outcome {
                Outcome::BrokenOff(until) => Some(until),
                _ => None,
            },
        },
    );
    outcome
}

/// The contract a package becomes.
pub fn contract(today: GameDate, terms: &Terms) -> Contract {
    Contract {
        appearance_bonus: terms.appearance_bonus,
        goal_bonus: terms.goal_bonus,
        release_clause: terms.release_clause,
        ..Contract::new(terms.wage, contract_expiry(today, terms.years))
    }
}

/// Signs (or renews) on agreed terms: the contract, the promised role, and
/// the signing and agent's fees.
pub(crate) fn sign(world: &mut World, club: ClubId, player: PlayerId, terms: &Terms) {
    let today = world.date();
    let contract = contract(today, terms);
    if world.players().club(player) == Some(club) {
        world.players.contract[player.index()] = Some(contract);
    } else {
        world.sign_player(player, club, contract);
    }
    world.players.squad_status[player.index()] = terms.status;
    pay_fees(world, club, player, terms);
}

/// The signing fee and the agent's fee.
pub(crate) fn pay_fees(world: &mut World, club: ClubId, player: PlayerId, terms: &Terms) {
    let fin = &mut world.clubs[club].finances;
    fin.spend(|l| &mut l.signing_fees, terms.signing_fee.max(0));
    let fee = agent(world, player).fee(terms);
    world.clubs[club].finances.spend(|l| &mut l.agent_fees, fee);
}

/// After a match: appearance and goal bonuses, paid by the player's club.
pub(crate) fn pay_bonuses(world: &mut World, club: ClubId, player: PlayerId, goals: u8) {
    let Some(c) = world.players().contract(player) else {
        return;
    };
    let due = Money::from(c.appearance_bonus) + Money::from(c.goal_bonus) * Money::from(goals);
    if due > 0 {
        world.clubs[club].finances.spend(|l| &mut l.bonuses, due);
    }
}

/// Forget talks that went quiet.
pub(crate) fn tidy(world: &mut World) {
    let today = world.date();
    world.relations.negotiations.retain(|_, t| {
        today.days_since(t.last) <= STALE_DAYS || t.closed_until.is_some_and(|u| u > today)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patience_runs_two_to_four_offers() {
        assert_eq!(Agent { greed: 1.0 }.patience(), 2);
        assert_eq!(Agent { greed: 0.0 }.patience(), 4);
    }

    #[test]
    fn plain_terms_have_no_extras() {
        let t = Terms::plain(1000, 3, SquadStatus::Regular);
        assert_eq!((t.appearance_bonus, t.goal_bonus, t.signing_fee), (0, 0, 0));
        assert!(t.release_clause.is_none());
    }
}
