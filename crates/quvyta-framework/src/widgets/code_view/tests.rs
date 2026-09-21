//! Tests for the code view.

use super::*;
use crate::color::Rgb;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::ScrollView;

struct Demo {
    copies: u32,
}

impl App for Demo {
    type Msg = ();
    fn update(&mut self, _: ()) -> Command<()> {
        self.copies += 1;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        let code = "fn main() {\n    println!(\"a fairly long line that wraps\");\n}\n";
        ui.add(CodeView::new(code, Language::Rust).on_copy(())).fill();
    }
}

#[test]
fn numbers_colours_and_wraps() {
    let h = Harness::new(Demo { copies: 0 }, 36, 7);
    let screen = h.screen();
    assert_eq!(
        screen,
        "\n  1  fn main() {\n  2      println!(\"a fairly long l\n       ine that wraps\");\n  3  }\n\n\n"
    );
    let keyword = h.env().theme().style("code-token", Some("keyword"), &[]).paint("fg");
    assert!(keyword.is_some());
    let (x, y) = h.find("fn").map(|(x, y)| (x as u16, y as u16)).unwrap_or_default();
    assert_eq!(h.fg(x, y), h.env().theme().color("accent"));
}

struct Script;

impl App for Script {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(CodeView::new("cd \"$srcdir\" # in", Language::Shell)).fill();
    }
}

#[test]
fn colours_shell_variables() {
    let h = Harness::new(Script, 30, 3);
    let theme = h.env().theme();
    let at = |text: &str| h.find(text).map(|(x, y)| (x as u16, y as u16)).unwrap_or_default();
    let (x, y) = at("$srcdir");
    assert_eq!(h.fg(x, y), theme.style("code-token", Some("variable"), &[]).paint("fg").map(|p| p.at(0.0)));
    assert_ne!(h.fg(x, y), theme.style("code-token", Some("string"), &[]).paint("fg").map(|p| p.at(0.0)));
    let (x, y) = at("# in");
    assert_eq!(h.fg(x, y), theme.style("code-token", Some("comment"), &[]).paint("fg").map(|p| p.at(0.0)));
}

#[test]
fn copies_on_c() {
    let mut h = Harness::new(Demo { copies: 0 }, 40, 6);
    h.press("tab").press("c");
    assert_eq!(h.app().copies, 1);
    assert!(h.copied()[0].starts_with("fn main()"));
}

/// A code view with every kind of line look, for the tests below.
struct Review {
    code: String,
    marks: Vec<LineMark>,
    highlights: Vec<(std::ops::RangeInclusive<usize>, LineTone)>,
}

impl App for Review {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        let mut view = CodeView::new(self.code.clone(), Language::Shell).line_marks(self.marks.iter().copied());
        for (lines, tone) in &self.highlights {
            view = view.highlight_lines(lines.clone(), *tone);
        }
        ui.add(view).fill();
    }
}

fn review(marks: Vec<LineMark>, highlights: Vec<(std::ops::RangeInclusive<usize>, LineTone)>) -> Harness<Review> {
    let code = "same=1\nadded=2\nremoved=3\nlast=4".to_owned();
    Harness::new(Review { code, marks, highlights }, 40, 6)
}

fn style_color(h: &Harness<Review>, widget: &str, variant: Option<&str>, key: &str) -> Option<Rgb> {
    h.env().theme().style(widget, variant, &[]).paint(key).map(|paint| paint.at(0.0))
}

fn cell(h: &Harness<Review>, text: &str) -> (u16, u16) {
    let (x, y) = h.find(text).unwrap_or_else(|| panic!("{text:?} on screen:\n{}", h.screen()));
    (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0))
}

