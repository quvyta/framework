//! Calendar dates in the proleptic Gregorian calendar, times of day, and the two together.
//!
//! The arithmetic counts days since 1970-01-01 with Howard Hinnant's civil-calendar
//! algorithms, which are exact for every date an `i32` year can hold.
//!
//! [`Date`] and [`TimeOfDay`] carry no time zone. [`DateTime`] carries the offset from UTC of
//! the moment it holds, and [`local_offset_minutes`] reads that offset from the system; where the
//! system does not say, [`local_offset`] returns `None` instead of a made-up zero.

use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

mod zone;

/// Seconds in a day.
const DAY: i64 = 86_400;

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

    /// Today in UTC, from the system clock.
    #[must_use]
    pub fn today_utc() -> Self {
        Self::from_days(unix_now().div_euclid(DAY))
    }

    /// Today where the machine stands, from the system clock and the system time zone. Falls back
    /// to [`Date::today_utc`] where the zone is unknown; [`local_offset`] tells you which it is.
    #[must_use]
    pub fn today_local() -> Self {
        DateTime::now_local().date
    }

    /// Reads `YYYY-MM-DD`: four or more digits for the year, then two for the month and the day,
    /// with a leading `-` for a year before the era. The calendar is checked, so 2026-02-30 is an
    /// error, and surrounding spaces are ignored.
    ///
    /// ```
    /// use qframe::date::Date;
    ///
    /// assert_eq!(Date::parse("2026-09-17"), Ok(Date::new(2026, 9, 17).expect("a real day")));
    /// assert!(Date::parse("2026-02-30").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        let (negative, digits) = match trimmed.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, trimmed),
        };
        let mut parts = digits.split('-');
        let (Some(year), Some(month), Some(day), None) = (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(format!("`{trimmed}` is no date; a date is written YYYY-MM-DD"));
        };
        let number = |part: &str, width: usize, exact: bool, name: &str| -> Result<i32, String> {
            let length = if exact { part.len() == width } else { part.len() >= width };
            let digits = length && part.chars().all(|c| c.is_ascii_digit());
            part.parse::<i32>()
                .ok()
                .filter(|_| digits)
                .ok_or_else(|| format!("`{part}` is no {name}; it takes {width} digits"))
        };
        let year = number(year, 4, false, "year")?;
        let month = number(month, 2, true, "month")?;
        let day = number(day, 2, true, "day")?;
        let year = if negative { -year } else { year };
        let parts = u8::try_from(month).ok().zip(u8::try_from(day).ok());
        parts
            .and_then(|(month, day)| Self::new(year, month, day))
            .ok_or_else(|| format!("{trimmed} is not a day this calendar has"))
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

    /// The date the way the active language writes it, with the whole month name:
    /// `September 18, 2026`, `18. September 2026`, `2026年9月18日`.
    ///
    /// This is the form a [`DatePicker`](crate::widgets::DatePicker) shows, so an application
    /// that writes a date of its own with it puts both in the same order. Composing a date by
    /// hand is what makes one screen say `September 18` beside a field saying `18 September`.
    ///
    /// A language whose month names change inside a date gives that form as
    /// `quvyta.date.month-in-date-*`: Russian `января` where the heading says `Январь`.
    #[must_use]
    pub fn written(self) -> String {
        crate::t!("quvyta.date.format", day = u32::from(self.day), month = self.month_in_date(), year = self.year)
    }

    /// The date with the short month name, for somewhere too narrow for [`written`](Self::written):
    /// `Sep 18, 2026`, `18. Sep 2026`.
    #[must_use]
    pub fn written_short(self) -> String {
        crate::t!("quvyta.date.format-short", day = u32::from(self.day), month = self.month_short(), year = self.year)
    }

    /// The day and the month with the whole month name, without the year: `September 18`,
    /// `18 Eylül`. The form a heading over a day's own screen wants.
    #[must_use]
    pub fn day_and_month(self) -> String {
        crate::t!("quvyta.date.format-day-month-long", day = u32::from(self.day), month = self.month_in_date())
    }

    /// The day and the short month, without the year: `Sep 18`, `18. Sep`. The narrowest form a
    /// date field falls back to.
    #[must_use]
    pub fn day_and_month_short(self) -> String {
        crate::t!("quvyta.date.format-day-month", day = u32::from(self.day), month = self.month_short())
    }

    /// The whole month name in the form this language uses inside a date.
    fn month_in_date(self) -> String {
        crate::i18n::translate_active_if_known(&format!("quvyta.date.month-in-date-{}", self.month))
            .unwrap_or_else(|| crate::t!(&format!("quvyta.date.month-{}", self.month)))
    }

    /// The short month name.
    fn month_short(self) -> String {
        crate::t!(&format!("quvyta.date.month-short-{}", self.month))
    }
}

