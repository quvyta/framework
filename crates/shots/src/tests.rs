//! Tests of the whole picture: small scenes, drawn and checked as text and as PNG bytes.

use qframe::color::Rgb;
use qframe::prelude::*;

use crate::Shot;
use crate::screen::{Cell, Palette, Screen};

const CANVAS: Rgb = Rgb::new(0x0e, 0x0f, 0x18);
const TEXT: Rgb = Rgb::new(0xee, 0xf2, 0xff);
const ACCENT: Rgb = Rgb::new(0x81, 0x8c, 0xf8);
const RAISED: Rgb = Rgb::new(0x20, 0x23, 0x36);

fn palette() -> Palette {
    Palette { ground: CANVAS, text: TEXT, muted: Rgb::new(0x67, 0x6b, 0xa6), title_ground: RAISED }
}

fn cell(symbol: &str) -> Cell {
    Cell {
        symbol: symbol.to_owned(),
        width: qframe::text::width(symbol).max(1),
        fg: TEXT,
        bg: CANVAS,
        bold: false,
        italic: false,
        underline: false,
    }
}

/// A one-row screen from `cells`, padded with spaces to `width`.
fn screen(width: u16, cells: Vec<Cell>) -> Screen {
    let mut cells = cells;
    cells.resize(usize::from(width), cell(" "));
    Screen { width, height: 1, cells, palette: palette() }
}

fn shot(screen: Screen) -> Shot {
    Shot { screen, title: None }
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

struct Scene;

impl App for Scene {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Text::new("Quvyta draws").bold());
        ui.add(Text::new("shape by tone").color("accent"));
    }
}

#[test]
fn the_same_scene_gives_identical_files() {
    let first = Shot::of(&Harness::new(Scene, 24, 3)).title("qframe");
    let second = Shot::of(&Harness::new(Scene, 24, 3)).title("qframe");
    assert_eq!(first.to_svg(), second.to_svg());
    assert_eq!(first.to_png().expect("renders"), second.to_png().expect("renders"));
}

#[test]
fn only_the_glyphs_used_are_embedded() {
    let svg = shot(screen(6, vec![cell("a"), cell("b"), cell("b"), cell("a")])).to_svg();
    assert_eq!(count(&svg, "<path id="), 2, "{svg}");
    assert_eq!(count(&svg, "<use "), 4);
    // Spaces have no outline and are neither defined nor placed.
    let blank = shot(screen(4, vec![])).to_svg();
    assert_eq!(count(&blank, "<path id="), 0);
}

#[test]
fn bold_and_italic_are_separate_outlines() {
    let styled = |bold, italic| Cell { bold, italic, ..cell("a") };
    let svg = shot(screen(4, vec![styled(false, false), styled(true, false), styled(false, true), styled(true, true)]))
        .to_svg();
    let outlines: Vec<&str> = svg
        .lines()
        .filter(|line| line.starts_with("<path id="))
        .map(|line| line.split(" d=").nth(1).unwrap_or(""))
        .collect();
    assert_eq!(outlines.len(), 4);
    for (i, a) in outlines.iter().enumerate() {
        for b in &outlines[i + 1..] {
            assert_ne!(a, b, "every style has its own outline");
        }
    }
}

#[test]
fn wide_characters_take_two_cells() {
    // The Nerd Font has no CJK: the character is reported, and what follows it still lands two
    // cells on.
    let wide = Cell { width: 2, ..cell("中") };
    let rest = Cell { symbol: String::new(), width: 0, ..cell(" ") };
    let shot = shot(screen(5, vec![wide, rest, cell("x")]));
    let svg = shot.to_svg();
    assert_eq!(shot.missing(), vec!['中']);
    assert!(svg.contains("x=\"34\" y=\"16\""), "x sits in the third cell:\n{svg}");
}

#[test]
fn a_known_wide_glyph_is_centred_in_its_two_cells() {
    let wide = Cell { width: 2, ..cell("→") };
    let svg = shot(screen(3, vec![wide])).to_svg();
    assert!(svg.contains("x=\"20.5\""), "centred between 16 and 34:\n{svg}");
}

