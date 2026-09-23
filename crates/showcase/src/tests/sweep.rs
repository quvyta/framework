//! One sweep that drives every showcase page through the three conditions the catalogue promises:
//! a narrow terminal, ASCII glyphs and a terminal with a palette (sixteen colours, and text in
//! 256).
//!
//! Every check gathers findings instead of stopping at the first one, so a run names every page
//! and condition that misses the promise at once. Findings that are known and not fixed yet sit in
//! [`EXCEPTIONS`], each saying whether it is a defect or an honest limit and why; a finding that no
//! longer happens fails the sweep as well, so the list shrinks as the components improve and never
//! rots. [`EXCEPTION_COUNT`] fixes the length, so another line cannot slip in unnoticed.
//!
//! The design note that goes with this sweep walks through the list.

use std::collections::BTreeSet;

use qframe::color::Rgb;
use qframe::icons::GlyphMode;
use qframe::runtime::Harness;

use crate::app::{Msg, Showcase};
use crate::pages::PAGES;

/// Sections the narrow sweep draws: the demo, where the components live, and the guide, the one
/// section whose text the shell has to wrap itself.
const NARROW_SECTIONS: [(usize, &str); 2] = [(0, "demo"), (2, "guide")];

/// Narrow widths every page is drawn at: the promised forty columns, and one narrower width that
/// shows what happens past the promise.
const NARROW: [u16; 2] = [40, 30];

/// Rows used for the narrow renders: tall enough that a finding is about the width alone.
const NARROW_HEIGHT: u16 = 44;

/// The checks, named as they appear in a failure.
mod check {
    /// Text cut with `…` where the row beneath it was free, so it could have wrapped instead.
    pub const CUT: &str = "narrow-cut";
    /// A row draws past the right edge of the terminal.
    pub const OVERFLOW: &str = "narrow-overflow";
    /// A section shows almost nothing once the terminal is narrow.
    pub const BLANK: &str = "narrow-blank";
    /// A character outside ASCII drawn on a demo in ASCII glyph mode.
    pub const NON_ASCII: &str = "ascii-non-ascii";
    /// A shape the aesthetics constitution forbids in every mode.
    pub const SHAPE: &str = "ascii-shape";
    /// Text that stops reading against what is behind it once colours are reduced to sixteen.
    pub const CONTRAST: &str = "colour-contrast";
    /// Text that stops reading against what is behind it once colours are reduced to 256.
    pub const CONTRAST_256: &str = "colour-contrast-256";
    /// Background tones that all collapse into one colour once reduced to sixteen.
    pub const FLAT: &str = "colour-flat";
    /// The theme's raised surface tones collapse into the ground once reduced to sixteen, so tab
    /// strips, raised panels and dialogs lose their shape even on a page whose accents keep it
    /// from looking flat.
    pub const SURFACES: &str = "colour-surfaces";
}

/// What an accepted finding is: something the component could do better, or something the
/// condition cannot hold.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A real bug: there is room to behave better and the component does not use it. Belongs on a
    /// work list, not on this one.
    Defect,
    /// An honest limit: the shape cannot fit the width, or the character is the only one that
    /// carries the meaning.
    Limit,
}

/// A finding the sweep tolerates for now.
struct Exception {
    /// The page it belongs to, or `*` when the shell draws it on every page.
    page: &'static str,
    check: &'static str,
    /// Text the finding must contain, so an exception covers one thing and not a whole check.
    detail: &'static str,
    kind: Kind,
    /// Why it is tolerated, in one line.
    reason: &'static str,
}