#[test]
fn diff_marks_sign_and_tint_their_lines_in_every_glyph_mode() {
    use crate::icons::GlyphMode;
    let mut h = review(vec![LineMark::Unchanged, LineMark::Added, LineMark::Removed], Vec::new());
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        h.set_glyph_mode(mode);
        let screen = h.screen();
        for banned in ['[', ']', '(', ')', '{', '}', '|'] {
            assert!(!screen.contains(banned), "{mode:?}: {screen}");
        }
        let added = h.env().icons().glyph("line-added").into_owned();
        let removed = h.env().icons().glyph("line-removed").into_owned();
        let lines: Vec<&str> = screen.lines().collect();
        assert!(
            lines[1].starts_with("    1  same"),
            "{mode:?}: an unchanged line keeps the sign column blank: {screen}"
        );
        assert!(lines[2].starts_with(&format!("  {added} 2  added")), "{mode:?}: {screen}");
        // The removed line is the old file's second: the added line above it is in the new file
        // only, so it takes no number from the old one.
        assert!(lines[3].starts_with(&format!("  {removed} 2  removed")), "{mode:?}: {screen}");
        if mode == GlyphMode::Ascii {
            assert_eq!((added.as_str(), removed.as_str()), ("+", "-"));
        }
        let ground = style_color(&h, "code", None, "bg");
        let (x, y) = cell(&h, "same");
        assert_eq!(h.bg(x, y), ground, "unchanged lines keep the ground");
        let (x, y) = cell(&h, "added");
        assert_eq!(h.bg(x, y), style_color(&h, "code-line", Some("added"), "bg"));
        assert_eq!(h.bg(0, y), h.bg(x, y), "the tint runs across the whole row");
        assert_eq!(h.fg(2, y), style_color(&h, "code-line", Some("added"), "fg"), "the sign takes the status colour");
        let (x, y) = cell(&h, "removed");
        assert_eq!(h.bg(x, y), style_color(&h, "code-line", Some("removed"), "bg"));
        assert_ne!(h.bg(x, y), ground);
    }
}

#[test]
fn highlighted_lines_have_their_own_tone_and_sign() {
    let h = review(Vec::new(), vec![(2..=2, LineTone::Warning), (4..=9, LineTone::Accent)]);
    let ground = style_color(&h, "code", None, "bg");
    let selection = style_color(&h, "text-selection", None, "bg");
    let (x, y) = cell(&h, "added");
    let warning = h.bg(x, y);
    assert_eq!(warning, style_color(&h, "code-line", Some("warning"), "bg"));
    assert_ne!(warning, ground);
    assert_ne!(warning, selection);
    let sign = h.env().icons().glyph("warning").into_owned();
    assert!(h.screen().lines().nth(2).is_some_and(|line| line.starts_with(&format!("  {sign} 2"))), "{}", h.screen());
    let (x, y) = cell(&h, "last");
    let accent = h.bg(x, y);
    assert_eq!(
        accent,
        style_color(&h, "code-line", Some("accent"), "bg"),
        "a range past the end stops at the last line"
    );
    assert!(accent != ground && accent != selection && accent != warning);
    assert_eq!(h.fg(2, y), style_color(&h, "code-line", Some("accent"), "fg"), "the accent tone carries the pillar");
    let (x, y) = cell(&h, "same");
    assert_eq!(h.bg(x, y), ground);
}

#[test]
fn a_highlight_wins_over_a_diff_mark_and_tints_every_wrapped_row() {
    let code = "short\nthis line is long enough to wrap onto more rows\nend".to_owned();
    let marks = vec![LineMark::Unchanged, LineMark::Added];
    let h = Harness::new(Review { code, marks, highlights: vec![(2..=2, LineTone::Warning)] }, 26, 8);
    let tone = style_color(&h, "code-line", Some("warning"), "bg");
    let (_, first) = cell(&h, "this");
    let rows: Vec<u16> = (first..first + 3).filter(|y| h.bg(20, *y) == tone).collect();
    assert!(rows.len() >= 2, "every visual row of the line is tinted: {rows:?}\n{}", h.screen());
    let (_, end) = cell(&h, "end");
    assert_ne!(h.bg(10, end), tone, "the next line is not");
}

#[test]
fn marked_and_highlighted_lines_stay_readable_in_every_theme() {
    use crate::theme::ThemeRegistry;
    let registry = ThemeRegistry::builtin();
    let tokens = [
        "keyword",
        "type",
        "function",
        "macro",
        "string",
        "number",
        "attribute",
        "lifetime",
        "punctuation",
        "table",
        "key",
        "variable",
        "plain",
    ];
    for (id, _) in registry.list() {
        let theme = registry.resolve(&id).theme.expect("resolves");
        let color = |widget: &str, variant: Option<&str>, key: &str| {
            theme.style(widget, variant, &[]).paint(key).map(|paint| paint.at(0.0)).expect("defined")
        };
        let ground = color("code", None, "bg");
        for line in ["added", "removed", "warning", "accent"] {
            let bg = color("code-line", Some(line), "bg");
            assert!(bg.perceptual_distance(ground) >= 0.03, "theme {id}: {line} lines stand out from the ground");
            let sign = color("code-line", Some(line), "fg");
            assert!(sign.contrast_ratio(bg) >= 3.0, "theme {id}: the {line} sign reads on its line");
            for token in tokens {
                let fg = color("code-token", Some(token), "fg");
                let ratio = fg.contrast_ratio(bg);
                assert!(ratio >= 4.5, "theme {id}: {token} on a {line} line has contrast {ratio:.2}");
            }
            for faint in [color("code-token", Some("comment"), "fg"), color("code-line-number", None, "fg")] {
                let ratio = faint.contrast_ratio(bg);
                assert!(ratio >= 2.2, "theme {id}: faint text on a {line} line has contrast {ratio:.2}");
            }
        }
    }
}

