//! Drawing a screen as SVG: a rounded ground, merged background runs, block elements as exact
//! rectangles and every other character as a glyph outline from the embedded font.
//!
//! The picture holds no text and no font: browsers showing it through `<img>` (as GitHub does)
//! and the PNG rasteriser read the same outlines, so both look alike on every machine, and only
//! the glyphs the screen uses are embedded.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use qframe::color::Rgb;

use crate::Shot;
use crate::font::{self, GlyphKey};
use crate::geometry::{CELL_H, CELL_W, MARGIN, RADIUS, TITLE_H, num};
use crate::screen::Cell;

/// The finished picture and the characters the font could not draw.
pub(crate) struct Drawing {
    pub svg: String,
    pub missing: BTreeSet<char>,
}

/// A picture's shapes without the `<svg>` element around them, and the size they need. A larger
/// picture, such as a card, places them with a transform instead of drawing the screen again.
pub(crate) struct Body {
    pub content: String,
    pub width: f32,
    pub height: f32,
    pub missing: BTreeSet<char>,
}

/// Paths grouped by fill colour, in the order colours first appear, so the output is stable.
#[derive(Default)]
struct Fills {
    order: Vec<(Rgb, String)>,
    index: HashMap<Rgb, usize>,
}

impl Fills {
    fn entry(&mut self, color: Rgb) -> &mut String {
        let next = self.order.len();
        let at = *self.index.entry(color).or_insert(next);
        if at == next {
            self.order.push((color, String::new()));
        }
        &mut self.order[at].1
    }

    /// Adds a rectangle to the colour's path. One path per colour means rectangles of the same
    /// colour that touch are filled as one shape: scaled down in a browser, no hairline shows
    /// between two rows of the same tone.
    fn rect(&mut self, color: Rgb, x: f32, y: f32, w: f32, h: f32) {
        let d = self.entry(color);
        let _ = write!(d, "M{} {}h{}v{}h{}z", num(x), num(y), num(w), num(h), num(-w));
    }
}

/// Glyph outlines, defined once each, and where they are placed, grouped by colour.
#[derive(Default)]
struct Glyphs {
    defs: Vec<String>,
    ids: HashMap<GlyphKey, Option<usize>>,
    uses: Fills,
    missing: BTreeSet<char>,
}

/// The cells a character is drawn in: their top left corner and their width, in pixels.
#[derive(Clone, Copy)]
struct Slot {
    x: f32,
    y: f32,
    width: f32,
}

impl Glyphs {
    /// Places character `c` centred in `slot`. A glyph narrower than its cells, such as a CJK
    /// glyph in two, sits in their middle as a terminal draws it.
    fn place(&mut self, c: char, bold: bool, italic: bool, color: Rgb, slot: Slot) {
        // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
        // results are dropped for that reason and no other.
        let Some((key, advance)) = font::glyph(c, bold) else {
            self.missing.insert(c);
            return;
        };
        let key = GlyphKey { italic, ..key };
        let defs = &mut self.defs;
        let id = *self.ids.entry(key).or_insert_with(|| {
            let outline = font::outline(key)?;
            defs.push(outline);
            Some(defs.len() - 1)
        });
        if let Some(id) = id {
            let x = slot.x + (slot.width - advance) / 2.0;
            let _ = write!(self.uses.entry(color), "<use href=\"#g{id}\" x=\"{}\" y=\"{}\"/>", num(x), num(slot.y));
        }
    }
}

/// Parts of a cell a block element covers, as (left, top, right, bottom) fractions, with the
/// share of the text colour for the shades. Terminals draw these geometrically so they tile;
/// font outlines stop short of the cell's edges and would leave seams in bars and charts.
fn block(c: char) -> Option<(Vec<[f32; 4]>, f32)> {
    const UL: [f32; 4] = [0.0, 0.0, 0.5, 0.5];
    const UR: [f32; 4] = [0.5, 0.0, 1.0, 0.5];
    const LL: [f32; 4] = [0.0, 0.5, 0.5, 1.0];
    const LR: [f32; 4] = [0.5, 0.5, 1.0, 1.0];
    let eighths = |n: u32| n as f32 / 8.0;
    let code = u32::from(c);
    let parts = match code {
        0x2580 => vec![[0.0, 0.0, 1.0, 0.5]],
        0x2581..=0x2588 => vec![[0.0, 1.0 - eighths(code - 0x2580), 1.0, 1.0]],
        0x2589..=0x258F => vec![[0.0, 0.0, eighths(0x2590 - code), 1.0]],
        0x2590 => vec![[0.5, 0.0, 1.0, 1.0]],
        0x2591..=0x2593 => return Some((vec![[0.0, 0.0, 1.0, 1.0]], eighths(2 * (code - 0x2590)))),
        0x2594 => vec![[0.0, 0.0, 1.0, 0.125]],
        0x2595 => vec![[0.875, 0.0, 1.0, 1.0]],
        0x2596 => vec![LL],
        0x2597 => vec![LR],
        0x2598 => vec![UL],
        0x2599 => vec![UL, LL, LR],
        0x259A => vec![UL, LR],
        0x259B => vec![UL, UR, LL],
        0x259C => vec![UL, UR, LR],
        0x259D => vec![UR],
        0x259E => vec![UR, LL],
        0x259F => vec![UR, LL, LR],
        _ => return None,
    };
    Some((parts, 1.0))
}

