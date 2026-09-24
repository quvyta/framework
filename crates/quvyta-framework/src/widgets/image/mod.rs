//! Pictures in the terminal: decoded pixels drawn by a terminal that speaks the kitty graphics
//! protocol or sixel, and with half blocks in any terminal with 256 colours or more, over SSH too.

mod data;
mod kitty;
mod resample;
#[cfg(test)]
mod tests;

pub use data::{ImageData, ImageError};
pub(crate) use kitty::{Dim, Halves, Picture, PicturePlacement, Placing, resolve};
use resample::Half;
pub(crate) use resample::crop_pixels;

use super::EmptyState;
use crate::geometry::{Rect, Size};
use crate::graphics::Graphics;
use crate::i18n::translate_active;
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

/// A picture, drawn by the terminal itself where it can and with half blocks elsewhere.
///
/// Where [`Env::graphics`](crate::env::Env::graphics) is [`Graphics::Kitty`], the terminal draws
/// the picture in real pixels: its cells get the theme's `canvas` ground and the terminal places
/// the picture over them, under text. The pixels are sent once, at the size [`ImageData`] keeps,
/// and every later frame only places them; a frame whose pictures did not move writes nothing for
/// them. What is painted over the picture hides it: along one side, the picture is cut to the part
/// left showing; in the middle or a corner, or under a dialog's dimmed backdrop, that frame draws
/// it with half blocks.
///
/// Where it is [`Graphics::Sixel`], the terminal paints the picture into the cells, shrunk to the
/// pixels they cover (a cell is taken to be 10 × 20 pixels where the terminal does not report its
/// size) and reduced to 252 colours. Text written over a sixel wipes it, so the picture is drawn
/// this way only while nothing at all is painted over it: a menu, a dialog's backdrop or anything
/// else over any of it draws it with half blocks for those frames, and it is painted again once
/// uncovered. A frame that changes nothing under it writes nothing for it.
///
/// Everywhere else every cell shows two pixels, one above the other: `▀` in the upper pixel's
/// colour on the lower pixel's. A cell is about twice as tall as it is wide, so these pixels are close to square and
/// a picture keeps its shape. Shrinking averages every pixel it covers (a box filter), so a photo
/// does not flicker into noise. The cells are worked out once and kept in the widget's memory;
/// they are worked out again only when the area's size, the fit or the picture changes, so a
/// frame that repaints the same picture only copies them. Cells the picture does not reach keep
/// what is under it, so a picture can be the ground other widgets are drawn on.
///
/// At [`ColorDepth::Ansi256`](crate::color::ColorDepth::Ansi256) every half takes its nearest palette entry. Where
/// [`Graphics::can_draw`] is false (the sixteen standard colours, ASCII glyph mode, or
/// `QUVYTA_GRAPHICS=none`), pixels are not shown and are never drawn with characters: the widget
/// says instead what the picture is (the file's name, its format and size) and that this
/// terminal cannot show pictures. An application that would rather show nothing asks
/// [`Graphics::can_draw`] first.
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
        if !cx.env().graphics().can_draw() {
            return Widget::<()>::measure(&self.cannot_show(), cx, available);
        }
        let (width, height) = resample::measure(&self.data, (available.width, available.height), self.fit);
        Size::new(width, height)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        match cx.env().graphics() {
            Graphics::Kitty | Graphics::Sixel => {
                self.paint_for_terminal(cx, area);
                return;
            }
            Graphics::None => {
                Widget::<()>::paint(&self.cannot_show(), cx, area);
                return;
            }
            Graphics::HalfBlock => {}
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
            if let Some(half) = cells.get(usize::from(row) * columns + usize::from(column)) {
                paint_half(cell, *half);
            }
        });
        cx.memory::<Cells>().cells = cells;
    }
}

/// Paints what one cell shows of a picture with half blocks; an empty half keeps what is under it.
fn paint_half(cell: &mut ratatui_core::buffer::Cell, half: Half) {
    let (symbol, fg, bg) = match half {
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
}