/// Two hundred numbered lines in a scroll view, revealing one of them.
struct Long {
    reveal: Option<usize>,
}

impl App for Long {
    type Msg = Option<usize>;
    fn update(&mut self, reveal: Option<usize>) -> Command<Option<usize>> {
        self.reveal = reveal;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Option<usize>>) {
        let code: Vec<String> = (1..=200).map(|n| format!("echo line-{n}")).collect();
        ui.add_with(ScrollView::new(), |ui| {
            let mut view = CodeView::new(code.join("\n"), Language::Shell);
            if let Some(line) = self.reveal {
                view = view.reveal(line);
            }
            ui.add(view).fill_width();
        })
        .fill();
    }
}

fn shows(h: &Harness<Long>, line: usize) -> bool {
    h.screen().lines().any(|row| row.trim_end().ends_with(&format!("line-{line}")))
}

#[test]
fn reveal_brings_a_line_into_the_scroll_view_at_once_with_reduced_motion() {
    let mut h = Harness::new(Long { reveal: None }, 40, 10);
    h.set_reduced_motion(true);
    assert!(shows(&h, 1) && !shows(&h, 150));
    h.send(Some(150));
    assert!(shows(&h, 150), "{}", h.screen());
    assert!(shows(&h, 152), "a little context below the line: {}", h.screen());
    // Focusing the code view, taller than the scroll view, does not throw the line away.
    h.press("tab");
    assert!(shows(&h, 150), "{}", h.screen());
    h.press("tab");
    assert!(shows(&h, 150), "{}", h.screen());
    // Revealing happens when the line changes; the user may scroll away from it.
    h.press("shift+tab").press("home");
    assert!(shows(&h, 1) && !shows(&h, 150), "{}", h.screen());
    h.send(Some(20));
    assert!(shows(&h, 20), "{}", h.screen());
    h.send(Some(150));
    assert!(shows(&h, 150), "{}", h.screen());
}

#[test]
fn reveal_glides_with_motion() {
    let mut h = Harness::new(Long { reveal: None }, 40, 10);
    h.send(Some(150));
    assert!(!shows(&h, 150), "the view moves over a few frames: {}", h.screen());
    h.advance(std::time::Duration::from_millis(40));
    assert!(!shows(&h, 1) && !shows(&h, 150), "on its way: {}", h.screen());
    h.advance(std::time::Duration::from_secs(1));
    assert!(shows(&h, 150), "{}", h.screen());
}

/// A diff of two versions of a file, with the numbers each line carries in the file it came from.
struct Diff {
    marks: Vec<LineMark>,
    numbers: Option<Vec<Option<usize>>>,
    reveal: Option<usize>,
}

impl Default for Diff {
    fn default() -> Self {
        Self { marks: DIFF_MARKS.to_vec(), numbers: None, reveal: None }
    }
}

/// The lines of the diff below: one kept, two taken out, one put in, one kept.
const DIFF_MARKS: [LineMark; 5] =
    [LineMark::Unchanged, LineMark::Removed, LineMark::Removed, LineMark::Added, LineMark::Unchanged];

const DIFF_CODE: &str = "keep=a\ngone=b\ngone=c\nnew=d\nkeep=e";

impl App for Diff {
    type Msg = Option<usize>;
    fn update(&mut self, reveal: Option<usize>) -> Command<Option<usize>> {
        self.reveal = reveal;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Option<usize>>) {
        let mut view = CodeView::new(DIFF_CODE, Language::Shell).line_marks(self.marks.iter().copied());
        if let Some(numbers) = &self.numbers {
            view = view.line_numbers_from(numbers.iter().copied());
        }
        if let Some(number) = self.reveal {
            view = view.reveal_number(number);
        }
        ui.add(view).fill();
    }
}

/// The number column of each code row, in order, with `None` where the column is blank.
fn gutter_numbers(screen: &str) -> Vec<Option<usize>> {
    screen
        .lines()
        .filter(|line| line.contains('='))
        .map(|line| line.split_whitespace().find_map(|word| word.parse().ok()))
        .collect()
}

