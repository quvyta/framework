//! Showing pictures on a terminal that speaks sixel: the pixels are painted into the cells, so
//! every frame that writes a cell under a picture paints that part of it again.
//!
//! A sixel is not kept by the terminal under a number, as a kitty picture is: it is written over
//! the cells like text, and text written later over a cell wipes the pixels in it. The pixels
//! of a cell nothing wrote over stay, whichever piece of which frame put them there. So the
//! screen remembers, cell by cell, whose pixels it shows, and a frame sends pixels only for the
//! cells of its places that do not already show them: the cells it writes, the cells that had
//! other pixels or none, and every cell after the terminal lost the screen. Those cells are cut
//! into rectangles by the rule the places come from (see
//! [`resolve`](crate::widgets::image::resolve)), each one a sixel of its own crop, at most
//! [`MOST_PLACES`] a picture; more than that sends the places that hold them whole. A window
//! dragged over a wallpaper then sends only the strip it uncovered. A cell that shows pixels no
//! place wants any more and that the frame does not write is written again, so its pixels go.
//! A frame whose places and cells are what the terminal shows writes nothing.
//!
//! The picture is shrunk to the pixels its cells cover on screen and reduced to a fixed palette
//! of 252 colours: six levels of red and blue, seven of green, to which the eye is most
//! sensitive. Its height is cut down to a whole number of six-pixel bands, so the last band never
//! reaches past the last row of its cells and the screen never scrolls; the pixels are worked
//! out at the cells' full height and the rows past the last band left out, so the pieces of a
//! picture meet without stretching, and those few rows show the ground `resolve` gave the cells.

use std::io::Write;

use ratatui_core::buffer::{Buffer, Cell};
use ratatui_core::style::Color;

use crate::color::Rgb;
use crate::widgets::image::{MOST_PLACES, PicturePlacement, crop_pixels, rectangles};

/// The size of a cell in pixels when the terminal does not say: a common one at ordinary sizes.
pub(crate) const CELL: (u16, u16) = (10, 20);

/// The levels of red, green and blue in the palette.
const LEVELS: (u32, u32, u32) = (6, 7, 6);

/// The sixel pictures a screen shows, and what they were encoded into.
#[derive(Debug)]
pub(crate) struct SixelPictures {
    /// For every cell of the screen, row by row, whose pixels it shows: the key of the stretch
    /// of the picture (see `Stretch::key`), `None` for none, or none known.
    showing: Vec<Option<u64>>,
    /// The columns and rows `showing` is for.
    size: (u16, u16),
    /// The size of a cell in pixels.
    cell: (u16, u16),
    /// Pieces already encoded, kept while their picture is painted.
    encoded: Vec<Encoded>,
}

impl Default for SixelPictures {
    fn default() -> Self {
        Self { showing: Vec::new(), size: (0, 0), cell: CELL, encoded: Vec::new() }
    }
}

/// One piece's pixels, encoded.
#[derive(Debug)]
struct Encoded {
    /// The picture, its crop and the size in pixels of its cells.
    key: (u64, (u32, u32, u32, u32), (u32, u32)),
    bytes: Vec<u8>,
    /// For each cell of the last row, the colour of the pixels below the last band, which the
    /// sixel leaves out; empty when the bands fill the cells.
    short: Vec<Rgb>,
}

/// What a frame writes for its sixel pictures, after its cells.
#[derive(Debug, Default)]
pub(crate) struct SixelFrame {
    /// Cells to write again although they did not change, since a picture no longer shown
    /// covers them: column and row.
    pub(crate) repaint: Vec<(u16, u16)>,
    /// Cells of the last row of a piece whose bands stop a few pixels short of its bottom,
    /// written before the pictures with the colour of those pixels as their ground: column, row
    /// and the cell.
    pub(crate) short: Vec<(u16, u16, Cell)>,
    /// The pictures, each after moving the cursor to its first cell.
    pub(crate) bytes: Vec<u8>,
}