impl fmt::Display for Date {
    /// Writes `YYYY-MM-DD`, the form [`Date::parse`] reads. A year before the era keeps its sign
    /// in front of four digits, and a year of five digits or more is written in full.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (sign, year) = if self.year < 0 { ("-", self.year.unsigned_abs()) } else { ("", self.year.unsigned_abs()) };
        write!(f, "{sign}{year:04}-{:02}-{:02}", self.month, self.day)
    }
}

impl FromStr for Date {
    type Err = String;

    /// The same as [`Date::parse`].
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// A time of day on a 24-hour clock, with no date and no time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimeOfDay {
    /// Hour, 0 to 23.
    pub hour: u8,
    /// Minute, 0 to 59.
    pub minute: u8,
    /// Second, 0 to 59.
    pub second: u8,
}

impl TimeOfDay {
    /// The last moment of a day, and so the largest value of each part.
    pub const LARGEST: Self = Self { hour: 23, minute: 59, second: 59 };

    /// The time `hour:minute:second`; each part is capped at its largest value.
    #[must_use]
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour: hour.min(Self::LARGEST.hour),
            minute: minute.min(Self::LARGEST.minute),
            second: second.min(Self::LARGEST.second),
        }
    }

    /// Reads `h:mm` or `h:mm:ss` with every part in range; surrounding spaces are ignored and a
    /// missing second is zero. Out-of-range parts such as `24:00` are `None`, not capped.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.trim().split(':').collect();
        if !(2..=3).contains(&parts.len()) {
            return None;
        }
        let largest = [Self::LARGEST.hour, Self::LARGEST.minute, Self::LARGEST.second];
        let mut values = [0u8; 3];
        for (index, part) in parts.iter().enumerate() {
            let valid = (1..=2).contains(&part.len()) && part.chars().all(|c| c.is_ascii_digit());
            values[index] = part.parse().ok().filter(|value| valid && *value <= largest[index])?;
        }
        Some(Self { hour: values[0], minute: values[1], second: values[2] })
    }

    /// Seconds from midnight, 0 to 86 399.
    #[must_use]
    pub fn seconds_since_midnight(self) -> u32 {
        u32::from(self.hour) * 3_600 + u32::from(self.minute) * 60 + u32::from(self.second)
    }

    /// The time `seconds` after midnight; a value past the end of a day wraps into the next one.
    #[must_use]
    pub fn from_seconds_since_midnight(seconds: u32) -> Self {
        let seconds = seconds % 86_400;
        Self {
            hour: u8::try_from(seconds / 3_600).unwrap_or(0),
            minute: u8::try_from(seconds / 60 % 60).unwrap_or(0),
            second: u8::try_from(seconds % 60).unwrap_or(0),
        }
    }
}

impl fmt::Display for TimeOfDay {
    /// Writes `hh:mm:ss`, the same in every language.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

/// A date and a time of day together, with the offset from UTC the moment was read at.
///
/// The date and the time are local: they are what a clock on the wall shows. `offset_minutes`
/// is what turns them back into an instant, which is what [`DateTime::to_unix`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateTime {
    /// The local date.
    pub date: Date,
    /// The local time of day.
    pub time: TimeOfDay,
    /// Minutes local time is ahead of UTC; negative west of it.
    pub offset_minutes: i16,
}

impl DateTime {
    /// Now where the machine stands, from the system clock and [`local_offset_minutes`].
    #[must_use]
    pub fn now_local() -> Self {
        Self::from_unix(unix_now(), local_offset_minutes())
    }

    /// The moment `seconds` after 1970-01-01 00:00 UTC, written in an offset of
    /// `offset_minutes`.
    #[must_use]
    pub fn from_unix(seconds: i64, offset_minutes: i16) -> Self {
        let local = seconds.saturating_add(i64::from(offset_minutes) * 60);
        let day_seconds = u32::try_from(local.rem_euclid(DAY)).unwrap_or(0);
        Self {
            date: Date::from_days(local.div_euclid(DAY)),
            time: TimeOfDay::from_seconds_since_midnight(day_seconds),
            offset_minutes,
        }
    }

