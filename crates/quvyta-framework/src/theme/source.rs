//! Reading a theme file into an unresolved [`ThemeSource`].

use std::collections::BTreeMap;
use std::time::Duration;

use toml::de::{DeTable, DeValue};

use super::motion::{MOTION_KEYS, parse_duration};
use super::paint::Expr;
use super::selector::Selector;
use super::style::{RawProp, allowed_words};
use crate::animation::{self, CellAnimation};
use crate::diagnostics::{Diagnostic, Location};
use crate::doc::{self, Doc, Value};
use crate::icons::{self, IconGlyphs};

/// A motion value as written.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MotionValue {
    Duration(Duration),
    Flag(bool),
}

/// Properties of one rule or typography role, in file order, with their locations.
pub(crate) type RawProps = Vec<(String, RawProp, Location)>;

/// A theme file after parsing, before `extends` and colour tokens are resolved.
#[derive(Debug, Clone)]
pub(crate) struct ThemeSource {
    pub(crate) name: String,
    pub(crate) extends: Option<(String, Location)>,
    pub(crate) icon_set: Option<String>,
    pub(crate) colors: Vec<(String, Expr, Location)>,
    pub(crate) motion: Vec<(String, MotionValue)>,
    pub(crate) typography: Vec<(String, RawProps)>,
    pub(crate) styles: Vec<(Selector, RawProps)>,
    pub(crate) icons: BTreeMap<String, IconGlyphs>,
    pub(crate) animations: BTreeMap<String, CellAnimation>,
}

const TOP_LEVEL: [&str; 7] = ["meta", "colors", "motion", "typography", "style", "icons", "animations"];

/// Parses `text`. Returns `None` only when the file cannot be used at all (syntax error or
/// missing `[meta] name`); every other problem is reported and the entry skipped.
pub(crate) fn parse(file: &str, text: &str, report: &mut Vec<Diagnostic>) -> Option<ThemeSource> {
    let doc = Doc::new(file, text);
    let root = match doc.parse() {
        Ok(root) => root,
        Err(diagnostic) => {
            report.push(diagnostic);
            return None;
        }
    };
    for (key, value) in &root {
        if !TOP_LEVEL.contains(&key.get_ref().as_ref()) {
            report.push(doc.error(
                &value.span(),
                format!("unknown section `{}`; expected one of: {}", key.get_ref(), TOP_LEVEL.join(", ")),
            ));
        }
    }

    let Some(meta) = section(&doc, &root, "meta", report) else {
        report.push(Diagnostic::error(None, format!("{file}: missing [meta] table with a `name`")));
        return None;
    };
    let name = match doc::get(meta, "name").map(|v| doc.string(v, "meta.name")) {
        Some(Ok(name)) => name.to_owned(),
        Some(Err(diagnostic)) => {
            report.push(diagnostic);
            return None;
        }
        None => {
            report.push(Diagnostic::error(None, format!("{file}: [meta] needs a `name`")));
            return None;
        }
    };
    let optional_string = |key: &str, report: &mut Vec<Diagnostic>| -> Option<(String, Location)> {
        let value = doc::get(meta, key)?;
        match doc.string(value, &format!("meta.{key}")) {
            Ok(text) => Some((text.to_owned(), doc.locate(&value.span()))),
            Err(diagnostic) => {
                report.push(diagnostic);
                None
            }
        }
    };
    let extends = optional_string("extends", report);
    let icon_set = optional_string("icon-set", report).map(|(id, _)| id);
    for (key, value) in meta {
        if !["name", "extends", "icon-set"].contains(&key.get_ref().as_ref()) {
            report.push(doc.error(&value.span(), format!("unknown key `meta.{}`", key.get_ref())));
        }
    }

    let mut source = ThemeSource {
        name,
        extends,
        icon_set,
        colors: Vec::new(),
        motion: Vec::new(),
        typography: Vec::new(),
        styles: Vec::new(),
        icons: BTreeMap::new(),
        animations: BTreeMap::new(),
    };

    if let Some(colors) = section(&doc, &root, "colors", report) {
        for (key, value) in colors {
            let parsed = doc
                .string(value, &format!("colors.{}", key.get_ref()))
                .and_then(|text| Expr::parse(text).map_err(|message| doc.error(&value.span(), message)));
            match parsed {
                Ok(Expr::Pulse(..)) => report.push(doc.error(
                    &value.span(),
                    format!("colour token `{}` cannot be a pulse(); use pulse() in style rules", key.get_ref()),
                )),
                Ok(expr) => {
                    source.colors.push((key.get_ref().to_string(), expr, doc.locate(&value.span())));
                }
                Err(diagnostic) => report.push(diagnostic),
            }
        }
    }

    if let Some(motion) = section(&doc, &root, "motion", report) {
        for (key, value) in motion {
            let key = key.get_ref().as_ref();
            if !MOTION_KEYS.contains(&key) {
                report.push(doc.error(
                    &value.span(),
                    format!("unknown motion key `{key}`; expected one of: {}", MOTION_KEYS.join(", ")),
                ));
                continue;
            }
            let parsed = if key == "slide" {
                value
                    .get_ref()
                    .as_bool()
                    .map(MotionValue::Flag)
                    .ok_or_else(|| doc.error(&value.span(), "motion.slide must be true or false"))
            } else {
                doc.string(value, &format!("motion.{key}")).and_then(|text| {
                    parse_duration(text).map(MotionValue::Duration).map_err(|message| doc.error(&value.span(), message))
                })
            };
            match parsed {
                Ok(parsed) => source.motion.push((key.to_owned(), parsed)),
                Err(diagnostic) => report.push(diagnostic),
            }
        }
    }

    if let Some(typography) = section(&doc, &root, "typography", report) {
        for (key, value) in typography {
            let role = key.get_ref().to_string();
            match doc.table(value, &format!("typography.{role}")) {
                Ok(table) => source.typography.push((role, props(&doc, table, None, report))),
                Err(diagnostic) => report.push(diagnostic),
            }
        }
    }

    if let Some(styles) = section(&doc, &root, "style", report) {
        for (key, value) in styles {
            let selector = match Selector::parse(key.get_ref()) {
                Ok(selector) => selector,
                Err(message) => {
                    report.push(doc.error(&value.span(), message));
                    continue;
                }
            };
            match doc.table(value, &format!("style.\"{selector}\"")) {
                Ok(table) => {
                    let widget = selector.widget().to_owned();
                    source.styles.push((selector, props(&doc, table, Some(&widget), report)));
                }
                Err(diagnostic) => report.push(diagnostic),
            }
        }
    }

    if let Some(icon_table) = section(&doc, &root, "icons", report) {
        icons::read_icon_table(&doc, icon_table, &mut source.icons, report);
    }

    if let Some(animation_table) = section(&doc, &root, "animations", report) {
        animation::read_animation_table(&doc, animation_table, &mut source.animations, report);
    }

    Some(source)
}

