//! Transfers and loans: offers, valuations, personal terms, and the
//! completion of deals. How AI clubs behave in the market is in
//! [`crate::market`].
//!
//! A deal runs in steps, so nothing waits on a person:
//!
//! 1. A buyer makes an [`Offer`] during a transfer window.
//! 2. The selling club answers: AI clubs the next day, human managers
//!    within [`HUMAN_RESPONSE_DAYS`] (no answer is a rejection). It can
//!    accept, reject or name its price (a counter-offer the buyer can
//!    accept).
//! 3. Once the clubs agree, the player decides on personal terms.
//! 4. Agreed deals complete in the next daily update, even if the window
//!    has just closed.

use crate::finance::Money;
use crate::ids::{ClubId, PlayerId};
use crate::people::{age_years, wage_demand};
use crate::player::Contract;
use crate::squad::contract_expiry;
use crate::world::World;
use fs_core::GameDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Days a human manager has to answer an offer.
pub const HUMAN_RESPONSE_DAYS: u32 = 3;
/// Resolved offers are kept this long, for the UI.
pub const KEEP_RESOLVED_DAYS: u32 = 60;
/// Transfer records are kept for this many seasons.
pub const HISTORY_SEASONS: i32 = 5;

/// Whether the transfer window is open: 1 July to 31 August, and January.
pub fn window_open(date: GameDate) -> bool {
    matches!(date.ymd().1, 7 | 8 | 1)
}

/// What a buyer proposes.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferKind {
    /// A permanent move for a fee, on a new contract.
    Transfer { fee: Money },
    /// A loan to the end of the season. The borrower pays `wage_share`
    /// percent of the player's wages; the contract stays with the parent.
    Loan { wage_share: u8 },
}

/// Why an offer failed.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    /// The selling club wants more.
    TooLow,
    /// The selling club will not sell at any price now.
    NotForSale,
    /// Nobody answered in time.
    Expired,
    /// The player did not want the move.
    PlayerRefused,
    /// The buyer can no longer afford it or has no room.
    BuyerCannotComplete,
    /// The player moved elsewhere or the buyer withdrew.
    Withdrawn,
}

/// Where an offer stands.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferStatus {
    /// Waiting for the selling club.
    Pending,
    /// The selling club wants this fee instead; waiting for the buyer.
    Countered {
        fee: Money,
    },
    /// Clubs and player agreed; completes in the next daily update.
    Agreed,
    Completed,
    Refused(Refusal),
}

impl OfferStatus {
    /// Whether the offer is still in play.
    pub fn is_open(self) -> bool {
        matches!(
            self,
            OfferStatus::Pending | OfferStatus::Countered { .. } | OfferStatus::Agreed
        )
    }
}

/// An offer for a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: u32,
    /// The day the offer (or its latest revision) was made.
    pub date: GameDate,
    pub buyer: ClubId,
    pub seller: ClubId,
    pub player: PlayerId,
    pub kind: OfferKind,
    /// The contract offered to the player (transfers only).
    pub wage: u32,
    pub years: u8,
    pub status: OfferStatus,
    /// The selling club must answer by this day.
    pub respond_by: GameDate,
    /// When the status last changed.
    pub updated: GameDate,
}

/// A player out on loan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loan {
    pub parent: ClubId,
    pub borrower: ClubId,
    /// The loan ends at the 1 July rollover after this date.
    pub until: GameDate,
    /// Percent of wages the borrower pays.
    pub wage_share: u8,
}

/// A completed move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferRecord {
    pub date: GameDate,
    pub player: PlayerId,
    pub from: ClubId,
    pub to: ClubId,
    /// Zero for loans.
    pub fee: Money,
    pub loan: bool,
}

/// How a club has listed one of its players.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Listing {
    /// For sale.
    Transfer,
    /// Available to borrow.
    Loan,
}

/// The transfer market's state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Market {
    next_id: u32,
    /// Open offers and recently resolved ones, oldest first.
    pub(crate) offers: Vec<Offer>,
    /// Players out on loan.
    pub(crate) loans: BTreeMap<PlayerId, Loan>,
    /// Players their clubs have listed.
    pub(crate) listed: BTreeMap<PlayerId, Listing>,
    /// Completed moves, oldest first.
    pub(crate) history: Vec<TransferRecord>,
}

impl Market {
    pub fn offers(&self) -> &[Offer] {
        &self.offers
    }

    pub fn offer(&self, id: u32) -> Option<&Offer> {
        self.offers.iter().find(|o| o.id == id)
    }

    pub(crate) fn offer_mut(&mut self, id: u32) -> Option<&mut Offer> {
        self.offers.iter_mut().find(|o| o.id == id)
    }

    pub fn loans(&self) -> &BTreeMap<PlayerId, Loan> {
        &self.loans
    }

    pub fn loan(&self, player: PlayerId) -> Option<&Loan> {
        self.loans.get(&player)
    }

    pub fn listing(&self, player: PlayerId) -> Option<Listing> {
        self.listed.get(&player).copied()
    }