/// Findings that are known, defects first and limits after. Both kinds are written out in
/// the design note that goes with this sweep; every defect there is work for 0.1.14
/// rather than a permanent resident of this list.
const EXCEPTIONS: &[Exception] = &[
    // Defects: the shell, on every page.
    Exception {
        page: "*",
        check: check::CUT,
        detail: "row 2 `",
        kind: Kind::Defect,
        reason: "the title row runs the page name into the group counter and cuts the result, with a free row under it",
    },
    Exception {
        page: "*",
        check: check::NON_ASCII,
        detail: "draws `·`",
        kind: Kind::Defect,
        reason: "the title row separates the group from the count with `·` in ASCII mode too",
    },
    // Defects: a label cut at forty columns although the row beneath it was free.
    Exception {
        page: "checkbox",
        check: check::CUT,
        detail: "Send anonymous usage da…",
        kind: Kind::Defect,
        reason: "a checkbox label stays on one line instead of wrapping under its mark",
    },
    Exception {
        page: "help-layer",
        check: check::CUT,
        detail: "Show the keys of this s…",
        kind: Kind::Defect,
        reason: "the help layer's own description does not wrap",
    },
    Exception {
        page: "sparkline",
        check: check::CUT,
        detail: "Press a column or drag ac…",
        kind: Kind::Defect,
        reason: "the sentence under the sparkline does not wrap",
    },
    Exception {
        page: "heatmap",
        check: check::CUT,
        detail: "Click a day, or move with…",
        kind: Kind::Defect,
        reason: "the sentence under the heatmap does not wrap",
    },
    Exception {
        page: "text-area",
        check: check::CUT,
        detail: "What changed in this …",
        kind: Kind::Defect,
        reason: "the text area's placeholder is cut although the field is several rows tall",
    },
    Exception {
        page: "card-grid",
        check: check::CUT,
        detail: "Plain text notes tha…",
        kind: Kind::Defect,
        reason: "a card's body is cut although the card has room for a second line",
    },
    Exception {
        page: "file-manager",
        check: check::CUT,
        detail: "Keep operations inside …",
        kind: Kind::Defect,
        reason: "the sentence under the file manager does not wrap",
    },
    Exception {
        page: "clipboard",
        check: check::CUT,
        detail: "Paste into the note…",
        kind: Kind::Defect,
        reason: "the step's sentence does not wrap",
    },
    // Defect: text the showcase itself writes.
    Exception {
        page: "file-picker",
        check: check::NON_ASCII,
        detail: "draws `↑` in `↑↓ move",
        kind: Kind::Defect,
        reason: "the demo's key sentence writes the arrow keys as `↑↓` in ASCII mode too; the title row's `…` hid it until the cut got an ASCII mark",
    },
    Exception {
        page: "storage",
        check: check::SHAPE,
        detail: "(tests an",
        kind: Kind::Defect,
        reason: "the demo's own label puts an aside in brackets, where a second line would do",
    },
    // Limits: a shape that cannot fit the width.
    Exception {
        page: "bar-chart",
        check: check::CUT,
        detail: "M… T… W… T… F… S… S…",
        kind: Kind::Limit,
        reason: "seven day labels across forty columns leave one letter each; wrapping would break the axis",
    },
    Exception {
        page: "confirm",
        check: check::CUT,
        detail: "● postgres",
        kind: Kind::Limit,
        reason: "a row of the list the dialog is about, at thirty columns; wrapping would break the column beside it",
    },
    // Limits: the brackets are the content the page is showing.
    Exception {
        page: "layout",
        check: check::SHAPE,
        detail: "Cells(14)",
        kind: Kind::Limit,
        reason: "the layout demo names the lengths it uses, and those are written as Rust calls",
    },
    Exception {
        page: "markdown",
        check: check::SHAPE,
        detail: "[style.",
        kind: Kind::Limit,
        reason: "the markdown demo shows a document whose text has brackets in it",
    },
    Exception {
        page: "code-view",
        check: check::SHAPE,
        detail: "#[derive",
        kind: Kind::Limit,
        reason: "the code view shows Rust source, where brackets are the code",
    },
    Exception {
        page: "document",
        check: check::SHAPE,
        detail: "[engine]",
        kind: Kind::Limit,
        reason: "the data file demo shows a settings file, whose sections are written in brackets",
    },
    // Limit: a glyph that is the control itself.
    Exception {
        page: "spinner",
        check: check::SHAPE,
        detail: "| arc",
        kind: Kind::Limit,
        reason: "the ASCII spinner turns through `|`, which is the spinner rather than a separator",
    },
];

/// How many exceptions the list is allowed to hold. Adding one means changing this number, which
/// makes it a decision rather than an accident.
const EXCEPTION_COUNT: usize = 19;

/// One missed promise.
struct Finding {
    page: &'static str,
    check: &'static str,
    /// The render it was seen in, such as `demo at 40 columns`.
    when: String,
    detail: String,
}

