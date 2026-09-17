//! Progress bars.

use super::eighths;
use super::shimmer_text::sweep_light;
use crate::color::Rgb;
use crate::geometry::{Rect, Size};
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Width of the light band of an indeterminate bar, in cells.
const BAND: f32 = 6.0;

/// A bar that fills as work completes, or sweeps while the amount is unknown.
///
/// Determinate bars fill with eighth-cell precision; in ASCII mode they fill whole cells with
/// colour, rounded to the nearest cell like the charts. Indeterminate bars send a band of light
/// along the track over `motion.shimmer`.
/// Style keys: `progress` (`track`, `fill`) with variants such as `progress.success`,
/// `progress-label`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressBar {
    value: Option<f32>,
    percent: bool,
    variant: Option<String>,
}

impl ProgressBar {
    /// A bar at `value`, from 0 to 1.
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self { value: Some(value.clamp(0.0, 1.0)), percent: true, variant: None }
    }

    /// A bar for work of unknown size.
    #[must_use]
    pub fn indeterminate() -> Self {
        Self { value: None, percent: false, variant: None }
    }

    /// Shows or hides the percentage after a determinate bar; shown by default.
    #[must_use]
    pub fn percent(mut self, show: bool) -> Self {
        self.percent = show;
        self
    }

    /// Theme variant, e.g. `"success"` when finished or `"danger"` when failing.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }
}

impl<Msg: 'static> Widget<Msg> for ProgressBar {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(available.width, 1.min(available.height))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let style = cx.style("progress", self.variant.as_deref(), &[]);
        let track = style.color("track").unwrap_or_else(|| cx.color("raised"));
        let fill = style.color("fill").unwrap_or_else(|| cx.color("accent"));
        let Some(value) = self.value else {
            self.paint_sweep(cx, area, track, fill);
            return;
        };
        let label = if self.percent { format!(" {:>3.0}%", value * 100.0) } else { String::new() };
        let bar_width = area.width.saturating_sub(text::width(&label));
        let bar = Rect::new(area.x, area.y, bar_width, 1);
        cx.clear(bar, track);
        eighths::horizontal(cx, bar, eighths::eighths(value, bar_width), fill);
        if !label.is_empty() {
            let label_style = cx.style("progress-label", self.variant.as_deref(), &[]).text();
            cx.text(area.x + i32::from(bar_width), area.y, &label, label_style, text::width(&label));
        }
    }
}

impl ProgressBar {
    fn paint_sweep(&self, cx: &mut PaintCx<'_>, area: Rect, track: Rgb, fill: Rgb) {
        let reduced = cx.reduced_motion();
        let t = cx.cycle(cx.env().theme().motion().shimmer);
        for column in 0..area.width {
            // With reduced motion the whole track rests at a quarter of the light.
            let intensity = if reduced { 0.25 } else { sweep_light(t, f32::from(area.width), BAND, f32::from(column)) };
            cx.clear(Rect::new(area.x + i32::from(column), area.y, 1, 1), track.mix(fill, intensity));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(ProgressBar);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(self.0.clone()).fill_width();
        }
    }

    #[test]
    fn fills_with_eighth_cells_and_shows_percent() {
        let h = Harness::new(Demo(ProgressBar::new(0.53)), 15, 1);
        assert_eq!(h.screen(), "     ▎      53%\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(0, 0), theme.color("accent"));
        assert_eq!(h.bg(8, 0), theme.color("raised"));
    }

    #[test]
    fn ascii_rounds_to_the_nearest_whole_cell_like_the_charts() {
        let theme_fill = |h: &Harness<Demo>| h.env().theme().color("accent");
        let mut h = Harness::new(Demo(ProgressBar::new(0.55).percent(false)), 10, 1);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert_eq!(h.screen(), "\n", "no partial glyphs in ASCII mode");
        let filled = (0..10).filter(|x| h.bg(*x, 0) == theme_fill(&h)).count();
        assert_eq!(filled, 6, "5.5 cells round up to 6");
        let mut h = Harness::new(Demo(ProgressBar::new(0.54).percent(false)), 10, 1);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        let filled = (0..10).filter(|x| h.bg(*x, 0) == theme_fill(&h)).count();
        assert_eq!(filled, 5, "5.4 cells round down to 5");
        assert_eq!(h.bg(5, 0), h.env().theme().color("raised"));
    }

    #[test]
    fn indeterminate_band_moves() {
        let mut h = Harness::new(Demo(ProgressBar::indeterminate()), 20, 1);
        h.advance(Duration::from_millis(600));
        let first: Vec<_> = (0..20).map(|x| h.bg(x, 0)).collect();
        h.advance(Duration::from_millis(300));
        let second: Vec<_> = (0..20).map(|x| h.bg(x, 0)).collect();
        assert_ne!(first, second);
        assert!(h.screen().trim().is_empty());
    }
}
