//! Badges: small status pills.

use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

use super::cells;

/// Counts above this are shown as `99+`.
const COUNT_LIMIT: u32 = 99;

/// A status pill: a tinted surface with a marker dot and a word, and optionally a count.
///
/// The shape is the tint, one cell of padding on each side, never brackets. Tones come from
/// theme variants: no variant is neutral, and the built-in themes define `success`, `warning`,
/// `danger`, `info` and `accent`. The marker keeps status readable without colour.
///
/// Style keys: `badge` and `badge.<variant>` (`bg`, `fg`, `dot`), `badge-count` and
/// `badge-count.<variant>` (`bg`, `fg`, `bold`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badge {
    label: String,
    variant: Option<String>,
    count: Option<u32>,
}

impl Badge {
    /// A neutral badge reading `label`.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), variant: None, count: None }
    }

    /// Theme variant for the tone, e.g. `"success"`, `"warning"`, `"danger"`, `"info"` or
    /// `"accent"`.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Adds a count in a stronger segment after the label; counts above 99 read `99+`.
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    fn count_text(&self) -> Option<String> {
        self.count.map(|count| if count > COUNT_LIMIT { format!(" {COUNT_LIMIT}+ ") } else { format!(" {count} ") })
    }
}

impl<Msg: 'static> Widget<Msg> for Badge {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let dot = text::width(&cx.env().icons().glyph("dot"));
        let count = self.count_text().as_deref().map_or(0, text::width);
        // Padding, the dot, a space, the label, padding, then the count segment. A label wider than
        // any screen saturates instead of overflowing.
        let width = cells::sum([1, dot, 1, text::width(&self.label), 1, count]);
        Size::new(width, 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let variant = self.variant.as_deref();
        let style = cx.style("badge", variant, &[]);
        let surface = style.text();
        let bg = surface.bg.unwrap_or_else(|| cx.color("raised"));
        let fg = surface.fg.unwrap_or_else(|| cx.color("dim"));
        let dot_color = style.color("dot").unwrap_or(fg);

        let count = self.count_text();
        let count_width = count.as_deref().map_or(0, text::width).min(area.width);
        let pill_width = area.width - count_width;
        cx.clear(Rect::new(area.x, area.y, pill_width, 1), bg);

        let glyph = cx.env().icons().glyph("dot").into_owned();
        let mut x = area.x + 1;
        let right = area.x + i32::from(pill_width) - 1;
        let budget = |x: i32| crate::geometry::clamp_u16(right - x);
        x += i32::from(cx.text(x, area.y, &glyph, CellStyle::fg(dot_color), budget(x)));
        x += 1;
        let label = text::truncate(&self.label, budget(x)).into_owned();
        cx.text(x, area.y, &label, CellStyle { bg: None, ..surface }, budget(x));

        if let Some(count) = count {
            let count_style = cx.style("badge-count", variant, &[]).text();
            let count_bg = count_style.bg.unwrap_or(bg);
            let start = area.x + i32::from(pill_width);
            cx.clear(Rect::new(start, area.y, count_width, 1), count_bg);
            cx.text(start, area.y, &count, count_style, count_width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(Vec<Badge>);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.row(|ui| {
                for badge in &self.0 {
                    ui.add(badge.clone());
                }
            })
            .gap(1);
        }
    }

    #[test]
    fn draws_a_tinted_pill_with_a_marker() {
        let h = Harness::new(Demo(vec![Badge::new("Running").variant("success"), Badge::new("Idle")]), 30, 1);
        assert_eq!(h.screen(), " ● Running   ● Idle\n");
        let theme = h.env().theme();
        let success = theme.color("success").expect("token");
        let surface = theme.color("surface").expect("token");
        assert_eq!(h.bg(0, 0), Some(surface.mix(success, 0.16)));
        assert_eq!(h.fg(1, 0), Some(success));
        assert_eq!(h.fg(3, 0), Some(success));
        assert_eq!(h.bg(12, 0), theme.color("raised"));
        assert_eq!(h.fg(15, 0), theme.color("dim"));
    }

    #[test]
    fn count_segment_caps_at_ninety_nine() {
        let h = Harness::new(
            Demo(vec![Badge::new("Alerts").variant("danger").count(3), Badge::new("Logs").count(250)]),
            40,
            1,
        );
        assert_eq!(h.screen(), " ● Alerts  3   ● Logs  99+\n");
        assert_ne!(h.bg(10, 0), h.bg(8, 0), "the count sits on a stronger tint");
    }

    #[test]
    fn truncates_when_narrow_and_uses_ascii_marker() {
        let mut h = Harness::new(Demo(vec![Badge::new("Degraded performance").variant("warning")]), 12, 1);
        assert_eq!(h.screen(), " ● Degrade…\n");
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), " * Degrade…\n");
    }

    #[test]
    fn a_label_wider_than_any_screen_is_cut_without_overflowing() {
        let h = Harness::new(Demo(vec![Badge::new("x".repeat(70_000)).count(7)]), 12, 1);
        assert_eq!(h.screen(), " ● xxxx…  7\n");
    }
}
