//! The embedded fonts: glyph lookup and outlines turned into SVG path data.

use std::fmt::Write as _;
use std::sync::LazyLock;

use ttf_parser::{Face, GlyphId, OutlineBuilder};

use crate::geometry::{CELL_H, CELL_W, num};

/// JetBrains Mono Nerd Font Mono, Regular. Licence and checksums: `fonts/README.md`.
static REGULAR_DATA: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFontMono-Regular.ttf");
/// JetBrains Mono Nerd Font Mono, Bold.
static BOLD_DATA: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFontMono-Bold.ttf");
/// Noto Sans Mono CJK SC, Regular, cut down to the kana, CJK punctuation, full-width forms and
/// the ideographs of GB 2312 and JIS X 0208; how it was cut is in `fonts/README.md`.
static CJK_DATA: &[u8] = include_bytes!("../fonts/NotoSansMonoCJKsc-Regular-subset.otf");

/// The parsed faces. A face that fails to parse stays `None`, and every character is then
/// reported missing instead of the program stopping.
static REGULAR: LazyLock<Option<Face<'static>>> = LazyLock::new(|| Face::parse(REGULAR_DATA, 0).ok());
static BOLD: LazyLock<Option<Face<'static>>> = LazyLock::new(|| Face::parse(BOLD_DATA, 0).ok());
static CJK: LazyLock<Option<Face<'static>>> = LazyLock::new(|| Face::parse(CJK_DATA, 0).ok());

/// Which embedded face a glyph comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Source {
    Regular,
    Bold,
    /// The fallback for Chinese and Japanese, which JetBrains Mono does not cover. It has no bold:
    /// a bold cell draws it regular, as a terminal without a bold CJK font does.
    Cjk,
}

/// Slant of synthesised italics: tan(12°), the angle of most oblique monospace faces.
const SLANT: f32 = 0.2126;

/// A glyph as drawn: which face, which glyph and whether it is slanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct GlyphKey {
    pub source: Source,
    pub italic: bool,
    pub id: u16,
}

fn face(source: Source) -> Option<&'static Face<'static>> {
    match source {
        Source::Regular => REGULAR.as_ref(),
        Source::Bold => BOLD.as_ref(),
        Source::Cjk => CJK.as_ref(),
    }
}

/// The glyph of `c`, upright, and its advance in pixels. A bold cell whose character only the
/// regular face has is drawn regular, as terminals do, and a character JetBrains Mono lacks
/// comes from the CJK face.
pub(crate) fn glyph(c: char, bold: bool) -> Option<(GlyphKey, f32)> {
    let sources: &[Source] =
        if bold { &[Source::Bold, Source::Regular, Source::Cjk] } else { &[Source::Regular, Source::Cjk] };
    sources.iter().find_map(|&source| {
        let face = face(source)?;
        let id = face.glyph_index(c)?;
        let advance = f32::from(face.glyph_hor_advance(id).unwrap_or(0)) * scale(face);
        Some((GlyphKey { source, italic: false, id: id.0 }, advance))
    })
}

/// Pixels per font unit. JetBrains Mono's advance is exactly one cell; every face is drawn at the
/// same size per em, so a CJK glyph stands as tall as it would beside Latin text in a terminal
/// and is centred in its two cells rather than stretched to fill them.
fn scale(face: &Face<'_>) -> f32 {
    let Some(latin) = REGULAR.as_ref() else { return CELL_W / 600.0 };
    let advance = latin.glyph_index('M').and_then(|id| latin.glyph_hor_advance(id)).unwrap_or(600);
    let per_em = CELL_W / f32::from(advance.max(1)) * f32::from(latin.units_per_em());
    per_em / f32::from(face.units_per_em().max(1))
}

/// Distance from the cell's top to the baseline, centring the font's line box in the cell.
pub(crate) fn baseline() -> f32 {
    let Some(face) = face(Source::Regular) else { return CELL_H * 0.75 };
    let s = scale(face);
    let ascent = f32::from(face.ascender()) * s;
    let height = f32::from(face.ascender() - face.descender()) * s;
    (CELL_H - height) / 2.0 + ascent
}

/// SVG path data of a glyph with its origin at the cell's top left corner, or `None` for a glyph
/// without ink, such as a space. Baking the baseline in keeps every placement on whole pixels.
/// Every face shares the Latin baseline, so CJK and Latin text stand on one line.
pub(crate) fn outline(key: GlyphKey) -> Option<String> {
    let face = face(key.source)?;
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
            for c in ['M', 'i', 'W', '\u{f07b}'] {
                let (key, advance) = glyph(c, bold).expect("glyph present");
                assert_eq!(key.source, if bold { Source::Bold } else { Source::Regular });
                assert!((advance - CELL_W).abs() < 0.01, "{c} is {advance} px wide");
            }
        }
    }

    #[test]
    fn chinese_and_japanese_come_from_the_cjk_face_and_fit_two_cells() {
        for c in ['今', 'の', 'セ', '缓', '墙', '、', '。', '「', 'ー'] {
            for bold in [false, true] {
                let (key, advance) = glyph(c, bold).unwrap_or_else(|| panic!("{c} has a glyph"));
                assert_eq!(key.source, Source::Cjk, "{c}");
                assert!(advance > CELL_W && advance <= 2.0 * CELL_W, "{c} is {advance} px wide");
            }
        }
        assert_eq!(glyph('a', false).map(|(key, _)| key.source), Some(Source::Regular), "Latin stays JetBrains Mono");
    }

    #[test]
    fn baseline_sits_inside_the_cell() {
        let b = baseline();
        assert!(b > CELL_H / 2.0 && b < CELL_H, "baseline {b}");
    }
}
