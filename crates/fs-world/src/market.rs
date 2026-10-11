//! How AI clubs act in the transfer market, and the daily update that
//! moves deals along.
//!
//! Each day of a window, a few AI clubs look for a player: to fill a gap in
//! the squad, or to replace a weak starter. They judge players by their own
//! scouting estimates (so they overpay for some and miss others), bid near
//! their own valuation, and only for players who would plausibly come.
//! Selling clubs ask more for key players, less for players they have
//! listed, and will not weaken a thin squad. Clubs list surplus players
//! for sale and young players for loan when a window opens.

use crate::command::Event;
use crate::finance::Money;
use crate::ids::{ClubId, PlayerId};
use crate::people::{age_years, wage_demand};
use crate::player::Status;
use crate::scouting;
use crate::squad::{Group, first_team_age, value};
use crate::staff;
use crate::transfer::{
    self, HUMAN_RESPONSE_DAYS, Listing, Offer, OfferKind, OfferStatus, Refusal, accepts_loan,
    accepts_terms, round_money, valuation, window_open,
};
use crate::world::World;
use fs_core::{GameDate, Stream, par};

/// Chance an AI club looks for a player on a given window day.
const DAILY_ACTIVITY: f64 = 0.12;
/// Candidates an AI club considers per search.
const CANDIDATES: usize = 30;
/// Typical starters per group.
const STARTERS: [(Group, usize); 4] = [
    (Group::Goalkeeper, 1),
    (Group::Defender, 4),
    (Group::Midfielder, 4),
    (Group::Attacker, 2),
];
/// Minimum first-team players per group before a club will sell.
const MIN_GROUP: [(Group, usize); 4] = [
    (Group::Goalkeeper, 2),
    (Group::Defender, 6),
    (Group::Midfielder, 6),
    (Group::Attacker, 3),
];
/// AI clubs will not sell below this many first-team players.
const MIN_FIRST_TEAM: usize = 20;
/// AI clubs do not buy above this many first-team players.
const MAX_FIRST_TEAM: usize = 28;

fn group_of(world: &World, p: PlayerId) -> Group {
    Group::of(world.players().positions(p).best())
}

/// A club's first-team players (18+), best first by true value: what the
/// club knows of its own squad.
fn first_team(world: &World, club: ClubId) -> Vec<PlayerId> {
    let today = world.date();
    let mut v: Vec<(f64, PlayerId)> = world.clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&p| first_team_age(world, p, today))
        .map(|p| (value(world, p, today), p))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|(_, p)| p).collect()
}

/// Whether a player is on loan *at* `club` (so not the club's to sell).
fn loaned_in(world: &World, club: ClubId, p: PlayerId) -> bool {
    world.market().loan(p).is_some_and(|l| l.borrower == club)
}

/// The daily market update. Runs every day: agreed deals complete and
/// unanswered offers lapse even when the window is shut.
pub(crate) fn daily(world: &mut World, events: &mut Vec<Event>) {
    complete_agreed(world, events);
    lapse(world, events);
    sellers_answer(world, events);
    buyers_answer_counters(world, events);
    let today = world.date();
    if window_open(today) {
        if matches!(today.ymd(), (_, 7, 1) | (_, 8, 1) | (_, 1, 1)) {
            list_players(world);
        }
        make_offers(world, events);
    }
    transfer::prune(world);
}

fn is_human(world: &World, club: ClubId) -> bool {
    !world.ai_runs_squad(club)
}

fn notify(world: &World, events: &mut Vec<Event>, offer: &Offer) {
    if world.is_human_club(offer.buyer) || world.is_human_club(offer.seller) {
        events.push(Event::OfferUpdated {
            offer: offer.id,
            player: offer.player,
            buyer: offer.buyer,
            seller: offer.seller,
            status: offer.status,
        });
    }
}

fn set_status(world: &mut World, events: &mut Vec<Event>, id: u32, status: OfferStatus) {
    let today = world.date();
    let o = world.market.offer_mut(id).expect("offer exists");
    o.status = status;
    o.updated = today;
    let o = *o;
    notify(world, events, &o);
}

