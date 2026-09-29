//! The checks a multi-language application runs on its own locale files, in a test.
//!
//! Every file carrying every key with the right placeholders and the plural forms its language
//! needs can still be nine copies of English, and a check that only counts keys cannot see it.
//! [`locales`] runs the three checks on the shape of a set of files and [`own_words`] the one on
//! whether a language says anything of its own; each of [`same_keys`], [`same_placeholders`] and
//! [`plural_forms`] can be used on its own.
//!
//! ```rust
//! let en = "[meta]\nname = \"English\"\ncode = \"en\"\n\n[app]\nsave = \"Save\"\n";
//! let tr = "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\n\n[app]\nsave = \"Kaydet\"\n";
//! let files = [("en.toml", en), ("tr.toml", tr)];
//! let problems = qframe::i18n::check::locales(&files);
//! assert!(problems.is_empty(), "{problems:?}");
//! ```
//!
//! The files are read with the parser the runtime reads them with and the plural rules it chooses
//! their forms with, so a file these calls call complete is one the runtime can draw. The first
//! file is the reference: every other file is compared with it and nothing with anything else. A
//! file the parser cannot read whole is a [`Problem`] of its own and takes no part in the
//! comparison, and a reference that cannot be read leaves the files after it uncompared.

use std::fmt;
use std::ops::RangeInclusive;

use super::locale::{self, Locale, Message, Piece, Place, Template};
use super::plural::PluralCategory;
use crate::diagnostics::{Diagnostic, Location};

/// One problem found in a locale file.
///
/// Its [`Display`](fmt::Display) is what an editor opens: the file, the line, the column and what
/// is wrong. The message is for whoever reads the failing test, so it is written in English and
/// never comes from a language file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Problem {
    /// The file the problem is in, under the name it was given.
    pub file: String,
    /// The line, counting from 1. A problem with a whole file rather than a place in it is
    /// reported at its first line.
    pub line: usize,
    /// The column in characters, counting from 1.
    pub column: usize,
    /// The message the problem belongs to, when it belongs to one.
    pub key: Option<String>,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}: {}", self.file, self.line, self.column, self.message)
    }
}

/// Every key of the reference in every other file, no key of its own anywhere, the same
/// placeholders wherever both files have a message, and the plural forms every language's own
/// rule chooses from. [`own_words`] is the one left out: whether a language has been translated
/// is a decision the application makes, not a mistake it made.
#[must_use]
pub fn locales(files: &[(&str, &str)]) -> Vec<Problem> {
    let Loaded { files, mut problems } = load(files);
    problems.extend(key_problems(&files));
    problems.extend(placeholder_problems(&files));
    problems.extend(form_problems(&files));
    problems
}

/// Every key of the reference file in every other file, and no key one of them has on its own.
///
/// A key the reference has and another file lacks is a problem in the reference file, at the
/// line that key is written on, since that is where the key to translate is; a key another file
/// has and the reference does not is a problem in that file, at its own line, since nothing draws
/// it. The exception is a numbered key of a family a language may add on its own, such as
/// `quvyta.date.month-in-date-1` to `month-in-date-12`, which the month names of only some
/// languages need; once the reference itself has such a key, every file must have it too.
#[must_use]
pub fn same_keys(files: &[(&str, &str)]) -> Vec<Problem> {
    let Loaded { files, mut problems } = load(files);
    problems.extend(key_problems(&files));
    problems
}

/// The same `{name}` placeholders wherever both files have the message.
///
/// A placeholder is compared by name, so `{count}` where the reference writes `{n}` is a problem:
/// the argument the call passes is named `n`, and the message would show the placeholder itself.
/// Using one placeholder twice counts, since a message that writes `{name}` twice and a reference
/// that writes it once are not the same message; the order they are written in does not, since a
/// language puts its words where its own grammar wants them. A form only one of the two files
/// has is left to [`plural_forms`], and where one of them writes a plain string and the other a
/// plural table, the string is compared with the table's `other` form, the one a count falls back
/// to.
#[must_use]
pub fn same_placeholders(files: &[(&str, &str)]) -> Vec<Problem> {
    let Loaded { files, mut problems } = load(files);
    problems.extend(placeholder_problems(&files));
    problems
}

