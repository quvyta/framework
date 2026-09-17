//! Text that shows work in progress.

use unicode_segmentation::UnicodeSegmentation;

use crate::geometry::{Rect, Size};
use crate::motion::Easing;
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// How [`ShimmerText`] moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShimmerStyle {
    /// Light sweeps across the letters, one cell at a time.
    #[default]
    Sweep,
    /// Dots after the text count up, like someone typing.
    Dots,
}

/// A working message such as "processing": either light passing over the letters or growing
/// dots. One pass takes the theme's `motion.shimmer`; with reduced motion the text is still.
///
/// Style keys: `shimmer` with `fg` for the resting letters and `highlight` for the light.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShimmerText {
    text: String,
    style: ShimmerStyle,
}

/// Width of the band of light, in cells.
const BAND: f32 = 5.0;

/// How lit cell `cell` of `cells` is, from 0 to 1, when a band of light `band` cells wide has
/// swept `t` (0 to 1) of the way across. The band starts before the first cell and ends after
/// the last, eases in and out, and fades from its centre to its edges. The indeterminate progress
/// bar sweeps the same way.
pub(super) fn sweep_light(t: f32, cells: f32, band: f32, cell: f32) -> f32 {
    let travel = cells + band * 2.0;
    let center = Easing::EaseInOut.apply(t) * travel - band;
    let distance = ((cell + 0.5) - center).abs() / band;
    if distance >= 1.0 { 0.0 } else { (1.0 - distance).powf(1.6) }
}

impl ShimmerText {
    /// A sweeping shimmer over `text`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), style: ShimmerStyle::Sweep }
    }

    /// Chooses how it moves.
    #[must_use]
    pub fn style(mut self, style: ShimmerStyle) -> Self {
        self.style = style;
        self
    }
}

impl<Msg: 'static> Widget<Msg> for ShimmerText {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let dots = if self.style == ShimmerStyle::Dots { 3 } else { 0 };
        Size::new(text::width(&self.text).saturating_add(dots), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let style = cx.style("shimmer", None, &[]);
        let base = style.color("fg").unwrap_or_else(|| cx.color("dim"));
        let light = style.color("highlight").unwrap_or_else(|| cx.color("text"));
        let reduced = cx.reduced_motion();
        let t = cx.cycle(cx.env().theme().motion().shimmer);
        match self.style {
            ShimmerStyle::Sweep => {
                let graphemes: Vec<&str> = self.text.graphemes(true).collect();
                let mut x = area.x;
                for (index, grapheme) in graphemes.iter().enumerate() {
                    let intensity =
                        if reduced { 0.0 } else { sweep_light(t, graphemes.len() as f32, BAND, index as f32) };
                    let color = base.mix(light, intensity);
                    x += i32::from(cx.text(x, area.y, grapheme, CellStyle::fg(color), area.width));
                }
            }
            ShimmerStyle::Dots => {
                let count = if reduced { 3 } else { ((t * 4.0) as usize).min(3) };
                let shown = format!("{}{}", self.text, ".".repeat(count));
                cx.text(area.x, area.y, &shown, CellStyle::fg(base), area.width);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(ShimmerStyle);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(ShimmerText::new("processing").style(self.0)).fill_width();
        }
    }

    #[test]
    fn light_travels_across_letters() {
        let mut h = Harness::new(Demo(ShimmerStyle::Sweep), 20, 1);
        assert_eq!(h.screen(), "processing\n");
        h.advance(Duration::from_millis(500));
        let lit_early: Vec<_> = (0..10).map(|x| h.fg(x, 0)).collect();
        h.advance(Duration::from_millis(400));
        let lit_later: Vec<_> = (0..10).map(|x| h.fg(x, 0)).collect();
        assert_ne!(lit_early, lit_later);
    }

    #[test]
    fn dots_count_up_and_rest_when_reduced() {
        let mut h = Harness::new(Demo(ShimmerStyle::Dots), 20, 1);
        assert_eq!(h.screen(), "processing\n");
        h.advance(Duration::from_millis(900));
        assert_eq!(h.screen(), "processing..\n");
        h.set_reduced_motion(true);
        assert_eq!(h.screen(), "processing...\n");
    }
}
