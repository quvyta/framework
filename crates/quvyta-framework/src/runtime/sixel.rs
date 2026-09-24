//! Showing pictures on a terminal that speaks sixel: the pixels are painted into the cells, so
//! every frame that changes what lies under a picture paints it again.
//!
//! A sixel is not kept by the terminal under a number, as a kitty picture is: it is written over
//! the cells like text, and text written later over it wipes it. So a picture is written only
//! where the whole of it shows (see [`resolve`](crate::widgets::image::resolve)), and written
//! again whenever it is new, moved or cut differently, after the terminal lost the screen, or
//! when a cell under it was written this frame. A place no longer shown leaves its pixels behind
//! until its cells are written; the cells the frame did not change are written again for it.
//! A frame whose places and cells are what the terminal shows writes nothing.
//!
//! The picture is shrunk to the pixels its cells cover on screen and reduced to a fixed palette
//! of 252 colours: six levels of red and blue, seven of green, to which the eye is most
//! sensitive. Its height is cut down to a whole number of six-pixel bands, so the last band never
//! reaches past the last row of its cells and the screen never scrolls.

use std::io::Write;

use ratatui_core::buffer::Buffer;

use crate::color::Rgb;
use crate::widgets::image::{PicturePlacement, crop_pixels};

/// The size of a cell in pixels when the terminal does not say: a common one at ordinary sizes.
pub(crate) const CELL: (u16, u16) = (10, 20);

/// The levels of red, green and blue in the palette.
const LEVELS: (u32, u32, u32) = (6, 7, 6);

/// The sixel pictures a screen shows, and what they were encoded into.
#[derive(Debug)]
pub(crate) struct SixelPictures {
    /// The places written and still showing.
    shown: Vec<PicturePlacement>,
    /// The size of a cell in pixels.
    cell: (u16, u16),
    /// Pictures already encoded, kept while their picture is painted.
    encoded: Vec<Encoded>,
}

impl Default for SixelPictures {
    fn default() -> Self {
        Self { shown: Vec::new(), cell: CELL, encoded: Vec::new() }
    }
}

/// One place's pixels, encoded.
#[derive(Debug)]
struct Encoded {
    /// The picture, its crop and the size in pixels it was shrunk to.
    key: (u64, (u32, u32, u32, u32), (u32, u32)),
    bytes: Vec<u8>,
}

/// What a frame writes for its sixel pictures, after its cells.
#[derive(Debug, Default)]
pub(crate) struct SixelFrame {
    /// Cells to write again although they did not change, since a picture no longer shown
    /// covers them: column and row.
    pub(crate) repaint: Vec<(u16, u16)>,
    /// The pictures, each after moving the cursor to its first cell.
    pub(crate) bytes: Vec<u8>,
}

impl SixelPictures {
    /// Takes the size of a cell in pixels, as the terminal reports it.
    pub(crate) fn set_cell(&mut self, cell: (u16, u16)) {
        if cell.0 > 0 && cell.1 > 0 {
            self.cell = cell;
        }
    }

    /// What takes the terminal from the places shown to `placements`, given the cells on screen,
    /// `shown` (`None` when the terminal's contents are not known), and the cells about to be
    /// written, `now`. `painted` holds the identities of the pictures on screen in any way this
    /// frame: their encodings are kept for when they are shown whole again.
    pub(crate) fn frame(
        &mut self,
        placements: &[PicturePlacement],
        painted: &[u64],
        shown: Option<&Buffer>,
        now: &Buffer,
    ) -> SixelFrame {
        // A screen of another size was cleared, so nothing on it is known.
        let shown = shown.filter(|shown| shown.area == now.area);
        let mut out = SixelFrame::default();
        if let Some(before) = shown {
            for gone in self.shown.iter().filter(|old| !placements.contains(old)) {
                for position in positions(gone, now) {
                    if before[position] == now[position] && !out.repaint.contains(&position) {
                        out.repaint.push(position);
                    }
                }
            }
        }
        for placement in placements {
            let written_over =
                shown.is_none_or(|before| positions(placement, now).any(|position| before[position] != now[position]));
            if written_over || !self.shown.contains(placement) {
                self.write(&mut out.bytes, placement);
            }
        }
        self.encoded.retain(|encoded| painted.contains(&encoded.key.0));
        self.shown = placements.to_vec();
        out
    }

    /// Forgets the places shown, for when the terminal is handed to a program or left: the
    /// program's own output wipes them.
    pub(crate) fn release(&mut self) {
        self.shown.clear();
    }

    /// Writes `placement`: the cursor moved to its first cell, then its pixels.
    fn write(&mut self, out: &mut Vec<u8>, placement: &PicturePlacement) {
        let (column, row, columns, rows) = placement.cells;
        let size = pixel_size((columns, rows), self.cell);
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let key = (placement.data.id(), placement.crop, size);
        let index = match self.encoded.iter().position(|encoded| encoded.key == key) {
            Some(index) => index,
            None => {
                let pixels = crop_pixels(&placement.data, placement.crop, size.0, size.1);
                self.encoded.push(Encoded { key, bytes: encode(&pixels, size.0, size.1) });
                self.encoded.len() - 1
            }
        };
        let _ = write!(out, "\x1b[{};{}H", u32::from(row) + 1, u32::from(column) + 1);
        out.extend_from_slice(&self.encoded[index].bytes);
    }
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
        let placement = PicturePlacement { data: data.clone(), number: 1, cells: (1, 1, 2, 1), crop: (0, 0, 4, 4) };
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
        let placement = PicturePlacement { data: data.clone(), number: 1, cells: (1, 1, 2, 2), crop: (0, 0, 4, 4) };
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