/// The plural forms each language's own plural rule can choose from.
///
/// A form a language's rule chooses and one of its messages does not have is a problem, named
/// with the first count that chooses it — Russian `few` is what 2 is, Japanese never chooses
/// anything but `other`. A form the rule never chooses is left alone: it is never drawn, so a
/// language keeping one costs nothing.
///
/// A plain string where the reference has a plural table is the language's own choice and is not
/// a problem: Turkish counts 1 and 5 with the same words and writes `"{n} öğeyi kes"` where
/// English has two forms, and the runtime draws that string for every count. Nothing in the text
/// says whether a language would have said it another way, so nothing here guesses.
///
/// The reference is read with the others, since a form its own rule chooses and it does not have
/// is as wrong as one missing from another language.
#[must_use]
pub fn plural_forms(files: &[(&str, &str)]) -> Vec<Problem> {
    let Loaded { files, mut problems } = load(files);
    problems.extend(form_problems(&files));
    problems
}

/// Whether every file says its own words: at least `at_least` of the values of each file after
/// the reference must differ from the reference's. A file whose values are all English passes
/// every other check here, while a word two languages happen to share is normal, which is what a
/// share rather than a single value is for. `0.8` asks for a real translation.
///
/// Left out of both the number of values and the share:
///
/// - a plural table, whose forms are counted one by one by [`plural_forms`] and whose several
///   forms are a language's wording as a whole;
/// - a value with no letters in it, which says nothing in any language: a pure placeholder like
///   `{n}`, a number like `7`, the shape of a clock `{hour}:{minute}`;
/// - a key name: a value made only of ASCII letters, digits and the punctuation of a shortcut,
///   with the `+` that joins the keys of one, such as `ctrl+s`. A single word like `on` is a word
///   rather than a key name, so it is counted.
///
/// A file with nothing left to say in words is not reported, and a key it does not have at all is
/// left to [`same_keys`]. A file that says too little is one problem, at its first line, naming
/// how many of its values are the same.
#[must_use]
pub fn own_words(files: &[(&str, &str)], at_least: f32) -> Vec<Problem> {
    let Loaded { files, mut problems } = load(files);
    let Some((reference_file, reference)) = files.first() else {
        return problems;
    };
    for (file, locale) in &files[1..] {
        let (same, total) = share(reference, locale);
        if total == 0 {
            continue;
        }
        let differing = (total - same) as f32 / total as f32;
        if differing < at_least {
            problems.push(Problem {
                file: (*file).to_owned(),
                line: 1,
                column: 1,
                key: None,
                // Written so the count reads the same however many there are: a test failure is
                // read by a person in a hurry.
                message: format!(
                    "it says the same words as `{reference_file}` in {same} of its {total} values, so it reads as `{}`; at least {at_least} of them must be its own words",
                    reference.code
                ),
            });
        }
    }
    problems
}

/// The files of one call, read once: every file the parser could read whole, and what it said
/// about the ones it could not.
struct Loaded<'a> {
    /// `(name, locale)` in the order given, the reference first.
    files: Vec<(&'a str, Locale)>,
    /// What is wrong with a file the parser did not return whole: a broken line, a missing
    /// `[meta]`, a value that is not a string. The runtime reports the same things as diagnostics,
    /// and a check that stayed quiet about them would pass a file it cannot draw.
    problems: Vec<Problem>,
}

/// Reads every file of a call, keeping what the parser said about each of them.
fn load<'a>(files: &[(&'a str, &'a str)]) -> Loaded<'a> {
    let mut loaded = Loaded { files: Vec::new(), problems: Vec::new() };
    for (index, (file, text)) in files.iter().enumerate() {
        let mut report = Vec::new();
        if let Some(locale) = locale::parse(file, text, &mut report) {
            loaded.files.push((file, locale));
        }
        loaded.problems.extend(report.into_iter().map(|problem| Problem::reported(file, &problem)));
        // The first file is the reference; without one there is nothing to compare the rest with,
        // so they keep only what the parser said about them.
        if index == 0 && loaded.files.is_empty() {
            break;
        }
    }
    loaded
}

impl Problem {
    /// A problem the parser gave about a file, at the place it named or at the top of the file.
    fn reported(file: &str, diagnostic: &Diagnostic) -> Self {
        let (line, column) = diagnostic.location.as_ref().map_or((1, 1), |at| (at.line, at.column));
        Self { file: file.to_owned(), line, column, key: None, message: diagnostic.message.clone() }
    }