#[test]
fn the_numbers_of_a_diff_are_the_numbers_of_the_files_the_lines_came_from() {
    let h = Harness::new(Diff::default(), 40, 7);
    // The old file numbers the lines it lost 2 and 3; the new file numbers what it gained 2, and
    // the line that stayed is its line 3. Counting the text from the top would say 1 2 3 4 5.
    assert_eq!(gutter_numbers(&h.screen()), [Some(1), Some(2), Some(3), Some(2), Some(3)], "{}", h.screen());
}

#[test]
fn numbers_given_outright_are_the_ones_drawn() {
    let numbers = vec![Some(120), None, Some(121), Some(122), Some(123)];
    let h = Harness::new(Diff { numbers: Some(numbers), ..Diff::default() }, 40, 7);
    assert_eq!(
        gutter_numbers(&h.screen()),
        [Some(120), None, Some(121), Some(122), Some(123)],
        "a hunk far into the file keeps its own numbers, and a line without one is blank:\n{}",
        h.screen()
    );
}

/// A long diff in a scroll view: a hundred lines the old file lost, then two hundred it kept, so
/// a line's place in the text is nowhere near the number its file gives it.
struct LongDiff {
    reveal: Option<usize>,
}

impl App for LongDiff {
    type Msg = Option<usize>;
    fn update(&mut self, reveal: Option<usize>) -> Command<Option<usize>> {
        self.reveal = reveal;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Option<usize>>) {
        let gone = (1..=100).map(|n| format!("echo old-{n}"));
        let kept = (1..=200).map(|n| format!("echo new-{n}"));
        let code: Vec<String> = gone.chain(kept).collect();
        let marks = std::iter::repeat_n(LineMark::Removed, 100).chain(std::iter::repeat_n(LineMark::Unchanged, 200));
        ui.add_with(ScrollView::new(), |ui| {
            let mut view = CodeView::new(code.join("\n"), Language::Shell).line_marks(marks);
            if let Some(number) = self.reveal {
                view = view.reveal_number(number);
            }
            ui.add(view).fill_width();
        })
        .fill();
    }
}

fn shows_line(h: &Harness<LongDiff>, text: &str) -> bool {
    h.screen().lines().any(|row| row.trim_end().ends_with(text))
}

#[test]
fn revealing_a_number_finds_the_line_the_new_file_numbers_that_way() {
    let mut h = Harness::new(LongDiff { reveal: None }, 40, 10);
    h.set_reduced_motion(true);
    assert!(shows_line(&h, "old-1") && !shows_line(&h, "new-50"));
    // Two lines are numbered 50: one the old file lost and one the new file keeps. A finding that
    // says `file:50` means the file as it is now, so the kept line is the one to go to.
    h.send(Some(50));
    assert!(shows_line(&h, "new-50"), "{}", h.screen());
    assert!(!shows_line(&h, "old-50"), "the old file's 50 is left alone: {}", h.screen());
    // A number only the new file has is found all the same.
    h.send(Some(150));
    assert!(shows_line(&h, "new-150"), "{}", h.screen());
    // Counting the text from the top would have gone to line 150, which is `new-50`.
    assert!(!shows_line(&h, "new-50"), "{}", h.screen());
}

#[test]
fn revealing_a_number_no_line_carries_moves_nothing() {
    let mut h = Harness::new(LongDiff { reveal: None }, 40, 10);
    h.set_reduced_motion(true);
    h.send(Some(4000));
    assert!(shows_line(&h, "old-1"), "the view stays where it was: {}", h.screen());
}

#[test]
fn without_marks_or_numbers_the_lines_are_counted_from_the_top_as_before() {
    let h = Harness::new(Diff { marks: Vec::new(), ..Diff::default() }, 40, 7);
    assert_eq!(gutter_numbers(&h.screen()), [Some(1), Some(2), Some(3), Some(4), Some(5)], "{}", h.screen());
}

#[test]
fn a_comment_that_spans_lines_colours_every_line_it_covers() {
    // The layout walks the tokens once for the whole file rather than once per line, and a
    // comment, a string or an attribute can reach across several lines. If the walk stepped past
    // such a token when it finished a line, every later line of it would lose its colour.
    let code = "let a = 1;\n/* one\n   two\n   three */\nlet b = 2;\n";
    let rows = code_rows(code, Language::Rust, 80);
    let coloured = |row: &CodeRow| row.pieces.iter().all(|(_, token)| *token == Token::Comment);
    assert!(coloured(&rows[1]), "{:?}", rows[1]);
    assert!(coloured(&rows[2]), "{:?}", rows[2]);
    assert!(coloured(&rows[3]), "{:?}", rows[3]);
    assert!(rows[4].pieces.iter().any(|(_, token)| *token == Token::Keyword), "{:?}", rows[4]);
}