fn complete_agreed(world: &mut World, events: &mut Vec<Event>) {
    let agreed: Vec<u32> = world
        .market
        .offers
        .iter()
        .filter(|o| o.status == OfferStatus::Agreed)
        .map(|o| o.id)
        .collect();
    for id in agreed {
        match transfer::complete(world, id) {
            Ok(r) => {
                if world.is_human_club(r.from) || world.is_human_club(r.to) {
                    events.push(Event::TransferCompleted {
                        player: r.player,
                        from: r.from,
                        to: r.to,
                        fee: r.fee,
                        loan: r.loan,
                    });
                }
            }
            Err(why) => set_status(world, events, id, OfferStatus::Refused(why)),
        }
    }
}

/// Offers awaiting a human answer lapse after their deadline.
fn lapse(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let stale: Vec<u32> = world
        .market
        .offers
        .iter()
        .filter(|o| o.status.is_open() && o.status != OfferStatus::Agreed && o.respond_by < today)
        .map(|o| o.id)
        .collect();
    for id in stale {
        set_status(world, events, id, OfferStatus::Refused(Refusal::Expired));
    }
}

/// What an AI club would want for one of its players.
pub fn asking_price(world: &World, club: ClubId, player: PlayerId) -> Option<Money> {
    let team = first_team(world, club);
    let rank = team.iter().position(|&p| p == player);
    let g = group_of(world, player);
    let in_group = team.iter().filter(|&&p| group_of(world, p) == g).count();
    let min_group = MIN_GROUP.iter().find(|m| m.0 == g).map_or(0, |m| m.1);
    let listed = world.market().listing(player) == Some(Listing::Transfer);
    if !listed && rank.is_some() && (team.len() <= MIN_FIRST_TEAM || in_group <= min_group) {
        return None;
    }
    let mut importance = match rank {
        _ if listed => 0.9,
        Some(r) if r < 11 => 1.6,
        Some(r) if r < 18 => 1.25,
        _ => 1.05,
    };
    let (_, month, day) = world.date().ymd();
    if rank.is_some_and(|r| r < 11) && (month == 8 && day >= 20 || month == 1 && day >= 20) {
        importance += 0.3;
    }
    let c = &world.clubs()[club];
    if c.finances().balance < 0 {
        importance *= 0.85;
    }
    Some(round_money(
        transfer::market_value(world, player) as f64 * importance,
    ))
}

/// AI selling clubs answer offers made before today. For each player the
/// best offer is answered first; once one is agreed the others are
/// refused.
fn sellers_answer(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let mut pending: Vec<Offer> = world
        .market
        .offers
        .iter()
        .filter(|o| {
            o.status == OfferStatus::Pending && o.date < today && !is_human(world, o.seller)
        })
        .copied()
        .collect();
    // Best first: transfers by fee, then loans by the share of wages the
    // borrower would pay.
    let fee = |o: &Offer| match o.kind {
        OfferKind::Transfer { fee } => fee,
        OfferKind::Loan { wage_share } => i64::from(wage_share) - 1_000,
    };
    pending.sort_by(|a, b| {
        a.player
            .cmp(&b.player)
            .then(fee(b).cmp(&fee(a)))
            .then(a.id.cmp(&b.id))
    });
    let mut agreed: Vec<PlayerId> = Vec::new();
    for o in pending {
        if agreed.contains(&o.player) || world.players().club(o.player) != Some(o.seller) {
            set_status(
                world,
                events,
                o.id,
                OfferStatus::Refused(Refusal::Withdrawn),
            );
            continue;
        }
        let status = match o.kind {
            OfferKind::Transfer { fee } => match asking_price(world, o.seller, o.player) {
                None => OfferStatus::Refused(Refusal::NotForSale),
                Some(ask) if fee >= ask => {
                    if accepts_terms(world, o.player, o.buyer, o.wage) {
                        OfferStatus::Agreed
                    } else {
                        OfferStatus::Refused(Refusal::PlayerRefused)
                    }
                }
                Some(ask) if fee * 10 >= ask * 7 => OfferStatus::Countered { fee: ask },
                Some(_) => OfferStatus::Refused(Refusal::TooLow),
            },
            OfferKind::Loan { wage_share } => {
                let team = first_team(world, o.seller);
                let rank = team.iter().position(|&p| p == o.player);
                let young = age_years(world.players().bio(o.player).birth_date, today) < 23.0;
                let listed = world.market().listing(o.player) == Some(Listing::Loan);
                let spare = listed || (young && rank.is_none_or(|r| r >= 14) && wage_share >= 50);
                if !spare {
                    OfferStatus::Refused(Refusal::NotForSale)
                } else if accepts_loan(world, o.player, o.buyer) {
                    OfferStatus::Agreed
                } else {
                    OfferStatus::Refused(Refusal::PlayerRefused)
                }
            }
        };
        if status == OfferStatus::Agreed {
            agreed.push(o.player);
        }
        if let OfferStatus::Countered { .. } = status {
            let o = world.market.offer_mut(o.id).expect("offer exists");
            o.respond_by = today.add_days(HUMAN_RESPONSE_DAYS);
        }
        set_status(world, events, o.id, status);
    }
}

