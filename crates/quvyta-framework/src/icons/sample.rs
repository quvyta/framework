//! A row of sample glyphs in one glyph mode, for the user to judge with their own eyes.

use std::borrow::Cow;

use super::GlyphMode;
use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Cells between two glyphs.
const GAP: u16 = 2;

/// A few icons drawn in one [`GlyphMode`], whatever mode the application draws in.
///
/// No program can learn which font a terminal draws with, so the last word on Nerd Font glyphs
/// is the user's eye: a setup screen puts a Nerd sample beside a Unicode one and asks which reads
/// as shapes. A missing Nerd Font shows as boxes or question marks. Place one after installing
/// the font (see [`nerd_font`](super::nerd_font)) to show whether the terminal picked it up.
///
/// ```
/// use qframe::icons::{GlyphMode, GlyphSample};
///
/// let nerd = GlyphSample::new(GlyphMode::Nerd);
/// let unicode = GlyphSample::new(GlyphMode::Unicode).keys(["folder", "check"]);
/// # let _ = (nerd, unicode);
/// ```
///
/// The glyphs come from the active icon set, so a theme's icons show as they would be drawn.
/// They use the `text` colour on no background of their own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphSample {
    mode: GlyphMode,
    keys: Vec<String>,
}

impl GlyphSample {
    /// The icons shown unless [`GlyphSample::keys`] names others: four whose Nerd glyphs lie in
    /// the private use area, so none of them looks right without a Nerd Font.
    pub const KEYS: [&str; 4] = ["folder", "check", "search", "settings"];

    /// A sample of [`GlyphSample::KEYS`] in `mode`.
    #[must_use]
    pub fn new(mode: GlyphMode) -> Self {
        Self { mode, keys: Self::KEYS.iter().map(|key| (*key).to_owned()).collect() }
    }

    /// Shows these icon keys instead, in order. A key the icon set lacks is left out.
    #[must_use]
    pub fn keys(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keys = keys.into_iter().map(Into::into).collect();
        self
    }

    fn glyphs<'a>(&'a self, icons: &'a super::Icons) -> impl Iterator<Item = Cow<'a, str>> + 'a {
        self.keys.iter().filter_map(|key| icons.glyphs(key)).map(|glyphs| Cow::Borrowed(glyphs.for_mode(self.mode)))
    }
}

impl<Msg: 'static> Widget<Msg> for GlyphSample {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let widths: Vec<u16> = self.glyphs(cx.env().icons()).map(|glyph| text::width(&glyph)).collect();
        if widths.is_empty() {
            return Size::new(0, 0);
        }
        let gaps = u16::try_from(widths.len() - 1).unwrap_or(u16::MAX).saturating_mul(GAP);
        let width = widths.into_iter().fold(gaps, u16::saturating_add);
        Size::new(width, 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let glyphs: Vec<String> = self.glyphs(cx.env().icons()).map(Cow::into_owned).collect();
        let style = CellStyle::fg(cx.color("text"));
        let right = area.x + i32::from(area.width);
        let mut x = area.x;
        for glyph in glyphs {
            let room = crate::geometry::clamp_u16(right - x);
            let width = text::width(&glyph);
            // A glyph is never cut: one that no longer fits ends the row.
            if width > room {
                break;
            }
            cx.text(x, area.y, &glyph, style, room);
            x += i32::from(width + GAP);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(Vec<GlyphSample>);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.column(|ui| {
                for sample in &self.0 {
                    ui.add(sample.clone());
                }
            });
        }
    }

    #[test]
    fn draws_each_mode_whatever_the_application_draws_in() {
        let samples = vec![
            GlyphSample::new(GlyphMode::Nerd),
            GlyphSample::new(GlyphMode::Unicode),
            GlyphSample::new(GlyphMode::Ascii),
        ];
        let mut h = Harness::new(Demo(samples), 20, 3);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "\u{f07b}  \u{f00c}  \u{f002}  \u{f013}\n■  ✓  ⌕  ▤\n#  v  /  *\n");
        assert_eq!(h.fg(0, 1), h.env().theme().color("text"));
    }

    #[test]
    fn chosen_keys_skip_unknown_ones_and_never_cut_a_glyph() {
        let sample = GlyphSample::new(GlyphMode::Unicode).keys(["check", "no-such-icon", "folder"]);
        let h = Harness::new(Demo(vec![sample.clone()]), 20, 1);
        assert_eq!(h.screen(), "✓  ■\n");
        let h = Harness::new(Demo(vec![sample]), 3, 1);
        assert_eq!(h.screen(), "✓\n");
    }
}
