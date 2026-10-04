//! Progress bars.

use super::eighths;
use super::shimmer_text::sweep_light;
use crate::geometry::{Rect, Size};
use crate::text;
use crate::theme::State;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Width of the light band of an indeterminate bar, in cells.
const BAND: f32 = 6.0;

/// A bar that fills as work completes, or sweeps while the amount is unknown.
///
/// Determinate bars fill with eighth-cell precision; in ASCII mode they fill whole cells with
/// colour, rounded to the nearest cell like the charts. Indeterminate bars send a band of light
/// along the track over `motion.shimmer`. A progress bar is a readout: it takes no focus and
/// answers no pointer. To show the same bar and let a person click and drag it, use a
/// [`SeekBar`](super::SeekBar), which draws exactly this picture.
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
        let Some(value) = self.value else {
            paint_sweep(cx, area, self.variant.as_deref());
            return;
        };
        paint_bar(cx, area, value, self.percent, "progress", self.variant.as_deref(), &[]);
    }
}

/// The bar a [`ProgressBar`] and a [`SeekBar`](super::SeekBar) draw: the fill on a quiet track, in
/// eighths of a cell, with the percentage written after it when `percent` says so. `key` is the
/// style the theme reads, so a bar a person can drag can step lighter under the pointer while a
/// plain progress bar stays as it is; the percentage is the same label in both, so it takes the
/// same style. Written once so that putting one bar where the other stood cannot change the
/// picture.
pub(crate) fn paint_bar(
    cx: &mut PaintCx<'_>,
    area: Rect,
    value: f32,
    percent: bool,
    key: &str,
    variant: Option<&str>,
    states: &[State],
) {
    let style = cx.style(key, variant, states);
    let track = style.color("track").unwrap_or_else(|| cx.color("raised"));
    let fill = style.color("fill").unwrap_or_else(|| cx.color("accent"));
    let label = label(value, percent);
    let bar = bar_area(area, value, percent);
    cx.clear(bar, track);
    eighths::horizontal(cx, bar, eighths::eighths(value, bar.width), fill);
    if !label.is_empty() {
        let label_style = cx.style("progress-label", variant, &[]).text();
        cx.text(bar.right(), bar.y, &label, label_style, text::width(&label));
    }
}

/// The cells a determinate bar's fill lives in: the whole area less the percentage written after
/// it. A [`SeekBar`](super::SeekBar) presses and hovers over exactly these cells, so what a press
/// means never drifts from what is drawn.
pub(crate) fn bar_area(area: Rect, value: f32, percent: bool) -> Rect {
    let width = area.width.saturating_sub(text::width(&label(value, percent)));
    Rect::new(area.x, area.y, width, 1)
}

/// The percentage written after a determinate bar, empty when it is not shown.
fn label(value: f32, percent: bool) -> String {
    if percent { format!(" {:>3.0}%", value * 100.0) } else { String::new() }
}

/// Sweeps a band of light along `area`, for a bar whose size is not known.
fn paint_sweep(cx: &mut PaintCx<'_>, area: Rect, variant: Option<&str>) {
    let style = cx.style("progress", variant, &[]);
    let track = style.color("track").unwrap_or_else(|| cx.color("raised"));
    let fill = style.color("fill").unwrap_or_else(|| cx.color("accent"));
    let reduced = cx.reduced_motion();
    let t = cx.cycle(cx.env().theme().motion().shimmer);
    for column in 0..area.width {
        // With reduced motion the whole track rests at a quarter of the light.
        let intensity = if reduced { 0.25 } else { sweep_light(t, f32::from(area.width), BAND, f32::from(column)) };
        cx.clear(Rect::new(area.x + i32::from(column), area.y, 1, 1), track.mix(fill, intensity));
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
