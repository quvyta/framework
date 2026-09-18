//! Reading a length of time written by hand, such as `1 h 30 min`, `90 dk` or `2:15`, and writing
//! one back in the active language.

use std::time::Duration;

use crate::i18n::{Arg, I18n};

/// The longest length a duration holds: 99 hours, 59 minutes and 59 seconds, in seconds. Two
/// digits of hours cover any timer, target or timeout, and keep the field one width.
pub(crate) const LONGEST: u64 = 99 * 3600 + 59 * 60 + 59;

/// A unit of a length of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    /// Hours: `h`, `hour`, `sa`, `saat` and the other words of the language files.
    Hours,
    /// Minutes: `min`, `m`, `dk`, `dakika` and the other words of the language files.
    Minutes,
    /// Seconds: `s`, `sec`, `sn`, `saniye` and the other words of the language files.
    Seconds,
}

impl DurationUnit {
    /// Every unit, largest first; the index of a unit is its segment in the field.
    pub(crate) const ALL: [Self; 3] = [Self::Hours, Self::Minutes, Self::Seconds];

    /// Seconds in one of this unit.
    pub(crate) fn seconds(self) -> u64 {
        match self {
            Self::Hours => 3600,
            Self::Minutes => 60,
            Self::Seconds => 1,
        }
    }

    /// The stem of this unit's language keys.
    fn stem(self) -> &'static str {
        match self {
            Self::Hours => "hour",
            Self::Minutes => "minute",
            Self::Seconds => "second",
        }
    }

    /// The next smaller unit, which a bare number after this one is read in.
    fn below(self) -> Option<Self> {
        match self {
            Self::Hours => Some(Self::Minutes),
            Self::Minutes => Some(Self::Seconds),
            Self::Seconds => None,
        }
    }

    /// The short word shown after a number in the active language: `h`, `min`, `s`.
    pub(crate) fn short(self, i18n: &I18n) -> String {
        i18n.translate(&format!("quvyta.duration.{}s", self.stem()), &[])
    }
}

/// Why a written length of time could not be read. [`message`](Self::message) says it in the
/// active language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationError {
    /// Nothing but spaces.
    Empty,
    /// A character that has no place in a length of time, such as `-` or `/`.
    Character(char),
    /// A word that is no unit of time in any known language, such as `days`.
    UnknownUnit(String),
    /// A number that cannot be read, such as `1.2.3`.
    BadNumber(String),
    /// A unit with no number before it, such as the `h` of `h 30`.
    MissingNumber(String),
    /// A number with no unit that cannot take one from its place, such as the `30` of `1 30 min`.
    MissingUnit(String),
    /// A unit written twice, such as `1 h 2 h`.
    RepeatedUnit(DurationUnit),
    /// Text with a colon that is not `h:mm` or `h:mm:ss` with minutes and seconds under 60.
    BadClock(String),
    /// Longer than 99 hours, 59 minutes and 59 seconds.
    TooLarge,
}

impl DurationError {
    /// What is wrong, in the active language of `i18n`, from the `quvyta.duration.*` keys.
    #[must_use]
    pub fn message(&self, i18n: &I18n) -> String {
        let (key, text) = match self {
            Self::Empty => ("empty", String::new()),
            Self::Character(c) => ("character", c.to_string()),
            Self::UnknownUnit(word) => ("unit", word.clone()),
            Self::BadNumber(number) => ("number", number.clone()),
            Self::MissingNumber(word) => ("no-number", word.clone()),
            Self::MissingUnit(number) => ("no-unit", number.clone()),
            Self::RepeatedUnit(unit) => {
                ("repeated", i18n.translate(&format!("quvyta.duration.{}-name", unit.stem()), &[]))
            }
            Self::BadClock(text) => ("clock", text.clone()),
            Self::TooLarge => ("too-large", write(Duration::from_secs(LONGEST), true, i18n)),
        };
        i18n.translate(&format!("quvyta.duration.{key}"), &[("text", Arg::Text(text))])
    }
}