/// Collects findings, keeping the first one per page, check and render, so one broken row neither
/// buries the rest of the sweep nor hides the same component failing in another condition.
#[derive(Default)]
struct Findings {
    seen: BTreeSet<(&'static str, &'static str, String)>,
    items: Vec<Finding>,
}

impl Findings {
    fn add(&mut self, page: &'static str, check: &'static str, when: &str, detail: String) {
        if self.seen.insert((page, check, when.to_owned())) {
            self.items.push(Finding { page, check, when: when.to_owned(), detail });
        }
    }
}

impl Exception {
    /// Whether this exception is the one a finding describes.
    fn covers(&self, finding: &Finding) -> bool {
        (self.page == "*" || self.page == finding.page)
            && self.check == finding.check
            && finding.detail.contains(self.detail)
    }
}

/// What a sixteen-colour terminal shows for text in `fg` on `bg`, on a screen whose ground is
/// `ground`: the same reduction the framework applies to a sixteen-colour frame.
fn reduced_pair(fg: Rgb, bg: Rgb, ground: Rgb) -> (Rgb, Rgb) {
    (Rgb::from_ansi16(fg.to_ansi16_text(bg, ground)), Rgb::from_ansi16(bg.to_ansi16_on(ground)))
}

/// What a 256-colour terminal shows for text in `fg` on `bg`: the same reduction the framework
/// applies to a 256-colour frame.
fn reduced_pair_256(fg: Rgb, bg: Rgb) -> (Rgb, Rgb) {
    (Rgb::from_ansi256(fg.to_ansi256_text(bg)), Rgb::from_ansi256(bg.to_ansi256()))
}

/// Contrast text keeps against its background after the reduction, measured with the same WCAG
/// ratio the theme's readability check uses. That check asks 4.5 of true-colour body text, which
/// no reduction to sixteen colours survives, so here the bar is the weaker "still a different
/// colour to the eye"; below it a glyph starts to disappear into what is behind it.
const MIN_CONTRAST: f64 = 1.6;

/// Whether a glyph is text rather than a fill. Blocks, shades and rails carry their meaning in
/// their colour, so they are drawn in the colour behind them on purpose and say nothing about
/// readability.
fn is_text(glyph: char) -> bool {
    glyph.is_alphanumeric() || (glyph.is_ascii_punctuation() && glyph != '_')
}

/// One showcase, drawn at `width` × `height` in `mode`, for the whole sweep to walk its pages
/// with: building a showcase per page would spend the run in setup rather than in rendering.
fn showcase(width: u16, height: u16, mode: GlyphMode) -> Harness<Showcase> {
    let mut harness = Harness::with_env(Showcase::new(), super::env(), width, height);
    harness.set_locale("en").set_glyph_mode(mode);
    harness
}

/// Shows `page`, with its transition finished.
fn open(harness: &mut Harness<Showcase>, page: &str) {
    harness.send(Msg::Open(page.to_owned()));
    harness.advance(std::time::Duration::from_secs(1));
}

/// Shows `section` of the page the harness is on, with its transition finished.
fn section(harness: &mut Harness<Showcase>, section: usize) -> String {
    harness.send(Msg::Section(section));
    harness.advance(std::time::Duration::from_secs(1));
    harness.screen()
}

/// Whether the row beneath a cut line is free over the columns the cut text uses, which means the
/// text had somewhere to wrap to. The last drawn row is not counted: below it is the space the
/// shell keeps, not room the component owns.
fn row_below_is_free(lines: &[&str], row: usize, from_column: usize) -> bool {
    let Some(below) = lines.get(row + 1) else { return false };
    below.chars().count() <= from_column && lines[row + 2..].iter().any(|line| !line.trim().is_empty())
}

/// Whether a cut line is running prose rather than a row of a grid.
///
/// Prose is what could have wrapped; a list row, a table row or a control with its value on the
/// right is a single line by design, and wrapping it would break the column it belongs to. The
/// difference a screen shows is the gap: prose separates its words with one space, while columns
/// are held apart by several. A cut in the middle of a line is a column too — the text after it
/// is the next column.
fn cut_prose(line: &str) -> bool {
    let text = line.trim();
    text.ends_with('…') && !text.contains("   ") && text.split_whitespace().count() >= 3
}

