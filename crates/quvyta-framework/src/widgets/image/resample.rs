//! Working a picture out into cells: where it lands for a fit, and the colour of every half cell.
//!
//! An area of `w` × `h` cells is `w` × `2h` half cells, but a half cell is not a square pixel: it
//! is as wide as a cell and half as tall, so a picture drawn at exactly the pixels of its area
//! fills it the way the terminal really shows it. Where the terminal reports no cell size, and
//! for [`Fit::Center`], which shows a picture at its own size, a half cell is one square pixel.

use super::{Fit, ImageData};
use crate::color::Rgb;

/// What a set of worked-out cells was worked out for: the picture, the area's width and height,
/// the fit and the size of a cell in the terminal's pixels, `None` where it reports none. A cell
/// of another size works them out again, since a half block is as wide as a cell and half as tall.
pub(super) type CellKey = (u64, u16, u16, Fit, Option<(u16, u16)>);

/// What one cell shows of the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Half {
    /// Nothing: the picture does not reach this cell.
    Empty,
    /// Only the upper pixel; the lower half keeps what is under the picture.
    Top(Rgb),
    /// Only the lower pixel; the upper half keeps what is under the picture.
    Bottom(Rgb),
    /// Both pixels, upper then lower.
    Both(Rgb, Rgb),
}

/// Where a picture lands in an area of half cells: the part of the picture shown, in the picture's
/// pixels (a cover crop can start between two of them), and the half cells of the area it covers.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    source: (f64, f64, f64, f64),
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// What one half cell is in the terminal's own pixels: a cell's width and half its height where
/// the terminal reports its cells, and one square pixel where it reports none, which is all a half
/// block was taken to be then.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HalfCell {
    width: f64,
    height: f64,
}

impl HalfCell {
    /// The half cell of a terminal whose cell is `cell` pixels wide and high, `None` and a cell of
    /// no size where the terminal reports none.
    fn of(cell: Option<(u16, u16)>) -> Self {
        match cell {
            Some((width, height)) if width > 0 && height > 0 => {
                Self { width: f64::from(width), height: f64::from(height) / 2.0 }
            }
            _ => Self::square(),
        }
    }

    /// One square pixel: the half cell of a terminal that says nothing about its cells.
    fn square() -> Self {
        Self { width: 1.0, height: 1.0 }
    }

    /// A picture of `size` pixels as half cells, width and height.
    fn picture(self, size: (u32, u32)) -> (f64, f64) {
        (f64::from(size.0) / self.width, f64::from(size.1) / self.height)
    }

    /// A part of the picture laid out in half cells, in the picture's own pixels.
    fn pixels(self, part: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
        (part.0 * self.width, part.1 * self.height, part.2 * self.width, part.3 * self.height)
    }
}

/// Where a picture of `image` pixels lands in `area` half cells, both non-zero, for `fit`; a half
/// cell is `cell` pixels, so a picture keeps the shape the terminal really shows it in.
fn place(image: (u32, u32), area: (u32, u32), fit: Fit, cell: HalfCell) -> Placement {
    // A picture at its own size is one pixel per half cell, whatever shape a cell really has.
    let cell = match fit {
        Fit::Center => HalfCell::square(),
        _ => cell,
    };
    let (iw, ih) = cell.picture(image);
    let (aw, ah) = (f64::from(area.0), f64::from(area.1));
    let (source, x, y, width, height) = match fit {
        Fit::Contain => {
            let scale = (aw / iw).min(ah / ih);
            let width = to_cells(iw * scale).clamp(1, area.0);
            let height = to_cells(ih * scale).clamp(1, area.1);
            ((0.0, 0.0, iw, ih), (area.0 - width) / 2, (area.1 - height) / 2, width, height)
        }
        Fit::Cover => {
            let scale = (aw / iw).max(ah / ih);
            let (shown_w, shown_h) = ((aw / scale).min(iw), (ah / scale).min(ih));
            (((iw - shown_w) / 2.0, (ih - shown_h) / 2.0, shown_w, shown_h), 0, 0, area.0, area.1)
        }
        Fit::Center => {
            let (width, height) = (image.0.min(area.0), image.1.min(area.1));
            let left = f64::from((image.0 - width) / 2);
            let top = f64::from((image.1 - height) / 2);
            (
                (left, top, f64::from(width), f64::from(height)),
                (area.0 - width) / 2,
                (area.1 - height) / 2,
                width,
                height,
            )
        }
    };
    Placement { source: cell.pixels(source), x, y, width, height }
}

