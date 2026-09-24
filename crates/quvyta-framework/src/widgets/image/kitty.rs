//! Pictures the terminal draws itself, with the kitty graphics protocol: what an [`Image`] leaves
//! in the frame while it paints, and how the finished frame decides what of each picture shows.
//!
//! A picture is not painted into cells. Its cells get the canvas ground and a mark (a space in an
//! unusual, invisible text colour), and the frame records a [`Picture`]. Blends painted over the
//! screen afterwards, a dialog's backdrop or a window's shadow, are recorded too, as [`Dim`]s.
//! Once everything is painted, [`resolve`] looks at those cells again. A cell that still holds
//! the mark is one nothing was painted over; one whose colours are exactly what the recorded
//! blends over it make of the mark and the ground lies under a backdrop or a shadow; anything
//! else was painted over. The cells nothing was painted over are split into rectangles and the
//! terminal places the picture once in each, its source cut to match, so a window, an icon or a
//! menu over a picture is never hidden under it. Dimmed cells are drawn with half blocks in the
//! same frame, dimmed as the blend dims them. A picture cut into too many pieces is drawn with
//! half blocks for that frame, as a terminal without pictures shows it.

use ratatui_core::buffer::Buffer;
use ratatui_core::style::{Color, Modifier};

use super::resample::{self, Half};
use super::{Fit, Image, ImageData, paint_half};
use crate::color::Rgb;
use crate::geometry::Rect;
use crate::style::to_color;
use crate::widget::PaintCx;

/// What an [`Image`] asked the terminal to show in a frame, before anything was painted over it.
#[derive(Debug, Clone)]
pub(crate) struct Picture {
    data: ImageData,
    fit: Fit,
    /// The image's whole area, for drawing it with half blocks instead.
    area: Rect,
    /// The cells the whole picture covers.
    cells: Rect,
    /// The part of the picture shown in `cells`, in the picture's pixels.
    source: (f64, f64, f64, f64),
    /// The part of `cells` inside the area the image was allowed to paint.
    visible: Rect,
    /// The text colour of this picture's marked cells.
    marker: Rgb,
    /// The ground of the marked cells.
    ground: Rgb,
}

impl Picture {
    /// The identity of the picture shown.
    pub(crate) fn image(&self) -> u64 {
        self.data.id()
    }
}

/// Where the terminal shows a picture in a frame: the cells, and the part of the picture's pixels
/// stretched over them.
#[derive(Debug, Clone)]
pub(crate) struct PicturePlacement {
    pub(crate) data: ImageData,
    /// Tells apart the places one picture is shown in the same frame, counted from 1.
    pub(crate) number: u32,
    /// Column, row, columns and rows on screen.
    pub(crate) cells: (u16, u16, u16, u16),
    /// Left, top, width and height in the picture's pixels.
    pub(crate) crop: (u32, u32, u32, u32),
}

impl PartialEq for PicturePlacement {
    fn eq(&self, other: &Self) -> bool {
        self.data.id() == other.data.id()
            && self.number == other.number
            && self.cells == other.cells
            && self.crop == other.crop
    }
}

impl Eq for PicturePlacement {}

/// The text colour that marks a picture's cells on `ground`. A space shows no text colour, so
/// the mark is invisible. Each channel is far from the ground's and a little off the extremes, so
/// it is not a text colour a theme would paint a space with.
fn marker(ground: Rgb) -> Rgb {
    let far = |channel: u8, offset: u8| if channel < 128 { 255 - offset } else { offset };
    Rgb::new(far(ground.r, 3), far(ground.g, 7), far(ground.b, 4))
}

/// A blend of the colours already painted in a rectangle toward one colour, as
/// [`PaintCx::tint`] paints it for a dialog's backdrop or a window's shadow, recorded so a
/// picture beneath knows it was dimmed and by how much, instead of guessing it from colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Dim {
    /// The cells blended, already cut to what the painter could reach.
    rect: Rect,
    color: Rgb,
    amount: f32,
    /// How many pictures were recorded before it: only those lie beneath it.
    over: usize,
}

impl Dim {
    /// A blend of `rect` toward `color` by `amount`, painted after the first `over` pictures.
    pub(crate) fn new(rect: Rect, color: Rgb, amount: f32, over: usize) -> Self {
        Self { rect, color, amount, over }
    }
}