fn section<'t, 'a>(
    doc: &Doc<'a>,
    root: &'t DeTable<'a>,
    name: &str,
    report: &mut Vec<Diagnostic>,
) -> Option<&'t DeTable<'a>> {
    let value = doc::get(root, name)?;
    match doc.table(value, name) {
        Ok(table) => Some(table),
        Err(diagnostic) => {
            report.push(diagnostic);
            None
        }
    }
}

fn props(doc: &Doc<'_>, table: &DeTable<'_>, widget: Option<&str>, report: &mut Vec<Diagnostic>) -> RawProps {
    let mut out = Vec::new();
    for (key, value) in table {
        let key = key.get_ref().as_ref();
        let parsed = match widget.and_then(|widget| allowed_words(widget, key)) {
            Some(words) => word(doc, key, value, words),
            None => prop(doc, key, value),
        };
        match parsed {
            Ok(parsed) => out.push((key.to_owned(), parsed, doc.locate(&value.span()))),
            Err(diagnostic) => report.push(diagnostic),
        }
    }
    out
}

fn prop(doc: &Doc<'_>, key: &str, value: &Value<'_>) -> Result<RawProp, Diagnostic> {
    let cells = |v: &DeValue<'_>| integer(v).and_then(|n| u16::try_from(n).ok());
    match value.get_ref() {
        DeValue::String(text) => Expr::parse(text)
            .map(RawProp::Expr)
            .map_err(|message| doc.error(&value.span(), message)),
        DeValue::Boolean(flag) => Ok(RawProp::Flag(*flag)),
        DeValue::Integer(_) => cells(value.get_ref())
            .map(RawProp::Cells)
            .ok_or_else(|| doc.error(&value.span(), format!("`{key}` must be a cell count from 0 to 65535"))),
        DeValue::Array(items) if items.len() == 2 => match (cells(items[0].get_ref()), cells(items[1].get_ref())) {
            (Some(vertical), Some(horizontal)) => Ok(RawProp::Pair(vertical, horizontal)),
            _ => Err(doc.error(&value.span(), format!("`{key}` must be two cell counts like [0, 2]"))),
        },
        other => Err(doc.error(
            &value.span(),
            format!(
                "`{key}` has an unsupported {} value; use a colour string, true/false, a cell count or [vertical, horizontal]",
                other.type_str()
            ),
        )),
    }
}