/// The block element a cell shows, if its whole symbol is one.
fn cell_block(cell: &Cell) -> Option<(Vec<[f32; 4]>, f32)> {
    let mut chars = cell.symbol.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => block(c),
        _ => None,
    }
}

/// Splits the cell into tiles along every edge of `parts`, each marked inked when a part covers
/// it, with neighbours of the same kind in a row joined. The tiles cover the cell exactly once.
pub(crate) fn tiles(parts: &[[f32; 4]]) -> Vec<([f32; 4], bool)> {
    let cuts = |a: usize, b: usize| {
        let mut edges: Vec<f32> = parts.iter().flat_map(|p| [p[a], p[b]]).chain([0.0, 1.0]).collect();
        edges.sort_by(f32::total_cmp);
        edges.dedup();
        edges
    };
    let (xs, ys) = (cuts(0, 2), cuts(1, 3));
    let mut tiles = Vec::new();
    for rows in ys.windows(2) {
        let (t, b) = (rows[0], rows[1]);
        let mut current: Option<([f32; 4], bool)> = None;
        for cols in xs.windows(2) {
            let (l, r) = (cols[0], cols[1]);
            let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
            let inked = parts.iter().any(|p| p[0] <= cx && cx <= p[2] && p[1] <= cy && cy <= p[3]);
            current = match current {
                Some((mut tile, kind)) if kind == inked => {
                    tile[2] = r;
                    Some((tile, kind))
                }
                Some(done) => {
                    tiles.push(done);
                    Some(([l, t, r, b], inked))
                }
                None => Some(([l, t, r, b], inked)),
            };
        }
        tiles.extend(current);
    }
    tiles
}

/// The mouse pointer's outline with its tip at the origin, in picture units: a classic arrow a
/// little shorter than a cell is tall, so it covers one glyph and never hides a whole word.
const POINTER: &str = "M0 0V15L3.6 11.6L6.1 17.2L8.4 16.2L5.9 10.7H10.8Z";

/// Where the pointer's tip sits inside its cell, as fractions of the cell: left of the middle
/// and above it, as the hot spot of a pointer resting on a character looks.
const POINTER_TIP: (f32, f32) = (0.35, 0.3);

/// Draws `shot` as a standalone picture.
pub(crate) fn draw(shot: &Shot) -> Drawing {
    // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
    // results are dropped for that reason and no other.
    let body = body(shot);
    let (w, h) = (num(body.width), num(body.height));
    let mut svg = String::new();
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"
    );
    svg.push_str(&body.content);
    svg.push_str("</svg>\n");
    Drawing { svg, missing: body.missing }
}