#[test]
fn a_long_file_costs_its_size_rather_than_its_square() {
    // Laying a line out by reading the whole file's tokens costs lines × tokens, so a file twice
    // as long takes four times as long and a megabyte of code never finishes: the code view of a
    // real source file freezes and does not come back. The walk is one pass, so the cost grows
    // with the size of the file.
    //
    // The bound is deliberately far from the measurement — eight thousand lines take about a
    // sixth of a second here, so a machine twenty times slower still passes — because a bound
    // tight enough to measure anything would only cry wolf under load. It is finite because the
    // shape it guards against is not slowness but a freeze: taking the one pass away puts these
    // eight thousand lines back at seven to nine seconds, and a real file is far longer.
    let code = "    let value = compute(other, 1234);\n".repeat(8_000);
    let started = std::time::Instant::now();
    let rows = code_rows(&code, Language::Rust, 120);
    let spent = started.elapsed();
    assert_eq!(rows.len(), 8_000);
    assert!(spent < std::time::Duration::from_secs(3), "laying out eight thousand lines took {spent:?}");
}

#[test]
fn sources_are_shared_by_code_and_language_and_the_cache_stays_bounded() {
    let first = CodeView::<()>::new("fn main() {}", Language::Rust);
    let again = CodeView::<()>::new("fn main() {}", Language::Rust);
    assert!(Arc::ptr_eq(&first.source, &again.source), "the same code is laid out once");
    let other = CodeView::<()>::new("fn main() {}", Language::Plain);
    assert!(!Arc::ptr_eq(&first.source, &other.source), "another language colours it differently");
    for n in 0..CACHED_SOURCES * 3 {
        let _ = CodeView::<()>::new(format!("let release = {n};"), Language::Rust);
    }
    assert_eq!(SOURCES.with_borrow(Vec::len), CACHED_SOURCES);
    let fresh = CodeView::<()>::new("fn main() {}", Language::Rust);
    assert!(!Arc::ptr_eq(&first.source, &fresh.source), "the oldest sources are forgotten");
    for width in 10..20 {
        let _ = first.source.layout(width);
    }
    assert_eq!(first.source.layouts.lock().map(|layouts| layouts.len()).ok(), Some(CACHED_LAYOUTS));
}

/// A file of twenty thousand lines, about a megabyte, in a scroll view, the way a viewer shows
/// one.
struct Megabyte {
    code: String,
}

impl App for Megabyte {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add_with(ScrollView::new(), |ui| {
            ui.add(CodeView::new(self.code.as_str(), Language::Rust)).fill_width();
        })
        .fill();
    }
}

#[test]
fn scrolling_a_long_file_costs_a_screenful_rather_than_the_file() {
    // A view is built anew every frame and a scroll view measures its content more than once, so
    // a code view that colours and wraps its source whenever it is asked does all of that for a
    // whole megabyte several times on every key: each step down a real source file then takes a
    // fifth of a second in a release build and seconds in this one. Remembering the layout and
    // drawing only the rows on screen makes a step cost what the screen shows.
    //
    // The bound is far from the measurement — the forty steps below take about a sixteenth of a
    // second here even in a debug build, so a machine eighty times slower still passes — because
    // a loaded machine must not cry wolf. It is finite because it guards both halves: drawing
    // every row of the file rather than the screenful puts these steps at about ten seconds, and
    // laying the file out again on every measure puts them near a minute.
    let code: String = (0..20_000)
        .map(|n| match n % 4 {
            0 => format!("/// Answers the request numbered {n} of the batch.\n"),
            1 => format!("pub fn answer_{n}(request: &Request) -> Result<Reply, Error> {{\n"),
            2 => format!("    Ok(Reply::new(request.field(\"name-{n}\")?, {n}u32))\n"),
            _ => "}\n".to_owned(),
        })
        .collect();
    let mut h = Harness::new(Megabyte { code }, 120, 40);
    h.press("tab");
    let started = std::time::Instant::now();
    for step in 0..40 {
        h.press(if step % 2 == 0 { "down" } else { "up" });
    }
    let spent = started.elapsed();
    assert!(h.screen().contains("answer_1("), "{}", h.screen());
    assert!(spent < std::time::Duration::from_secs(5), "forty steps through a megabyte of code took {spent:?}");
}