impl SixelPictures {
    /// Takes the size of a cell in pixels, as the terminal reports it.
    pub(crate) fn set_cell(&mut self, cell: (u16, u16)) {
        if cell.0 > 0 && cell.1 > 0 && cell != self.cell {
            self.cell = cell;
            self.forget();
        }
    }

    /// What takes the terminal from the pixels it shows to `placements`, given the cells on
    /// screen, `shown` (`None` when the terminal's contents are not known), and the cells about
    /// to be written, `now`. `painted` holds the identities of the pictures on screen in any way
    /// this frame: their encodings are kept for when they are shown again.
    pub(crate) fn frame(
        &mut self,
        placements: &[PicturePlacement],
        painted: &[u64],
        shown: Option<&Buffer>,
        now: &Buffer,
    ) -> SixelFrame {
        let area = now.area;
        // A screen of another size was cleared, so nothing on it is known.
        let shown = shown.filter(|shown| shown.area == area);
        if shown.is_none() || self.size != (area.width, area.height) {
            self.showing = vec![None; usize::from(area.width) * usize::from(area.height)];
            self.size = (area.width, area.height);
        }
        let at = |(x, y): (u16, u16)| usize::from(y - area.y) * usize::from(area.width) + usize::from(x - area.x);
        // Whose pixels each cell is to show.
        let mut wanted: Vec<Option<u64>> = vec![None; self.showing.len()];
        let keys: Vec<u64> = placements.iter().map(|placement| placement.stretch.key(placement.data.id())).collect();
        for (placement, key) in placements.iter().zip(&keys) {
            for position in positions(placement, now) {
                wanted[at(position)] = Some(*key);
            }
        }
        let mut out = SixelFrame::default();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let index = at((x, y));
                // A cell written this frame loses whatever pixels it had.
                if shown.is_none_or(|before| before[(x, y)] != now[(x, y)]) {
                    self.showing[index] = None;
                } else if self.showing[index].is_some() && self.showing[index] != wanted[index] {
                    out.repaint.push((x, y));
                    self.showing[index] = None;
                }
            }
        }
        let mut done: Vec<u64> = Vec::new();
        for (first, key) in keys.iter().enumerate() {
            if done.contains(key) {
                continue;
            }
            done.push(*key);
            let places: Vec<&PicturePlacement> = placements
                .iter()
                .zip(&keys)
                .filter(|(_, other)| *other == key)
                .map(|(placement, _)| placement)
                .collect();
            let mut needed: Vec<(i32, i32)> = places
                .iter()
                .flat_map(|placement| positions(placement, now))
                .filter(|position| self.showing[at(*position)] != Some(*key))
                .map(|(x, y)| (i32::from(x), i32::from(y)))
                .collect();
            if needed.is_empty() {
                continue;
            }
            needed.sort_unstable_by_key(|&(x, y)| (y, x));
            let pieces: Vec<PicturePlacement> = match rectangles(&needed, MOST_PLACES) {
                Some(rects) => rects.into_iter().map(|rect| placements[first].part(rect)).collect(),
                None => places
                    .into_iter()
                    .filter(|placement| {
                        positions(placement, now).any(|position| self.showing[at(position)] != Some(*key))
                    })
                    .cloned()
                    .collect(),
            };
            for piece in pieces {
                if self.write(&mut out, &piece, now) {
                    for position in positions(&piece, now) {
                        self.showing[at(position)] = Some(*key);
                    }
                }
            }
        }
        self.encoded.retain(|encoded| painted.contains(&encoded.key.0));
        out
    }

    /// Forgets the pixels shown, for when the terminal is handed to a program or left: the
    /// program's own output wipes them.
    pub(crate) fn release(&mut self) {
        self.forget();
    }

    /// Takes the terminal to show no pixels anywhere.
    fn forget(&mut self) {
        self.showing.clear();
        self.size = (0, 0);
    }

    /// Writes `placement`: the cursor moved to its first cell, then its pixels, and the cells
    /// of its last row the bands leave short, taken from `now`. Whether anything was written:
    /// cells too small for a band of six pixels take none.
    fn write(&mut self, out: &mut SixelFrame, placement: &PicturePlacement, now: &Buffer) -> bool {
        let (column, row, columns, rows) = placement.cells;
        let full = (u32::from(columns) * u32::from(self.cell.0), u32::from(rows) * u32::from(self.cell.1));
        let (width, height) = pixel_size((columns, rows), self.cell);
        if width == 0 || height == 0 {
            return false;
        }
        let key = (placement.data.id(), placement.crop, full);
        let index = match self.encoded.iter().position(|encoded| encoded.key == key) {
            Some(index) => index,
            None => {
                // Worked out at the cells' full height, so the rows kept are the rows a taller
                // piece of the same picture shows there, then cut to whole bands.
                let mut pixels = crop_pixels(&placement.data, placement.crop, full.0, full.1);
                let short = below(&pixels, full, height, self.cell.0);
                pixels.truncate(usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(0));
                self.encoded.push(Encoded { key, bytes: encode(&pixels, width, height), short });
                self.encoded.len() - 1
            }
        };
        let encoded = &self.encoded[index];
        let last = row + rows - 1;
        for (offset, colour) in (0..columns).zip(&encoded.short) {
            let position = (column + offset, last);
            if position.0 >= now.area.right() || last >= now.area.bottom() {
                continue;
            }
            let mut cell = now[position].clone();
            // A frame reduced to the 256-colour palette keeps its grounds in it.
            cell.bg = match cell.bg {
                Color::Indexed(_) => Color::Indexed(colour.to_ansi256()),
                _ => Color::Rgb(colour.r, colour.g, colour.b),
            };
            out.short.push((position.0, position.1, cell));
        }
        let _ = write!(out.bytes, "\x1b[{};{}H", u32::from(row) + 1, u32::from(column) + 1);
        out.bytes.extend_from_slice(&encoded.bytes);
        true
    }
}

