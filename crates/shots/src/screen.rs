//! A copy of the harness screen with every colour resolved, so drawing needs no terminal types.

use qframe::color::Rgb;
use qframe::runtime::{App, Harness};
use ratatui_core::style::{Color, Modifier};

/// How far the title strip moves from the ground towards the text colour: a quiet step.
const TITLE_TINT: f32 = 0.04;

/// How far a dim cell's text moves towards its background, as terminals draw faint text.
const DIM: f32 = 0.45;

/// One terminal cell as it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cell {
    /// The grapheme drawn, empty for the second half of a wide character.
    pub symbol: String,
    /// Cells the symbol covers: 1, or 2 for a wide character.
    pub width: u16,
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// The theme colours around the grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Palette {
    /// The theme's `canvas`: the ground of the application itself, whatever the captured screen
    /// happens to show at its edge. A picture larger than the screen, such as a card, stands on
    /// it.
    pub canvas: Rgb,
    /// The theme's one accent colour, for the few words a picture leads with.
    pub accent: Rgb,
    /// The ground under everything: the rounded background and the margin. The background most
    /// of the screen's edge shows, else the theme's `canvas`.
    pub ground: Rgb,
    /// Text colour of cells that leave it to the terminal.
    pub text: Rgb,
    /// The title's text.
    pub muted: Rgb,
    /// The title strip: a small step from the ground towards the text.
    pub title_ground: Rgb,
}

/// The screen, row by row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Screen {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
    pub palette: Palette,
}

impl Screen {
    pub(crate) fn capture<A: App>(harness: &Harness<A>) -> Self {
        let theme = harness.env().theme();
        let color = |token: &str, fallback: Rgb| theme.color(token).unwrap_or(fallback);
        let canvas = color("canvas", Rgb::new(0x10, 0x12, 0x16));
        let text = color("text", Rgb::new(0xe6, 0xe6, 0xe6));
        let buffer = harness.buffer();
        let area = buffer.area;
        let mut cells = Vec::with_capacity(usize::from(area.width) * usize::from(area.height));
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &buffer[(x, y)];
                let modifier = cell.modifier;
                let bg = rgb(cell.bg).unwrap_or(canvas);
                let fg = rgb(cell.fg).unwrap_or(text);
                let fg = if modifier.contains(Modifier::DIM) { fg.mix(bg, DIM) } else { fg };
                let symbol = cell.symbol().to_owned();
                let width = if symbol.is_empty() { 0 } else { qframe::text::width(&symbol).max(1) };
                cells.push(Cell {
                    symbol,
                    width,
                    fg,
                    bg,
                    bold: modifier.contains(Modifier::BOLD),
                    italic: modifier.contains(Modifier::ITALIC),
                    underline: modifier.contains(Modifier::UNDERLINED),
                });
            }
        }
        // The ground continues the screen's own edge, so the margin reads as more of the
        // application and never as a band of another tone around it.
        let ground = edge_color(&cells, area.width, area.height).unwrap_or(canvas);
        let palette = Palette {
            canvas,
            accent: color("accent", text),
            ground,
            text,
            muted: color("muted", ground.mix(text, 0.5)),
            title_ground: ground.mix(text, TITLE_TINT),
        };
        Self { width: area.width, height: area.height, cells, palette }
    }

    pub(crate) fn row(&self, y: u16) -> &[Cell] {
        let start = usize::from(y) * usize::from(self.width);
        &self.cells[start..start + usize::from(self.width)]
    }
}

/// The background most cells on the screen's outer edge share; ties go to the colour met first,
/// reading the edge from the top left corner clockwise.
pub(crate) fn edge_color(cells: &[Cell], width: u16, height: u16) -> Option<Rgb> {
    let (w, h) = (usize::from(width), usize::from(height));
    if w == 0 || h == 0 {
        return None;
    }
    let mut edge: Vec<usize> = (0..w).collect();
    edge.extend((1..h).map(|y| y * w + w - 1));
    edge.extend((0..w.saturating_sub(1)).rev().map(|x| (h - 1) * w + x).filter(|&i| h > 1 && i >= w));
    edge.extend((1..h.saturating_sub(1)).rev().map(|y| y * w).filter(|_| w > 1));
    let mut counts: Vec<(Rgb, usize)> = Vec::new();
    for i in edge {
        let bg = cells[i].bg;
        match counts.iter_mut().find(|(color, _)| *color == bg) {
            Some((_, n)) => *n += 1,
            None => counts.push((bg, 1)),
        }
    }
    // `max_by_key` keeps the last of equal counts; reversing first keeps the first met.
    counts.into_iter().rev().max_by_key(|(_, n)| *n).map(|(color, _)| color)
}

/// A cell colour as 24-bit RGB. Palette colours only appear when the harness draws for a
/// reduced colour depth; those cells fall back to the theme's text and ground.
fn rgb(color: Color) -> Option<Rgb> {
    match color {
        Color::Rgb(r, g, b) => Some(Rgb::new(r, g, b)),
        _ => None,
    }
}
