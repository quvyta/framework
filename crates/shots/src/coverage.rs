//! What the embedded fonts cover, fixed in a file and checked against what the framework draws.
//!
//! Two questions are asked here at every commit, the same question [`crate::Shot::missing`]
//! answers for one picture: which characters do the fonts have a glyph for (`fonts/coverage.txt`,
//! so a new or re-cut font file shows as a diff in a list rather than as a box in a picture
//! someone happens to draw), and can every character the framework itself can put on screen be
//! drawn?

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use ttf_parser::Face;

use crate::font::{self, Source};

/// The list of covered characters, kept beside the fonts.
const LIST: &str = "fonts/coverage.txt";

/// Set when the fonts changed on purpose and the list is to be written instead of checked.
const WRITE: &str = "QUVYTA_FONT_COVERAGE";

/// Header of the list, so whoever reads the diff knows what they are looking at.
const HEADER: &str = "\
# Every character the embedded fonts have a glyph for, one section per face and one range per
# line. Written by the `covered_characters_are_the_ones_listed` test of quvyta-framework-shots:
# after a font file is added or re-cut, run the tests once with QUVYTA_FONT_COVERAGE=write and
# read the diff. A range that disappears here is a picture that would have drawn a box.
";

/// A file of this crate, by its path in the source tree.
fn own(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

/// Cells of the list a line may hold: long enough to read, short enough that a changed font moves
/// only the lines it touches.
const LINE: usize = 90;

/// Every character `face` maps to a glyph, as `U+XXXX` and `U+XXXX-U+XXXX` ranges, a few to a line.
fn ranges(face: &Face<'_>) -> String {
    // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
    // results are dropped for that reason and no other.
    let mut codes = BTreeSet::new();
    for table in face.tables().cmap.into_iter().flat_map(|cmap| cmap.subtables) {
        table.codepoints(|code| {
            if table.glyph_index(code).is_some() {
                codes.insert(code);
            }
        });
    }
    let mut out = String::new();
    let mut line = String::new();
    let mut codes = codes.into_iter().peekable();
    while let Some(first) = codes.next() {
        let mut last = first;
        while codes.peek() == Some(&(last + 1)) {
            last = codes.next().unwrap_or(last);
        }
        let range = if first == last { format!("U+{first:04X}") } else { format!("U+{first:04X}-U+{last:04X}") };
        if !line.is_empty() && line.len() + 2 + range.len() > LINE {
            let _ = writeln!(out, "{line}");
            line.clear();
        }
        if !line.is_empty() {
            line.push_str(", ");
        }
        line.push_str(&range);
    }
    if !line.is_empty() {
        let _ = writeln!(out, "{line}");
    }
    out
}

/// The list as the embedded fonts say it is now.
fn coverage() -> String {
    // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
    // results are dropped for that reason and no other.
    let mut out = HEADER.to_owned();
    for (name, source) in
        [("latin", Source::Regular), ("latin-bold", Source::Bold), ("symbols", Source::Symbols), ("cjk", Source::Cjk)]
    {
        let face = font::face(source).expect("face parses");
        let _ = write!(out, "\n[{name}] {}\n{}", font::file_name(source), ranges(face));
    }
    out
}

#[test]
fn covered_characters_are_the_ones_listed() {
    let path = own(LIST);
    let now = coverage();
    if std::env::var_os(WRITE).is_some() {
        std::fs::write(&path, &now).expect("write the list");
        return;
    }
    let listed = std::fs::read_to_string(&path).expect("read the list");
    assert!(
        listed == now,
        "the embedded fonts no longer cover what {LIST} lists; \
         run the tests once with {WRITE}=write and read the diff"
    );
}

/// Every character in the framework's own asset files: the texts of all nine locales, the icon
/// sets, the themes and the key hints. Reading the files is the whole question: a character that
/// reaches a screen reaches it from one of them.
fn characters_of_the_framework() -> BTreeSet<char> {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../quvyta-framework/assets");
    let mut chars = BTreeSet::new();
    for folder in ["locales", "icons", "themes", "keymaps"] {
        let folder = assets.join(folder);
        let files = std::fs::read_dir(&folder).unwrap_or_else(|e| panic!("read {}: {e}", folder.display()));
        for file in files {
            let path = file.expect("a file of the folder").path();
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            chars.extend(text.chars().filter(|c| !c.is_control()));
        }
    }
    assert!(chars.len() > 300, "only {} characters found; are the asset files there?", chars.len());
    chars
}

#[test]
fn every_character_the_framework_can_draw_has_a_glyph() {
    // The distinct characters of every locale, icon set and theme are a few hundred, ideographs
    // among them, so each one is looked up rather than whole Unicode ranges guessed at.
    let missing: Vec<String> = characters_of_the_framework()
        .into_iter()
        .filter(|&c| font::glyph(c, false).is_none() || font::glyph(c, true).is_none())
        .map(|c| format!("U+{:04X} {c}", u32::from(c)))
        .collect();
    assert!(missing.is_empty(), "no embedded font can draw: {}", missing.join(", "));
}