/// The most an AI club would pay for a player, from its own estimate.
fn max_price(world: &World, buyer: ClubId, player: PlayerId) -> Money {
    let ratings = staff::ratings(world, buyer);
    let ability = scouting::perceived_ability(world, buyer, player, &ratings);
    let ps = world.players();
    let age = age_years(ps.bio(player).birth_date, world.date());
    let years = ps.contract(player).map_or(0.0, |c| {
        c.expires.days_since(world.date()).max(0) as f64 / 365.25
    });
    let v = valuation(
        ability,
        f64::from(ps.potential(player)).max(ability),
        age,
        years,
    );
    (v as f64 * 1.25) as Money
}

/// AI buyers answer counter-offers made before today.
fn buyers_answer_counters(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let countered: Vec<Offer> = world
        .market
        .offers
        .iter()
        .filter(|o| matches!(o.status, OfferStatus::Countered { .. }) && o.updated < today)
        .filter(|o| !is_human(world, o.buyer))
        .copied()
        .collect();
    for o in countered {
        let OfferStatus::Countered { fee } = o.status else {
            continue;
        };
        let budget = world.clubs()[o.buyer].finances().transfer_budget;
        let status = if fee <= budget && fee <= max_price(world, o.buyer, o.player) {
            if accepts_terms(world, o.player, o.buyer, o.wage) {
                world.market.offer_mut(o.id).expect("offer").kind = OfferKind::Transfer { fee };
                OfferStatus::Agreed
            } else {
                OfferStatus::Refused(Refusal::PlayerRefused)
            }
        } else {
            OfferStatus::Refused(Refusal::Withdrawn)
        };
        set_status(world, events, o.id, status);
    }
}

/// When a window opens (and on 1 August), AI clubs list surplus players
/// for sale and promising youngsters who are not playing for loan. World
/// generation calls this too, since a new world starts on the first day
/// of the window.
pub fn list_players(world: &mut World) {
    let today = world.date();
    let clubs: Vec<ClubId> = world
        .clubs()
        .ids()
        .filter(|&c| world.ai_runs_squad(c))
        .collect();
    let mut listings: Vec<(PlayerId, Option<Listing>)> = Vec::new();
    for club in clubs {
        let team: Vec<PlayerId> = first_team(world, club)
            .into_iter()
            .filter(|&p| !loaned_in(world, club, p))
            .collect();
        let bar: u32 = team
            .iter()
            .take(11)
            .map(|&p| u32::from(world.players().ability(p)))
            .sum::<u32>()
            / team.len().clamp(1, 11) as u32;
        for (rank, &p) in team.iter().enumerate() {
            let age = age_years(world.players().bio(p).birth_date, today);
            let listing = if rank >= 25 {
                Some(Listing::Transfer)
            } else if rank >= 16
                && age < 22.0
                && u32::from(world.players().potential(p)) + 40 >= bar
            {
                Some(Listing::Loan)
            } else {
                None
            };
            listings.push((p, listing));
        }
        // Clubs deep in debt sell their most valuable player.
        let f = world.clubs()[club].finances();
        let potential = crate::economy::revenue_potential(world.clubs()[club].reputation);
        if f.balance < -potential / 4
            && let Some(&star) = team.first()
        {
            listings.push((star, Some(Listing::Transfer)));
        }
    }
    for (p, l) in listings {
        match l {
            Some(l) => {
                world.market.listed.insert(p, l);
            }
            None => {
                world.market.listed.remove(&p);
            }
        }
    }
}