    /// A problem about a message, at the place that message is written.
    fn at(place: &Location, key: &str, message: String) -> Self {
        Self { file: place.file.clone(), line: place.line, column: place.column, key: Some(key.to_owned()), message }
    }
}

/// Where a message of `locale` is written: a message only enters a file together with the place it
/// was written at, so every message of a file that could be read has one.
fn place_of<'l>(locale: &'l Locale, key: &str) -> &'l Place {
    locale.places.get(key).expect("every message of a file keeps the line it was written on")
}

/// Every key the reference has and another file lacks, and every key another file has alone.
fn key_problems(files: &[(&str, Locale)]) -> Vec<Problem> {
    let Some((reference_file, reference)) = files.first() else {
        return Vec::new();
    };
    let mut problems = Vec::new();
    for (file, locale) in &files[1..] {
        for key in reference.messages.keys() {
            if !locale.messages.contains_key(key) {
                let message = format!("`{key}` is missing from `{file}`");
                problems.push(Problem::at(&place_of(reference, key).key, key, message));
            }
        }
        for key in locale.messages.keys() {
            if !reference.messages.contains_key(key) && !optional_key(key) {
                let message = format!("`{key}` is in `{file}` but not in `{reference_file}`, so nothing draws it");
                problems.push(Problem::at(&place_of(locale, key).key, key, message));
            }
        }
    }
    problems
}

/// Key families a language may add on its own, each written with the `-` that its numbers follow:
/// the month names a language writes its own way inside a date, which Russian, Spanish, French
/// and Brazilian Portuguese among the nine do. A key of such a family is a language's own answer,
/// not a key the reference forgot.
const OPTIONAL_FAMILIES: [&str; 1] = ["quvyta.date.month-in-date-"];