    /// Seconds since 1970-01-01 00:00 UTC.
    ///
    /// ```
    /// use qframe::date::DateTime;
    ///
    /// // Noon in Istanbul is 09:00 UTC.
    /// let noon = DateTime::from_unix(1_773_997_200, 180);
    /// assert_eq!(noon.time.to_string(), "12:00:00");
    /// assert_eq!(noon.to_unix(), 1_773_997_200);
    /// ```
    #[must_use]
    pub fn to_unix(&self) -> i64 {
        self.date
            .to_days()
            .saturating_mul(DAY)
            .saturating_add(i64::from(self.time.seconds_since_midnight()))
            .saturating_sub(i64::from(self.offset_minutes) * 60)
    }
}

/// Minutes local time is ahead of UTC right now, or `None` when the system does not say.
///
/// The offset is read from the system time zone file — what `TZ` names, or `/etc/localtime` —
/// once per process. `None` means there was no such file or it could not be understood: Windows,
/// a system with no zone database, or a `TZ` holding a POSIX rule rather than a zone name. An
/// application that wants to be exact can tell its user the local time is unknown instead of
/// showing UTC as if it were local.
#[must_use]
pub fn local_offset() -> Option<i16> {
    zone::offset_minutes(unix_now())
}

/// Minutes local time is ahead of UTC right now, and 0 when the system does not say.
///
/// Use [`local_offset`] where the difference between "UTC" and "unknown" matters.
#[must_use]
pub fn local_offset_minutes() -> i16 {
    local_offset().unwrap_or(0)
}

