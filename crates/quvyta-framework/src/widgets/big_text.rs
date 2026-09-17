//! Big text: digits and letters drawn several rows tall, for clocks, counters and titles.

use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::GlyphMode;
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Pixel rows of every glyph.
const PIXEL_ROWS: usize = 5;

/// The bitmap of `c`, one string per pixel row, `#` for a lit pixel. Lowercase letters use the
/// uppercase forms; characters without a form are drawn as a space.
fn glyph(c: char) -> [&'static str; PIXEL_ROWS] {
    match c.to_ascii_uppercase() {
        '0' => ["###", "#.#", "#.#", "#.#", "###"],
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["###", "..#", "###", "#..", "###"],
        '3' => ["###", "..#", ".##", "..#", "###"],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' => ["###", "#..", "###", "..#", "###"],
        '6' => ["###", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", "..#", "..#", "..#"],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "###"],
        ':' => [".", "#", ".", "#", "."],
        '.' => [".", ".", ".", ".", "#"],
        '%' => ["#.#", "..#", ".#.", "#..", "#.#"],
        '-' => ["...", "...", "###", "...", "..."],
        'A' => [".#.", "#.#", "###", "#.#", "#.#"],
        'B' => ["##.", "#.#", "##.", "#.#", "##."],
        'C' => [".##", "#..", "#..", "#..", ".##"],
        'D' => ["##.", "#.#", "#.#", "#.#", "##."],
        'E' => ["###", "#..", "##.", "#..", "###"],
        'F' => ["###", "#..", "##.", "#..", "#.."],
        'G' => [".##", "#..", "#.#", "#.#", ".##"],
        'H' => ["#.#", "#.#", "###", "#.#", "#.#"],
        'I' => ["###", ".#.", ".#.", ".#.", "###"],
        'J' => ["..#", "..#", "..#", "#.#", ".#."],
        'K' => ["#.#", "#.#", "##.", "#.#", "#.#"],
        'L' => ["#..", "#..", "#..", "#..", "###"],
        'M' => ["#...#", "##.##", "#.#.#", "#...#", "#...#"],
        'N' => ["#..#", "##.#", "#.##", "#..#", "#..#"],
        'O' => [".#.", "#.#", "#.#", "#.#", ".#."],
        'P' => ["##.", "#.#", "##.", "#..", "#.."],
        'Q' => [".#.", "#.#", "#.#", "##.", ".##"],
        'R' => ["##.", "#.#", "##.", "#.#", "#.#"],
        'S' => [".##", "#..", ".#.", "..#", "##."],
        'T' => ["###", ".#.", ".#.", ".#.", ".#."],
        'U' => ["#.#", "#.#", "#.#", "#.#", "###"],
        'V' => ["#.#", "#.#", "#.#", "#.#", ".#."],
        'W' => ["#...#", "#...#", "#.#.#", "##.##", "#...#"],
        'X' => ["#.#", "#.#", ".#.", "#.#", "#.#"],
        'Y' => ["#.#", "#.#", ".#.", ".#.", ".#."],
        'Z' => ["###", "..#", ".#.", "#..", "###"],
        _ => ["..", "..", "..", "..", ".."],
    }
}

/// Text drawn large from block elements: digits, `:`, `.`, `%`, `-` and the letters A to Z.
///
/// Each glyph is five pixels tall. With Unicode and Nerd Font glyphs two pixels share a cell
/// through half blocks, so the text is three rows tall; ASCII mode draws one pixel per cell in
/// the background colour, five rows tall. Glyphs are separated by one column. When the area is too
/// small, the text is drawn at normal size in bold instead of being cut.
///
/// Style keys: `big-text` and `big-text.<variant>` (`fg`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigText {
    text: String,
    variant: Option<String>,
}

impl BigText {
    /// Big `text`, e.g. `"14:32"` or `"98%"`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), variant: None }
    }

    /// Theme variant, e.g. `"accent"`.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    fn big_width(&self) -> u16 {
        // Saturating: a long text is wider than any screen and falls back to plain text anyway.
        let glyphs = self
            .text
            .chars()
            .map(|c| clamp_u16(i32::try_from(glyph(c)[0].len()).unwrap_or(0)))
            .fold(0, u16::saturating_add);
        let gaps = clamp_u16(i32::try_from(self.text.chars().count()).unwrap_or(i32::MAX) - 1);
        glyphs.saturating_add(gaps)
    }
}