/// Players at clubs, by group, sorted by true ability, plus the open
/// offers: the day's search index.
struct Index {
    by_group: [Vec<(u16, PlayerId)>; 4],
    /// Players listed for sale, by group, sorted by true ability.
    listed: [Vec<(u16, PlayerId)>; 4],
    /// Players listed for loan, likewise.
    loanable: [Vec<(u16, PlayerId)>; 4],
    offered: std::collections::BTreeSet<(ClubId, PlayerId)>,
}

fn group_slot(g: Group) -> usize {
    match g {
        Group::Goalkeeper => 0,
        Group::Defender => 1,
        Group::Midfielder => 2,
        Group::Attacker => 3,
    }
}

impl Index {
    fn build(world: &World) -> Self {
        let (y, m, d) = world.date().ymd();
        // Born on or before this day: 18 or older.
        let adult = GameDate::from_ymd(y - 18, m, d.min(28)).expect("valid date");
        let ps = world.players();
        let loans = world.market().loans();
        let mut by_group: [Vec<(u16, PlayerId)>; 4] = Default::default();
        let mut listed: [Vec<(u16, PlayerId)>; 4] = Default::default();
        let mut loanable: [Vec<(u16, PlayerId)>; 4] = Default::default();
        for (&p, &l) in world.market().listed() {
            if ps.club(p).is_none() || loans.contains_key(&p) {
                continue;
            }
            let slot = group_slot(group_of(world, p));
            match l {
                Listing::Transfer => listed[slot].push((ps.ability(p), p)),
                Listing::Loan => loanable[slot].push((ps.ability(p), p)),
            }
        }
        for p in ps.ids() {
            if ps.club(p).is_none()
                || ps.status(p) != Status::Active
                || ps.bio(p).birth_date > adult
                || loans.contains_key(&p)
            {
                continue;
            }
            by_group[group_slot(group_of(world, p))].push((ps.ability(p), p));
        }
        for v in by_group
            .iter_mut()
            .chain(listed.iter_mut())
            .chain(loanable.iter_mut())
        {
            v.sort_unstable();
        }
        let offered = world
            .market()
            .offers()
            .iter()
            .filter(|o| o.status.is_open())
            .map(|o| (o.buyer, o.player))
            .collect();
        Self {
            by_group,
            listed,
            loanable,
            offered,
        }
    }

    /// Up to `n` of the best players in `pool` with true ability in
    /// `lo..=hi`.
    fn best_in(pool: &[(u16, PlayerId)], lo: u16, hi: u16, n: usize) -> Vec<PlayerId> {
        let a = pool.partition_point(|&(x, _)| x < lo);
        let b = pool.partition_point(|&(x, _)| x <= hi);
        pool[a..b].iter().rev().take(n).map(|&(_, p)| p).collect()
    }

    /// Up to `n` players of a group listed for sale, with true ability in
    /// `lo..=hi`.
    fn listed(&self, g: Group, lo: u16, hi: u16, n: usize) -> Vec<PlayerId> {
        Self::best_in(&self.listed[group_slot(g)], lo, hi, n)
    }

    /// Likewise, players listed for loan.
    fn loanable(&self, g: Group, lo: u16, hi: u16, n: usize) -> Vec<PlayerId> {
        Self::best_in(&self.loanable[group_slot(g)], lo, hi, n)
    }