/// The blends painted over the cell at `x`, `y` after the picture numbered `index`, in the order
/// they were painted.
fn dims_over(dims: &[Dim], index: usize, x: i32, y: i32) -> impl Iterator<Item = &Dim> + Clone {
    dims.iter().filter(move |dim| dim.over > index && dim.rect.contains(x, y))
}

/// `colour` after every blend in `dims`, blended exactly as [`PaintCx::tint`] blends it.
fn dimmed<'a>(colour: Rgb, dims: impl Iterator<Item = &'a Dim>) -> Rgb {
    dims.fold(colour, |colour, dim| colour.mix(dim.color, dim.amount))
}

/// What became of one of a picture's cells by the end of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fate {
    /// Nothing was painted over it.
    Free,
    /// Only recorded blends were painted over it.
    Dimmed,
    /// Something was painted over it.
    Covered,
}

/// What became of `cell`, at `x`, `y`, which the picture numbered `index` marked. A cell is
/// dimmed only when blends were recorded over it and they turn the mark and the ground into
/// exactly the cell's colours; any other change covers it, however much its colours look like a
/// blend.
fn fate(cell: &ratatui_core::buffer::Cell, picture: &Picture, dims: &[Dim], index: usize, x: i32, y: i32) -> Fate {
    if cell.symbol() != " " || !cell.modifier.is_empty() {
        return Fate::Covered;
    }
    let (Color::Rgb(fr, fg, fb), Color::Rgb(br, bg, bb)) = (cell.fg, cell.bg) else {
        return Fate::Covered;
    };
    let (fg, bg) = (Rgb::new(fr, fg, fb), Rgb::new(br, bg, bb));
    if fg == picture.marker && bg == picture.ground {
        return Fate::Free;
    }
    let over = dims_over(dims, index, x, y);
    if over.clone().next().is_some() && dimmed(picture.marker, over.clone()) == fg && dimmed(picture.ground, over) == bg
    {
        Fate::Dimmed
    } else {
        Fate::Covered
    }
}

impl Image {
    /// Paints the image as a picture the terminal draws: its cells marked, its place recorded.
    pub(super) fn paint_for_terminal(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let resample::OnTerminal { cells: (x, y, width, height), source } =
            resample::terminal_cells(&self.data, area.width, area.height, self.fit);
        let cells = Rect::new(area.x + i32::from(x), area.y + i32::from(y), width, height);
        let visible = cells.intersect(cx.clip);
        let ground = cx.color("canvas");
        let marker = marker(ground);
        let (fg, bg) = (to_color(marker), to_color(ground));
        cx.decoration(area);
        cx.each_cell_within(cells, |_, _, cell| {
            cell.set_symbol(" ");
            cell.fg = fg;
            cell.bg = bg;
            cell.modifier = Modifier::empty();
        });
        if visible.is_empty() {
            return;
        }
        cx.frame.pictures.push(Picture {
            data: self.data.clone(),
            fit: self.fit,
            area,
            cells,
            source,
            visible,
            marker,
            ground,
        });
    }
}

/// Half-block cells worked out for pictures that had something painted over their middle, kept
/// while the pictures stay covered, so a dialog open over a wallpaper costs the work once.
#[derive(Debug, Default)]
pub(crate) struct Halves {
    entries: Vec<Worked>,
}

/// Half-block cells of one picture.
#[derive(Debug)]
struct Worked {
    /// The picture, the area's width and height, and the fit.
    key: (u64, u16, u16, Fit),
    cells: Vec<Half>,
    /// Whether the frame being resolved used them.
    used: bool,
}

impl Halves {
    fn cells(&mut self, picture: &Picture) -> &[Half] {
        let key = (picture.data.id(), picture.area.width, picture.area.height, picture.fit);
        let index = match self.entries.iter().position(|entry| entry.key == key) {
            Some(index) => index,
            None => {
                let cells = resample::cells(&picture.data, key.1, key.2, key.3);
                self.entries.push(Worked { key, cells, used: false });
                self.entries.len() - 1
            }
        };
        let entry = &mut self.entries[index];
        entry.used = true;
        &entry.cells
    }

