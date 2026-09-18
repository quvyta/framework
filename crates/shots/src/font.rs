//! The embedded font: glyph lookup and outlines turned into SVG path data.

use std::fmt::Write as _;
use std::sync::LazyLock;

use ttf_parser::{Face, GlyphId, OutlineBuilder};

use crate::geometry::{CELL_H, CELL_W, num};

/// JetBrains Mono Nerd Font Mono, Regular. Licence and checksums: `fonts/README.md`.
static REGULAR_DATA: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFontMono-Regular.ttf");
/// JetBrains Mono Nerd Font Mono, Bold.
static BOLD_DATA: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFontMono-Bold.ttf");

/// The parsed faces. A face that fails to parse stays `None`, and every character is then
/// reported missing instead of the program stopping.
static REGULAR: LazyLock<Option<Face<'static>>> = LazyLock::new(|| Face::parse(REGULAR_DATA, 0).ok());
static BOLD: LazyLock<Option<Face<'static>>> = LazyLock::new(|| Face::parse(BOLD_DATA, 0).ok());

/// Slant of synthesised italics: tan(12°), the angle of most oblique monospace faces.
const SLANT: f32 = 0.2126;

/// A glyph as drawn: which face, which glyph and whether it is slanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct GlyphKey {
    pub bold: bool,
    pub italic: bool,
    pub id: u16,
}

fn face(bold: bool) -> Option<&'static Face<'static>> {
    if bold { BOLD.as_ref() } else { REGULAR.as_ref() }
}

/// The glyph of `c` in the regular or bold face, if the font has one.
pub(crate) fn glyph(c: char, bold: bool) -> Option<u16> {
    face(bold)?.glyph_index(c).map(|id| id.0)
}

/// Pixels per font unit: every glyph of the font is one cell wide.
fn scale(face: &Face<'_>) -> f32 {
    let advance = face.glyph_index('M').and_then(|id| face.glyph_hor_advance(id)).unwrap_or(600);
    CELL_W / f32::from(advance.max(1))
}

/// Distance from the cell's top to the baseline, centring the font's line box in the cell.
pub(crate) fn baseline() -> f32 {
    let Some(face) = face(false) else { return CELL_H * 0.75 };
    let s = scale(face);
    let ascent = f32::from(face.ascender()) * s;
    let height = f32::from(face.ascender() - face.descender()) * s;
    (CELL_H - height) / 2.0 + ascent
}

/// SVG path data of a glyph with its origin at the cell's top left corner, or `None` for a glyph
/// without ink, such as a space. Baking the baseline in keeps every placement on whole pixels.
pub(crate) fn outline(key: GlyphKey) -> Option<String> {
    let face = face(key.bold)?;
    let s = scale(face);
    // Slanting around the baseline pushes the top right; shifting back by half the x-height keeps
    // the slanted letter centred in its cell.
    let x_height = f32::from(face.x_height().unwrap_or(550));
    let (slant, shift) = if key.italic { (SLANT, -SLANT * x_height / 2.0) } else { (0.0, 0.0) };
    let mut path = PathData { d: String::new(), s, slant, shift, top: baseline() };
    face.outline_glyph(GlyphId(key.id), &mut path)?;
    Some(path.d)
}

/// Collects outline commands as SVG path data in pixels, y pointing down.
struct PathData {
    d: String,
    s: f32,
    slant: f32,
    shift: f32,
    top: f32,
}

impl PathData {
    fn point(&mut self, x: f32, y: f32) {
        let px = (x + self.slant * y + self.shift) * self.s;
        let py = self.top - y * self.s;
        let _ = write!(self.d, "{} {}", num(px), num(py));
    }
}

impl OutlineBuilder for PathData {
    fn move_to(&mut self, x: f32, y: f32) {
        self.d.push('M');
        self.point(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.d.push('L');
        self.point(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.d.push('Q');
        self.point(x1, y1);
        self.d.push(' ');
        self.point(x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.d.push('C');
        self.point(x1, y1);
        self.d.push(' ');
        self.point(x2, y2);
        self.d.push(' ');
        self.point(x, y);
    }

    fn close(&mut self) {
        self.d.push('Z');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_faces_parse_and_are_one_cell_wide() {
        for bold in [false, true] {
            let face = face(bold).expect("embedded face parses");
            let s = scale(face);
            for c in ['M', 'i', 'W', '\u{f07b}'] {
                let id = face.glyph_index(c).expect("glyph present");
                let advance = f32::from(face.glyph_hor_advance(id).expect("advance")) * s;
                assert!((advance - CELL_W).abs() < 0.01, "{c} is {advance} px wide");
            }
        }
    }

    #[test]
    fn baseline_sits_inside_the_cell() {
        let b = baseline();
        assert!(b > CELL_H / 2.0 && b < CELL_H, "baseline {b}");
    }
}