/// Draws `shot`'s shapes: its screen, with a title strip when it has a title, a mouse pointer on
/// the cell it names and square corners when asked.
pub(crate) fn body(shot: &Shot) -> Body {
    // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
    // results are dropped for that reason and no other.
    let (screen, title, pointer) = (&shot.screen, shot.title.as_deref(), shot.pointer);
    let palette = screen.palette;
    let top = if title.is_some() { TITLE_H } else { 0.0 };
    let grid_w = f32::from(screen.width) * CELL_W;
    let grid_h = f32::from(screen.height) * CELL_H;
    let width = grid_w + 2.0 * MARGIN;
    let height = top + grid_h + 2.0 * MARGIN;
    let (left, grid_top) = (MARGIN, top + MARGIN);

    // Backgrounds and block elements share one path per colour, and no two of their shapes
    // overlap: where a bar's cells meet the partial block that ends it, both belong to one
    // shape, so no hairline shows when a browser scales the picture.
    let mut shapes = Fills::default();
    // Underlines lie on top of their cell's background, so they are painted afterwards.
    let mut overlay = Fills::default();
    let mut glyphs = Glyphs::default();
    for y in 0..screen.height {
        let py = grid_top + f32::from(y) * CELL_H;
        let row = screen.row(y);
        // Background runs: consecutive cells of one colour become one rectangle. The ground is
        // already there, so cells of its colour draw nothing. A block element's cell is split
        // into the parts of its text colour and the parts of its background.
        let mut x = 0;
        while x < row.len() {
            let px = left + x as f32 * CELL_W;
            if let Some((parts, share)) = cell_block(&row[x]) {
                let cell = &row[x];
                let ink = if share < 1.0 { cell.bg.mix(cell.fg, share) } else { cell.fg };
                let span = f32::from(cell.width.max(1)) * CELL_W;
                for ([l, t, r, b], inked) in tiles(&parts) {
                    let color = if inked { ink } else { cell.bg };
                    if color != palette.ground {
                        shapes.rect(color, px + l * span, py + t * CELL_H, (r - l) * span, (b - t) * CELL_H);
                    }
                }
                x += 1;
                continue;
            }
            let bg = row[x].bg;
            let run = row[x..].iter().take_while(|cell| cell.bg == bg && cell_block(cell).is_none()).count();
            if bg != palette.ground {
                shapes.rect(bg, px, py, run as f32 * CELL_W, CELL_H);
            }
            x += run;
        }
        for (x, cell) in row.iter().enumerate() {
            if cell.width == 0 || cell_block(cell).is_some() {
                continue;
            }
            let px = left + x as f32 * CELL_W;
            let span = f32::from(cell.width) * CELL_W;
            for c in cell.symbol.chars().filter(|c| !c.is_whitespace()) {
                glyphs.place(c, cell.bold, cell.italic, cell.fg, Slot { x: px, y: py, width: span });
            }
            if cell.underline {
                overlay.rect(cell.fg, px, py + font::baseline() + 2.0, span, 1.0);
            }
        }
    }

    let mut svg = String::new();
    let (w, h) = (num(width), num(height));
    if let Some(title) = title {
        let cells: f32 = title.chars().map(|c| f32::from(qframe::text::width(&c.to_string()))).sum();
        let mut x = ((width - cells * CELL_W) / 2.0).round();
        let y = ((TITLE_H - CELL_H) / 2.0).round();
        for c in title.chars() {
            let span = f32::from(qframe::text::width(&c.to_string())) * CELL_W;
            if !c.is_whitespace() {
                glyphs.place(c, false, false, palette.muted, Slot { x, y, width: span });
            }
            x += span;
        }
    }
    svg.push_str("<defs>\n");
    for (id, d) in glyphs.defs.iter().enumerate() {
        let _ = writeln!(svg, "<path id=\"g{id}\" d=\"{d}\"/>");
    }
    svg.push_str("</defs>\n");
    let radius = if shot.square { 0.0 } else { RADIUS };
    let r = num(radius);
    let _ = writeln!(svg, "<rect width=\"{w}\" height=\"{h}\" rx=\"{r}\" fill=\"{}\"/>", palette.ground);
    if title.is_some() {
        // The strip shares the ground's top corners and ends square above the grid.
        let _ = writeln!(
            svg,
            "<path fill=\"{}\" d=\"M0 {r}A{r} {r} 0 0 1 {r} 0H{}A{r} {r} 0 0 1 {w} {r}V{}H0Z\"/>",
            palette.title_ground,
            num(width - radius),
            num(TITLE_H),
        );
    }
    for fills in [&shapes, &overlay] {
        for (color, d) in &fills.order {
            let _ = writeln!(svg, "<path fill=\"{color}\" d=\"{d}\"/>");
        }
    }
    for (color, uses) in &glyphs.uses.order {
        let _ = writeln!(svg, "<g fill=\"{color}\">{uses}</g>");
    }
    if let Some((x, y)) = pointer.filter(|&(x, y)| x < screen.width && y < screen.height) {
        let tip_x = left + (f32::from(x) + POINTER_TIP.0) * CELL_W;
        let tip_y = grid_top + (f32::from(y) + POINTER_TIP.1) * CELL_H;
        // The ground's outline keeps the arrow readable over a background of the text's own tone.
        let _ = writeln!(
            svg,
            "<path transform=\"translate({} {})\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\" \
             stroke-linejoin=\"round\" d=\"{POINTER}\"/>",
            num(tip_x),
            num(tip_y),
            palette.text,
            palette.ground,
        );
    }
    Body { content: svg, width, height, missing: glyphs.missing }
}
