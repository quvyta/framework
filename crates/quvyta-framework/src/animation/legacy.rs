//! The spinner icons of earlier versions. Spinner frames used to be icons holding one glyph per
//! frame (`spinner-arc = { nerd = "◜◠◝◞◡◟", … }`). Those keys are no longer icons; an icon set or
//! theme that still writes one replaces the glyphs of the matching animation, keeping its timing,
//! playback and colours.

use std::collections::BTreeMap;
use std::sync::Arc;

use unicode_segmentation::UnicodeSegmentation;

use super::{AnimationFrame, CellAnimation, MAX_FRAMES, check_glyph};
use crate::icons::{GlyphMode, IconGlyphs};

/// Former icon keys and the animations they now replace the glyphs of.
pub(crate) const LEGACY_ICONS: [(&str, &str); 7] = [
    ("spinner", "spinner-dots"),
    ("spinner-arc", "spinner-arc"),
    ("spinner-done", "spinner-done"),
    ("spinner-orbit", "spinner-orbit"),
    ("spinner-pop", "spinner-pop"),
    ("spinner-quarters", "spinner-quarters"),
    ("spinner-slices", "spinner-slices"),
];

const MODES: [GlyphMode; 3] = [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii];

/// The frames of each glyph mode, one grapheme each.
fn split(glyphs: &IconGlyphs) -> [Vec<&str>; 3] {
    MODES.map(|mode| glyphs.for_mode(mode).graphemes(true).collect())
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// How many frames play every mode's sequence whole: modes may have different frame counts, and
/// the animation shows each mode's frames in the same order over time.
fn frame_count(frames: &[Vec<&str>; 3]) -> usize {
    frames.iter().map(Vec::len).fold(1, |count, len| count / gcd(count, len) * len)
}

/// Checks the glyphs of a former spinner icon: every frame one cell, and a frame count that fits.
pub(crate) fn check_legacy(glyphs: &IconGlyphs) -> Result<(), String> {
    let frames = split(glyphs);
    for (mode, list) in MODES.iter().zip(&frames) {
        for glyph in list {
            check_glyph(glyph, *mode)?;
        }
    }
    let count = frames.iter().try_fold(1_usize, |count, list| {
        let next = count / gcd(count, list.len()) * list.len();
        (next <= MAX_FRAMES).then_some(next)
    });
    match count {
        Some(_) => Ok(()),
        None => Err(format!("these frame counts need more than {MAX_FRAMES} frames to play in step")),
    }
}

/// Replaces the frames of animation `name` in `animations` with the glyphs of a former spinner
/// icon. Timing, playback and colour mode stay; each new frame takes the colour and duration of the
/// frame at the same relative place in the animation it replaces, so a finish that blends into
/// success still does with fewer frames.
pub(crate) fn apply_legacy(animations: &mut BTreeMap<String, Arc<CellAnimation>>, name: &str, glyphs: &IconGlyphs) {
    let frames = split(glyphs);
    let count = frame_count(&frames);
    let base = animations.get(name).map_or_else(CellAnimation::new, |animation| CellAnimation::clone(animation));
    let mut replaced = CellAnimation { frames: Vec::with_capacity(count), ..base.clone() };
    for index in 0..count {
        let [nerd, unicode, ascii] = &frames;
        let mut frame = AnimationFrame::new(ascii[index % ascii.len()])
            .unicode(unicode[index % unicode.len()])
            .nerd(nerd[index % nerd.len()]);
        if let Some(model) = relative(&base.frames, index, count) {
            frame.color.clone_from(&model.color);
            frame.duration = model.duration;
        }
        replaced.frames.push(frame);
    }
    replaced.rest = base.rest.map(|rest| rest.min(count.saturating_sub(1)));
    animations.insert(name.to_owned(), Arc::new(replaced));
}

/// The frame of `frames` at the place of `index` among `count` frames.
fn relative(frames: &[AnimationFrame], index: usize, count: usize) -> Option<&AnimationFrame> {
    let last = frames.len().checked_sub(1)?;
    if count <= 1 {
        return frames.first();
    }
    // Rounded to the nearest frame: index / (count - 1) of the way through.
    let at = (index * last * 2 + (count - 1)) / ((count - 1) * 2);
    frames.get(at)
}
