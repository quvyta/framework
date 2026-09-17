//! The studio's working copy of an animation: every field as typed, so a half-written glyph or
//! colour stays on screen while the preview plays what is valid.

use std::time::Duration;

use qframe::animation::{
    AnimationFrame, CellAnimation, CellColor, ColorMode, FrameTime, Playback, check_glyph, is_valid_name,
};
use qframe::icons::GlyphMode;

/// The glyph modes in the order the studio shows their columns.
pub const MODES: [GlyphMode; 3] = [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii];

/// The `[motion]` keys a frame time can follow, most useful first.
pub const MOTION_KEYS: [&str; 9] =
    ["spinner", "step", "flash", "enter", "shimmer", "pulse-period", "cursor-blink", "page", "hover-delay"];

/// One frame as typed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FrameDraft {
    /// Nerd Font, Unicode and ASCII glyphs, in [`MODES`] order; empty falls back.
    pub glyphs: [String; 3],
    /// A colour expression; empty takes the widget's colour.
    pub color: String,
    /// A motion key or duration; empty uses the animation's frame time.
    pub duration: String,
}

/// How long frames last, as chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeDraft {
    /// A `[motion]` key, by index into [`MOTION_KEYS`].
    Motion(usize),
    /// Fixed milliseconds.
    Millis(u32),
}

/// Which field a problem is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Glyph(usize),
    Color,
    Duration,
}

/// Something the studio cannot use as typed, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The frame, counted from 0, or `None` for the animation itself.
    pub frame: Option<usize>,
    pub field: Field,
    pub message: String,
}

/// A working animation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub name: String,
    pub time: TimeDraft,
    pub playback: Playback,
    pub colors: ColorMode,
    /// The rest frame, counted from 0; `None` rests where the playback does.
    pub rest: Option<usize>,
    pub frames: Vec<FrameDraft>,
}

impl Draft {
    /// A new animation called `name`: one dot, looping at the spinner pace.
    pub fn new(name: String) -> Self {
        let dot = FrameDraft { glyphs: [String::new(), "●".to_owned(), "*".to_owned()], ..FrameDraft::default() };
        Self {
            name,
            time: TimeDraft::Motion(0),
            playback: Playback::Loop,
            colors: ColorMode::Step,
            rest: None,
            frames: vec![dot],
        }
    }

    /// The fields of `animation`, named `name`.
    pub fn from_animation(name: &str, animation: &CellAnimation) -> Self {
        let frames = animation
            .frames()
            .iter()
            .map(|frame| FrameDraft {
                glyphs: MODES.map(|mode| frame.own_glyph(mode).unwrap_or_default().to_owned()),
                color: frame.frame_color().map(|color| color.as_str().to_owned()).unwrap_or_default(),
                duration: frame.frame_duration().map(|time| time.to_string()).unwrap_or_default(),
            })
            .collect();
        let time = match animation.time() {
            FrameTime::Motion(key) => TimeDraft::Motion(MOTION_KEYS.iter().position(|k| *k == key).unwrap_or(0)),
            FrameTime::Fixed(duration) => TimeDraft::Millis(u32::try_from(duration.as_millis()).unwrap_or(u32::MAX)),
        };
        Self {
            name: name.to_owned(),
            time,
            playback: animation.play_mode(),
            colors: animation.color_mode(),
            rest: animation.rest_frame(),
            frames,
        }
    }

    /// The frame time chosen.
    pub fn frame_time(&self) -> FrameTime {
        match self.time {
            TimeDraft::Motion(index) => FrameTime::Motion(MOTION_KEYS[index.min(MOTION_KEYS.len() - 1)]),
            TimeDraft::Millis(millis) => FrameTime::Fixed(Duration::from_millis(u64::from(millis.max(1)))),
        }
    }

