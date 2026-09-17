//! Calendar dates in the proleptic Gregorian calendar, without time or time zone.
//!
//! The arithmetic counts days since 1970-01-01 with Howard Hinnant's civil-calendar
//! algorithms, which are exact for every date an `i32` year can hold.

use std::time::{SystemTime, UNIX_EPOCH};

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    /// Monday to Sunday.
    pub const ALL: [Self; 7] =
        [Self::Monday, Self::Tuesday, Self::Wednesday, Self::Thursday, Self::Friday, Self::Saturday, Self::Sunday];

    /// 1 for Monday up to 7 for Sunday (ISO 8601).
    #[must_use]
    pub fn number(self) -> u8 {
        match self {
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
            Self::Sunday => 7,
        }
    }

    /// The weekday numbered `number` (1 Monday to 7 Sunday).
    #[must_use]
    pub fn from_number(number: u8) -> Option<Self> {
        Self::ALL.get(usize::from(number).checked_sub(1)?).copied()
    }

    /// Days from `start` to this weekday going forward, `0..7`.
    #[must_use]
    pub fn days_since(self, start: Self) -> u8 {
        (self.number() + 7 - start.number()) % 7
    }
}

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    /// The date `year`-`month`-`day`, or `None` when no such day exists.
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        (1..=12).contains(&month).then_some(())?;
        (day >= 1 && day <= days_in_month(year, month)).then_some(Self { year, month, day })
    }

    /// Today in UTC, from the system clock. The framework does not read the local time zone;
    /// applications that know it can pass their own "today" to date widgets.
    #[must_use]
    pub fn today_utc() -> Self {
        let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
        Self::from_days(i64::try_from(seconds / 86_400).unwrap_or(0))
    }

    /// The year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    /// The month, 1 to 12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, from 1.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }

    /// Days since 1970-01-01 (negative before).
    #[must_use]
    pub fn to_days(self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year.rem_euclid(400);
        let month = i64::from(self.month);
        let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The date `days` after 1970-01-01. Years beyond the range of `i32` are clamped to it.
    #[must_use]
    pub fn from_days(days: i64) -> Self {
        let z = days.saturating_add(719_468);
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let mp = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Self {
            year: i32::try_from(year).unwrap_or(if year < 0 { i32::MIN } else { i32::MAX }),
            month: u8::try_from(month).unwrap_or(1),
            day: u8::try_from(day).unwrap_or(1),
        }
    }

    /// The day of the week.
    #[must_use]
    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday.
        let index = (self.to_days() + 3).rem_euclid(7);
        Weekday::ALL[usize::try_from(index).unwrap_or(0)]
    }

    /// The date `days` later (earlier when negative).
    #[must_use]
    pub fn add_days(self, days: i64) -> Self {
        Self::from_days(self.to_days().saturating_add(days))
    }

    /// The same day `months` later (earlier when negative), moved back to the last day of a
    /// shorter month: January 31 plus one month is February 28 or 29.
    #[must_use]
    pub fn add_months(self, months: i32) -> Self {
        let index = i64::from(self.year) * 12 + i64::from(self.month) - 1 + i64::from(months);
        let year = i32::try_from(index.div_euclid(12)).unwrap_or(self.year);
        let month = u8::try_from(index.rem_euclid(12) + 1).unwrap_or(1);
        let day = self.day.min(days_in_month(year, month));
        Self { year, month, day }
    }

    /// The first day of this date's month.
    #[must_use]
    pub fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    /// The date the week containing this date starts on, for weeks starting on `start`.
    #[must_use]
    pub fn start_of_week(self, start: Weekday) -> Self {
        self.add_days(-i64::from(self.weekday().days_since(start)))
    }
}

/// Whether `year` has a February 29.
#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// How many days `month` of `year` has; 0 for a month outside 1 to 12.
#[must_use]
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).expect("valid date")
    }

    #[test]
    fn validates_days_and_leap_years() {
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(2026, 2, 29).is_none());
        assert!(Date::new(1900, 2, 29).is_none());
        assert!(Date::new(2000, 2, 29).is_some());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::new(2026, 4, 31).is_none());
        assert!(Date::new(2026, 4, 0).is_none());
    }

    #[test]
    fn counts_days_from_the_epoch_both_ways() {
        assert_eq!(date(1970, 1, 1).to_days(), 0);
        assert_eq!(date(2000, 3, 1).to_days(), 11_017);
        assert_eq!(date(1969, 12, 31).to_days(), -1);
        assert_eq!(date(2026, 9, 16).to_days(), 20_712);
        for days in (-800_000..800_000).step_by(997) {
            assert_eq!(Date::from_days(days).to_days(), days);
        }
        assert_eq!(Date::from_days(-719_468), date(0, 3, 1));
    }

    #[test]
    fn knows_weekdays() {
        assert_eq!(date(1970, 1, 1).weekday(), Weekday::Thursday);
        assert_eq!(date(2026, 9, 16).weekday(), Weekday::Wednesday);
        assert_eq!(date(2000, 1, 1).weekday(), Weekday::Saturday);
        assert_eq!(date(1900, 1, 1).weekday(), Weekday::Monday);
        assert_eq!(Weekday::Sunday.days_since(Weekday::Monday), 6);
        assert_eq!(Weekday::Monday.days_since(Weekday::Sunday), 1);
        assert_eq!(Weekday::from_number(7), Some(Weekday::Sunday));
        assert_eq!(Weekday::from_number(0), None);
    }

    #[test]
    fn adds_days_and_months() {
        assert_eq!(date(2026, 12, 31).add_days(1), date(2027, 1, 1));
        assert_eq!(date(2024, 3, 1).add_days(-1), date(2024, 2, 29));
        assert_eq!(date(2026, 1, 31).add_months(1), date(2026, 2, 28));
        assert_eq!(date(2024, 1, 31).add_months(1), date(2024, 2, 29));
        assert_eq!(date(2026, 1, 15).add_months(-1), date(2025, 12, 15));
        assert_eq!(date(2026, 3, 31).add_months(-13), date(2025, 2, 28));
        assert_eq!(date(2026, 9, 16).add_months(24), date(2028, 9, 16));
    }

    #[test]
    fn extreme_day_counts_do_not_overflow() {
        assert_eq!(Date::from_days(i64::MAX).year(), i32::MAX);
        assert_eq!(Date::from_days(i64::MIN).year(), i32::MIN);
        assert_eq!(date(2026, 9, 16).add_days(i64::MAX).year(), i32::MAX);
    }

    #[test]
    fn finds_week_starts() {
        let wednesday = date(2026, 9, 16);
        assert_eq!(wednesday.start_of_week(Weekday::Monday), date(2026, 9, 14));
        assert_eq!(wednesday.start_of_week(Weekday::Sunday), date(2026, 9, 13));
        assert_eq!(date(2026, 9, 13).start_of_week(Weekday::Sunday), date(2026, 9, 13));
        assert_eq!(wednesday.first_of_month(), date(2026, 9, 1));
    }

    #[test]
    fn today_is_a_real_date() {
        let today = Date::today_utc();
        assert!(today.year() >= 2024 && Date::new(today.year(), today.month(), today.day()).is_some());
    }
}
