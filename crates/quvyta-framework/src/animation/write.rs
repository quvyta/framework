//! Writing an animation back as the `[animations.<name>]` table the loaders read.

use std::fmt::Write as _;

use super::CellAnimation;
use crate::icons::GlyphMode;

impl CellAnimation {
    /// The `[animations.<name>]` table for this animation, ready to paste into an icon set, theme
    /// or animation file. Reading it back gives the same animation. Glyphs a frame leaves to its
    /// fallback are left out, and Nerd Font glyphs from the private use areas are written as
    /// `\u` escapes so they survive fonts and clipboards that cannot show them.
    #[must_use]
    pub fn to_toml(&self, name: &str) -> String {
        let mut out = format!("[animations.{name}]\n");
        // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
        // results are dropped for that reason and no other.
        let _ = writeln!(out, "frame = {}", quoted(&self.frame_time.to_string()));
        let _ = writeln!(out, "playback = {}", quoted(self.playback.name()));
        let _ = writeln!(out, "colors = {}", quoted(self.colors.name()));
        if let Some(rest) = self.rest {
            let _ = writeln!(out, "rest = {}", rest + 1);
        }
        out.push_str("frames = [\n");
        for frame in &self.frames {
            let mut fields = Vec::new();
            for (key, mode) in [("nerd", GlyphMode::Nerd), ("unicode", GlyphMode::Unicode), ("ascii", GlyphMode::Ascii)]
            {
                if let Some(glyph) = frame.own_glyph(mode) {
                    fields.push(format!("{key} = {}", quoted(glyph)));
                }
            }
            if let Some(color) = &frame.color {
                fields.push(format!("color = {}", quoted(color.as_str())));
            }
            if let Some(duration) = frame.duration {
                fields.push(format!("duration = {}", quoted(&duration.to_string())));
            }
            let _ = writeln!(out, "  {{ {} }},", fields.join(", "));
        }
        out.push_str("]\n");
        out
    }
}

/// `text` as a TOML basic string.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
    // results are dropped for that reason and no other.
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() || is_private_use(c) => {
                let code = u32::from(c);
                if code > 0xFFFF {
                    let _ = write!(out, "\\U{code:08X}");
                } else {
                    let _ = write!(out, "\\u{code:04X}");
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Whether `c` is in a private use area, where Nerd Font glyphs live.
fn is_private_use(c: char) -> bool {
    matches!(u32::from(c), 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x10_0000..=0x10_FFFD)
}