    /// Drops the cells the frame just finished did not use.
    fn settle(&mut self) {
        self.entries.retain(|entry| entry.used);
        for entry in &mut self.entries {
            entry.used = false;
        }
    }
}

/// The most places one picture is split into in a frame. A picture whose free cells take more
/// rectangles than this is drawn with half blocks for that frame instead.
pub(crate) const MOST_PLACES: usize = 64;

/// How the terminal can show part of a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placing {
    /// The terminal keeps the picture apart from the text and can show it in pieces, each cut to
    /// its own rectangle, under text: kitty.
    Split,
    /// The picture is painted into the cells and whatever is written over it later wipes it, so
    /// it is shown only where all of it that may show does: sixel.
    Whole,
}

/// Decides, once a frame is painted, where the terminal shows each of its `pictures`, given the
/// blends `dims` painted over them.
///
/// With [`Placing::Whole`] a picture is shown only when every cell it may paint still holds its
/// mark and no later picture lies over any of them; a picture cut by its clip is shown on the
/// part within the clip. Anything else, a dialog's backdrop included, draws it with half blocks
/// for that frame. What follows describes [`Placing::Split`].
///
/// A picture takes the cells that still hold its mark and nothing else; a picture painted later
/// over an earlier one takes its cells. Those cells are split into rectangles, row runs merged
/// down while they line up, and the terminal shows the part of the picture that falls in each,
/// every one its own place. Cells a recorded blend dimmed are drawn with half blocks in the same
/// frame, blended as the blend blended them, while the rest stays pixels. When the free cells
/// need more than [`MOST_PLACES`] rectangles, the whole picture is drawn with half blocks for
/// that frame. `halves` is whether the terminal can show half blocks; where it cannot, those
/// cells keep the ground.
pub(crate) fn resolve(
    buf: &mut Buffer,
    pictures: &[Picture],
    dims: &[Dim],
    cache: &mut Halves,
    halves: bool,
    placing: Placing,
) -> Vec<PicturePlacement> {
    let mut placements: Vec<PicturePlacement> = Vec::new();
    let screen = Rect::new(i32::from(buf.area.x), i32::from(buf.area.y), buf.area.width, buf.area.height);
    for (index, picture) in pictures.iter().enumerate() {
        let above = &pictures[index + 1..];
        let visible = picture.visible.intersect(screen);
        let mut free = Vec::new();
        let mut blended = Vec::new();
        for y in visible.y..visible.bottom() {
            for x in visible.x..visible.right() {
                if above.iter().any(|later| later.visible.contains(x, y)) {
                    continue;
                }
                let Some(cell) = buf.cell(cell_at(x, y)) else { continue };
                match fate(cell, picture, dims, index, x, y) {
                    Fate::Free => free.push((x, y)),
                    Fate::Dimmed => blended.push((x, y)),
                    Fate::Covered => {}
                }
            }
        }
        // Cells under a later picture, dimmed or covered are not free, so every cell is free only
        // when nothing at all lies over the picture.
        let whole = !free.is_empty() && free.len() == usize::from(visible.width) * usize::from(visible.height);
        let rects = match placing {
            Placing::Split => rectangles(&free, MOST_PLACES),
            Placing::Whole if whole => Some(vec![visible]),
            Placing::Whole => None,
        };
        let halved = match rects {
            Some(rects) => {
                let before = placements.iter().filter(|placed| placed.data.id() == picture.data.id()).count();
                for (offset, rect) in rects.into_iter().enumerate() {
                    placements.push(PicturePlacement {
                        data: picture.data.clone(),
                        number: u32::try_from(before + offset + 1).unwrap_or(u32::MAX),
                        cells: (
                            u16::try_from(rect.x).unwrap_or(0),
                            u16::try_from(rect.y).unwrap_or(0),
                            rect.width,
                            rect.height,
                        ),
                        crop: crop(picture, rect),
                    });
                }
                blended
            }
            None => {
                blended.extend(free);
                blended
            }
        };
        if halves && !halved.is_empty() {
            let cells = cache.cells(picture);
            let columns = usize::from(picture.area.width);
            for (x, y) in halved {
                let (Ok(column), Ok(row)) = (usize::try_from(x - picture.area.x), usize::try_from(y - picture.area.y))
                else {
                    continue;
                };
                let (Some(half), Some(cell)) = (cells.get(row * columns + column), buf.cell_mut(cell_at(x, y))) else {
                    continue;
                };
                let over = dims_over(dims, index, x, y);
                let half = match *half {
                    Half::Empty => Half::Empty,
                    Half::Top(top) => Half::Top(dimmed(top, over)),
                    Half::Bottom(bottom) => Half::Bottom(dimmed(bottom, over)),
                    Half::Both(top, bottom) => Half::Both(dimmed(top, over.clone()), dimmed(bottom, over)),
                };
                paint_half(cell, half);
            }
        }
    }
    cache.settle();
    placements
}

