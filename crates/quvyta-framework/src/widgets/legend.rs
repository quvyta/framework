//! Legends: the names behind the series tones of a chart.

use super::cells;
use crate::geometry::{Rect, Size};
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Cells of colour that stand for a series.
const SWATCH: u16 = 2;

/// Cells between a swatch and its name.
const GAP: u16 = 1;

/// Cells between two named series.
const SPACING: u16 = 2;

/// The names behind the tones of a chart: a patch of colour and the name it stands for.
///
/// The n-th name takes the theme's n-th series tone
/// ([`Theme::series_color`](crate::theme::Theme::series_color)), which is how every chart picks
/// its tones, so a legend beside a chart names the same colours the chart drew. A legend is not
/// decoration: in a stacked bar or a tinted grid the meaning of a tone lives nowhere else, and
/// the theme carries only five series tones, so a sixth series shares the first one's tone and
/// the name is the only thing that tells them apart.
///
/// [`tones`](Self::tones) pins each name to a tone of the palette instead of its position, the
/// same index its series was given with [`Series::tone`](super::Series::tone), so a category
/// keeps its colour however many others are named beside it.
///
/// Names lie in a row and wrap onto further rows when the area is too narrow; `vertical` puts
/// each on its own row. A name that does not fit is cut with `…`. The swatch is two cells of
/// colour, never a bracketed marker, and it works the same in every glyph mode because it is
/// made of colour rather than characters.
///
/// Style keys: `legend` (`fg` for the names).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Legend {
    names: Vec<String>,
    tones: Vec<usize>,
    vertical: bool,
}

impl Legend {
    /// A legend naming the series of a chart, in the order the chart draws them.
    #[must_use]
    pub fn new(names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { names: names.into_iter().map(Into::into).collect(), tones: Vec::new(), vertical: false }
    }

    /// The palette index of every name, in the order of the names: the n-th name takes the
    /// theme's `tones[n]`-th series tone. A name past the end of `tones` keeps the tone of its
    /// position.
    #[must_use]
    pub fn tones(mut self, tones: impl IntoIterator<Item = usize>) -> Self {
        self.tones = tones.into_iter().collect();
        self
    }

    /// Puts every name on its own row, for a legend beside a chart rather than under it.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// The palette index of the name at `position`.
    fn tone_index(&self, position: usize) -> usize {
        self.tones.get(position).copied().unwrap_or(position)
    }

    /// The width one name takes: the swatch, a gap and the name itself.
    fn item_width(name: &str) -> u16 {
        cells::sum([SWATCH, GAP, text::width(name)])
    }

    /// Where every name goes in `width` cells: its column and row, in order.
    ///
    /// Names are laid out from the left and wrap when the next one would not fit; a name wider
    /// than the whole width still gets its own row, where it is cut while painting.
    fn places(&self, width: u16) -> Vec<(u16, u16)> {
        let mut places = Vec::with_capacity(self.names.len());
        let (mut x, mut y): (u16, u16) = (0, 0);
        for name in &self.names {
            let item = Self::item_width(name);
            if self.vertical {
                places.push((0, y));
                y = y.saturating_add(1);
                continue;
            }
            if x > 0 && cells::sum([x, item]) > width {
                x = 0;
                y = y.saturating_add(1);
            }
            places.push((x, y));
            x = cells::sum([x, item, SPACING]);
        }
        places
    }
}