/// A length in half cells, rounded to the nearest whole one.
fn to_cells(length: f64) -> u32 {
    // Lengths here are at most a screen's cells, far inside u32; `as` saturates in any case.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixels = length.round().max(0.0) as u32;
    pixels
}

/// The cells, `width` columns by `height` rows, that `fit` needs for `data` given room of
/// `width` × `height` cells; `cell` is the size of a cell in pixels, `None` where the terminal
/// reports none.
pub(super) fn measure(data: &ImageData, room: (u16, u16), fit: Fit, cell: Option<(u16, u16)>) -> (u16, u16) {
    if room.0 == 0 || room.1 == 0 {
        return (0, 0);
    }
    match fit {
        Fit::Cover => room,
        Fit::Contain | Fit::Center => {
            let area = (u32::from(room.0), u32::from(room.1) * 2);
            let placed = place((data.width(), data.height()), area, fit, HalfCell::of(cell));
            let width = u16::try_from(placed.width).unwrap_or(room.0);
            let height = u16::try_from(placed.height.div_ceil(2)).unwrap_or(room.1);
            (width.min(room.0), height.min(room.1))
        }
    }
}

/// Where the terminal draws `data` itself in an area of `columns` × `rows` cells, both non-zero,
/// for `fit`: the cells the picture covers (column, row, width and height, from the area's top
/// left) and the part of the picture shown there, in the picture's pixels.
///
/// The terminal stretches the picture over the cells it is given, so the cells are counted one
/// wide and two half cells tall whatever the terminal's own cells look like: the picture's shape
/// on screen is then the terminal's own business. A picture that would end half way down a cell
/// covers that cell, stretched by less than half a row, since the terminal places a picture on
/// whole cells only.
pub(super) fn terminal_cells(data: &ImageData, columns: u16, rows: u16, fit: Fit) -> OnTerminal {
    let area = (u32::from(columns), u32::from(rows) * 2);
    let placed = place((data.width(), data.height()), area, fit, HalfCell::square());
    let top = placed.y / 2;
    let bottom = (placed.y + placed.height).div_ceil(2);
    let cells = (
        u16::try_from(placed.x).unwrap_or(0),
        u16::try_from(top).unwrap_or(0),
        u16::try_from(placed.width).unwrap_or(columns).min(columns),
        u16::try_from(bottom - top).unwrap_or(rows).min(rows),
    );
    OnTerminal { cells, source: placed.source }
}

/// Where the terminal draws a picture in an area; see [`terminal_cells`].
pub(super) struct OnTerminal {
    /// Column, row, columns and rows, from the area's top left.
    pub(super) cells: (u16, u16, u16, u16),
    /// Left, top, width and height in the picture's pixels.
    pub(super) source: (f64, f64, f64, f64),
}

/// The cells of an area of `columns` × `rows` cells showing `data` with `fit`, row after row;
/// `cell` is the size of a cell in pixels, `None` where the terminal reports none, and then a
/// half cell is one square pixel.
pub(super) fn cells(data: &ImageData, columns: u16, rows: u16, fit: Fit, cell: Option<(u16, u16)>) -> Vec<Half> {
    let area = (u32::from(columns), u32::from(rows) * 2);
    if area.0 == 0 || area.1 == 0 {
        return Vec::new();
    }
    let placed = place((data.width(), data.height()), area, fit, HalfCell::of(cell));
    let pixels = resample(data, placed.source, placed.width, placed.height);
    let at = |px: u32, py: u32| -> Option<Rgb> {
        let (x, y) = (px.checked_sub(placed.x)?, py.checked_sub(placed.y)?);
        if x >= placed.width || y >= placed.height {
            return None;
        }
        pixels.get(usize::try_from(y * placed.width + x).ok()?).copied()
    };
    let mut out = Vec::with_capacity(usize::from(columns) * usize::from(rows));
    for row in 0..u32::from(rows) {
        for column in 0..u32::from(columns) {
            out.push(match (at(column, row * 2), at(column, row * 2 + 1)) {
                (Some(top), Some(bottom)) => Half::Both(top, bottom),
                (Some(top), None) => Half::Top(top),
                (None, Some(bottom)) => Half::Bottom(bottom),
                (None, None) => Half::Empty,
            });
        }
    }
    out
}