/// Seconds since 1970-01-01 00:00 UTC, from the system clock; negative before it.
fn unix_now() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => i64::try_from(since.as_secs()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_secs()).unwrap_or(i64::MAX).saturating_neg(),
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

    #[test]
    fn reads_and_writes_iso_dates() {
        assert_eq!(Date::parse("2026-09-17"), Ok(date(2026, 9, 17)));
        assert_eq!("2026-09-17".parse::<Date>(), Ok(date(2026, 9, 17)));
        assert_eq!(Date::parse("  2026-09-17\n"), Ok(date(2026, 9, 17)), "spaces around it are ignored");
        assert_eq!(date(2026, 9, 17).to_string(), "2026-09-17");
        assert_eq!(date(999, 1, 2).to_string(), "0999-01-02");
        assert_eq!(Date::parse("-0044-03-15"), Ok(date(-44, 3, 15)));
        assert_eq!(date(-44, 3, 15).to_string(), "-0044-03-15");
        for days in (-400_000..400_000).step_by(499) {
            let value = Date::from_days(days);
            assert_eq!(Date::parse(&value.to_string()), Ok(value), "{value}");
        }
    }

    #[test]
    fn iso_dates_that_are_no_date_are_refused() {
        for text in [
            "",
            "2026",
            "2026-09",
            "2026-09-17-01",
            "2026/09/17",
            "20260917",
            "2026-9-17",
            "2026-09-7",
            "2026-09-017",
            "26-09-17",
            "2026-aa-17",
            "2026-09-1x",
            "2026-13-40",
            "2026-02-30",
            "2026-04-31",
            "2026-00-10",
            "2026-09-00",
            "1900-02-29",
            "+2026-09-17",
        ] {
            assert!(Date::parse(text).is_err(), "`{text}` was read as a date");
        }
        // Leap years are the calendar's, not a guess.
        assert_eq!(Date::parse("2024-02-29"), Ok(date(2024, 2, 29)));
        assert_eq!(Date::parse("2000-02-29"), Ok(date(2000, 2, 29)));
        assert!(Date::parse("2100-02-29").is_err());
        // The message names what was wrong.
        assert!(Date::parse("2026-02-30").is_err_and(|error| error.contains("not a day this calendar has")));
        assert!(Date::parse("2026-9-17").is_err_and(|error| error.contains("2 digits")));
    }

    #[test]
    fn times_of_day_are_capped_read_and_written() {
        assert_eq!(TimeOfDay::new(25, 70, 90), TimeOfDay::LARGEST);
        assert_eq!(TimeOfDay::new(9, 30, 0).to_string(), "09:30:00");
        assert_eq!(TimeOfDay::parse("9:30"), Some(TimeOfDay::new(9, 30, 0)));
        assert_eq!(TimeOfDay::parse(" 09:30:15 "), Some(TimeOfDay::new(9, 30, 15)));
        assert_eq!(TimeOfDay::parse("24:00"), None, "out of range is refused, not capped");
        assert_eq!(TimeOfDay::parse("9"), None);
        assert_eq!(TimeOfDay::parse("9:30:15:20"), None);
        assert_eq!(TimeOfDay::parse("nine:thirty"), None);
        assert_eq!(TimeOfDay::default(), TimeOfDay::new(0, 0, 0));
        assert!(TimeOfDay::new(9, 30, 0) < TimeOfDay::new(9, 30, 1), "times compare in clock order");
    }

    #[test]
    fn seconds_since_midnight_go_both_ways() {
        assert_eq!(TimeOfDay::new(0, 0, 0).seconds_since_midnight(), 0);
        assert_eq!(TimeOfDay::LARGEST.seconds_since_midnight(), 86_399);
        assert_eq!(TimeOfDay::new(9, 30, 15).seconds_since_midnight(), 34_215);
        for seconds in (0..86_400).step_by(37) {
            assert_eq!(TimeOfDay::from_seconds_since_midnight(seconds).seconds_since_midnight(), seconds);
        }
        assert_eq!(TimeOfDay::from_seconds_since_midnight(86_400), TimeOfDay::new(0, 0, 0), "a day wraps");
    }

    #[test]
    fn a_moment_and_its_unix_second_agree() {
        let epoch = DateTime::from_unix(0, 0);
        assert_eq!(epoch.date, date(1970, 1, 1));
        assert_eq!(epoch.time, TimeOfDay::new(0, 0, 0));
        assert_eq!(epoch.to_unix(), 0);

        // The same instant in Istanbul and in Los Angeles.
        let istanbul = DateTime::from_unix(1_773_997_200, 180);
        assert_eq!((istanbul.date.to_string(), istanbul.time.to_string()), ("2026-03-20".into(), "12:00:00".into()));
        let angeles = DateTime::from_unix(1_773_997_200, -420);
        assert_eq!((angeles.date.to_string(), angeles.time.to_string()), ("2026-03-20".into(), "02:00:00".into()));
        assert_eq!(istanbul.to_unix(), angeles.to_unix());

        // An offset can push the local day over either end of the UTC day.
        let before = DateTime::from_unix(0, -180);
        assert_eq!((before.date.to_string(), before.time.to_string()), ("1969-12-31".into(), "21:00:00".into()));
        assert_eq!(before.to_unix(), 0);
        let after = DateTime::from_unix(-1, 120);
        assert_eq!((after.date.to_string(), after.time.to_string()), ("1970-01-01".into(), "01:59:59".into()));
        assert_eq!(after.to_unix(), -1);

        for seconds in (-2_000_000_000..2_000_000_000).step_by(1_000_003) {
            for offset in [-720, -270, 0, 180, 345, 840] {
                assert_eq!(DateTime::from_unix(seconds, offset).to_unix(), seconds, "{seconds} at {offset}");
            }
        }
    }

    #[test]
    fn extreme_unix_seconds_do_not_overflow() {
        for seconds in [i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX] {
            for offset in [i16::MIN, -1, 0, 1, i16::MAX] {
                let moment = DateTime::from_unix(seconds, offset);
                assert!(Date::new(moment.date.year(), moment.date.month(), moment.date.day()).is_some());
                let _ = moment.to_unix();
            }
        }
    }

    #[test]
    fn the_local_offset_is_either_known_or_said_to_be_unknown() {
        match local_offset() {
            Some(minutes) => {
                assert_eq!(local_offset_minutes(), minutes);
                assert!((-720..=840).contains(&minutes), "{minutes} minutes is no time zone offset");
            }
            None => assert_eq!(local_offset_minutes(), 0, "an unknown offset counts as UTC"),
        }
    }

    #[test]
    fn local_today_and_now_agree_with_utc_within_the_offset() {
        // One reading answers everything that must agree exactly; the clock is read again only
        // where a midnight passing in between is allowed for, since two readings either side of
        // one are a day apart and this test would otherwise fail every night for one second.
        let before = Date::today_local();
        let now = DateTime::now_local();
        let after = Date::today_local();
        assert_eq!(now.offset_minutes, local_offset_minutes());
        assert!(now.date == before || now.date == after, "{} is not the local day {before}", now.date);
        let difference = now.date.to_days() - DateTime::from_unix(now.to_unix(), 0).date.to_days();
        assert!((-1..=1).contains(&difference), "the local day is at most a day from the UTC one");
        assert!(now.date.year() >= 2024, "{}", now.date);
        // The offset is exactly what turns the local reading back into the UTC one.
        let utc = DateTime::from_unix(now.to_unix(), 0);
        let minutes = (now.date.to_days() - utc.date.to_days()) * 1_440
            + i64::from(now.time.seconds_since_midnight()) / 60
            - i64::from(utc.time.seconds_since_midnight()) / 60;
        assert_eq!(minutes, i64::from(now.offset_minutes));
    }
}
