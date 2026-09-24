//! Pictures the terminal draws itself, with the kitty graphics protocol: what an [`Image`] leaves
//! in the frame while it paints, and how the finished frame decides what of each picture shows.
//!
//! A picture is not painted into cells. Its cells get the canvas ground and a mark (a space in an
//! unusual, invisible text colour), and the frame records a [`Picture`]. Once everything is
//! painted, [`resolve`] looks at those cells again. A cell that still holds the mark is one
//! nothing was painted over; one whose two colours were blended toward another colour lies under
//! the dimmed backdrop of a dialog; anything else was painted over. When the cells nothing was
//! painted over form one rectangle, the terminal is asked to place the picture there with its
//! source cut to match, so a menu along one side of a picture is never hidden under it. When they
//! do not (something in the middle, or a corner), or the picture lies under a backdrop, the
//! picture is drawn with half blocks for that frame, as a terminal without pictures shows it,
//! dimmed as the backdrop dims it.

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
/// the mark is invisible. Each channel is far from the ground's, so a blend of the two toward a
/// backdrop colour can be undone (see [`Tint`]), and a little off the extremes, so it is not a
/// text colour a theme would paint a space with.
fn marker(ground: Rgb) -> Rgb {
    let far = |channel: u8, offset: u8| if channel < 128 { 255 - offset } else { offset };
    Rgb::new(far(ground.r, 3), far(ground.g, 7), far(ground.b, 4))
}

/// A blend of every colour toward one colour by the same amount, as a dialog's backdrop dims the
/// screen: `channel × keep + add`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Tint {
    keep: f32,
    add: [f32; 3],
}

impl Tint {
    /// The blend that turned `marker` into `fg` and `ground` into `bg`, if one blend explains
    /// both: two colours known before and after are enough to find it.
    fn between(marker: Rgb, ground: Rgb, fg: Rgb, bg: Rgb) -> Option<Self> {
        let pairs =
            [(marker.r, ground.r, fg.r, bg.r), (marker.g, ground.g, fg.g, bg.g), (marker.b, ground.b, fg.b, bg.b)];
        let keeps = pairs.map(|(m, g, f, b)| (f32::from(f) - f32::from(b)) / (f32::from(m) - f32::from(g)));
        let keep = keeps.iter().sum::<f32>() / 3.0;
        // Rounding moves each channel by at most one step in over a hundred.
        if !(-0.02..=1.02).contains(&keep) || keeps.iter().any(|each| (each - keep).abs() > 0.03) {
            return None;
        }
        let keep = keep.clamp(0.0, 1.0);
        let add = pairs.map(|(_, g, _, b)| f32::from(b) - keep * f32::from(g));
        Some(Self { keep, add })
    }

    fn apply(self, colour: Rgb) -> Rgb {
        let channel = |value: u8, add: f32| {
            // Within 0..=255 once clamped.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let channel = (f32::from(value) * self.keep + add).round().clamp(0.0, 255.0) as u8;
            channel
        };
        Rgb::new(channel(colour.r, self.add[0]), channel(colour.g, self.add[1]), channel(colour.b, self.add[2]))
    }
}

/// What became of one of a picture's cells by the end of the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Fate {
    /// Nothing was painted over it.
    Free,
    /// A backdrop dimmed it.
    Tinted(Tint),
    /// Something was painted over it.
    Covered,
}