/// Reads a property that holds one of `words`.
fn word(doc: &Doc<'_>, key: &str, value: &Value<'_>, words: &'static [&'static str]) -> Result<RawProp, Diagnostic> {
    value
        .get_ref()
        .as_str()
        .and_then(|text| words.iter().find(|word| **word == text))
        .map(|word| RawProp::Word(word))
        .ok_or_else(|| doc.error(&value.span(), format!("`{key}` must be one of: {}", words.join(", "))))
}

/// Reads an integer value, if `value` is one that fits in `i64`.
fn integer(value: &DeValue<'_>) -> Option<i64> {
    let int = value.as_integer()?;
    i64::from_str_radix(int.as_str(), int.radix()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEME: &str = r##"
[meta]
name = "Test"
extends = "monochrome"
icon-set = "default"

[colors]
accent = "#38BDF8"
soft = "mix($accent, $surface, 34%)"
glow = "pulse($accent, #fff)"

[motion]
flash = "90ms"
slide = false
speed = "1s"

[typography]
title = { fg = "$text", bold = true }

[style.button]
bg = "$raised"
padding = [0, 2]
gap = 1

[style."button.primary:hover"]
bg = "pulse($accent, $accent-2)"

[style."Button"]
bg = "#000"

[style.switch]
knob = 7.5

[style."scrollbar.side"]
style = "wavy"

[icons]
check = { nerd = "c", unicode = "c", ascii = "c" }
"##;

    #[test]
    fn reads_every_section() {
        let mut report = Vec::new();
        let source = parse("test.toml", THEME, &mut report).expect("usable theme");
        assert_eq!(source.name, "Test");
        assert_eq!(source.extends.as_ref().map(|e| e.0.as_str()), Some("monochrome"));
        assert_eq!(source.icon_set.as_deref(), Some("default"));
        assert_eq!(source.colors.len(), 2);
        assert_eq!(
            source.motion,
            vec![
                ("flash".to_owned(), MotionValue::Duration(Duration::from_millis(90))),
                ("slide".to_owned(), MotionValue::Flag(false)),
            ]
        );
        assert_eq!(source.typography.len(), 1);
        let selectors: Vec<String> = source.styles.iter().map(|(s, _)| s.to_string()).collect();
        assert_eq!(selectors, vec!["button", "button.primary:hover", "switch", "scrollbar.side"]);
        assert_eq!(source.styles[0].1.len(), 3);
        assert!(source.icons.contains_key("check"));
    }

    #[test]
    fn reports_each_problem_with_its_line() {
        let mut report = Vec::new();
        parse("test.toml", THEME, &mut report);
        let lines: Vec<(usize, &str)> =
            report.iter().map(|d| (d.location.as_ref().map_or(0, |l| l.line), d.message.as_str())).collect();
        assert_eq!(report.len(), 5, "{lines:?}");
        assert!(lines.iter().any(|(_, m)| m.contains("`style` must be one of: block, half, thin, dots")));
        assert!(lines.iter().any(|(line, m)| *line == 10 && m.contains("cannot be a pulse()")));
        assert!(lines.iter().any(|(line, m)| *line == 15 && m.contains("unknown motion key `speed`")));
        assert!(lines.iter().any(|(_, m)| m.contains("invalid widget name `Button`")));
        assert!(lines.iter().any(|(_, m)| m.contains("`knob` has an unsupported float value")));
    }

    #[test]
    fn syntax_error_makes_theme_unusable() {
        let mut report = Vec::new();
        assert!(parse("broken.toml", "[meta\nname = 1", &mut report).is_none());
        assert_eq!(report.len(), 1);
    }

    #[test]
    fn missing_name_makes_theme_unusable() {
        let mut report = Vec::new();
        assert!(parse("anon.toml", "[colors]\naccent = \"#fff\"\n", &mut report).is_none());
        assert!(report[0].message.contains("missing [meta]"));
    }
}