    /// The animation built from every valid field, and a problem for each field left out. A frame
    /// without a valid ASCII glyph is left out whole; a broken Unicode or Nerd glyph falls back, a
    /// broken colour takes the widget's colour and a broken duration the frame time.
    pub fn build(&self) -> (CellAnimation, Vec<Problem>) {
        let mut problems = Vec::new();
        if !is_valid_name(&self.name) {
            problems.push(Problem {
                frame: None,
                field: Field::Name,
                message: format!("`{}` may use only lowercase letters, digits and `-`", self.name),
            });
        }
        let mut animation =
            CellAnimation::new().frame_time(self.frame_time()).playback(self.playback).colors(self.colors);
        for (index, draft) in self.frames.iter().enumerate() {
            let mut problem = |field, message: String| problems.push(Problem { frame: Some(index), field, message });
            let mut glyphs = [None, None, None];
            for (column, mode) in MODES.iter().enumerate() {
                match glyph(&draft.glyphs[column], *mode) {
                    Ok(found) => glyphs[column] = found,
                    Err(message) => problem(Field::Glyph(column), message),
                }
            }
            let [nerd, unicode, ascii] = glyphs;
            let Some(ascii) = ascii else {
                if draft.glyphs[2].is_empty() {
                    problem(Field::Glyph(2), "every frame needs an ASCII glyph".to_owned());
                }
                continue;
            };
            let mut frame = AnimationFrame::new(ascii);
            if let Some(unicode) = unicode {
                frame = frame.unicode(unicode);
            }
            if let Some(nerd) = nerd {
                frame = frame.nerd(nerd);
            }
            if !draft.color.trim().is_empty() {
                match CellColor::parse(&draft.color) {
                    Ok(color) => frame = frame.color(color),
                    Err(message) => problem(Field::Color, message),
                }
            }
            if !draft.duration.trim().is_empty() {
                match FrameTime::parse(&draft.duration) {
                    Ok(time) => frame = frame.duration(time),
                    Err(message) => problem(Field::Duration, message),
                }
            }
            animation = animation.frame(frame);
        }
        if let Some(rest) = self.rest.filter(|rest| *rest < animation.frames().len()) {
            animation = animation.rest(rest);
        }
        (animation, problems)
    }
}

/// Reads a glyph field: empty falls back, `U+F0995` or `\uF0995` names a code point, anything
/// else is the glyph itself.
pub fn glyph(text: &str, mode: GlyphMode) -> Result<Option<String>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let code = text.strip_prefix("U+").or_else(|| text.strip_prefix("\\u")).or_else(|| text.strip_prefix("\\U"));
    let glyph = match code {
        Some(hex) if !hex.is_empty() => u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .map(String::from)
            .ok_or_else(|| format!("`{text}` is not a code point"))?,
        _ => text.to_owned(),
    };
    check_glyph(&glyph, mode)?;
    Ok(Some(glyph))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_built_in_animation_survives_the_round_trip_through_its_fields() {
        let icons = qframe::icons::IconSetRegistry::builtin().icons(
            "default",
            &std::collections::BTreeMap::new(),
            GlyphMode::Unicode,
        );
        for name in icons.animation_names() {
            let animation = icons.animation(name).expect("listed");
            let (built, problems) = Draft::from_animation(name, animation).build();
            assert!(problems.is_empty(), "{name}: {problems:?}");
            assert_eq!(built, **animation, "{name}");
        }
    }

    #[test]
    fn broken_fields_are_left_out_with_a_reason() {
        let mut draft = Draft::new("Bad Name".to_owned());
        draft.frames.push(FrameDraft {
            glyphs: ["中".to_owned(), "U+25D0".to_owned(), "o".to_owned()],
            color: "mix($accent)".to_owned(),
            duration: "soon".to_owned(),
        });
        draft
            .frames
            .push(FrameDraft { glyphs: [String::new(), "◑".to_owned(), String::new()], ..FrameDraft::default() });
        let (built, problems) = draft.build();
        assert_eq!(built.frames().len(), 2, "the frame without ASCII is left out");
        assert_eq!(built.glyph(1, GlyphMode::Unicode), "◐", "a code point names the glyph");
        assert_eq!(built.glyph(1, GlyphMode::Nerd), "◐", "the wide Nerd glyph falls back");
        assert!(built.frames()[1].frame_color().is_none());
        let fields: Vec<(Option<usize>, Field)> = problems.iter().map(|p| (p.frame, p.field)).collect();
        assert_eq!(
            fields,
            [
                (None, Field::Name),
                (Some(1), Field::Glyph(0)),
                (Some(1), Field::Color),
                (Some(1), Field::Duration),
                (Some(2), Field::Glyph(2)),
            ]
        );
        assert_eq!(glyph("\\uF0995", GlyphMode::Nerd), Ok(Some("\u{F0995}".to_owned())));
        assert!(glyph("U+ZZ", GlyphMode::Nerd).is_err());
    }
}
