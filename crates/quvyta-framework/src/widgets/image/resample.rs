//! Working a picture out into cells: where it lands for a fit, and the colour of every half cell.
//!
//! An area of `w` × `h` cells is `w` × `2h` pixels: a cell is about twice as tall as it is wide,
//! and a half block splits it into two pixels, one above the other, which are then close to square.

use super::{Fit, ImageData};
use crate::color::Rgb;

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

/// Where a picture lands in an area of pixels: the part of the picture shown, in the picture's
/// pixels (a cover crop can start between two of them), and the pixels of the area it covers.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    source: (f64, f64, f64, f64),
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Where a picture of `image` pixels lands in `area` pixels, both non-zero, for `fit`.
fn place(image: (u32, u32), area: (u32, u32), fit: Fit) -> Placement {
    let (iw, ih) = (f64::from(image.0), f64::from(image.1));
    let (aw, ah) = (f64::from(area.0), f64::from(area.1));
    match fit {
        Fit::Contain => {
            let scale = (aw / iw).min(ah / ih);
            let width = to_pixels(iw * scale).clamp(1, area.0);
            let height = to_pixels(ih * scale).clamp(1, area.1);
            Placement { source: (0.0, 0.0, iw, ih), x: (area.0 - width) / 2, y: (area.1 - height) / 2, width, height }
        }
        Fit::Cover => {
            let scale = (aw / iw).max(ah / ih);
            let (shown_w, shown_h) = ((aw / scale).min(iw), (ah / scale).min(ih));
            Placement {
                source: ((iw - shown_w) / 2.0, (ih - shown_h) / 2.0, shown_w, shown_h),
                x: 0,
                y: 0,
                width: area.0,
                height: area.1,
            }
        }
        Fit::Center => {
            let (width, height) = (image.0.min(area.0), image.1.min(area.1));
            let left = f64::from((image.0 - width) / 2);
            let top = f64::from((image.1 - height) / 2);
            Placement {
                source: (left, top, f64::from(width), f64::from(height)),
                x: (area.0 - width) / 2,
                y: (area.1 - height) / 2,
                width,
                height,
            }
        }
    }
}

/// A length in pixels, rounded to the nearest whole one.
fn to_pixels(length: f64) -> u32 {
    // Lengths here are at most a screen's pixels, far inside u32; `as` saturates in any case.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixels = length.round().max(0.0) as u32;
    pixels
}

/// The cells, `width` columns by `height` rows, that `fit` needs for `data` given room of
/// `width` × `height` cells.
pub(super) fn measure(data: &ImageData, room: (u16, u16), fit: Fit) -> (u16, u16) {
    if room.0 == 0 || room.1 == 0 {
        return (0, 0);
    }
    match fit {
        Fit::Cover => room,
        Fit::Contain | Fit::Center => {
            let area = (u32::from(room.0), u32::from(room.1) * 2);
            let placed = place((data.width(), data.height()), area, fit);
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
/// The picture lands where half blocks would put it, widened to whole cells: a picture that
/// would end half way down a cell covers that cell, stretched by less than half a row, since the
/// terminal places a picture on whole cells only.
pub(super) fn terminal_cells(data: &ImageData, columns: u16, rows: u16, fit: Fit) -> OnTerminal {
    let area = (u32::from(columns), u32::from(rows) * 2);
    let placed = place((data.width(), data.height()), area, fit);
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

/// The cells of an area of `columns` × `rows` cells showing `data` with `fit`, row after row.
pub(super) fn cells(data: &ImageData, columns: u16, rows: u16, fit: Fit) -> Vec<Half> {
    let area = (u32::from(columns), u32::from(rows) * 2);
    if area.0 == 0 || area.1 == 0 {
        return Vec::new();
    }
    let placed = place((data.width(), data.height()), area, fit);
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
