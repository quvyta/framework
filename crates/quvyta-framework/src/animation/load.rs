//! Reading `[animations.<name>]` tables from icon set, theme and application files.

use std::collections::BTreeMap;

use toml::de::{DeTable, DeValue};

use super::{
    AnimationFrame, CellAnimation, CellColor, ColorMode, FrameTime, MAX_FRAMES, Playback, check_glyph, is_valid_name,
};
use crate::diagnostics::Diagnostic;
use crate::doc::{self, Doc, Value};
use crate::icons::GlyphMode;

/// Keys of an animation table.
const ANIMATION_KEYS: [&str; 5] = ["frame", "playback", "colors", "rest", "frames"];

/// Keys of one frame.
const FRAME_KEYS: [&str; 5] = ["nerd", "unicode", "ascii", "color", "duration"];

/// Reads every `<name> = { … }` of an `[animations]` table into `animations`. A broken animation
/// is reported with its location and skipped as a whole, so the one it would replace stays.
pub(crate) fn read_animation_table(
    doc: &Doc<'_>,
    table: &DeTable<'_>,
    animations: &mut BTreeMap<String, CellAnimation>,
    report: &mut Vec<Diagnostic>,
) {
    for (key, value) in table {
        let name = key.get_ref();
        if !is_valid_name(name) {
            report.push(
                doc.error(
                    &key.span(),
                    format!("animation name `{name}` may use only lowercase letters, digits and `-`"),
                ),
            );
            continue;
        }
        match read_animation(doc, name, value) {
            Ok(animation) => {
                animations.insert(name.to_string(), animation);
            }
            Err(diagnostic) => report.push(diagnostic),
        }
    }
}

/// Reads a file holding only `[animations.<name>]` tables, such as the animations an application
/// or a tool saved. Returns the usable animations in file order and every problem found; a broken
/// animation is skipped, a file that is not TOML yields no animations.
#[must_use]
pub fn parse_animations(file: &str, text: &str) -> (Vec<(String, CellAnimation)>, Vec<Diagnostic>) {
    let doc = Doc::new(file, text);
    let mut report = Vec::new();
    let root = match doc.parse() {
        Ok(root) => root,
        Err(diagnostic) => return (Vec::new(), vec![diagnostic]),
    };
    let mut found = Vec::new();
    for (key, value) in &root {
        if key.get_ref() != "animations" {
            report.push(doc.error(&key.span(), format!("unknown section `{}`; expected animations", key.get_ref())));
            continue;
        }
        let table = match doc.table(value, "animations") {
            Ok(table) => table,
            Err(diagnostic) => {
                report.push(diagnostic);
                continue;
            }
        };
        let mut animations = BTreeMap::new();
        read_animation_table(&doc, table, &mut animations, &mut report);
        // Keep the order of the file, which a map would lose.
        for (name, _) in table {
            if let Some(animation) = animations.remove(name.get_ref().as_ref()) {
                found.push((name.get_ref().to_string(), animation));
            }
        }
    }
    (found, report)
}