    /// Up to `n` players of a group with true ability in `lo..=hi`, spread
    /// evenly through that band, starting at `offset`.
    fn sample(&self, g: Group, lo: u16, hi: u16, n: usize, offset: usize) -> Vec<PlayerId> {
        let v = &self.by_group[group_slot(g)];
        let a = v.partition_point(|&(x, _)| x < lo);
        let b = v.partition_point(|&(x, _)| x <= hi);
        let band = &v[a..b];
        if band.is_empty() {
            return Vec::new();
        }
        let step = (band.len() / n).max(1);
        (0..n.min(band.len()))
            .map(|k| band[(offset + k * step) % band.len()].1)
            .collect()
    }
}

/// What an AI club wants: a group and the ability a signing must beat.
fn need(world: &World, club: ClubId) -> Option<(Group, u16)> {
    let team = first_team(world, club);
    if team.len() >= MAX_FIRST_TEAM {
        return None;
    }
    let ps = world.players();
    let xi: Vec<u16> = team.iter().take(11).map(|&p| ps.ability(p)).collect();
    let bar = xi.iter().map(|&x| u32::from(x)).sum::<u32>() / xi.len().max(1) as u32;
    let mut best: Option<(Group, u16, i32)> = None;
    for (g, starters) in STARTERS {
        let mut abil: Vec<u16> = team
            .iter()
            .filter(|&&p| group_of(world, p) == g)
            .map(|&p| ps.ability(p))
            .collect();
        abil.sort_unstable_by(|a, b| b.cmp(a));
        let min = MIN_GROUP.iter().find(|m| m.0 == g).map_or(0, |m| m.1);
        // Gap: a missing body counts as a big hole; otherwise the weakest
        // starter's shortfall against the team.
        let (gap, target) = if abil.len() < min {
            (200, (bar as i32 - 60).max(0) as u16)
        } else {
            let weakest = abil.get(starters - 1).copied().unwrap_or(0);
            let short = bar as i32 - i32::from(weakest);
            (short, weakest.saturating_add(30))
        };
        if gap > 40 && best.is_none_or(|b| gap > b.2) {
            best = Some((g, target, gap));
        }
    }
    best.map(|(g, t, _)| (g, t))
}

/// Years an AI club offers a player of this age.
fn years_for(age: f64) -> u8 {
    match age {
        a if a < 24.0 => 4,
        a if a < 29.0 => 3,
        a if a < 32.0 => 2,
        _ => 1,
    }
}

/// One AI club's search: the best player it believes beats its target,
/// can afford, and could persuade to come.
fn search(world: &World, index: &Index, club: ClubId) -> Option<Offer> {
    let today = world.date();
    let (group, target) = need(world, club)?;
    let c = &world.clubs()[club];
    let f = c.finances();
    let rep = c.reputation;
    let wage_room = f.wage_budget - crate::economy::player_wage_bill(world, club);
    let ratings = staff::ratings(world, club);
    let mut rng = world.seed().derive(
        Stream::Transfers,
        u64::from(today.days()),
        u64::from(club.raw()),
    );
    let (lo, hi) = (target.saturating_sub(40), target.saturating_add(220));
    // Listed players get looked at first.
    let mut candidates = index.listed(group, lo, hi, CANDIDATES / 2);
    candidates.extend(index.sample(group, lo, hi, CANDIDATES, rng.below(1_000) as usize));
    let mut best: Option<(f64, PlayerId, Money, u32)> = None;
    for p in candidates {
        let Some(seller) = world.players().club(p) else {
            continue;
        };
        if seller == club || index.offered.contains(&(club, p)) {
            continue;
        }
        let age = age_years(world.players().bio(p).birth_date, today);
        let max_age = if group == Group::Goalkeeper {
            33.0
        } else {
            31.0
        };
        if age > max_age {
            continue;
        }
        let listed = world.market().listing(p) == Some(Listing::Transfer);
        if !listed && u32::from(world.clubs()[seller].reputation) > u32::from(rep) + 1_500 {
            continue;
        }
        let seen = scouting::perceived_ability(world, club, p, &ratings);
        if seen < f64::from(target) {
            continue;
        }
        let ps = world.players();
        let demand = wage_demand(ps.ability(p), ps.potential(p), age as u32);
        let wage = (f64::from(demand) * 1.1) as u32;
        if Money::from(wage) > wage_room {
            continue;
        }
        let years = ps
            .contract(p)
            .map_or(0.0, |c| c.expires.days_since(today).max(0) as f64 / 365.25);
        let fee = round_money(
            valuation(seen, f64::from(ps.potential(p)).max(seen), age, years) as f64
                * rng.range_f64(0.85, 1.05),
        );
        if fee > f.transfer_budget {
            continue;
        }
        if best.is_none_or(|b| seen > b.0) {
            best = Some((seen, p, fee, wage));
        }
    }
    let (_, player, fee, wage) = best?;
    let age = age_years(world.players().bio(player).birth_date, today);
    Some(Offer {
        id: 0,
        date: today,
        buyer: club,
        seller: world.players().club(player)?,
        player,
        kind: OfferKind::Transfer { fee },
        wage,
        years: years_for(age),
        status: OfferStatus::Pending,
        respond_by: today.add_days(1),
        updated: today,
    })
}

