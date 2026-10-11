//! How money moves: the revenue model, monthly income and costs, gate
//! receipts, prize money, and the budgets boards set each summer.
//!
//! The model is anchored to the wage scale. A club's *revenue potential*
//! is what it would need to pay a squad of its standing about 60% of its
//! income in wages, so clubs that live within their budgets stay solvent,
//! and clubs that overspend drift into debt. See `docs/WORLD.md` section 7.

use crate::finance::{Ledger, Money};
use crate::ids::{ClubId, LeagueId};
use crate::world::World;
use fs_core::{Stream, math};

/// Revenue potential per unit of [`wage_scale`]: a 25-man squad's wage
/// spread, over a year, at 60% of income. Tuned with `fs-cli economy`.
const REVENUE_PER_WAGE_UNIT: f64 = 3_900.0;
/// Shares of revenue potential.
const BROADCAST_SHARE: f64 = 0.35;
const PRIZE_SHARE: f64 = 0.10;
const RUNNING_COST_SHARE: f64 = 0.15;
/// Share of revenue potential the board allows in player wages.
pub const WAGE_BUDGET_SHARE: f64 = 0.66;
/// Months of wages a board keeps in reserve before releasing money for
/// transfers.
const RESERVE_MONTHS: f64 = 3.0;

/// The weekly wage of a typical first-team player at a club of this
/// reputation (0-10000).
pub fn wage_scale(reputation: u16) -> f64 {
    let ability = 0.085 * f64::from(reputation) + 60.0;
    1000.0 * math::exp((ability - 400.0) / 80.0)
}

/// What a club of this reputation can earn in a season, roughly.
pub fn revenue_potential(reputation: u16) -> Money {
    (REVENUE_PER_WAGE_UNIT * wage_scale(reputation)) as Money
}

/// Supporters who would come to a home game.
pub fn fans(reputation: u16) -> u32 {
    let r = f64::from(reputation) / 10_000.0;
    (1_200.0 + 65_000.0 * math::powf(r, 2.5)) as u32
}

/// Average ticket price.
pub fn ticket_price(reputation: u16) -> Money {
    let r = f64::from(reputation) / 10_000.0;
    (4.0 + 75.0 * r * r).round() as Money
}

/// Mean revenue potential of a league's clubs.
fn league_mean_potential(world: &World, league: LeagueId) -> f64 {
    let clubs = world.leagues()[league].clubs();
    let total: f64 = clubs
        .iter()
        .map(|&c| revenue_potential(world.clubs()[c].reputation) as f64)
        .sum();
    total / clubs.len().max(1) as f64
}

/// A club's yearly broadcast money: an equal share of its league's pot.
pub fn broadcast_income(world: &World, club: ClubId) -> Money {
    world.clubs()[club].league().map_or(0, |l| {
        (BROADCAST_SHARE * league_mean_potential(world, l)) as Money
    })
}

/// Expected gate receipts over a season.
fn expected_matchday(world: &World, club: ClubId) -> Money {
    let c = &world.clubs()[club];
    let home_games =
        c.league()
            .map_or(0, |l| world.leagues()[l].clubs().len().saturating_sub(1)) as Money;
    let crowd = Money::from(fans(c.reputation).min(c.facilities.capacity));
    home_games * crowd * ticket_price(c.reputation)
}

/// Yearly sponsorship and merchandise: whatever the club's standing earns
/// beyond gates, television and prizes.
pub fn commercial_income(world: &World, club: ClubId) -> Money {
    let rep = world.clubs()[club].reputation;
    let potential = revenue_potential(rep);
    let prize = world.clubs()[club].league().map_or(0, |l| {
        (PRIZE_SHARE * league_mean_potential(world, l)) as Money
    });
    let rest = potential - expected_matchday(world, club) - broadcast_income(world, club) - prize;
    rest.max(potential / 50)
}

/// Weekly wages a club pays its players, including its share for players
/// on loan in either direction.
pub fn player_wage_bill(world: &World, club: ClubId) -> Money {
    let loaned_out: Money = world
        .market()
        .loans()
        .iter()
        .filter(|(_, l)| l.parent == club)
        .map(|(&p, _)| crate::transfer::wage_paid(world, club, p))
        .sum();
    squad_wage_bill(world, club) + loaned_out
}

/// Weekly wages a club pays its own squad, loanees included.
fn squad_wage_bill(world: &World, club: ClubId) -> Money {
    world.clubs()[club]
        .squad()
        .iter()
        .map(|&p| crate::transfer::wage_paid(world, club, p))
        .sum()
}