/// Rows the big form takes in `mode`.
fn big_rows(mode: GlyphMode) -> u16 {
    if mode == GlyphMode::Ascii { 5 } else { 3 }
}

impl<Msg: 'static> Widget<Msg> for BigText {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let rows = big_rows(cx.env().glyph_mode());
        let big = Size::new(self.big_width(), rows);
        if big.width <= available.width && big.height <= available.height {
            big
        } else {
            Size::new(text::width(&self.text), 1).min(available)
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let mut style = cx.style("big-text", self.variant.as_deref(), &[]).text();
        style.bg = None;
        let color = style.fg.unwrap_or_else(|| cx.color("text"));
        let mode = cx.env().glyph_mode();
        if self.big_width() > area.width || big_rows(mode) > area.height {
            let shown = text::truncate(&self.text, area.width).into_owned();
            cx.text(area.x, area.y, &shown, CellStyle::fg(color).with_bold(true), area.width);
            return;
        }
        let mut x = area.x;
        for c in self.text.chars() {
            let rows = glyph(c);
            let width = clamp_u16(i32::try_from(rows[0].len()).unwrap_or(0));
            for column in 0..usize::from(width) {
                let lit = |row: usize| rows.get(row).is_some_and(|line| line.as_bytes()[column] == b'#');
                let cell_x = x + i32::try_from(column).unwrap_or(0);
                if mode == GlyphMode::Ascii {
                    for row in 0..PIXEL_ROWS {
                        if lit(row) {
                            cx.clear(Rect::new(cell_x, area.y + i32::try_from(row).unwrap_or(0), 1, 1), color);
                        }
                    }
                    continue;
                }
                for cell_row in 0..3 {
                    let symbol = match (lit(cell_row * 2), lit(cell_row * 2 + 1)) {
                        (true, true) => "█",
                        (true, false) => "▀",
                        (false, true) => "▄",
                        (false, false) => continue,
                    };
                    let y = area.y + i32::try_from(cell_row).unwrap_or(0);
                    cx.text(cell_x, y, symbol, CellStyle::fg(color), 1);
                }
            }
            x += i32::from(width) + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(&'static str);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(BigText::new(self.0).variant("accent"));
        }
    }

    #[test]
    fn digits_and_colon_in_half_blocks() {
        let h = Harness::new(Demo("12:05"), 20, 3);
        assert_eq!(h.screen(), "▄█  ▀▀█ ▄ █▀█ █▀▀\n █  █▀▀ ▄ █ █ ▀▀█\n▀▀▀ ▀▀▀   ▀▀▀ ▀▀▀\n");
        assert_eq!(h.fg(0, 0), h.env().theme().color("accent"));
    }

    #[test]
    fn letters_and_percent() {
        let h = Harness::new(Demo("OK 9%"), 20, 3);
        assert_eq!(h.screen(), "▄▀▄ █ █    █▀█ ▀ █\n█ █ █▀▄    ▀▀█ ▄▀\n ▀  ▀ ▀    ▀▀▀ ▀ ▀\n");
    }

    #[test]
    fn ascii_uses_coloured_cells_five_rows_tall() {
        let mut h = Harness::new(Demo("7"), 6, 5);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "\n\n\n\n\n");
        let accent = h.env().theme().color("accent");
        assert_eq!(h.bg(0, 0), accent);
        assert_eq!(h.bg(2, 4), accent);
        assert_ne!(h.bg(0, 4), accent);
    }

    #[test]
    fn falls_back_to_plain_bold_text_when_small() {
        let h = Harness::new(Demo("12:05"), 20, 2);
        assert_eq!(h.screen(), "12:05\n\n");
        assert!(h.is_bold(0, 0));
    }

    #[test]
    fn text_wider_than_any_screen_falls_back_without_overflowing() {
        let h = Harness::new(Demo("12:05".repeat(4_000).leak()), 8, 3);
        assert_eq!(h.screen(), "12:0512…\n\n\n");
    }
}