/// Reads a length of time written by hand, in any language `i18n` knows.
///
/// Three forms are read:
///
/// - **Numbers with units**, in any order, each unit once: `1 h 30 min`, `1sa30dk`, `90 dk`,
///   `2 hours`, `1,5 sa`. The unit words come from `quvyta.duration.hour-words`,
///   `minute-words` and `second-words` of every language, the active one first; case does not
///   matter, and Turkish dotted and dotless i read alike. A number after a unit with no unit of
///   its own takes the next smaller one: `1 h 30` is an hour and a half. A decimal point or comma
///   splits a unit (`1.5 h` is 90 minutes), rounded to the second. Minutes and seconds may run
///   past 60 and carry over: `1 h 90 min` is two hours and a half.
/// - **A clock face**, `h:mm` or `h:mm:ss`: `2:15` is two hours and a quarter, never two minutes.
///   Minutes and seconds are one or two digits under 60.
/// - **A bare number**, read in minutes: `25` is 25 minutes, the length of most timers.
///
/// # Errors
///
/// A [`DurationError`] naming the first part that could not be read, or
/// [`DurationError::TooLarge`] past 99 hours, 59 minutes and 59 seconds.
pub fn parse_duration(text: &str, i18n: &I18n) -> Result<Duration, DurationError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(DurationError::Empty);
    }
    let seconds = if text.contains(':') { clock(text)? } else { with_units(&tokens(text)?, i18n)? };
    if seconds > u128::from(LONGEST) {
        return Err(DurationError::TooLarge);
    }
    Ok(Duration::from_secs(u64::try_from(seconds).unwrap_or(LONGEST)))
}

/// Writes `duration` as the field copies it, in the active language: `1 h 30 min`, `45 min`,
/// `2 h`, and `0 min` for nothing. Parts that are zero are left out; seconds only with `seconds`.
pub(crate) fn write(duration: Duration, seconds: bool, i18n: &I18n) -> String {
    let total = duration.as_secs();
    let parts = [total / 3600, total / 60 % 60, total % 60];
    let shown = if seconds { 3 } else { 2 };
    let written: Vec<String> = DurationUnit::ALL[..shown]
        .iter()
        .zip(parts)
        .filter(|(_, value)| *value > 0)
        .map(|(unit, value)| format!("{value} {}", unit.short(i18n)))
        .collect();
    if written.is_empty() { format!("0 {}", DurationUnit::Minutes.short(i18n)) } else { written.join(" ") }
}

/// Digits past which a number is longer than any length can be.
const MAX_DIGITS: usize = 12;

/// A number as written: its text, the whole part and the digits after the decimal mark.
#[derive(Debug)]
struct Number {
    text: String,
    whole: u128,
    fraction: String,
}

impl Number {
    /// Seconds in this many `unit`s, the fraction rounded to the nearest second.
    fn seconds(&self, unit: DurationUnit) -> u128 {
        let unit = u128::from(unit.seconds());
        // Nine digits resolve any fraction of an hour far below a second.
        let digits = &self.fraction[..self.fraction.len().min(9)];
        let scale = 10u128.pow(u32::try_from(digits.len()).unwrap_or(0));
        let fraction: u128 = digits.parse().unwrap_or(0);
        self.whole * unit + (fraction * unit + scale / 2) / scale
    }
}

#[derive(Debug)]
enum Token {
    Number(Number),
    Word(String),
}

/// Splits `text` into numbers and words. Spaces, and commas or points that are not inside a
/// number, only separate: `1 h, 30 min` reads like `1 h 30 min`.
fn tokens(text: &str) -> Result<Vec<Token>, DurationError> {
    let chars: Vec<char> = text.chars().collect();
    let is_mark = |c: char| c == '.' || c == ',';
    let digit_at = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        let start = i;
        if c.is_ascii_digit() || (is_mark(c) && digit_at(i + 1)) {
            let run = |i: &mut usize| {
                while digit_at(*i) {
                    *i += 1;
                }
            };
            run(&mut i);
            let whole_end = i;
            let mut marks = 0;
            while chars.get(i).is_some_and(|c| is_mark(*c)) && digit_at(i + 1) {
                marks += 1;
                i += 1;
                run(&mut i);
            }
            let written: String = chars[start..i].iter().collect();
            let whole: String = chars[start..whole_end].iter().collect();
            if marks > 1 || whole.is_empty() {
                return Err(DurationError::BadNumber(written));
            }
            if whole.trim_start_matches('0').len() > MAX_DIGITS {
                return Err(DurationError::TooLarge);
            }
            let fraction = if marks == 1 { chars[whole_end + 1..i].iter().collect() } else { String::new() };
            out.push(Token::Number(Number { text: written, whole: whole.parse().unwrap_or(0), fraction }));
        } else if c.is_alphabetic() {
            while chars.get(i).is_some_and(|c| c.is_alphabetic()) {
                i += 1;
            }
            out.push(Token::Word(chars[start..i].iter().collect()));
        } else if c.is_whitespace() || is_mark(c) {
            i += 1;
        } else {
            return Err(DurationError::Character(c));
        }
    }
    Ok(out)
}