#[test]
fn no_frame_border_or_stroke_is_drawn() {
    let svg = Shot::of(&Harness::new(Scene, 24, 3)).title("qframe").to_svg();
    assert_eq!(count(&svg, "<rect"), 1, "only the rounded ground is a rect:\n{svg}");
    for banned in ["stroke", "<line", "<polyline", "filter", "shadow", "<image", "<script", "<text", "href=\"http"] {
        assert!(!svg.contains(banned), "`{banned}` in:\n{svg}");
    }
}

#[test]
fn block_elements_fill_exact_parts_of_the_cell() {
    let pillar = Cell { fg: ACCENT, ..cell("▌") };
    let svg = shot(screen(2, vec![pillar, cell("█")])).to_svg();
    assert!(svg.contains(&format!("<path fill=\"{ACCENT}\" d=\"M16 16h4.5v20h-4.5z\"/>")), "{svg}");
    assert!(svg.contains(&format!("<path fill=\"{TEXT}\" d=\"M25 16h9v20h-9z\"/>")), "{svg}");
    assert_eq!(count(&svg, "<use "), 0);
}

#[test]
fn backgrounds_merge_into_one_path_per_colour() {
    let raised = |s| Cell { bg: RAISED, ..cell(s) };
    let svg = shot(screen(6, vec![raised("a"), raised("b"), cell("c"), raised("d")])).to_svg();
    assert!(svg.contains(&format!("<path fill=\"{RAISED}\" d=\"M16 16h18v20h-18zM43 16h9v20h-9z\"/>")), "{svg}");
}

#[test]
fn the_title_sits_in_a_strip_above_the_grid() {
    let plain = Shot::of(&Harness::new(Scene, 24, 3));
    let titled = plain.clone().title("qframe");
    let height = |svg: &str| svg.split("height=\"").nth(1).and_then(|rest| rest.split('"').next()).map(str::to_owned);
    assert_eq!(height(&plain.to_svg()).as_deref(), Some("92"));
    assert_eq!(height(&titled.to_svg()).as_deref(), Some("124"));
}

#[test]
fn the_png_is_twice_the_size() {
    let png = shot(screen(4, vec![cell("q")])).to_png().expect("renders");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_eq!((width, height), (2 * (4 * 9 + 32), 2 * (20 + 32)));
}

#[test]
fn save_appends_both_extensions() {
    let path = std::path::Path::new("docs/screenshots/home.v2");
    assert_eq!(crate::with_extension(path, "svg"), std::path::Path::new("docs/screenshots/home.v2.svg"));
}

#[test]
fn the_ground_continues_the_screens_edge() {
    // A 4 x 3 screen: the edge is raised but for one cell, the middle is canvas.
    let mut cells: Vec<Cell> = (0..12).map(|_| Cell { bg: RAISED, ..cell(" ") }).collect();
    for i in [3, 5, 6] {
        cells[i].bg = CANVAS;
    }
    assert_eq!(crate::screen::edge_color(&cells, 4, 3), Some(RAISED));
    assert_eq!(crate::screen::edge_color(&cells[..1], 1, 1), Some(RAISED));
    assert_eq!(crate::screen::edge_color(&[], 0, 0), None);
}

#[test]
fn a_bar_and_the_block_that_ends_it_are_one_shape() {
    let filled = |s| Cell { bg: ACCENT, ..cell(s) };
    let end = Cell { fg: ACCENT, bg: RAISED, ..cell("▌") };
    let svg = shot(screen(4, vec![filled(" "), filled(" "), end])).to_svg();
    assert!(svg.contains(&format!("<path fill=\"{ACCENT}\" d=\"M16 16h18v20h-18zM34 16h4.5v20h-4.5z\"/>")), "{svg}");
    // The block's other half keeps its own background, without overlapping the inked half.
    assert!(svg.contains(&format!("<path fill=\"{RAISED}\" d=\"M38.5 16h4.5v20h-4.5z\"/>")), "{svg}");
}

#[test]
fn quadrants_split_the_cell_without_overlap() {
    let tiles = crate::svg::tiles(&[[0.0, 0.0, 0.5, 0.5], [0.5, 0.5, 1.0, 1.0]]);
    let area: f32 = tiles.iter().map(|([l, t, r, b], _)| (r - l) * (b - t)).sum();
    assert!((area - 1.0).abs() < f32::EPSILON);
    assert_eq!(tiles.iter().filter(|(_, inked)| *inked).count(), 2);
}