/// For each cell across a piece of `full` pixels, cells `cell` pixels wide, the average colour
/// of its `pixels` from row `height` down, which a sixel of `height` rows leaves out. Empty when
/// there are none.
fn below(pixels: &[Rgb], full: (u32, u32), height: u32, cell: u16) -> Vec<Rgb> {
    if height >= full.1 || cell == 0 {
        return Vec::new();
    }
    let (width, cell) = (usize::try_from(full.0).unwrap_or(0), usize::from(cell));
    let rows = usize::try_from(height).unwrap_or(0)..usize::try_from(full.1).unwrap_or(0);
    (0..width / cell)
        .map(|nth| {
            let mut sum = [0u64; 3];
            let mut count = 0u64;
            for y in rows.clone() {
                for pixel in pixels.iter().skip(y * width + nth * cell).take(cell) {
                    sum[0] += u64::from(pixel.r);
                    sum[1] += u64::from(pixel.g);
                    sum[2] += u64::from(pixel.b);
                    count += 1;
                }
            }
            let mean = |total: u64| u8::try_from((total + count / 2) / count.max(1)).unwrap_or(255);
            Rgb::new(mean(sum[0]), mean(sum[1]), mean(sum[2]))
        })
        .collect()
}

/// The cells of `placement` within `buffer`.
fn positions(placement: &PicturePlacement, buffer: &Buffer) -> impl Iterator<Item = (u16, u16)> {
    let (column, row, columns, rows) = placement.cells;
    let area = buffer.area;
    let right = column.saturating_add(columns).min(area.right());
    let bottom = row.saturating_add(rows).min(area.bottom());
    (row..bottom).flat_map(move |y| (column..right).map(move |x| (x, y)))
}

/// The pixels a picture over `cells` (columns, rows) is shrunk to, for cells of `cell` pixels:
/// every pixel across, and down to the last whole band of six, so the picture never reaches
/// past its last row.
pub(crate) fn pixel_size(cells: (u16, u16), cell: (u16, u16)) -> (u32, u32) {
    let width = u32::from(cells.0) * u32::from(cell.0);
    let height = u32::from(cells.1) * u32::from(cell.1);
    (width, height / 6 * 6)
}