/// What became of `cell`, which `picture` marked.
fn fate(cell: &ratatui_core::buffer::Cell, picture: &Picture) -> Fate {
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
    Tint::between(picture.marker, picture.ground, fg, bg).map_or(Fate::Covered, Fate::Tinted)
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

/// Decides, once a frame is painted, where the terminal shows each of its `pictures`.
///
/// A picture takes the cells that still hold its mark and nothing else; a picture painted later
/// over an earlier one takes its cells. When those cells form one rectangle, the terminal shows
/// the part of the picture that falls in it. When they do not, or a backdrop dimmed any of the
/// picture's cells, the picture is drawn with half blocks in its free and dimmed cells instead,
/// the dimmed ones blended as the backdrop blended them. `halves` is whether the terminal can
/// show half blocks; where it cannot, those cells keep the ground.
pub(crate) fn resolve(
    buf: &mut Buffer,
    pictures: &[Picture],
    cache: &mut Halves,
    halves: bool,
) -> Vec<PicturePlacement> {
    let mut placements: Vec<PicturePlacement> = Vec::new();
    let screen = Rect::new(i32::from(buf.area.x), i32::from(buf.area.y), buf.area.width, buf.area.height);
    for (index, picture) in pictures.iter().enumerate() {
        let above = &pictures[index + 1..];
        let visible = picture.visible.intersect(screen);
        let mut shown = Vec::new();
        let mut dimmed = false;
        for y in visible.y..visible.bottom() {
            for x in visible.x..visible.right() {
                if above.iter().any(|later| later.visible.contains(x, y)) {
                    continue;
                }
                let Some(cell) = buf.cell(cell_at(x, y)) else { continue };
                match fate(cell, picture) {
                    Fate::Free => shown.push((x, y, None)),
                    Fate::Tinted(tint) => {
                        dimmed = true;
                        shown.push((x, y, Some(tint)));
                    }
                    Fate::Covered => {}
                }
            }
        }
        let Some(bounds) = bounds(shown.iter().map(|&(x, y, _)| (x, y))) else { continue };
        if !dimmed && shown.len() == usize::from(bounds.width) * usize::from(bounds.height) {
            let number = 1 + placements.iter().filter(|placed| placed.data.id() == picture.data.id()).count();
            placements.push(PicturePlacement {
                data: picture.data.clone(),
                number: u32::try_from(number).unwrap_or(u32::MAX),
                cells: (
                    u16::try_from(bounds.x).unwrap_or(0),
                    u16::try_from(bounds.y).unwrap_or(0),
                    bounds.width,
                    bounds.height,
                ),
                crop: crop(picture, bounds),
            });
        } else if halves {
            let cells = cache.cells(picture);
            let columns = usize::from(picture.area.width);
            for (x, y, tint) in shown {
                let (Ok(column), Ok(row)) = (usize::try_from(x - picture.area.x), usize::try_from(y - picture.area.y))
                else {
                    continue;
                };
                let (Some(half), Some(cell)) = (cells.get(row * columns + column), buf.cell_mut(cell_at(x, y))) else {
                    continue;
                };
                let half = match (*half, tint) {
                    (half, None) => half,
                    (Half::Empty, Some(_)) => Half::Empty,
                    (Half::Top(top), Some(tint)) => Half::Top(tint.apply(top)),
                    (Half::Bottom(bottom), Some(tint)) => Half::Bottom(tint.apply(bottom)),
                    (Half::Both(top, bottom), Some(tint)) => Half::Both(tint.apply(top), tint.apply(bottom)),
                };
                paint_half(cell, half);
            }
        }
    }
    cache.settle();
    placements
}

/// A screen cell's position in the buffer; cells off screen never hold a mark, so they are
/// never asked for.
fn cell_at(x: i32, y: i32) -> (u16, u16) {
    (u16::try_from(x).unwrap_or(u16::MAX), u16::try_from(y).unwrap_or(u16::MAX))
}

/// The smallest rectangle around `cells`; `None` when there are none.
fn bounds(mut cells: impl Iterator<Item = (i32, i32)>) -> Option<Rect> {
    let (x, y) = cells.next()?;
    let (mut left, mut top, mut right, mut bottom) = (x, y, x, y);
    for (x, y) in cells {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    let width = u16::try_from(right - left + 1).ok()?;
    let height = u16::try_from(bottom - top + 1).ok()?;
    Some(Rect::new(left, top, width, height))
}

/// The part of `picture`'s pixels that lands in `shown`, a part of its cells: the source is
/// stretched evenly over the cells, so a cut at a cell is a cut at the same share of the source.
fn crop(picture: &Picture, shown: Rect) -> (u32, u32, u32, u32) {
    let (sx, sy, sw, sh) = picture.source;
    let cells = picture.cells;
    let along = |from: i32, to: i32, start: f64, length: f64, cells_start: i32, cells: u16, limit: u32| {
        let per_cell = length / f64::from(cells.max(1));
        let begin = start + f64::from(from - cells_start) * per_cell;
        let end = start + f64::from(to - cells_start) * per_cell;
        let first = pixels(begin).min(limit.saturating_sub(1));
        let last = pixels(end).clamp(first + 1, limit.max(first + 1));
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
