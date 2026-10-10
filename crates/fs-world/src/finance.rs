//! Club money: income, spending, budgets and accounts.
//!
//! Amounts are whole currency units in `i64`. Income and spending are
//! recorded in the club's season ledger as they happen, and the ledger is
//! archived at the 1 July rollover. See `docs/WORLD.md` section 7.

use serde::{Deserialize, Serialize};

/// An amount of money.
pub type Money = i64;

/// A season's income and spending, by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    pub matchday: Money,
    pub broadcast: Money,
    pub commercial: Money,
    pub prize_money: Money,
    pub player_sales: Money,
    pub player_wages: Money,
    pub staff_wages: Money,
    pub player_purchases: Money,
    pub running_costs: Money,
}

impl Ledger {
    pub fn income(&self) -> Money {
        self.matchday + self.broadcast + self.commercial + self.prize_money + self.player_sales
    }

    pub fn expenses(&self) -> Money {
        self.player_wages + self.staff_wages + self.player_purchases + self.running_costs
    }

    /// Income minus spending.
    pub fn net(&self) -> Money {
        self.income() - self.expenses()
    }

    /// Income other than player sales: what the club earns by playing.
    pub fn operating_income(&self) -> Money {
        self.income() - self.player_sales
    }
}

/// One finished season's accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeasonAccounts {
    /// The season's first year.
    pub year: i32,
    pub ledger: Ledger,
    /// Balance on 30 June.
    pub closing_balance: Money,
}

/// A club's money.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finances {
    /// Cash in the bank; negative when overdrawn.
    pub balance: Money,
    /// What the board lets the manager spend on fees this season, net of
    /// sales already banked into it.
    pub transfer_budget: Money,
    /// The most the board lets the club pay in player wages, per week.
    pub wage_budget: Money,
    /// This season so far.
    pub season: Ledger,
    /// Finished seasons, oldest first (the last ten are kept).
    pub history: Vec<SeasonAccounts>,
}

/// Seasons of accounts kept.
pub const HISTORY_SEASONS: usize = 10;

impl Finances {
    /// Records income: adds to the balance and to one ledger line.
    pub(crate) fn earn(&mut self, line: fn(&mut Ledger) -> &mut Money, amount: Money) {
        self.balance += amount;
        *line(&mut self.season) += amount;
    }

    /// Records spending.
    pub(crate) fn spend(&mut self, line: fn(&mut Ledger) -> &mut Money, amount: Money) {
        self.balance -= amount;
        *line(&mut self.season) += amount;
    }

    /// Closes the season's ledger.
    pub(crate) fn archive(&mut self, year: i32) {
        self.history.push(SeasonAccounts {
            year,
            ledger: self.season,
            closing_balance: self.balance,
        });
        if self.history.len() > HISTORY_SEASONS {
            self.history.remove(0);
        }
        self.season = Ledger::default();
    }
}

/// A club's ground and training facilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facilities {
    /// Stadium capacity.
    pub capacity: u32,
    /// Training ground, 1-20.
    pub training: u8,
    /// Youth academy, 1-20.
    pub youth: u8,
}

impl Default for Facilities {
    fn default() -> Self {
        Self {
            capacity: 10_000,
            training: 10,
            youth: 10,
        }
    }
}

/// The facilities rating (1-20) a club of this reputation usually has.
pub fn usual_facilities(reputation: u16) -> f64 {
    4.0 + 13.0 * f64::from(reputation) / 10_000.0
}
