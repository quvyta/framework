//! CLDR plural categories for integer counts.

/// A plural category, as used in locale files: `{ one = "…", other = "…" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralCategory {
    /// Used by some languages for 0.
    Zero,
    /// Singular.
    One,
    /// Dual.
    Two,
    /// Paucal, e.g. Russian 2–4.
    Few,
    /// Many, e.g. Russian 5–20.
    Many,
    /// Everything else; always required.
    Other,
}

impl PluralCategory {
    /// Every category.
    pub const ALL: [Self; 6] = [Self::Zero, Self::One, Self::Two, Self::Few, Self::Many, Self::Other];

    /// The name used in locale files.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::One => "one",
            Self::Two => "two",
            Self::Few => "few",
            Self::Many => "many",
            Self::Other => "other",
        }
    }

    /// Looks a category up by name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }

    /// The category of count `n` in language `language` (e.g. `"tr"`).
    ///
    /// Covers the integer rules of the CLDR plural data for East Asian languages (no
    /// plural), French and Portuguese, the East Slavic languages, Polish, Czech and Slovak,
    /// Arabic, and the common one/other rule for every other language.
    #[must_use]
    pub fn of(language: &str, n: i64) -> Self {
        let n = n.unsigned_abs();
        let (mod10, mod100) = (n % 10, n % 100);
        match language {
            "ja" | "zh" | "ko" | "vi" | "th" | "id" | "ms" => Self::Other,
            "fr" | "pt" => {
                if n <= 1 {
                    Self::One
                } else {
                    Self::Other
                }
            }
            "ru" | "uk" | "be" => {
                if mod10 == 1 && mod100 != 11 {
                    Self::One
                } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
                    Self::Few
                } else {
                    Self::Many
                }
            }
            "pl" => {
                if n == 1 {
                    Self::One
                } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
                    Self::Few
                } else {
                    Self::Many
                }
            }
            "cs" | "sk" => match n {
                1 => Self::One,
                2..=4 => Self::Few,
                _ => Self::Other,
            },
            "ar" => match n {
                0 => Self::Zero,
                1 => Self::One,
                2 => Self::Two,
                _ if (3..=10).contains(&mod100) => Self::Few,
                _ if (11..=99).contains(&mod100) => Self::Many,
                _ => Self::Other,
            },
            _ => {
                if n == 1 {
                    Self::One
                } else {
                    Self::Other
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PluralCategory::{self, Few, Many, One, Other, Two, Zero};

    #[test]
    fn default_rule_is_one_other() {
        assert_eq!(PluralCategory::of("en", 1), One);
        assert_eq!(PluralCategory::of("en", 0), Other);
        assert_eq!(PluralCategory::of("tr", 5), Other);
        assert_eq!(PluralCategory::of("tr", -1), One);
    }

    #[test]
    fn language_specific_rules() {
        assert_eq!(PluralCategory::of("ja", 1), Other);
        assert_eq!(PluralCategory::of("fr", 0), One);
        assert_eq!(PluralCategory::of("ru", 21), One);
        assert_eq!(PluralCategory::of("ru", 22), Few);
        assert_eq!(PluralCategory::of("ru", 12), Many);
        assert_eq!(PluralCategory::of("pl", 21), Many);
        assert_eq!(PluralCategory::of("cs", 3), Few);
        assert_eq!(PluralCategory::of("ar", 0), Zero);
        assert_eq!(PluralCategory::of("ar", 2), Two);
        assert_eq!(PluralCategory::of("ar", 103), Few);
    }
}