/// `cells`, listed row by row and left to right within a row, as rectangles that cover exactly
/// them: each row's runs of neighbouring cells, a run merged into the rectangle above it when
/// that rectangle ended on the row before with the same left edge and width. `None` when that
/// takes more than `most` rectangles.
fn rectangles(cells: &[(i32, i32)], most: usize) -> Option<Vec<Rect>> {
    let mut done: Vec<Rect> = Vec::new();
    // Rectangles that reach down to the row before the one being read.
    let mut open: Vec<Rect> = Vec::new();
    let mut row: Vec<Rect> = Vec::new();
    let mut index = 0;
    while index < cells.len() {
        let (_, y) = cells[index];
        row.clear();
        while index < cells.len() && cells[index].1 == y {
            let (x, _) = cells[index];
            match row.last_mut() {
                Some(run) if run.right() == x => run.width = run.width.saturating_add(1),
                _ => row.push(Rect::new(x, y, 1, 1)),
            }
            index += 1;
        }
        let mut next = Vec::with_capacity(row.len());
        for run in &row {
            let above = open.iter().position(|rect| rect.x == run.x && rect.width == run.width && rect.bottom() == y);
            next.push(match above {
                Some(at) => {
                    let mut rect = open.swap_remove(at);
                    rect.height = rect.height.saturating_add(1);
                    rect
                }
                None => *run,
            });
        }
        done.append(&mut open);
        open = next;
        if done.len() + open.len() > most {
            return None;
        }
    }
    done.append(&mut open);
    done.sort_by_key(|rect| (rect.y, rect.x));
    Some(done)
}

/// A screen cell's position in the buffer; cells off screen never hold a mark, so they are
/// never asked for.
fn cell_at(x: i32, y: i32) -> (u16, u16) {
    (u16::try_from(x).unwrap_or(u16::MAX), u16::try_from(y).unwrap_or(u16::MAX))
}

/// The part of `picture`'s pixels that lands in `shown`, a part of its cells: the source is
/// stretched evenly over the cells, so a cut at a cell is a cut at the same share of the source.
///
/// Each edge of the crop is worked out from the cell edge alone, the same way whichever side of
/// it a rectangle lies on, so two rectangles that meet at a cell edge meet at the same pixel:
/// the parts of one picture placed side by side neither miss nor repeat a pixel.
fn crop(picture: &Picture, shown: Rect) -> (u32, u32, u32, u32) {
    let (sx, sy, sw, sh) = picture.source;
    let cells = picture.cells;
    let along = |from: i32, to: i32, start: f64, length: f64, cells_start: i32, cells: u16, limit: u32| {
        let per_cell = length / f64::from(cells.max(1));
        let edge = |cell: i32| pixels(start + f64::from(cell - cells_start) * per_cell);
        let first = edge(from).min(limit.saturating_sub(1));
        // At least one pixel, even where a cell is narrower than a pixel of the source.
        let last = edge(to).clamp(first + 1, limit.max(first + 1));
        (first, last - first)
    };
    let (x, width) = along(shown.x, shown.right(), sx, sw, cells.x, cells.width, picture.data.width());
    let (y, height) = along(shown.y, shown.bottom(), sy, sh, cells.y, cells.height, picture.data.height());
    (x, y, width, height)
}

/// A position in pixels, rounded to the nearest whole one.
fn pixels(length: f64) -> u32 {
    // Positions here are within a decoded picture, far inside u32; `as` saturates in any case.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixels = length.round().max(0.0) as u32;
    pixels
}

#[cfg(test)]
#[path = "kitty_tests.rs"]
mod tests;