/// Narrow widths: nothing cut where it could wrap, nothing past the right edge, nothing blank.
fn sweep_narrow(found: &mut Findings) {
    for width in NARROW {
        let mut harness = showcase(width, NARROW_HEIGHT, GlyphMode::Unicode);
        for page in PAGES {
            open(&mut harness, page.id);
            for (index, name) in NARROW_SECTIONS {
                let when = format!("{name} at {width} columns");
                let screen = section(&mut harness, index);
                let lines: Vec<&str> = screen.lines().collect();
                for (row, line) in lines.iter().enumerate() {
                    let drawn = qframe::text::width(line);
                    if drawn > width {
                        found.add(page.id, check::OVERFLOW, &when, format!("row {row} draws {drawn} cells"));
                    }
                    let Some(byte) = line.find('…') else { continue };
                    // Only the demo: a guide shows authored prose the shell wraps itself, and the
                    // code blocks in it are lines that must not wrap.
                    if index == 0 && cut_prose(line) && row_below_is_free(&lines, row, line[..byte].chars().count()) {
                        found.add(
                            page.id,
                            check::CUT,
                            &when,
                            format!("row {row} `{}` is cut with a free row below", line.trim()),
                        );
                    }
                }
                let filled = lines.iter().filter(|line| !line.trim().is_empty()).count();
                if filled < 4 {
                    found.add(page.id, check::BLANK, &when, format!("only {filled} rows carry anything"));
                }
            }
        }
    }
}

/// Shapes the aesthetics constitution forbids in every mode, ASCII included.
fn forbidden_shape(line: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    for (index, c) in chars.iter().enumerate() {
        if "[](){}".contains(*c) {
            return Some(format!("`{c}` wraps text"));
        }
        if *c == '|' {
            let before = index.checked_sub(1).and_then(|i| chars.get(i)).copied().unwrap_or(' ');
            let after = chars.get(index + 1).copied().unwrap_or(' ');
            if !before.is_alphanumeric() && !after.is_alphanumeric() {
                return Some("`|` separates".to_owned());
            }
        }
    }
    ["===", "-->", "<--", "+--", "--+"].iter().find(|d| line.contains(**d)).map(|d| format!("`{d}` decorates"))
}

/// ASCII glyph mode on the demo of every page: nothing outside ASCII, and none of the forbidden
/// shapes. Only the demo, because the other sections show authored prose and Rust source, where a
/// bracket or a dash is the content rather than a drawn shape.
fn sweep_ascii(found: &mut Findings) {
    for width in [super::SIZE.0, NARROW[0]] {
        let mut harness = showcase(width, super::SIZE.1, GlyphMode::Ascii);
        for page in PAGES {
            open(&mut harness, page.id);
            let when = format!("demo at {width} columns");
            let screen = section(&mut harness, 0);
            for (row, line) in screen.lines().enumerate() {
                if let Some(bad) = line.chars().find(|c| !c.is_ascii()) {
                    found.add(
                        page.id,
                        check::NON_ASCII,
                        &when,
                        format!("row {row} draws `{bad}` in `{}`", line.trim()),
                    );
                }
                if let Some(shape) = forbidden_shape(line) {
                    found.add(page.id, check::SHAPE, &when, format!("row {row} {shape} in `{}`", line.trim()));
                }
            }
        }
    }
}

/// Theme tones a page lifts off the ground: raised surfaces, the selected surface and floating
/// ones.
const LIFTED: [&str; 3] = ["raised", "active", "overlay"];

/// Sixteen colours: text on a demo keeps reading, and the tones a page leans on stay apart; text
/// keeps reading in 256 colours too. The modal page is drawn a second time with its dialog open,
/// so the dimmed page behind a layer is held to the same bar.
fn sweep_reduced_colours(found: &mut Findings) {
    let (width, height) = super::SIZE;
    let mut harness = showcase(width, height, GlyphMode::Unicode);
    for page in PAGES {
        open(&mut harness, page.id);
        section(&mut harness, 0);
        read_reduced_colours(&harness, found, page.id, "demo");
    }
    open(&mut harness, "modal");
    section(&mut harness, 0);
    harness.click_text("Rename project").advance(std::time::Duration::from_secs(1));
    read_reduced_colours(&harness, found, "modal", "demo with its dialog open");
}

