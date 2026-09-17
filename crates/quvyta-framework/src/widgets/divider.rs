//! Dividers: separation by space and tone, never by a drawn line.

use crate::geometry::{Rect, Size};
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// A pause between groups of content.
///
/// A plain divider is one empty row: space is the quietest separator and it never competes with
/// the content. A caption turns it into a quiet heading for what follows; a band paints the
/// divider in a tone one step away from its surroundings, the way surfaces separate everywhere
/// else. It never draws a line character.
///
/// Style keys: `divider` (`band`), `divider-label` (`fg`, `bold`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divider {
    space: u16,
    label: Option<String>,
    band: bool,
    vertical: bool,
}

impl Divider {
    /// One empty row.
    #[must_use]
    pub fn new() -> Self {
        Self { space: 1, label: None, band: false, vertical: false }
    }

    /// Rows (or columns, when vertical) of space; 1 by default.
    #[must_use]
    pub fn space(mut self, cells: u16) -> Self {
        self.space = cells;
        self
    }

    /// A caption on its own row after the space, introducing the content below it. Horizontal
    /// dividers only; write it the way panel titles are written, e.g. `"STOPPED"`.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Paints the divider in the band tone instead of leaving it transparent.
    #[must_use]
    pub fn band(mut self) -> Self {
        self.band = true;
        self
    }

    /// Separates side by side content with `space` columns. Give it `.fill_height()` so it spans
    /// the row.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    fn caption(&self) -> Option<&str> {
        self.label.as_deref().filter(|_| !self.vertical)
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: 'static> Widget<Msg> for Divider {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.vertical {
            // One row; `fill_height` stretches it across the row it separates.
            return Size::new(self.space, 1).min(available);
        }
        let label_row = u16::from(self.caption().is_some());
        Size::new(available.width, self.space.saturating_add(label_row)).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if self.band {
            let style = cx.style("divider", None, &[]);
            let band = style.color("band").unwrap_or_else(|| cx.color("raised"));
            cx.clear(area, band);
        }
        if let Some(label) = self.caption()
            && area.height > 0
        {
            let mut style = cx.style("divider-label", None, &[]).text();
            style.bg = None;
            let y = area.bottom() - 1;
            let shown = text::truncate(label, area.width).into_owned();
            cx.text(area.x, y, &shown, style, area.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::Text;

    struct Demo(Divider);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.column(|ui| {
                ui.add(Text::new("web"));
                ui.add(self.0.clone());
                ui.add(Text::new("db"));
            })
            .fill();
        }
    }

    #[test]
    fn plain_divider_is_space() {
        let h = Harness::new(Demo(Divider::new()), 12, 4);
        assert_eq!(h.screen(), "web\n\ndb\n\n");
    }

    #[test]
    fn caption_sits_after_the_space_and_truncates() {
        let h = Harness::new(Demo(Divider::new().label("STOPPED SERVICES")), 10, 5);
        assert_eq!(h.screen(), "web\n\nSTOPPED S…\ndb\n\n");
        assert_eq!(h.fg(0, 2), h.env().theme().color("muted"));
    }

    #[test]
    fn band_paints_a_tone_without_characters() {
        let h = Harness::new(Demo(Divider::new().band().space(2)), 8, 5);
        assert_eq!(h.screen(), "web\n\n\ndb\n\n");
        let theme = h.env().theme();
        let band = theme.color("surface").expect("token").mix(theme.color("raised").expect("token"), 0.6);
        assert_eq!(h.bg(3, 1), Some(band));
        assert_eq!(h.bg(3, 2), Some(band));
    }

    struct Columns;

    impl App for Columns {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.row(|ui| {
                ui.add(Text::new("cpu"));
                ui.add(Divider::new().vertical().space(2).band().label("ignored")).fill_height();
                ui.add(Text::new("mem"));
            })
            .fill();
        }
    }

    #[test]
    fn vertical_divider_is_a_column_gap() {
        let h = Harness::new(Columns, 12, 2);
        assert_eq!(h.screen(), "cpu  mem\n\n");
        assert_eq!(h.bg(3, 1), h.bg(4, 0));
    }

    #[test]
    fn a_huge_space_with_a_caption_fills_the_area() {
        let h = Harness::new(Demo(Divider::new().space(u16::MAX).label("STOPPED")), 10, 4);
        assert_eq!(h.screen(), "web\n\n\nSTOPPED\n", "the caption keeps the last row");
    }
}
