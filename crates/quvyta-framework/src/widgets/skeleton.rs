//! Skeletons: the shape of content that is still loading.

use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::GlyphMode;
use crate::motion::Easing;
use crate::style::CellStyle;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Width of the band of light, in cells.
const BAND: f32 = 10.0;

/// Widths of successive text lines, in percent of the width; the last line is always short.
const LINE_WIDTHS: [u16; 4] = [100, 86, 94, 72];

/// Width of the last text line, in percent.
const LAST_LINE: u16 = 58;

/// Height of a block that is not given one, in rows.
const BLOCK_ROWS: u16 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Lines(u16),
    Avatar,
    Block,
}

/// A placeholder in the shape of content that is on its way: text lines, an avatar or a block.
///
/// Shapes are drawn in a quiet tone, and the signature sweep passes over them: a band of light
/// that moves across the screen once per `motion.shimmer`, blended cell by cell. The band is
/// placed by screen column, so skeletons side by side share one sweep. With reduced motion the
/// shapes stand still. Text lines are drawn in the upper half of each row so lines read apart;
/// ASCII mode fills whole cells.
///
/// Style keys: `skeleton` (`bg` for the shapes, `highlight` for the light).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skeleton {
    shape: Shape,
}

impl Skeleton {
    /// `count` lines of text of varied widths, the last one short, like a paragraph.
    #[must_use]
    pub fn lines(count: u16) -> Self {
        Self { shape: Shape::Lines(count.max(1)) }
    }

    /// A two-cell avatar or icon.
    #[must_use]
    pub fn avatar() -> Self {
        Self { shape: Shape::Avatar }
    }

    /// A block that fills the area it gets: a chart, an image, a card. Three rows unless its
    /// node is given a height.
    #[must_use]
    pub fn block() -> Self {
        Self { shape: Shape::Block }
    }
}

impl<Msg: 'static> Widget<Msg> for Skeleton {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let size = match self.shape {
            Shape::Lines(count) => Size::new(available.width, count),
            Shape::Avatar => Size::new(2, 1),
            Shape::Block => Size::new(available.width, BLOCK_ROWS),
        };
        size.min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let style = cx.style("skeleton", None, &[]);
        let base = style.color("bg").unwrap_or_else(|| cx.color("raised"));
        let light = style.color("highlight").unwrap_or_else(|| cx.color("active"));
        let sweep = Sweep::new(cx);
        let ascii = cx.env().glyph_mode() == GlyphMode::Ascii;
        match self.shape {
            Shape::Lines(count) => {
                for row in 0..count.min(area.height) {
                    let percent = if row + 1 == count && count > 1 {
                        LAST_LINE
                    } else {
                        LINE_WIDTHS[usize::from(row) % LINE_WIDTHS.len()]
                    };
                    let width = clamp_u16(i32::from(area.width) * i32::from(percent) / 100).max(1);
                    let y = area.y + i32::from(row);
                    for column in 0..width {
                        let x = area.x + i32::from(column);
                        let color = base.mix(light, sweep.intensity(x));
                        if ascii {
                            cx.clear(Rect::new(x, y, 1, 1), color);
                        } else {
                            cx.text(x, y, "▀", CellStyle::fg(color), 1);
                        }
                    }
                }
            }
            Shape::Avatar | Shape::Block => {
                for column in 0..area.width {
                    let x = area.x + i32::from(column);
                    cx.clear(Rect::new(x, area.y, 1, area.height), base.mix(light, sweep.intensity(x)));
                }
            }
        }
    }
}

/// Where the band of light is in this frame.
struct Sweep {
    center: Option<f32>,
}

impl Sweep {
    fn new(cx: &mut PaintCx<'_>) -> Self {
        if cx.reduced_motion() {
            return Self { center: None };
        }
        let t = cx.cycle(cx.env().theme().motion().shimmer);
        // The band crosses the whole screen, so every skeleton on it is lit in turn.
        let travel = f32::from(cx.buf.area.width) + BAND * 2.0;
        Self { center: Some(Easing::EaseInOut.apply(t) * travel - BAND) }
    }

    /// How lit screen column `x` is, from 0 to 1.
    fn intensity(&self, x: i32) -> f32 {
        let Some(center) = self.center else {
            return 0.0;
        };
        // Screen columns fit f32 exactly.
        let distance = ((x as f32 + 0.5) - center).abs() / BAND;
        if distance >= 1.0 { 0.0 } else { (1.0 - distance).powf(1.6) }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::color::Rgb;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    fn row_colors(h: &Harness<Demo>, y: u16, width: u16) -> Vec<Option<Rgb>> {
        (0..width).map(|x| h.fg(x, y)).collect()
    }

    struct Demo;

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.row(|ui| {
                ui.add(Skeleton::avatar());
                ui.add(Skeleton::lines(3)).width(crate::widget::Length::Cells(10));
            })
            .gap(1);
            ui.add(Skeleton::block()).width(crate::widget::Length::Cells(6)).height(crate::widget::Length::Cells(2));
        }
    }

    #[test]
    fn draws_lines_of_varied_widths_beside_an_avatar() {
        let h = Harness::new(Demo, 16, 5);
        assert_eq!(h.screen(), "   ▀▀▀▀▀▀▀▀▀▀\n   ▀▀▀▀▀▀▀▀\n   ▀▀▀▀▀\n\n\n");
        let raised = h.env().theme().color("raised");
        assert_eq!(h.bg(0, 0), raised);
        assert_eq!(h.bg(3, 3), raised);
    }

    #[test]
    fn light_sweeps_and_rests_under_reduced_motion() {
        let mut h = Harness::new(Demo, 16, 5);
        h.advance(Duration::from_millis(700));
        let early = row_colors(&h, 0, 16);
        h.advance(Duration::from_millis(250));
        let later = row_colors(&h, 0, 16);
        assert_ne!(early, later);
        h.set_reduced_motion(true);
        let raised = h.env().theme().color("raised");
        assert!(row_colors(&h, 0, 13).iter().skip(3).all(|color| *color == raised));
    }

    #[test]
    fn ascii_fills_whole_cells() {
        let mut h = Harness::new(Demo, 16, 5);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "\n\n\n\n\n");
        assert_eq!(h.bg(12, 0), h.env().theme().color("raised"));
    }
}