    pub fn listed(&self) -> &BTreeMap<PlayerId, Listing> {
        &self.listed
    }

    pub fn history(&self) -> &[TransferRecord] {
        &self.history
    }

    /// Records a new offer and returns its id.
    pub(crate) fn add(&mut self, mut offer: Offer) -> u32 {
        offer.id = self.next_id;
        self.next_id += 1;
        self.offers.push(offer);
        offer.id
    }

    /// The open offer from `buyer` for `player`, if any.
    pub fn open_offer(&self, buyer: ClubId, player: PlayerId) -> Option<&Offer> {
        self.offers
            .iter()
            .find(|o| o.buyer == buyer && o.player == player && o.status.is_open())
    }
}

/// Years of contract left, as a fraction of seasons.
fn years_left(world: &World, player: PlayerId) -> f64 {
    world.players().contract(player).map_or(0.0, |c| {
        f64::from(c.expires.days_since(world.date()).max(0) as u32) / 365.25
    })
}

/// What a player is worth on the market, from ability (`ability` may be a
/// club's estimate), potential, age and contract length.
pub fn valuation(ability: f64, potential: f64, age: f64, years_left: f64) -> Money {
    let demand = f64::from(wage_demand(
        ability.clamp(0.0, 1000.0) as u16,
        potential.clamp(0.0, 1000.0) as u16,
        age as u32,
    ));
    let multiple = match age {
        a if a < 21.0 => 4.0,
        a if a < 25.0 => 3.5,
        a if a < 29.0 => 3.0,
        a if a < 31.0 => 2.0,
        a if a < 33.0 => 1.2,
        _ => 0.5,
    };
    let contract = if years_left < 0.6 {
        0.35
    } else if years_left < 1.5 {
        0.75
    } else {
        1.0
    };
    round_money(demand * 52.0 * multiple * contract)
}

/// A fee rounded to a sensible figure: two significant digits.
pub fn round_money(x: f64) -> Money {
    if x <= 0.0 {
        return 0;
    }
    let v = x as Money;
    let mut step: Money = 100;
    while v / step >= 100 {
        step *= 10;
    }
    (v + step / 2) / step * step
}

/// What a player is really worth.
pub fn market_value(world: &World, player: PlayerId) -> Money {
    let ps = world.players();
    let age = age_years(ps.bio(player).birth_date, world.date());
    valuation(
        f64::from(ps.ability(player)),
        f64::from(ps.potential(player)),
        age,
        years_left(world, player),
    )
}

/// Whether a player would sign for `buyer` on `wage` a week. Players want
/// more money to step down to a smaller club, and will not drop too far
/// unless they are surplus where they are.
pub fn accepts_terms(world: &World, player: PlayerId, buyer: ClubId, wage: u32) -> bool {
    let ps = world.players();
    let age = age_years(ps.bio(player).birth_date, world.date()) as u32;
    let demand = f64::from(wage_demand(ps.ability(player), ps.potential(player), age));
    let to = f64::from(world.clubs()[buyer].reputation);
    let from = ps
        .club(player)
        .map_or(0.0, |c| f64::from(world.clubs()[c].reputation));
    let surplus = world.market().listing(player).is_some();
    if !surplus && to + 2_500.0 < from {
        return false;
    }
    let step_down = ((from - to) / 10_000.0).max(0.0) * 1.5;
    f64::from(wage) >= demand * (0.9 + step_down)
}

/// Whether a player would go on loan to `borrower`.
pub fn accepts_loan(world: &World, player: PlayerId, borrower: ClubId) -> bool {
    let ps = world.players();
    let young = age_years(ps.bio(player).birth_date, world.date()) < 23.0;
    let parent = ps.club(player).map_or(0, |c| world.clubs()[c].reputation);
    young || u32::from(world.clubs()[borrower].reputation) + 4_000 >= u32::from(parent)
}