/// The pixels of `crop` (left, top, width, height in the picture's pixels) of `data`, shrunk or
/// stretched to `width` × `height` with the same box filter as half blocks, row after row.
pub(crate) fn crop_pixels(data: &ImageData, crop: (u32, u32, u32, u32), width: u32, height: u32) -> Vec<Rgb> {
    let source = (f64::from(crop.0), f64::from(crop.1), f64::from(crop.2), f64::from(crop.3));
    resample(data, source, width, height)
}

/// The picture's pixels in `source` (x, y, width, height in its own pixels), resampled to
/// `width` × `height` pixels with a box filter: every new pixel is the average of the pixels it
/// covers, each counted by how much of it is covered. A shrunk photo then keeps its tones instead
/// of flickering between the pixels a point sample would happen to land on.
fn resample(data: &ImageData, source: (f64, f64, f64, f64), width: u32, height: u32) -> Vec<Rgb> {
    let columns = spans(source.0, source.2, width, data.width());
    let rows = spans(source.1, source.3, height, data.height());
    let pixels = data.pixels();
    let stride = usize::try_from(data.width()).unwrap_or(usize::MAX);
    let mut out = Vec::with_capacity(columns.len() * rows.len());
    for row in &rows {
        for column in &columns {
            let (mut r, mut g, mut b, mut total) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            for &(y, wy) in row {
                let line = &pixels[y * stride..(y + 1) * stride];
                for &(x, wx) in column {
                    let weight = wy * wx;
                    let pixel = line[x];
                    r += weight * f64::from(pixel.r);
                    g += weight * f64::from(pixel.g);
                    b += weight * f64::from(pixel.b);
                    total += weight;
                }
            }
            out.push(if total > 0.0 {
                Rgb::new(channel(r / total), channel(g / total), channel(b / total))
            } else {
                Rgb::new(0, 0, 0)
            });
        }
    }
    out
}

/// A channel value, rounded and kept within 0–255.
fn channel(value: f64) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let channel = value.round().clamp(0.0, 255.0) as u8;
    channel
}

/// For each of `count` new pixels along one side, the source pixels it covers and how much of each:
/// the new pixels split `length` source pixels from `start` evenly. `limit` is the source's side.
fn spans(start: f64, length: f64, count: u32, limit: u32) -> Vec<Vec<(usize, f64)>> {
    let step = length / f64::from(count.max(1));
    let last = limit.saturating_sub(1);
    (0..count)
        .map(|index| {
            let from = start + step * f64::from(index);
            let to = from + step;
            let first = to_index(from.floor()).min(last);
            let end = to_index(to.ceil()).clamp(first + 1, limit);
            let mut span: Vec<(usize, f64)> = (first..end)
                .filter_map(|pixel| {
                    let covered = to.min(f64::from(pixel) + 1.0) - from.max(f64::from(pixel));
                    // Rounding leaves slivers of a neighbour that are not really covered.
                    (covered > 1e-9).then(|| (usize::try_from(pixel).unwrap_or(usize::MAX), covered))
                })
                .collect();
            if span.is_empty() {
                span.push((usize::try_from(first).unwrap_or(0), 1.0));
            }
            span
        })
        .collect()
}

/// A pixel index from a whole, non-negative number.
fn to_index(value: f64) -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = value.max(0.0) as u32;
    index
}