/// The palette entry closest to `pixel`: each channel to its nearest level.
fn entry(pixel: Rgb) -> usize {
    let level = |value: u8, levels: u32| (u32::from(value) * (levels - 1) + 127) / 255;
    let (r, g, b) = (level(pixel.r, LEVELS.0), level(pixel.g, LEVELS.1), level(pixel.b, LEVELS.2));
    usize::try_from((r * LEVELS.1 + g) * LEVELS.2 + b).unwrap_or(0)
}

/// The colour of palette entry `index`, in the percentages sixel takes.
fn percentages(index: usize) -> (u32, u32, u32) {
    let index = u32::try_from(index).unwrap_or(0);
    let (r, g, b) = (index / (LEVELS.1 * LEVELS.2), index / LEVELS.2 % LEVELS.1, index % LEVELS.2);
    let percent = |level: u32, levels: u32| (level * 200 + (levels - 1)) / (2 * (levels - 1));
    (percent(r, LEVELS.0), percent(g, LEVELS.1), percent(b, LEVELS.2))
}

/// `pixels`, `width` × `height` row after row, as one sixel image: a device control string that
/// keeps what is under pixels of no colour (there are none), declares the size at square pixels,
/// defines the palette entries used and paints six rows at a time, colour by colour.
pub(crate) fn encode(pixels: &[Rgb], width: u32, height: u32) -> Vec<u8> {
    let entries = LEVELS.0 * LEVELS.1 * LEVELS.2;
    let (columns, rows) = (usize::try_from(width).unwrap_or(0), usize::try_from(height).unwrap_or(0));
    let indices: Vec<usize> = pixels.iter().take(columns * rows).map(|pixel| entry(*pixel)).collect();
    let mut used = vec![false; usize::try_from(entries).unwrap_or(0)];
    for index in &indices {
        used[*index] = true;
    }
    let mut out = Vec::with_capacity(indices.len() / 2);
    let _ = write!(out, "\x1bP0;1;0q\"1;1;{width};{height}");
    for (index, _) in used.iter().enumerate().filter(|(_, used)| **used) {
        let (r, g, b) = percentages(index);
        let _ = write!(out, "#{index};2;{r};{g};{b}");
    }
    // Per palette entry, the sixel of every column of the band being written.
    let mut bands: Vec<Vec<u8>> = vec![Vec::new(); used.len()];
    let mut present: Vec<usize> = Vec::new();
    for top in (0..rows).step_by(6) {
        present.clear();
        for (offset, row) in (top..rows.min(top + 6)).enumerate() {
            for (column, index) in indices[row * columns..(row + 1) * columns].iter().enumerate() {
                let band = &mut bands[*index];
                if band.is_empty() {
                    band.resize(columns, 0);
                    present.push(*index);
                }
                band[column] |= 1 << offset;
            }
        }
        if top > 0 {
            out.push(b'-');
        }
        present.sort_unstable();
        for (nth, index) in present.iter().enumerate() {
            if nth > 0 {
                out.push(b'$');
            }
            let _ = write!(out, "#{index}");
            runs(&mut out, &bands[*index]);
            bands[*index].clear();
        }
    }
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Writes one colour's sixels for a band, `bits` a column each, with repeats of four or more
/// written as `!` and a count; the empty sixels at the end are left out.
fn runs(out: &mut Vec<u8>, bits: &[u8]) {
    let end = bits.iter().rposition(|bits| *bits != 0).map_or(0, |last| last + 1);
    let mut at = 0;
    while at < end {
        let bits_here = bits[at];
        let length = bits[at..end].iter().take_while(|bits| **bits == bits_here).count();
        let sixel = 63 + bits_here;
        if length >= 4 {
            let _ = write!(out, "!{length}");
            out.push(sixel);
        } else {
            out.extend(std::iter::repeat_n(sixel, length));
        }
        at += length;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::ImageData;

    fn text(bytes: &[u8]) -> String {
        String::from_utf8(bytes.to_vec()).expect("sixel is ASCII")
    }

    #[test]
    fn two_colours_give_the_exact_sequence() {
        let (red, blue) = (Rgb::new(255, 0, 0), Rgb::new(0, 0, 255));
        let pixels: Vec<Rgb> = (0..6).flat_map(|_| [red, blue]).collect();
        assert_eq!(
            text(&encode(&pixels, 2, 6)),
            "\x1bP0;1;0q\"1;1;2;6#5;2;0;0;100#210;2;100;0;0#5?~$#210~\x1b\\",
            "blue in the right column, red in the left, its empty end left out"
        );
    }

    #[test]
    fn a_run_of_four_or_more_is_counted() {
        let white = Rgb::new(255, 255, 255);
        assert_eq!(text(&encode(&[white; 60], 10, 6)), "\x1bP0;1;0q\"1;1;10;6#251;2;100;100;100#251!10~\x1b\\");
        let pixels: Vec<Rgb> = (0..6).flat_map(|_| [white, white, white, Rgb::new(0, 0, 0)]).collect();
        assert!(
            text(&encode(&pixels, 4, 6)).ends_with("#0???~$#251~~~\x1b\\"),
            "three are shorter written out: {}",
            text(&encode(&pixels, 4, 6))
        );
    }

    #[test]
    fn a_band_under_six_rows_sets_only_its_rows_and_bands_are_separated() {
        let grey = Rgb::new(128, 128, 128);
        let encoded = text(&encode(&[grey; 7], 1, 7));
        let index = entry(grey);
        assert!(encoded.ends_with(&format!("#{index}~-#{index}@\x1b\\")), "{encoded}");
    }

    #[test]
    fn many_colours_share_at_most_256_palette_entries() {
        let (width, height) = (64u32, 48u32);
        let pixels: Vec<Rgb> = (0..width * height)
            .map(|at| {
                let byte = |shift: u32| u8::try_from((at.wrapping_mul(2_654_435_761) >> shift) & 255).expect("a byte");
                Rgb::new(byte(0), byte(8), byte(16))
            })
            .collect();
        let mut distinct = pixels.clone();
        distinct.sort_by_key(|pixel| (pixel.r, pixel.g, pixel.b));
        distinct.dedup();
        assert!(distinct.len() > 1000, "{} colours in", distinct.len());
        let encoded = text(&encode(&pixels, width, height));
        let entries = encoded.matches(";2;").count();
        assert!(entries <= 256 && entries > 100, "{entries} palette entries");
        for (index, _) in encoded.match_indices('#') {
            let number: String = encoded[index + 1..].chars().take_while(char::is_ascii_digit).collect();
            assert!(number.parse::<u32>().expect("an entry") < 256);
        }
    }

    #[test]
    fn the_palette_spans_black_to_white_in_every_channel() {
        assert_eq!(percentages(entry(Rgb::new(0, 0, 0))), (0, 0, 0));
        assert_eq!(percentages(entry(Rgb::new(255, 255, 255))), (100, 100, 100));
        assert_eq!(percentages(entry(Rgb::new(0, 128, 0))), (0, 50, 0), "green has a middle level");
        assert_eq!(percentages(entry(Rgb::new(102, 0, 0))), (40, 0, 0));
    }

    #[test]
    fn a_screen_of_another_size_gets_its_pictures_again_and_nothing_to_repaint() {
        use ratatui_core::layout::Rect;
        let data = ImageData::from_rgb(4, 4, &[90; 48]).expect("pixels");
        let placement = PicturePlacement::whole(&data, (1, 1, 2, 1));
        let (small, large) = (Buffer::empty(Rect::new(0, 0, 6, 3)), Buffer::empty(Rect::new(0, 0, 8, 4)));
        let mut sixels = SixelPictures::default();
        let first = sixels.frame(std::slice::from_ref(&placement), &[data.id()], None, &small);
        assert!(first.bytes.starts_with(b"\x1b[2;2H\x1bP"), "{:?}", text(&first.bytes));
        let idle = sixels.frame(std::slice::from_ref(&placement), &[data.id()], Some(&small), &small);
        assert!(idle.bytes.is_empty() && idle.repaint.is_empty());
        let resized = sixels.frame(std::slice::from_ref(&placement), &[data.id()], Some(&small), &large);
        assert_eq!(resized.bytes, first.bytes, "the terminal was cleared for the new size");
        let gone = sixels.frame(&[], &[], Some(&small), &large);
        assert!(gone.repaint.is_empty() && gone.bytes.is_empty(), "a cleared screen keeps no pixels to cover");
    }

    #[test]
    fn a_place_no_longer_shown_repaints_the_cells_the_frame_left_alone() {
        use ratatui_core::layout::Rect;
        let data = ImageData::from_rgb(4, 4, &[90; 48]).expect("pixels");
        let placement = PicturePlacement::whole(&data, (1, 1, 2, 2));
        let before = Buffer::empty(Rect::new(0, 0, 6, 4));
        let mut after = before.clone();
        after[(2, 2)].set_symbol("x");
        let mut sixels = SixelPictures::default();
        sixels.frame(std::slice::from_ref(&placement), &[data.id()], None, &before);
        let gone = sixels.frame(&[], &[data.id()], Some(&before), &after);
        assert_eq!(gone.repaint, [(1, 1), (2, 1), (1, 2)], "the changed cell is written anyway");
        assert!(gone.bytes.is_empty());
        assert_eq!(sixels.encoded.len(), 1, "kept while the picture is still painted");
        sixels.frame(&[], &[], Some(&after), &after);
        assert!(sixels.encoded.is_empty(), "dropped once it is not");
    }

    #[test]
    fn the_pixels_below_the_last_band_become_the_ground_of_the_last_row() {
        use ratatui_core::layout::Rect;
        // Nineteen rows of blue, then red: over three rows of twenty-pixel cells, the first row
        // shows the blue and, in its last pixel row, the first red one.
        let (blue, red) = ([0u8, 0, 255], [255u8, 0, 0]);
        let rgb: Vec<u8> = (0..60).flat_map(|row| if row < 19 { blue } else { red }).collect();
        let data = ImageData::from_rgb(1, 60, &rgb).expect("pixels");
        let whole = PicturePlacement::whole(&data, (0, 0, 1, 3));
        let first = whole.part(crate::geometry::Rect::new(0, 0, 1, 1));
        let now = Buffer::empty(Rect::new(0, 0, 2, 3));
        let mut sixels = SixelPictures::default();
        let written = sixels.frame(std::slice::from_ref(&first), &[data.id()], None, &now);
        let sixel = text(&written.bytes);
        assert!(sixel.contains("\"1;1;10;18#"), "{sixel}");
        assert_eq!(sixel.matches(";2;").count(), 1, "the eighteen rows kept are all blue, none squeezed: {sixel}");
        assert_eq!(written.short.len(), 1, "one cell in the last row");
        let (x, y, cell) = &written.short[0];
        assert_eq!((*x, *y), (0, 0));
        assert_eq!(cell.bg, ratatui_core::style::Color::Rgb(128, 0, 128), "half blue, half red");
        let full = sixels.frame(std::slice::from_ref(&whole), &[data.id()], None, &now);
        assert!(full.short.is_empty(), "sixty pixels are ten whole bands");
    }

    #[test]
    fn the_height_is_whole_bands_within_the_cells() {
        for cell in [(10, 20), (9, 17), (8, 16), (12, 25), (7, 5)] {
            for rows in 1..=12u16 {
                let (width, height) = pixel_size((3, rows), cell);
                let room = u32::from(rows) * u32::from(cell.1);
                assert_eq!(width, 3 * u32::from(cell.0));
                assert_eq!(height % 6, 0, "{cell:?} {rows}");
                assert!(height <= room && height + 6 > room, "{height} in {room}");
                if height == 0 {
                    continue;
                }
                let pixels = vec![Rgb::new(10, 20, 30); usize::try_from(width * height).expect("small")];
                let encoded = text(&encode(&pixels, width, height));
                assert!(encoded.contains(&format!("\"1;1;{width};{height}#")));
                let bands = u32::try_from(encoded.matches('-').count()).expect("few") + 1;
                assert_eq!(bands * 6, height, "{bands} bands for {height} pixels");
            }
        }
    }
}