/// A club that cannot afford a signing borrows a listed youngster.
fn loan_search(world: &World, index: &Index, club: ClubId) -> Option<Offer> {
    let today = world.date();
    let (group, target) = need(world, club)?;
    let rep = world.clubs()[club].reputation;
    let ratings = staff::ratings(world, club);
    let (lo, hi) = (target.saturating_sub(60), target.saturating_add(250));
    let (player, _) = index
        .loanable(group, lo, hi, CANDIDATES)
        .into_iter()
        .filter(|&p| {
            world.players().club(p).is_some_and(|s| {
                s != club && u32::from(world.clubs()[s].reputation) <= u32::from(rep) + 4_000
            }) && !index.offered.contains(&(club, p))
        })
        .map(|p| (p, scouting::perceived_ability(world, club, p, &ratings)))
        .filter(|&(_, seen)| seen + 30.0 >= f64::from(target))
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))?;
    Some(Offer {
        id: 0,
        date: today,
        buyer: club,
        seller: world.players().club(player)?,
        player,
        kind: OfferKind::Loan { wage_share: 75 },
        wage: 0,
        years: 0,
        status: OfferStatus::Pending,
        respond_by: today.add_days(1),
        updated: today,
    })
}

/// A few AI clubs look for players each window day.
fn make_offers(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let seed = world.seed();
    let active: Vec<ClubId> = world
        .clubs()
        .ids()
        .filter(|&c| world.ai_runs_squad(c))
        .filter(|&c| {
            seed.derive(
                Stream::Transfers,
                u64::from(today.days()),
                u64::from(c.raw()) | 1 << 40,
            )
            .chance(DAILY_ACTIVITY)
        })
        .collect();
    if active.is_empty() {
        return;
    }
    let index = Index::build(world);
    let mut proposals: Vec<Option<Offer>> = vec![None; active.len()];
    {
        let world = &*world;
        par::for_each_heavy(&mut proposals, |k, slot| {
            let club = active[k];
            *slot = search(world, &index, club).or_else(|| loan_search(world, &index, club));
        });
    }
    for offer in proposals.into_iter().flatten() {
        let id = world.market.add(offer);
        if world.is_human_club(offer.seller) {
            let o = world.market.offer_mut(id).expect("added");
            o.respond_by = today.add_days(HUMAN_RESPONSE_DAYS);
            events.push(Event::OfferReceived {
                offer: id,
                player: offer.player,
                buyer: offer.buyer,
                seller: offer.seller,
            });
        }
    }
}

/// The day a selling club must answer a new offer by.
pub(crate) fn respond_by(world: &World, seller: ClubId) -> GameDate {
    let days = if is_human(world, seller) {
        HUMAN_RESPONSE_DAYS
    } else {
        1
    };
    world.date().add_days(days)
}