/// Weekly wages each club still pays for players it has loaned out,
/// indexed by club.
fn loaned_out_bills(world: &World) -> Vec<Money> {
    let mut bills = vec![0; world.clubs().len()];
    for (&p, l) in world.market().loans() {
        bills[l.parent.index()] += crate::transfer::wage_paid(world, l.parent, p);
    }
    bills
}

/// Weekly wages of a club's staff.
pub fn staff_wage_bill(world: &World, club: ClubId) -> Money {
    world.clubs()[club]
        .staff()
        .iter()
        .filter_map(|&s| world.staff()[s].contract())
        .map(|c| Money::from(c.wage))
        .sum()
}

/// Weekly to monthly.
fn monthly(weekly: Money) -> Money {
    weekly * 52 / 12
}

/// The 1st of the month: sponsorship and television money in, wages and
/// running costs out.
pub(crate) fn monthly_accounts(world: &mut World) {
    let loaned_out = loaned_out_bills(world);
    let flows: Vec<(ClubId, [Money; 5])> = world
        .clubs()
        .ids()
        .map(|c| {
            let rep = world.clubs()[c].reputation;
            (
                c,
                [
                    commercial_income(world, c) / 12,
                    broadcast_income(world, c) / 12,
                    monthly(squad_wage_bill(world, c) + loaned_out[c.index()]),
                    monthly(staff_wage_bill(world, c)),
                    (RUNNING_COST_SHARE * revenue_potential(rep) as f64 / 12.0) as Money,
                ],
            )
        })
        .collect();
    for (c, [commercial, broadcast, wages, staff, running]) in flows {
        let f = &mut world.clubs[c].finances;
        f.earn(|l| &mut l.commercial, commercial);
        f.earn(|l| &mut l.broadcast, broadcast);
        f.spend(|l| &mut l.player_wages, wages);
        f.spend(|l| &mut l.staff_wages, staff);
        f.spend(|l| &mut l.running_costs, running);
    }
}

/// Gate receipts for a home match. Good form and a big visitor bring a
/// bigger crowd. Returns the attendance.
pub(crate) fn gate_receipts(world: &mut World, home: ClubId, away: ClubId, key: u64) -> u32 {
    let c = &world.clubs[home];
    let mut rng = world
        .seed
        .derive(Stream::Finance, u64::from(home.raw()), key);
    let pull = 0.9 + 0.2 * f64::from(world.clubs[away].reputation) / 10_000.0;
    let demand = f64::from(fans(c.reputation)) * pull * rng.range_f64(0.85, 1.1);
    let crowd = (demand as u32).min(c.facilities.capacity);
    let takings = Money::from(crowd) * ticket_price(c.reputation);
    world.clubs[home]
        .finances
        .earn(|l| &mut l.matchday, takings);
    crowd
}

/// End of season (before promotion and relegation): prize money by final
/// position, then each club's accounts are archived.
pub(crate) fn close_accounts(world: &mut World, year: i32) {
    let mut prizes: Vec<(ClubId, Money)> = Vec::new();
    for (lid, league) in world.leagues().iter() {
        let Some(season) = league.season() else {
            continue;
        };
        let table = season.table();
        let n = table.len() as f64;
        let pot = PRIZE_SHARE * league_mean_potential(world, lid) * n;
        let weights = n * (n + 1.0) / 2.0;
        for (i, row) in table.iter().enumerate() {
            prizes.push((row.club, (pot * (n - i as f64) / weights) as Money));
        }
    }
    for (club, prize) in prizes {
        world.clubs[club]
            .finances
            .earn(|l| &mut l.prize_money, prize);
    }
    for club in world.clubs.as_mut_slice() {
        club.finances.archive(year);
    }
}

/// The summer: boards set wage and transfer budgets from what the club
/// can earn and what it has in the bank.
pub(crate) fn set_budgets(world: &mut World) {
    let budgets: Vec<(Money, Money)> = world
        .clubs()
        .ids()
        .map(|c| {
            let club = &world.clubs()[c];
            let potential = revenue_potential(club.reputation) as f64;
            let base = (WAGE_BUDGET_SHARE * potential / 52.0) as Money;
            let reserve = (RESERVE_MONTHS * monthly(base) as f64) as Money;
            let spare = club.finances.balance - reserve;
            // Savings stretch the wage budget a little: a quarter of them
            // spread over four seasons.
            let wage_budget = base + spare.max(0) / 4 / (4 * 52);
            (wage_budget, (spare / 2).max(0))
        })
        .collect();
    for (club, (wages, transfers)) in world.clubs.as_mut_slice().iter_mut().zip(budgets) {
        club.finances.wage_budget = wages;
        club.finances.transfer_budget = transfers;
    }
}