impl<Msg: 'static> Widget<Msg> for Legend {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.names.is_empty() {
            return Size::default();
        }
        let places = self.places(available.width);
        let rows = places.last().map_or(0, |(_, y)| y.saturating_add(1));
        let width = places
            .iter()
            .zip(&self.names)
            .map(|((x, _), name)| cells::sum([*x, Self::item_width(name)]))
            .max()
            .unwrap_or(0);
        Size::new(width, rows).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() || self.names.is_empty() {
            return;
        }
        let mut style = cx.style("legend", None, &[]).text();
        style.bg = None;
        // A theme without the key still gives the names a colour of their own: a cell with none
        // takes whatever the terminal defaults to, and a dialog dims it into the ground.
        if style.fg.is_none() {
            style.fg = Some(cx.color("dim"));
        }
        let tones: Vec<crate::color::Rgb> =
            (0..self.names.len()).map(|index| cx.env().theme().series_color(self.tone_index(index))).collect();
        for ((x, y), (name, tone)) in self.places(area.width).into_iter().zip(self.names.iter().zip(tones)) {
            if i32::from(y) >= i32::from(area.height) {
                break;
            }
            let row = area.y + i32::from(y);
            let left = area.x + i32::from(x);
            let swatch = SWATCH.min(area.width.saturating_sub(x));
            cx.fill(Rect::new(left, row, swatch, 1), tone);
            let text_x = left + i32::from(swatch) + i32::from(GAP);
            let room = area.right().saturating_sub(text_x);
            let Ok(room) = u16::try_from(room) else {
                continue;
            };
            if room == 0 {
                continue;
            }
            let shown = text::truncate(name, room);
            cx.text(text_x, row, &shown, style, room);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    struct Demo(Legend);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(self.0.clone()).width(Length::Fill(1)).height(Length::Fill(1));
        }
    }

    fn harness(legend: Legend, width: u16, height: u16) -> Harness<Demo> {
        Harness::new(Demo(legend), width, height)
    }

    #[test]
    fn names_sit_after_their_series_tone() {
        let h = harness(Legend::new(["Rust", "Docs", "Review"]), 40, 1);
        assert_eq!(h.screen(), "   Rust     Docs     Review\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(0, 0), Some(theme.series_color(0)), "the first swatch is the first series tone");
        assert_eq!(h.bg(1, 0), Some(theme.series_color(0)), "the swatch is two cells wide");
        assert_eq!(h.bg(9, 0), Some(theme.series_color(1)));
        assert_eq!(h.bg(18, 0), Some(theme.series_color(2)));
        assert_ne!(theme.series_color(0), theme.series_color(1));
    }

    #[test]
    fn a_narrow_area_wraps_and_then_cuts() {
        let h = harness(Legend::new(["Rust", "Docs", "Review"]), 18, 3);
        assert_eq!(h.screen(), "   Rust     Docs\n   Review\n\n");
        let narrow = harness(Legend::new(["Rust", "Docs"]), 10, 2);
        assert_eq!(narrow.screen(), "   Rust\n   Docs\n", "one name a row when only one fits");
        let cut = harness(Legend::new(["Refactoring"]), 8, 1);
        assert_eq!(cut.screen(), "   Refa…\n");
    }

    #[test]
    fn vertical_puts_one_name_on_each_row() {
        let h = harness(Legend::new(["Rust", "Docs"]).vertical(), 20, 2);
        assert_eq!(h.screen(), "   Rust\n   Docs\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(0, 1), Some(theme.series_color(1)));
    }

    #[test]
    fn the_swatch_is_colour_in_every_glyph_mode() {
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            let mut h = harness(Legend::new(["Rust"]), 12, 1);
            h.set_glyph_mode(mode);
            assert_eq!(h.screen(), "   Rust\n", "{mode:?}");
            assert_eq!(h.bg(0, 0), Some(h.env().theme().series_color(0)), "{mode:?}");
        }
    }

    #[test]
    fn pinned_tones_follow_the_category_not_the_position() {
        let h = harness(Legend::new(["Docs", "Review"]).tones([1, 2]), 40, 1);
        let theme = h.env().theme();
        assert_eq!(h.screen(), "   Docs     Review\n", "the names sit exactly where they always do");
        assert_eq!(h.bg(0, 0), Some(theme.series_color(1)), "Docs keeps its own tone without Rust beside it");
        assert_eq!(h.bg(9, 0), Some(theme.series_color(2)));
        let short = harness(Legend::new(["Docs", "Review"]).tones([4]), 40, 1);
        assert_eq!(short.bg(0, 0), Some(theme.series_color(4)));
        assert_eq!(short.bg(9, 0), Some(theme.series_color(1)), "a name past the tones keeps its position's tone");
    }

    #[test]
    fn tones_that_match_the_positions_draw_the_same_legend() {
        for (width, height) in [(40, 1), (18, 3), (8, 1)] {
            let plain = harness(Legend::new(["Rust", "Docs", "Review"]), width, height);
            let pinned = harness(Legend::new(["Rust", "Docs", "Review"]).tones([0, 1, 2]), width, height);
            assert_eq!(plain.buffer(), pinned.buffer(), "{width}×{height}");
        }
    }

    #[test]
    fn nothing_to_name_draws_nothing_and_tiny_areas_survive() {
        let empty = harness(Legend::new(Vec::<String>::new()), 10, 1);
        assert_eq!(empty.screen(), "\n");
        for (width, height) in [(1, 1), (2, 1), (3, 1), (4, 2)] {
            let h = harness(Legend::new(["Rust", "Docs"]), width, height);
            assert_eq!(h.screen().lines().count(), usize::from(height), "{width}×{height}");
        }
    }
}