/// Whether `key` is a numbered key of a family a language may add on its own.
fn optional_key(key: &str) -> bool {
    OPTIONAL_FAMILIES.iter().any(|family| {
        key.strip_prefix(family).is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Every message of one file that the reference has too, compared form by form.
fn placeholder_problems(files: &[(&str, Locale)]) -> Vec<Problem> {
    let Some((_, reference)) = files.first() else {
        return Vec::new();
    };
    let mut problems = Vec::new();
    for (_, locale) in &files[1..] {
        for (key, wanted) in &reference.messages {
            // A key the file does not have is named by `same_keys`.
            let Some(given) = locale.messages.get(key) else { continue };
            let place = place_of(locale, key);
            for (category, here, there) in comparable(given, wanted) {
                let (here, there) = (placeholders(here), placeholders(there));
                if here == there {
                    continue;
                }
                let form = match given {
                    Message::Plain(_) => String::new(),
                    Message::Plural(_) => format!("'s `{}` form", category.name()),
                };
                let message = format!("`{key}`{form} takes the placeholders {here} where the reference takes {there}");
                problems.push(Problem::at(place.form(category), key, message));
            }
        }
    }
    problems
}

/// The `{name}` placeholders a message writes, each one once per use, in name order so a language
/// that puts its words in another order is not reported: `{n} {unit}`, or `none`.
fn placeholders(template: &Template) -> String {
    let mut names: Vec<&str> = template
        .0
        .iter()
        .filter_map(|piece| match piece {
            Piece::Text(_) => None,
            Piece::Arg(name) => Some(name.as_str()),
        })
        .collect();
    if names.is_empty() {
        return "none".to_owned();
    }
    names.sort_unstable();
    names.iter().map(|name| format!("{{{name}}}")).collect::<Vec<_>>().join(" ")
}

/// The forms of two messages that can be compared with each other, each with the category both
/// sides know it by: a plain string has no category of its own, so it is matched with the `other`
/// form of a table on the other side, the form a count falls back to.
fn comparable<'m>(given: &'m Message, wanted: &'m Message) -> Vec<(PluralCategory, &'m Template, &'m Template)> {
    let (given, wanted) = match (given, wanted) {
        (Message::Plain(template), Message::Plural(_)) => (vec![(PluralCategory::Other, template)], written(wanted)),
        (table @ Message::Plural(_), Message::Plain(template)) => {
            (written(table), vec![(PluralCategory::Other, template)])
        }
        (given, wanted) => (written(given), written(wanted)),
    };
    wanted
        .iter()
        .filter_map(|(category, wanted)| {
            let (_, given) = given.iter().copied().find(|(known, _)| known == category)?;
            Some((*category, given, *wanted))
        })
        .collect()
}

/// The forms of a message, a plain string being the `other` form of a message with no count in it.
fn written(message: &Message) -> Vec<(PluralCategory, &Template)> {
    match message {
        Message::Plain(template) => vec![(PluralCategory::Other, template)],
        Message::Plural(forms) => forms.iter().map(|(category, template)| (*category, template)).collect(),
    }
}

/// Every form a language's rule chooses from and a message of its own does not have.
fn form_problems(files: &[(&str, Locale)]) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (_, locale) in files {
        let needed = needed_forms(&locale.code);
        for (key, message) in &locale.messages {
            let Message::Plural(forms) = message else { continue };
            for (category, count) in &needed {
                if !forms.contains_key(category) {
                    let message =
                        format!("`{key}` has no `{}` form, which {} uses for {count}", category.name(), locale.code);
                    problems.push(Problem::at(&place_of(locale, key).key, key, message));
                }
            }
        }
    }
    problems
}

/// The counts a plural rule is read over, the range the built-in language files are checked with:
/// every form a rule can choose for a count a screen shows is in it.
const COUNTS: RangeInclusive<i64> = 0..=200;

/// The forms a language's own rule can choose from, each with the first count that chooses it.
fn needed_forms(language: &str) -> Vec<(PluralCategory, i64)> {
    let mut needed = Vec::new();
    for category in PluralCategory::ALL {
        if let Some(count) = COUNTS.clone().find(|n| PluralCategory::of(language, *n) == category) {
            needed.push((category, count));
        }
    }
    needed
}

/// How many of a file's values say the same words as the reference's, and how many of them could:
/// the values that are the same in every language by nature are left out of both.
fn share(reference: &Locale, locale: &Locale) -> (usize, usize) {
    let (mut same, mut total) = (0, 0);
    for (key, message) in &locale.messages {
        // A key only the reference has, or a plural table on either side, is left to the other
        // checks rather than read as a value the language failed to say its own way.
        let (Message::Plain(given), Some(Message::Plain(wanted))) = (message, reference.messages.get(key)) else {
            continue;
        };
        let given = given.words();
        if by_nature(&given) {
            continue;
        }
        total += 1;
        if given == wanted.words() {
            same += 1;
        }
    }
    (same, total)
}

/// Whether a value says the same thing in every language by nature rather than untranslated: it
/// has no letters in it, as a pure placeholder, a number and a shape like a clock do not, or it
/// is a key name, which the `+` of a shortcut gives away.
fn by_nature(words: &str) -> bool {
    !words.chars().any(char::is_alphabetic) || shortcut(words)
}

/// Whether `words` is a key name: it has the `+` that joins the keys of a shortcut, and nothing
/// but ASCII letters, digits and the punctuation one is written with.
fn shortcut(words: &str) -> bool {
    words.contains('+') && words.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_' | '.' | '/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = r#"[meta]
name = "English"
code = "en"

[app]
save = "Save"
count = { one = "{n} file", other = "{n} files" }
key = "ctrl+s"
"#;

    const TR: &str = r#"[meta]
name = "Türkçe"
code = "tr"
fallback = "en"

[app]
save = "Kaydet"
count = { one = "{n} dosya", other = "{n} dosya" }
key = "ctrl+s"
"#;

    /// Russian, whose plural rule chooses three forms where English chooses two.
    const RU: &str = r#"[meta]
name = "Русский"
code = "ru"

[app]
save = "Сохранить"
count = { one = "{n} файл", few = "{n} файла", many = "{n} файлов", other = "{n} файлов" }
key = "ctrl+s"
"#;

    /// The problems as `(file, line, key, message)`, which is what a test reads.
    fn shown(problems: &[Problem]) -> Vec<(String, usize, Option<String>, String)> {
        problems
            .iter()
            .map(|problem| (problem.file.clone(), problem.line, problem.key.clone(), problem.message.clone()))
            .collect()
    }

    #[test]
    fn a_complete_pair_of_files_has_no_problems() {
        let files = [("en.toml", EN), ("tr.toml", TR)];
        assert_eq!(shown(&locales(&files)), Vec::new());
        assert_eq!(shown(&same_keys(&files)), Vec::new());
        assert_eq!(shown(&same_placeholders(&files)), Vec::new());
        assert_eq!(shown(&plural_forms(&files)), Vec::new());
        assert_eq!(shown(&own_words(&files, 0.8)), Vec::new(), "Turkish wrote a word of its own");
        // A language whose rule chooses three forms keeps all three.
        let russian = [("en.toml", EN), ("ru.toml", RU)];
        assert_eq!(shown(&plural_forms(&russian)), Vec::new());
        assert_eq!(shown(&locales(&russian)), Vec::new());
    }

    #[test]
    fn a_missing_key_is_named_at_the_line_of_the_reference() {
        let tr = TR.replace("save = \"Kaydet\"\n", "");
        let files = [("en.toml", EN), ("tr.toml", tr.as_str())];
        let problems = same_keys(&files);
        assert_eq!(
            shown(&problems),
            vec![(
                "en.toml".to_owned(),
                6,
                Some("app.save".to_owned()),
                "`app.save` is missing from `tr.toml`".to_owned()
            )]
        );
        assert_eq!(problems[0].to_string(), "en.toml:6:1: `app.save` is missing from `tr.toml`");
        assert_eq!(locales(&files).len(), 1, "and once only through `locales`");
    }

    #[test]
    fn a_key_the_reference_does_not_have_is_named_at_its_own_line() {
        let tr = TR.replace("key = \"ctrl+s\"\n", "key = \"ctrl+s\"\nquit = \"Çık\"\n");
        let files = [("en.toml", EN), ("tr.toml", tr.as_str())];
        assert_eq!(
            shown(&same_keys(&files)),
            vec![(
                "tr.toml".to_owned(),
                10,
                Some("app.quit".to_owned()),
                "`app.quit` is in `tr.toml` but not in `en.toml`, so nothing draws it".to_owned()
            )]
        );
    }

    #[test]
    fn a_language_may_add_the_month_names_its_own_rule_needs() {
        // Russian declines a month name inside a date, so its file carries a numbered key family
        // the other languages do not have: its own answer, not a key the reference forgot.
        let ru = RU.replace("key = \"ctrl+s\"\n", "key = \"ctrl+s\"\n\n[quvyta.date]\nmonth-in-date-1 = \"января\"\n");
        let files = [("en.toml", EN), ("ru.toml", ru.as_str())];
        assert_eq!(shown(&same_keys(&files)), Vec::new());
    }

    #[test]
    fn a_placeholder_written_under_another_name_is_named() {
        let tr = TR.replace("{n} dosya", "{count} dosya");
        let files = [("en.toml", EN), ("tr.toml", tr.as_str())];
        assert_eq!(
            shown(&same_placeholders(&files)),
            vec![
                (
                    "tr.toml".to_owned(),
                    8,
                    Some("app.count".to_owned()),
                    "`app.count`'s `one` form takes the placeholders {count} where the reference takes {n}".to_owned()
                ),
                (
                    "tr.toml".to_owned(),
                    8,
                    Some("app.count".to_owned()),
                    "`app.count`'s `other` form takes the placeholders {count} where the reference takes {n}"
                        .to_owned()
                ),
            ]
        );
    }

    #[test]
    fn a_placeholder_left_out_of_one_form_only_is_named_there() {
        let en = EN.replace("other = \"{n} files\"", "other = \"{n} files in {dir}\"");
        let files = [("en.toml", en.as_str()), ("tr.toml", TR)];
        let problems = same_placeholders(&files);
        assert_eq!(problems.len(), 1, "only the form that differs: {problems:?}");
        assert!(problems[0].message.contains("{dir}"), "{}", problems[0]);
        assert!(!problems[0].message.contains("`one` form"), "the form beside it is untouched: {}", problems[0]);
    }

    #[test]
    fn a_language_that_counts_in_three_ways_needs_three_forms() {
        let ru = RU.replace("few = \"{n} файла\", ", "");
        let files = [("en.toml", EN), ("ru.toml", ru.as_str())];
        assert_eq!(
            shown(&plural_forms(&files)),
            vec![(
                "ru.toml".to_owned(),
                7,
                Some("app.count".to_owned()),
                "`app.count` has no `few` form, which ru uses for 2".to_owned()
            )]
        );
    }

    #[test]
    fn a_language_may_write_a_plain_string_where_the_reference_has_a_table() {
        // Turkish counts 1 and 5 with the same words, so its file writes one string where English
        // has two forms, and the runtime draws it for every count.
        let tr = TR.replace("count = { one = \"{n} dosya\", other = \"{n} dosya\" }", "count = \"{n} dosya\"");
        let files = [("en.toml", EN), ("tr.toml", tr.as_str())];
        assert_eq!(shown(&plural_forms(&files)), Vec::new());
        assert_eq!(shown(&locales(&files)), Vec::new(), "the placeholders still have to match");
    }

    #[test]
    fn a_file_copied_from_the_reference_is_complete_and_says_nothing_of_its_own() {
        let turkish = EN.replace("name = \"English\"", "name = \"Türkçe\"").replace("code = \"en\"", "code = \"tr\"");
        let files = [("en.toml", EN), ("tr.toml", turkish.as_str())];
        assert_eq!(shown(&locales(&files)), Vec::new(), "every key, placeholder and form is there");
        assert_eq!(
            shown(&own_words(&files, 0.8)),
            vec![(
                "tr.toml".to_owned(),
                1,
                None,
                "it says the same words as `en.toml` in 1 of its 1 values, so it reads as `en`; at least 0.8 of them must be its own words"
                    .to_owned()
            )]
        );
    }

    #[test]
    fn the_values_that_are_the_same_in_every_language_are_left_out_of_the_share() {
        // A file that translated everything there is to translate and nothing else: a shortcut, a
        // size made of placeholders and the first day of a week are a number.
        let en = EN.replace("key = \"ctrl+s\"", "key = \"ctrl+s\"\nsize = \"{n} {unit}\"\nfirst-weekday = \"1\"");
        let tr = TR.replace("key = \"ctrl+s\"", "key = \"ctrl+s\"\nsize = \"{n} {unit}\"\nfirst-weekday = \"1\"");
        let files = [("en.toml", en.as_str()), ("tr.toml", tr.as_str())];
        assert_eq!(shown(&own_words(&files, 0.8)), Vec::new(), "nothing was left to translate");
        // A file with nothing but those values has no words of its own to show, and is not
        // reported for that; the keys it lacks are named by `same_keys`.
        let bare = "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\n\n[app]\nkey = \"ctrl+s\"\nsize = \"{n} {unit}\"\n";
        let alone = [("en.toml", en.as_str()), ("tr.toml", bare)];
        assert_eq!(shown(&own_words(&alone, 0.8)), Vec::new());
        assert_eq!(shown(&same_keys(&alone)).len(), 3, "app.save, app.count and app.first-weekday");
    }

    #[test]
    fn a_broken_file_is_a_problem_and_not_a_panic() {
        let files = [("en.toml", EN), ("tr.toml", "[meta]\nname = \"Türkçe\"\ncode = \n")];
        for problems in [
            locales(&files),
            same_keys(&files),
            same_placeholders(&files),
            plural_forms(&files),
            own_words(&files, 0.8),
        ] {
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!(problems[0].file, "tr.toml");
            assert_eq!(problems[0].line, 3, "the line the parser stopped at: {}", problems[0]);
        }
        // A file without `[meta]` is named too, and the files beside it are still compared.
        let nameless = [("en.toml", EN), ("tr.toml", "[app]\nsave = \"Kaydet\"\n")];
        let problems = same_keys(&nameless);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].message.contains("[meta]"), "{}", problems[0]);
        // A reference that cannot be read leaves nothing to compare with, and says so once.
        let unreadable = [("en.toml", "[meta]\nname = \n"), ("tr.toml", TR)];
        let problems = same_keys(&unreadable);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].file, "en.toml");
    }

    #[test]
    fn the_frameworks_own_languages_pass_every_check() {
        let named: Vec<(String, &str)> =
            crate::assets::LOCALES.iter().map(|(code, text)| (format!("{code}.toml"), *text)).collect();
        // The first file is the reference, and the built-in locales are not in that order.
        let english = named.iter().position(|(name, _)| name == "en.toml").expect("English is one of them");
        let mut ordered = named.clone();
        ordered.swap(english, 0);
        let files: Vec<(&str, &str)> = ordered.iter().map(|(name, text)| (name.as_str(), *text)).collect();
        let problems = locales(&files);
        assert!(problems.is_empty(), "{}", problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"));
        let own = own_words(&files, 0.8);
        assert!(own.is_empty(), "{}", own.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"));
    }
}
