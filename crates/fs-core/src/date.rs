//! The in-game calendar.
//!
//! A [`GameDate`] is a day count from 1 January 1900, so date arithmetic is
//! integer arithmetic. Conversion to and from year/month/day uses Howard
//! Hinnant's proleptic Gregorian algorithms.

use core::fmt;
use serde::{Deserialize, Serialize};

/// A calendar day. Day 0 is 1900-01-01.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GameDate(u32);

/// Day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Days from 0000-03-01 (Hinnant's era epoch) to 1900-01-01.
const EPOCH_SHIFT: i64 = 693_901;

impl GameDate {
    /// Day 0: 1 January 1900.
    pub const EPOCH: GameDate = GameDate(0);

    /// Builds a date from its day number.
    pub const fn from_days(days: u32) -> Self {
        Self(days)
    }

    /// Days since 1900-01-01.
    pub const fn days(self) -> u32 {
        self.0
    }

    /// Builds a date from year, month (1-12) and day (1-31). Returns `None`
    /// for invalid dates or dates before 1900.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        let y = i64::from(year) - i64::from(month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(month);
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - EPOCH_SHIFT;
        u32::try_from(days).ok().map(Self)
    }

    /// Year, month (1-12) and day (1-31).
    pub fn ymd(self) -> (i32, u32, u32) {
        let z = i64::from(self.0) + EPOCH_SHIFT;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        (year, month, day)
    }

    /// The year.
    pub fn year(self) -> i32 {
        self.ymd().0
    }

    /// Day of the week. 1900-01-01 was a Monday.
    pub fn weekday(self) -> Weekday {
        use Weekday::*;
        [
            Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,
        ][(self.0 % 7) as usize]
    }

    /// The date `n` days later.
    ///
    /// # Panics
    /// Panics on overflow (more than eleven million years in the future).
    pub fn add_days(self, n: u32) -> Self {
        Self(self.0.checked_add(n).expect("GameDate overflow"))
    }

    /// Whole days from `earlier` to `self`, negative if `self` is earlier.
    pub fn days_since(self, earlier: GameDate) -> i64 {
        i64::from(self.0) - i64::from(earlier.0)
    }

    /// Age in whole years on `self` of someone born on `birth`.
    pub fn age_of(self, birth: GameDate) -> u32 {
        let (y, m, d) = self.ymd();
        let (by, bm, bd) = birth.ymd();
        let mut age = y - by;
        if (m, d) < (bm, bd) {
            age -= 1;
        }
        age.max(0) as u32
    }
}

/// Whether `year` is a Gregorian leap year.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in a month (1-12). Returns 0 for an invalid month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl fmt::Debug for GameDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for GameDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch() {
        assert_eq!(GameDate::from_ymd(1900, 1, 1), Some(GameDate::EPOCH));
        assert_eq!(GameDate::EPOCH.ymd(), (1900, 1, 1));
        assert_eq!(GameDate::EPOCH.weekday(), Weekday::Monday);
    }

    #[test]
    fn known_dates() {
        // 2000-01-01 is 36524 days after 1900-01-01 and was a Saturday.
        let d = GameDate::from_ymd(2000, 1, 1).unwrap();
        assert_eq!(d.days(), 36_524);
        assert_eq!(d.weekday(), Weekday::Saturday);
        let d = GameDate::from_ymd(2026, 10, 10).unwrap();
        assert_eq!(d.weekday(), Weekday::Saturday);
        assert_eq!(d.to_string(), "2026-10-10");
    }

    #[test]
    fn round_trip_every_day_for_three_centuries() {
        for days in 0..(300 * 366) {
            let d = GameDate::from_days(days);
            let (y, m, dd) = d.ymd();
            assert_eq!(GameDate::from_ymd(y, m, dd), Some(d));
        }
    }

    #[test]
    fn rejects_invalid() {
        assert_eq!(GameDate::from_ymd(2023, 2, 29), None);
        assert!(GameDate::from_ymd(2024, 2, 29).is_some());
        assert_eq!(GameDate::from_ymd(1900, 2, 29), None);
        assert_eq!(GameDate::from_ymd(2000, 13, 1), None);
        assert_eq!(GameDate::from_ymd(1899, 12, 31), None);
    }

    #[test]
    fn ages() {
        let birth = GameDate::from_ymd(2000, 6, 15).unwrap();
        assert_eq!(GameDate::from_ymd(2026, 6, 14).unwrap().age_of(birth), 25);
        assert_eq!(GameDate::from_ymd(2026, 6, 15).unwrap().age_of(birth), 26);
    }
}