/// Lower case for matching unit words, with the Turkish `İ`, `I` and `ı` all read as `i`, so
/// `DAKİKA`, `DAKIKA` and `MIN` match whatever the keyboard's language.
fn fold(word: &str) -> String {
    word.chars()
        .flat_map(|c| match c {
            'İ' | 'I' | 'ı' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// The unit `word` names: in the active language first, then in any language.
fn unit_of(word: &str, i18n: &I18n) -> Option<DurationUnit> {
    let word = fold(word);
    let names = |unit: DurationUnit| format!("quvyta.duration.{}-words", unit.stem());
    let matches = |list: &str| list.split(',').any(|candidate| fold(candidate.trim()) == word);
    let active = DurationUnit::ALL.into_iter().find(|unit| matches(&i18n.translate(&names(*unit), &[])));
    active.or_else(|| {
        DurationUnit::ALL.into_iter().find(|unit| i18n.in_every_locale(&names(*unit)).iter().any(|list| matches(list)))
    })
}

/// Seconds in numbers with units.
fn with_units(tokens: &[Token], i18n: &I18n) -> Result<u128, DurationError> {
    if tokens.is_empty() {
        return Err(DurationError::Empty);
    }
    let mut total = 0u128;
    let mut seen = Vec::new();
    let mut last = None;
    let mut waiting: Option<&Number> = None;
    let mut add = |number: &Number, unit: DurationUnit, seen: &mut Vec<DurationUnit>| {
        if seen.contains(&unit) {
            return Err(DurationError::RepeatedUnit(unit));
        }
        seen.push(unit);
        total += number.seconds(unit);
        Ok(())
    };
    for token in tokens {
        match token {
            Token::Number(number) => {
                if let Some(previous) = waiting {
                    return Err(DurationError::MissingUnit(previous.text.clone()));
                }
                waiting = Some(number);
            }
            Token::Word(word) => {
                let unit = unit_of(word, i18n).ok_or_else(|| DurationError::UnknownUnit(word.clone()))?;
                let number = waiting.take().ok_or_else(|| DurationError::MissingNumber(word.clone()))?;
                add(number, unit, &mut seen)?;
                last = Some(unit);
            }
        }
    }
    if let Some(number) = waiting {
        let unit = match last {
            None => DurationUnit::Minutes,
            Some(unit) => unit.below().ok_or_else(|| DurationError::MissingUnit(number.text.clone()))?,
        };
        add(number, unit, &mut seen)?;
    }
    Ok(total)
}

/// Seconds in `h:mm` or `h:mm:ss`.
fn clock(text: &str) -> Result<u128, DurationError> {
    let bad = || DurationError::BadClock(text.to_owned());
    let parts: Vec<&str> = text.split(':').map(str::trim).collect();
    if !(2..=3).contains(&parts.len())
        || parts.iter().any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(bad());
    }
    if parts[0].trim_start_matches('0').len() > MAX_DIGITS {
        return Err(DurationError::TooLarge);
    }
    let mut total: u128 = parts[0].parse::<u128>().map_err(|_| bad())? * 3600;
    for (part, unit) in parts[1..].iter().zip([60u128, 1]) {
        let value: u128 = part.parse().map_err(|_| bad())?;
        if part.len() > 2 || value >= 60 {
            return Err(bad());
        }
        total += value * unit;
    }
    Ok(total)
}
