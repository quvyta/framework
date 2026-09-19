//! The first day of the week by region, from the Unicode CLDR.
//!
//! A calendar page starts its week where the people of a country do, whatever language they
//! read it in: English in the United Kingdom starts on Monday and in the United States on Sunday.

use crate::date::Weekday;

/// Regions whose calendar weeks start on Sunday.
///
/// Copied from `<firstDay day="sun">` in `common/supplemental/supplementalData.xml` of CLDR 48
/// (tag `release-48-2`, <https://github.com/unicode-org/cldr>). The `alt="variant"` entry for
/// `GB` is a dictionary's alternative, not the default, and is left out.
const SUNDAY: [&str; 56] = [
    "AG", "AS", "BD", "BR", "BS", "BT", "BW", "BZ", "CA", "CO", "DM", "DO", "ET", "GT", "GU", "HK", "HN", "ID", "IL",
    "IN", "IS", "JM", "JP", "KE", "KH", "KR", "LA", "MH", "MM", "MO", "MT", "MX", "MZ", "NI", "NP", "PA", "PE", "PH",
    "PK", "PR", "PT", "PY", "SA", "SG", "SV", "TH", "TT", "TW", "UM", "US", "VE", "VI", "WS", "YE", "ZA", "ZW",
];

/// Regions whose calendar weeks start on Saturday; `<firstDay day="sat">` of the same file.
const SATURDAY: [&str; 14] = ["AF", "BH", "DJ", "DZ", "EG", "IQ", "IR", "JO", "KW", "LY", "OM", "QA", "SD", "SY"];

/// Regions whose calendar weeks start on Friday; `<firstDay day="fri">` of the same file.
const FRIDAY: [&str; 1] = ["MV"];

/// The first day of a calendar week in `region`, an uppercase region code such as `GB` or `419`.
/// CLDR gives Monday to the world (`001`) and so to every region it does not list otherwise.
pub(super) fn first_day(region: &str) -> Weekday {
    if SUNDAY.contains(&region) {
        Weekday::Sunday
    } else if SATURDAY.contains(&region) {
        Weekday::Saturday
    } else if FRIDAY.contains(&region) {
        Weekday::Friday
    } else {
        Weekday::Monday
    }
}

/// `GB` for `gb` or `GB`, `419` for `419`: a region is two letters or three digits. Anything else
/// is not a region.
pub(super) fn region_code(text: &str) -> Option<String> {
    let letters = text.len() == 2 && text.chars().all(|c| c.is_ascii_alphabetic());
    let digits = text.len() == 3 && text.chars().all(|c| c.is_ascii_digit());
    (letters || digits).then(|| text.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_start_their_weeks_where_cldr_says() {
        for (region, first) in [
            ("US", Weekday::Sunday),
            ("CA", Weekday::Sunday),
            ("BR", Weekday::Sunday),
            ("PT", Weekday::Sunday),
            ("JP", Weekday::Sunday),
            ("IL", Weekday::Sunday),
            ("GB", Weekday::Monday),
            ("AU", Weekday::Monday),
            ("AE", Weekday::Monday),
            ("TR", Weekday::Monday),
            ("DE", Weekday::Monday),
            ("EG", Weekday::Saturday),
            ("IR", Weekday::Saturday),
            ("MV", Weekday::Friday),
            ("419", Weekday::Monday),
            ("ZZ", Weekday::Monday),
        ] {
            assert_eq!(first_day(region), first, "{region}");
        }
    }

    #[test]
    fn the_tables_are_sorted_and_hold_regions_only() {
        for table in [&SUNDAY[..], &SATURDAY[..], &FRIDAY[..]] {
            assert!(table.windows(2).all(|pair| pair[0] < pair[1]), "{table:?}");
            assert!(table.iter().all(|region| region_code(region).as_deref() == Some(*region)), "{table:?}");
        }
    }

    #[test]
    fn a_region_is_two_letters_or_three_digits() {
        assert_eq!(region_code("gb").as_deref(), Some("GB"));
        assert_eq!(region_code("419").as_deref(), Some("419"));
        for none in ["", "G", "GBR", "4a", "12", "ü1"] {
            assert_eq!(region_code(none), None, "{none}");
        }
    }
}