/// Reads one screen as a sixteen-colour terminal would show it, and its text as a 256-colour one
/// would.
fn read_reduced_colours(harness: &Harness<Showcase>, found: &mut Findings, page: &'static str, when: &str) {
    let (width, height) = (harness.buffer().area.width, harness.buffer().area.height);
    let theme = harness.env().theme();
    let ground = theme.color("canvas").expect("every theme has a canvas");
    let lifted: Vec<Rgb> = LIFTED.iter().filter_map(|token| theme.color(token)).collect();
    let mut grounds: BTreeSet<[u8; 3]> = BTreeSet::new();
    let mut reduced_grounds: BTreeSet<u8> = BTreeSet::new();
    let mut surfaces: BTreeSet<u8> = BTreeSet::new();
    let mut worst: Option<(f64, char, u16, u16)> = None;
    let mut worst_256: Option<(f64, char, u16, u16)> = None;
    for y in 0..height {
        for x in 0..width {
            let Some(bg) = harness.bg(x, y) else { continue };
            grounds.insert([bg.r, bg.g, bg.b]);
            reduced_grounds.insert(bg.to_ansi16_on(ground));
            if lifted.contains(&bg) {
                surfaces.insert(bg.to_ansi16_on(ground));
            }
            let glyph = harness.buffer()[(x, y)].symbol().chars().next().filter(|c| is_text(*c));
            let (Some(glyph), Some(fg)) = (glyph, harness.fg(x, y)) else { continue };
            let (shown_fg, shown_bg) = reduced_pair_256(fg, bg);
            let ratio = shown_fg.contrast_ratio(shown_bg);
            if worst_256.is_none_or(|(low, ..)| ratio < low) {
                worst_256 = Some((ratio, glyph, x, y));
            }
            let (fg, bg) = reduced_pair(fg, bg, ground);
            let ratio = fg.contrast_ratio(bg);
            if worst.is_none_or(|(low, ..)| ratio < low) {
                worst = Some((ratio, glyph, x, y));
            }
        }
    }
    if let Some((ratio, glyph, x, y)) = worst.filter(|(ratio, ..)| *ratio < MIN_CONTRAST) {
        found.add(page, check::CONTRAST, when, format!("`{glyph}` at {x},{y} keeps only {ratio:.2}:1"));
    }
    if let Some((ratio, glyph, x, y)) = worst_256.filter(|(ratio, ..)| *ratio < MIN_CONTRAST) {
        found.add(page, check::CONTRAST_256, when, format!("`{glyph}` at {x},{y} keeps only {ratio:.2}:1"));
    }
    if grounds.len() >= 3 && reduced_grounds.len() < 2 {
        found.add(
            page,
            check::FLAT,
            when,
            format!("{} background tones collapse into one of the sixteen", grounds.len()),
        );
    }
    if surfaces.len() == 1 && surfaces.contains(&ground.to_ansi16_on(ground)) {
        found.add(page, check::SURFACES, when, "the raised surfaces fall onto the ground's colour".to_owned());
    }
}

/// Every page, at forty columns and narrower, in ASCII glyphs and in reduced colours: what the
/// catalogue promises of every component, checked in one place. What is not kept yet is in
/// [`EXCEPTIONS`] with its reason, so the debt is counted rather than forgotten.
#[test]
fn every_page_keeps_the_catalogue_promise() {
    assert_eq!(EXCEPTIONS.len(), EXCEPTION_COUNT, "the exception list changed length; say so on purpose");

    let mut found = Findings::default();
    sweep_narrow(&mut found);
    sweep_ascii(&mut found);
    sweep_reduced_colours(&mut found);

    let mut report = String::new();
    for finding in &found.items {
        if !EXCEPTIONS.iter().any(|allowed| allowed.covers(finding)) {
            report
                .push_str(&format!("\n  {} · {} · {}: {}", finding.page, finding.check, finding.when, finding.detail));
        }
    }
    for allowed in EXCEPTIONS {
        if !found.items.iter().any(|finding| allowed.covers(finding)) {
            let kind = if allowed.kind == Kind::Defect { "defect" } else { "limit" };
            report.push_str(&format!(
                "\n  {} · {}: the {kind} `{}` is gone, so drop the exception ({})",
                allowed.page, allowed.check, allowed.detail, allowed.reason
            ));
        }
    }
    assert!(report.is_empty(), "the catalogue's promise is not kept:{report}\n");
}