/// Facilities wear: the chance each season that a rating drops a point.
const WEAR: f64 = 0.2;
/// Cost of one point of training or youth facilities, as a share of
/// revenue potential.
const FACILITY_COST: f64 = 0.25;
/// Cost of adding a tenth to the stadium, as a share of revenue potential.
const STADIUM_COST: f64 = 0.2;

/// The summer: facilities wear, and boards with money to spare invest in
/// one improvement: a bigger stadium if it is full, otherwise the weaker
/// of the training ground and the academy.
pub(crate) fn invest(world: &mut World) {
    let year = crate::people::season_year(world.date()) as u64;
    let seed = world.seed();
    for (k, club) in world.clubs.as_mut_slice().iter_mut().enumerate() {
        let mut rng = seed.derive(Stream::Finance, year, k as u64 | 1 << 40);
        let f = &mut club.facilities;
        if rng.chance(WEAR) {
            f.training = f.training.saturating_sub(1).max(1);
        }
        if rng.chance(WEAR) {
            f.youth = f.youth.saturating_sub(1).max(1);
        }
        let potential = revenue_potential(club.reputation) as f64;
        let reserve = RESERVE_MONTHS / 12.0 * WAGE_BUDGET_SHARE * potential;
        let spare = club.finances.balance as f64 - reserve;
        if spare < 0.5 * potential {
            continue;
        }
        let (cost, upgrade): (f64, fn(&mut crate::finance::Facilities)) = if fans(club.reputation)
            > club.facilities.capacity
        {
            (STADIUM_COST, |f| f.capacity += f.capacity / 10)
        } else if club.facilities.training <= club.facilities.youth && club.facilities.training < 20
        {
            (FACILITY_COST, |f| f.training += 1)
        } else if club.facilities.youth < 20 {
            (FACILITY_COST, |f| f.youth += 1)
        } else {
            continue;
        };
        upgrade(&mut club.facilities);
        club.finances
            .spend(|l| &mut l.investment, (cost * potential) as Money);
    }
}

/// A new world's starting money: a cushion of a quarter of a season's
/// revenue, then budgets.
pub fn initialise(world: &mut World) {
    for club in world.clubs.as_mut_slice() {
        club.finances.balance = revenue_potential(club.reputation) / 4;
        club.finances.season = Ledger::default();
    }
    set_budgets(world);
}

/// The state of the world's finances, for health checks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EconomyHealth {
    /// Player wages over operating income in the last season: median,
    /// 10th and 90th percentiles.
    pub wage_ratio: [f64; 3],
    /// Share of clubs with a negative balance.
    pub in_debt: f64,
    /// Balance over revenue potential: median, 10th and 90th percentiles.
    pub balance_ratio: [f64; 3],
}

fn percentiles(mut v: Vec<f64>) -> [f64; 3] {
    if v.is_empty() {
        return [0.0; 3];
    }
    v.sort_by(f64::total_cmp);
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    [at(0.5), at(0.1), at(0.9)]
}

/// Measures the economy from the last archived season (or the current
/// one, before any has been archived).
pub fn health(world: &World) -> EconomyHealth {
    let mut wages = Vec::new();
    let mut balances = Vec::new();
    let mut debt = 0;
    for (_, c) in world.clubs().iter() {
        let f = c.finances();
        let ledger = f.history.last().map_or(f.season, |a| a.ledger);
        if ledger.operating_income() > 0 {
            wages.push(ledger.player_wages as f64 / ledger.operating_income() as f64);
        }
        balances.push(f.balance as f64 / revenue_potential(c.reputation).max(1) as f64);
        debt += usize::from(f.balance < 0);
    }
    EconomyHealth {
        wage_ratio: percentiles(wages),
        in_debt: debt as f64 / world.clubs().len().max(1) as f64,
        balance_ratio: percentiles(balances),
    }
}

impl std::fmt::Display for EconomyHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [m, lo, hi] = self.wage_ratio;
        let [bm, blo, bhi] = self.balance_ratio;
        write!(
            f,
            "wages/income {:.0}% ({:.0}-{:.0}%), balance/potential {:.2} ({:.2} to {:.2}), in debt {:.0}%",
            m * 100.0,
            lo * 100.0,
            hi * 100.0,
            bm,
            blo,
            bhi,
            self.in_debt * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bigger_clubs_earn_more_and_draw_more() {
        assert!(revenue_potential(9_000) > 100 * revenue_potential(3_000));
        assert!(fans(9_000) > 40_000 && fans(1_000) < 3_000);
        assert!(ticket_price(9_000) > ticket_price(2_000));
    }
}