/// Completes an agreed deal: money, registration, records. Returns the
/// record, or why it could not complete.
pub(crate) fn complete(world: &mut World, id: u32) -> Result<TransferRecord, Refusal> {
    let offer = *world.market.offer(id).expect("offer exists");
    let player = offer.player;
    if world.players.club(player) != Some(offer.seller) || world.market.loans.contains_key(&player)
    {
        return Err(Refusal::Withdrawn);
    }
    if world.clubs[offer.buyer].squad.len() >= crate::command::MAX_SQUAD {
        return Err(Refusal::BuyerCannotComplete);
    }
    let today = world.date;
    let fee = match offer.kind {
        OfferKind::Transfer { fee } => {
            world.clubs[offer.buyer]
                .finances
                .spend(|l| &mut l.player_purchases, fee);
            world.clubs[offer.buyer].finances.transfer_budget -= fee;
            world.clubs[offer.seller]
                .finances
                .earn(|l| &mut l.player_sales, fee);
            // Boards let managers reinvest half of what they sell for.
            world.clubs[offer.seller].finances.transfer_budget += fee / 2;
            world.sign_player(
                player,
                offer.buyer,
                Contract {
                    wage: offer.wage,
                    expires: contract_expiry(today, offer.years),
                },
            );
            fee
        }
        OfferKind::Loan { wage_share } => {
            world.move_player(player, offer.buyer);
            world.market.loans.insert(
                player,
                Loan {
                    parent: offer.seller,
                    borrower: offer.buyer,
                    until: crate::people::season_end(crate::people::season_year(today)),
                    wage_share: wage_share.min(100),
                },
            );
            0
        }
    };
    world.market.listed.remove(&player);
    world.scouting.cancel(offer.buyer, player);
    let record = TransferRecord {
        date: today,
        player,
        from: offer.seller,
        to: offer.buyer,
        fee,
        loan: matches!(offer.kind, OfferKind::Loan { .. }),
    };
    world.market.history.push(record);
    // Every other open offer for the player lapses.
    for o in world.market.offers.iter_mut() {
        if o.player == player && o.id != id && o.status.is_open() {
            o.status = OfferStatus::Refused(Refusal::Withdrawn);
            o.updated = today;
        }
    }
    let o = world.market.offer_mut(id).expect("offer exists");
    o.status = OfferStatus::Completed;
    o.updated = today;
    Ok(record)
}

/// 1 July: loaned players go back to their clubs.
pub(crate) fn end_loans(world: &mut World) -> Vec<(PlayerId, Loan)> {
    let today = world.date;
    let ending: Vec<(PlayerId, Loan)> = world
        .market
        .loans
        .iter()
        .filter(|(_, l)| l.until < today)
        .map(|(&p, &l)| (p, l))
        .collect();
    for &(p, l) in &ending {
        world.market.loans.remove(&p);
        if world.players.club(p) == Some(l.borrower) {
            world.move_player(p, l.parent);
        }
    }
    ending
}

/// Weekly wages a club pays for a player in its squad or out on loan.
pub fn wage_paid(world: &World, club: ClubId, player: PlayerId) -> Money {
    let Some(c) = world.players().contract(player) else {
        return 0;
    };
    let wage = Money::from(c.wage);
    match world.market().loan(player) {
        Some(l) if l.borrower == club => wage * Money::from(l.wage_share) / 100,
        Some(l) if l.parent == club => wage * Money::from(100 - l.wage_share) / 100,
        _ => wage,
    }
}

/// Drops resolved offers and transfer records past their keeping time.
pub(crate) fn prune(world: &mut World) {
    let today = world.date;
    world.market.offers.retain(|o| {
        o.status.is_open() || today.days_since(o.updated) < i64::from(KEEP_RESOLVED_DAYS)
    });
    let oldest = crate::people::season_year(today) - HISTORY_SEASONS;
    world
        .market
        .history
        .retain(|r| crate::people::season_year(r.date) >= oldest);
}

/// A season's market activity, for health checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarketSummary {
    pub transfers: usize,
    pub loans: usize,
    pub total_fees: Money,
    pub top_fee: Money,
}

impl MarketSummary {
    /// Moves in the season starting in `year`.
    pub fn of(world: &World, year: i32) -> Self {
        let mut s = MarketSummary::default();
        for r in world
            .market()
            .history()
            .iter()
            .filter(|r| crate::people::season_year(r.date) == year)
        {
            if r.loan {
                s.loans += 1;
            } else {
                s.transfers += 1;
                s.total_fees += r.fee;
                s.top_fee = s.top_fee.max(r.fee);
            }
        }
        s
    }
}

impl std::fmt::Display for MarketSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} transfers ({:.1}m in fees, top {:.1}m), {} loans",
            self.transfers,
            self.total_fees as f64 / 1e6,
            self.top_fee as f64 / 1e6,
            self.loans
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valuations_follow_ability_age_and_contract() {
        let peak = valuation(700.0, 700.0, 26.0, 3.0);
        assert!(valuation(800.0, 800.0, 26.0, 3.0) > 3 * peak);
        assert!(valuation(700.0, 700.0, 33.0, 3.0) < peak / 4);
        assert!(valuation(700.0, 700.0, 26.0, 0.3) < peak / 2);
        assert!(valuation(500.0, 800.0, 19.0, 3.0) > valuation(500.0, 500.0, 26.0, 3.0));
    }

    #[test]
    fn money_rounds_to_two_figures() {
        assert_eq!(round_money(1_234_567.0), 1_200_000);
        assert_eq!(round_money(87_654.0), 88_000);
        assert_eq!(round_money(420.0), 400);
        assert_eq!(round_money(-5.0), 0);
    }

    #[test]
    fn windows() {
        let d = |m, day| GameDate::from_ymd(2027, m, day).unwrap();
        assert!(window_open(d(7, 1)) && window_open(d(8, 31)) && window_open(d(1, 15)));
        assert!(!window_open(d(9, 1)) && !window_open(d(2, 1)) && !window_open(d(6, 30)));
    }
}