fn read_animation(doc: &Doc<'_>, name: &str, value: &Value<'_>) -> Result<CellAnimation, Diagnostic> {
    let table = doc.table(value, &format!("animation `{name}`"))?;
    if let Some((unknown, entry)) = table.iter().find(|(key, _)| !ANIMATION_KEYS.contains(&key.get_ref().as_ref())) {
        return Err(doc.error(
            &entry.span(),
            format!(
                "animation `{name}` has unknown key `{}`; expected one of: {}",
                unknown.get_ref(),
                ANIMATION_KEYS.join(", ")
            ),
        ));
    }
    let mut animation = CellAnimation::new();
    if let Some(entry) = doc::get(table, "frame") {
        let text = doc.string(entry, &format!("animation `{name}`.frame"))?;
        let time = FrameTime::parse(text).map_err(|message| doc.error(&entry.span(), message))?;
        animation = animation.frame_time(time);
    }
    if let Some(entry) = doc::get(table, "playback") {
        let text = doc.string(entry, &format!("animation `{name}`.playback"))?;
        let playback = Playback::from_name(text).ok_or_else(|| {
            doc.error(&entry.span(), format!("animation `{name}`.playback is `{text}`; use loop, once or bounce"))
        })?;
        animation = animation.playback(playback);
    }
    if let Some(entry) = doc::get(table, "colors") {
        let text = doc.string(entry, &format!("animation `{name}`.colors"))?;
        let colors = ColorMode::from_name(text).ok_or_else(|| {
            doc.error(&entry.span(), format!("animation `{name}`.colors is `{text}`; use step or blend"))
        })?;
        animation = animation.colors(colors);
    }
    let Some(frames) = doc::get(table, "frames") else {
        return Err(doc.error(&value.span(), format!("animation `{name}` has no `frames`")));
    };
    let DeValue::Array(items) = frames.get_ref() else {
        return Err(doc.error(
            &frames.span(),
            format!("animation `{name}`.frames must be an array of frames, found {}", frames.get_ref().type_str()),
        ));
    };
    if items.is_empty() {
        return Err(doc.error(&frames.span(), format!("animation `{name}` needs at least one frame")));
    }
    if items.len() > MAX_FRAMES {
        return Err(doc.error(
            &frames.span(),
            format!("animation `{name}` has {} frames; at most {MAX_FRAMES} are allowed", items.len()),
        ));
    }
    for (index, item) in items.iter().enumerate() {
        animation = animation.frame(read_frame(doc, &format!("animation `{name}` frame {}", index + 1), item)?);
    }
    if let Some(entry) = doc::get(table, "rest") {
        let count = items.len();
        let rest = integer(entry.get_ref())
            .and_then(|rest| usize::try_from(rest).ok())
            .filter(|rest| (1..=count).contains(rest))
            .ok_or_else(|| {
                doc.error(&entry.span(), format!("animation `{name}`.rest must be a frame number from 1 to {count}"))
            })?;
        animation = animation.rest(rest - 1);
    }
    Ok(animation)
}

fn read_frame(doc: &Doc<'_>, what: &str, value: &Value<'_>) -> Result<AnimationFrame, Diagnostic> {
    let table = doc.table(value, what)?;
    if let Some((unknown, entry)) = table.iter().find(|(key, _)| !FRAME_KEYS.contains(&key.get_ref().as_ref())) {
        return Err(doc.error(
            &entry.span(),
            format!("{what} has unknown key `{}`; expected one of: {}", unknown.get_ref(), FRAME_KEYS.join(", ")),
        ));
    }
    let glyph = |key: &str, mode: GlyphMode| -> Result<Option<String>, Diagnostic> {
        let Some(entry) = doc::get(table, key) else {
            return Ok(None);
        };
        let text = doc.string(entry, &format!("{what}.{key}"))?;
        check_glyph(text, mode).map_err(|message| doc.error(&entry.span(), format!("{what}.{key}: {message}")))?;
        Ok(Some(text.to_owned()))
    };
    let Some(ascii) = glyph("ascii", GlyphMode::Ascii)? else {
        return Err(doc.error(&value.span(), format!("{what} is missing its `ascii` glyph, which every frame needs")));
    };
    let mut frame = AnimationFrame::new(ascii);
    if let Some(unicode) = glyph("unicode", GlyphMode::Unicode)? {
        frame = frame.unicode(unicode);
    }
    if let Some(nerd) = glyph("nerd", GlyphMode::Nerd)? {
        frame = frame.nerd(nerd);
    }
    if let Some(entry) = doc::get(table, "color") {
        let text = doc.string(entry, &format!("{what}.color"))?;
        let color = CellColor::parse(text).map_err(|message| doc.error(&entry.span(), format!("{what}: {message}")))?;
        frame = frame.color(color);
    }
    if let Some(entry) = doc::get(table, "duration") {
        let text = doc.string(entry, &format!("{what}.duration"))?;
        let time = FrameTime::parse(text).map_err(|message| doc.error(&entry.span(), format!("{what}: {message}")))?;
        frame = frame.duration(time);
    }
    Ok(frame)
}

/// Reads an integer value, if `value` is one that fits in `i64`.
fn integer(value: &DeValue<'_>) -> Option<i64> {
    let int = value.as_integer()?;
    i64::from_str_radix(int.as_str(), int.radix()).ok()
}
