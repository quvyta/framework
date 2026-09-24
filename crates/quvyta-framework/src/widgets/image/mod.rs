//! Pictures in the terminal: decoded pixels drawn with half blocks, in any terminal with 256
//! colours or more, over SSH too.

mod data;
mod resample;
#[cfg(test)]
mod tests;

pub use data::{ImageData, ImageError};
use resample::Half;

use super::EmptyState;
use crate::color::ColorDepth;
use crate::env::Env;
use crate::geometry::{Rect, Size};
use crate::i18n::translate_active;
use crate::icons::GlyphMode;
use crate::style::to_color;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// How a picture fills the area it is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    /// The whole picture, as large as fits with its shape kept; the ground shows beside it.
    #[default]
    Contain,
    /// The whole area, with the picture's shape kept: what spills over is cut evenly from both
    /// sides. For wallpapers.
    Cover,
    /// The picture at its own size, one pixel per half cell, in the middle; what does not fit is
    /// cut evenly from both sides.
    Center,
}

/// A picture, drawn with half blocks.
///
/// Every cell shows two pixels, one above the other: `▀` in the upper pixel's colour on the lower
/// pixel's. A cell is about twice as tall as it is wide, so these pixels are close to square and
/// a picture keeps its shape. Shrinking averages every pixel it covers (a box filter), so a photo
/// does not flicker into noise. The cells are worked out once and kept in the widget's memory;
/// they are worked out again only when the area's size, the fit or the picture changes, so a
/// frame that repaints the same picture only copies them. Cells the picture does not reach keep
/// what is under it, so a picture can be the ground other widgets are drawn on.
///
/// At [`ColorDepth::Ansi256`] every half takes its nearest palette entry. With the sixteen
/// standard colours, or in ASCII glyph mode, pixels cannot be shown and are never drawn with
/// characters: the widget says instead what the picture is (the file's name, its format and
/// size) and that this terminal cannot show pictures.
///
/// Decoding is the application's work, in the background with
/// [`Command::perform`](crate::runtime::Command::perform), and so are the states before a picture
/// exists: a [`Spinner`](super::Spinner) while it decodes, an [`EmptyState`] when it could not be.
///
/// Measures the cells its fit needs within the room it is given: all of it for [`Fit::Cover`],
/// the picture's shape for [`Fit::Contain`], its own size for [`Fit::Center`].
///
/// Style keys: `empty-state-icon`, `empty-state-title` and `empty-state-message`, through the
/// empty state it shows where pictures cannot be drawn.
#[derive(Debug, Clone)]
pub struct Image {
    data: ImageData,
    fit: Fit,
}

impl Image {
    /// Draws `data`; a clone of it is kept, which costs a reference count.
    #[must_use]
    pub fn new(data: &ImageData) -> Self {
        Self { data: data.clone(), fit: Fit::default() }
    }

    /// How the picture fills its area; [`Fit::Contain`] when not set.
    #[must_use]
    pub fn fit(mut self, fit: Fit) -> Self {
        self.fit = fit;
        self
    }

    /// What is shown where pixels cannot be: what the picture is, and that it cannot be shown.
    fn cannot_show(&self) -> EmptyState<()> {
        let title = self.data.name().map_or_else(|| translate_active("quvyta.image.untitled", &[]), str::to_owned);
        let (width, height) = self.data.original_size();
        let size = [("width", width.into()), ("height", height.into())];
        let message = match self.data.kind() {
            Some(kind) => {
                let [width, height] = size;
                translate_active("quvyta.image.cannot-show-kind", &[("kind", kind.label().into()), width, height])
            }
            None => translate_active("quvyta.image.cannot-show", &size),
        };
        EmptyState::new(title).icon("file-image").message(message)
    }
}

/// Whether `env`'s terminal can show pixels as half blocks: 256 colours or more, and block
/// elements to draw with.
fn can_draw(env: &Env) -> bool {
    env.depth() != ColorDepth::Ansi16 && env.glyph_mode() != GlyphMode::Ascii
}

/// The cells last worked out, and what they were worked out for.
#[derive(Default)]
struct Cells {
    /// The picture, the area's width and height, and the fit.
    key: Option<(u64, u16, u16, Fit)>,
    cells: Vec<Half>,
}

#[cfg(test)]
thread_local! {
    /// How many times this thread worked cells out, for the tests that prove it happens rarely.
    static RESAMPLES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn resamples() -> u32 {
    RESAMPLES.with(std::cell::Cell::get)
}

impl<Msg: 'static> Widget<Msg> for Image {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if !can_draw(cx.env()) {
            return Widget::<()>::measure(&self.cannot_show(), cx, available);
        }
        let (width, height) = resample::measure(&self.data, (available.width, available.height), self.fit);
        Size::new(width, height)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        if !can_draw(cx.env()) {
            Widget::<()>::paint(&self.cannot_show(), cx, area);
            return;
        }
        let key = (self.data.id(), area.width, area.height, self.fit);
        let memory = cx.memory::<Cells>();
        if memory.key != Some(key) {
            memory.cells = resample::cells(&self.data, area.width, area.height, self.fit);
            memory.key = Some(key);
            #[cfg(test)]
            RESAMPLES.with(|count| count.set(count.get() + 1));
        }
        // Taken out while painting, since painting needs the context the memory lives in.
        let cells = std::mem::take(&mut memory.cells);
        cx.decoration(area);
        let columns = usize::from(area.width);
        cx.each_cell_within(area, |column, row, cell| {
            let Some(half) = cells.get(usize::from(row) * columns + usize::from(column)) else { return };
            let (symbol, fg, bg) = match *half {
                Half::Empty => return,
                Half::Top(top) => ("▀", top, None),
                Half::Bottom(bottom) => ("▄", bottom, None),
                Half::Both(top, bottom) => ("▀", top, Some(bottom)),
            };
            cell.set_symbol(symbol);
            cell.fg = to_color(fg);
            if let Some(bg) = bg {
                cell.bg = to_color(bg);
            }
            cell.modifier = ratatui_core::style::Modifier::empty();
        });
        cx.memory::<Cells>().cells = cells;
    }
}
