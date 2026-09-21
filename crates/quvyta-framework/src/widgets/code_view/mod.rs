//! Highlighted code.

use std::cell::RefCell;
use std::ops::{Bound, Range, RangeBounds};
use std::sync::{Arc, Mutex, PoisonError};

use unicode_segmentation::UnicodeSegmentation;

use super::cells;
use super::highlight::{Language, Token, highlight};
use crate::event::Event;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// One visual row of code: a line number on the first row of a source line, and coloured pieces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeRow {
    /// The source line the row shows part of, counted from 1.
    pub(crate) line: usize,
    pub(crate) number: Option<usize>,
    pub(crate) pieces: Vec<(String, Token)>,
}

/// Lays `code` out in rows no wider than `width` cells, wrapping long lines. Continuation
/// rows are indented by two cells.
pub(crate) fn code_rows(code: &str, language: Language, width: u16) -> Vec<CodeRow> {
    let tokens = highlight(code, language);
    let mut rows = Vec::new();
    let mut line_start = 0;
    // The tokens cover the text from start to end without gaps or overlaps, so walking them for
    // every line would read the whole file once per line and cost the square of its size: a
    // megabyte then takes minutes rather than milliseconds. `first` is the first token that
    // still reaches this line, and it only ever moves forward. It is not moved inside the loop
    // below, because a token can span several lines and the next line needs it again.
    let mut first = 0;
    for (index, line) in code.split('\n').enumerate() {
        let line_end = line_start + line.len();
        while first < tokens.len() && tokens[first].0.end <= line_start {
            first += 1;
        }
        let mut row = CodeRow { line: index + 1, number: Some(index + 1), pieces: Vec::new() };
        let mut used = 0u16;
        for (range, token) in tokens[first..].iter().take_while(|(range, _)| range.start < line_end) {
            let start = range.start.max(line_start);
            let end = range.end.min(line_end);
            if start >= end {
                continue;
            }
            for grapheme in code[start..end].graphemes(true) {
                let cell = if grapheme == "\t" { "    " } else { grapheme };
                let w = text::width(cell);
                if used.saturating_add(w) > width && used > 0 {
                    rows.push(std::mem::replace(
                        &mut row,
                        CodeRow { line: index + 1, number: None, pieces: vec![("  ".to_owned(), Token::Plain)] },
                    ));
                    used = 2;
                }
                match row.pieces.last_mut() {
                    Some((piece, last)) if last == token => piece.push_str(cell),
                    _ => row.pieces.push((cell.to_owned(), *token)),
                }
                used = used.saturating_add(w);
            }
        }
        rows.push(row);
        line_start = line_end + 1;
    }
    if code.ends_with('\n') {
        rows.pop();
    }
    rows
}

/// Paints `rows` in `area` using the `code-token.<kind>` and `code-line-number` styles.
pub(crate) fn paint_rows(cx: &mut PaintCx<'_>, area: Rect, rows: &[CodeRow], gutter: u16) {
    let visible = visible_rows(cx, area, rows.len());
    for (y, row) in rows.iter().enumerate().skip(visible.start).take(visible.len()) {
        let Ok(y) = u16::try_from(y) else { break };
        if y >= area.height {
            break;
        }
        let row_y = area.y + i32::from(y);
        // Line numbers help reading, not copying: clean copies leave the gutter out.
        if gutter > 0 {
            cx.decoration(Rect::new(area.x, row_y, gutter, 1));
        }
        if gutter > 0
            && let Some(number) = row.number
        {
            let style = cx.style("code-line-number", None, &[]).text();
            let label = format!("{number:>width$}", width = usize::from(gutter - 2));
            cx.text(area.x, row_y, &label, style, gutter);
        }
        let mut x = area.x + i32::from(gutter);
        for (piece, token) in &row.pieces {
            let style = cx.style("code-token", Some(token.variant()), &[]).text();
            x += i32::from(cx.text(x, row_y, piece, style, area.right().saturating_sub(x).try_into().unwrap_or(0)));
        }
    }
}

/// The indices of the `count` rows laid from the top of `area` that fall inside the visible area.
/// A scroll view hands its content the whole height of the file and clips it to the screen, so a
/// long file would otherwise style and draw every row it has only to have all but a screenful
/// thrown away.
fn visible_rows(cx: &PaintCx<'_>, area: Rect, count: usize) -> Range<usize> {
    let clip = cx.clip();
    let top = clip.y.max(area.y);
    let bottom = clip.bottom().min(area.bottom()).max(top);
    let row = |y: i32| usize::try_from(y - area.y).unwrap_or(0).min(count);
    row(top)..row(bottom)
}

/// Marks the padding of a code block at `rect` around `inner` as decoration, so clean copies of
/// a selection across the block keep only the code.
pub(crate) fn padding_decoration(cx: &mut PaintCx<'_>, rect: Rect, inner: Rect) {
    cx.decoration(Rect::new(rect.x, rect.y, rect.width, clamp_u16(inner.y - rect.y)));
    cx.decoration(Rect::new(rect.x, inner.bottom(), rect.width, clamp_u16(rect.bottom() - inner.bottom())));
    cx.decoration(Rect::new(rect.x, inner.y, clamp_u16(inner.x - rect.x), inner.height));
    cx.decoration(Rect::new(inner.right(), inner.y, clamp_u16(rect.right() - inner.right()), inner.height));
}

/// Width of the line number column for `code`, including two cells of spacing.
pub(crate) fn gutter_width(code: &str) -> u16 {
    gutter_for(code.split('\n').count())
}

/// Width of the line number column for code of `lines` lines, including two cells of spacing.
fn gutter_for(lines: usize) -> u16 {
    text::width(&lines.to_string()).saturating_add(2)
}

/// How a line of a diff changed; see [`CodeView::line_marks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineMark {
    /// The line is in both versions; drawn as any other line.
    #[default]
    Unchanged,
    /// The line is new: a success tint and a `+` sign.
    Added,
    /// The line is gone: a danger tint and a `−` sign.
    Removed,
}

/// The tone of highlighted lines; see [`CodeView::highlight_lines`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineTone {
    /// A line to look at, such as the one a "go to line" reached: an accent tint and the pillar.
    #[default]
    Accent,
    /// A line worth a careful look, such as a finding of a review: a warning tint and the
    /// warning icon.
    Warning,
}

impl LineTone {
    fn variant(self) -> &'static str {
        match self {
            Self::Accent => "accent",
            Self::Warning => "warning",
        }
    }
}

/// The sign of a marked or highlighted line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sign {
    /// An icon on the first row of the line.
    Icon(&'static str),
    /// The theme's pillar on every row of the line.
    Pillar,
}

/// Rows of context kept around a revealed line, so it does not land on the very edge of the
/// scroll view.
const REVEAL_CONTEXT: u16 = 2;

/// What a code view remembers between frames.
#[derive(Debug, Default)]
struct CodeMemory {
    /// The line last revealed, so the view scrolls to a line once and then lets the user move.
    revealed: Option<usize>,
}

/// How many sources a thread remembers. A screen shows a handful of code views at once, and a
/// guide page in the showcase shows a dozen short ones beside its demo.
const CACHED_SOURCES: usize = 16;

/// How many widths a source remembers its layout for. A scroll view may measure its content with
/// and without room for its scrollbar before it paints.
const CACHED_LAYOUTS: usize = 4;

thread_local! {
    /// The sources laid out last on this thread, most recently used first.
    static SOURCES: RefCell<Vec<Arc<Source>>> = const { RefCell::new(Vec::new()) };
}

/// Code in a language, with the layouts computed for it.
struct Source {
    code: String,
    language: Language,
    /// The number of source lines, counted once because the gutter's width follows it.
    lines: usize,
    /// Layouts at recent widths, most recently used first.
    layouts: Mutex<Vec<Arc<Layout>>>,
}

/// Code laid out at one width.
struct Layout {
    width: u16,
    rows: Vec<CodeRow>,
    /// Cells taken by the widest row.
    widest: u16,
}

impl Source {
    /// The source for `code` in `language`: the remembered one when this thread showed the same
    /// code recently, otherwise a fresh one that is remembered in place of the oldest.
    ///
    /// A view is built anew every frame and a scroll view measures its content more than once,
    /// so without this a megabyte of code is coloured and wrapped several times a frame.
    fn cached(code: String, language: Language) -> Arc<Self> {
        SOURCES.with_borrow_mut(|sources| {
            let source = match sources.iter().position(|source| source.language == language && source.code == code) {
                Some(index) => sources.remove(index),
                None => {
                    let lines = code.split('\n').count();
                    Arc::new(Self { code, language, lines, layouts: Mutex::new(Vec::new()) })
                }
            };
            sources.insert(0, Arc::clone(&source));
            sources.truncate(CACHED_SOURCES);
            source
        })
    }

    /// The code laid out `width` cells wide; computed once per width and remembered.
    fn layout(&self, width: u16) -> Arc<Layout> {
        let mut layouts = self.layouts.lock().unwrap_or_else(PoisonError::into_inner);
        let layout = match layouts.iter().position(|layout| layout.width == width) {
            Some(index) => layouts.remove(index),
            None => {
                let rows = code_rows(&self.code, self.language, width);
                let widest = rows
                    .iter()
                    .map(|row| cells::sum(row.pieces.iter().map(|(piece, _)| text::width(piece))))
                    .max()
                    .unwrap_or(0);
                Arc::new(Layout { width, rows, widest })
            }
        };
        layouts.insert(0, Arc::clone(&layout));
        layouts.truncate(CACHED_LAYOUTS);
        layout
    }
}

/// Code with syntax colours, line numbers and wrapping of long lines.
///
/// Building one in `view` every frame is cheap: the last few sources shown on a thread are
/// remembered with how they were laid out at the last few widths, and only the rows on screen
/// are drawn, so an unchanged file is neither coloured nor wrapped again however long it is.
///
/// While focused, `c` copies the code to the clipboard and flashes. The code is a text selection
/// region: a mouse drag selects inside it (turn it off with
/// [`NodeMut::selectable`](crate::widget::NodeMut::selectable)), and clean copies leave out the
/// line numbers and the signs of marked lines.
///
/// For reviews, [`line_marks`](Self::line_marks) shows a diff and
/// [`highlight_lines`](Self::highlight_lines) tints lines to look at; either adds a sign column
/// at the left edge. [`reveal`](Self::reveal) scrolls the enclosing
/// [`ScrollView`](crate::widgets::ScrollView) to a line.
///
/// Style keys: `code` (`bg`, `padding`) with `focus` and `pressed`; `code-line-number`;
/// `code-token.<kind>` where kind is `keyword`, `type`, `function`, `macro`, `string`,
/// `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `variable` or
/// `plain`; `code-line.<look>` (`bg` for the line, `fg` for its sign) where look is `added`,
/// `removed`, `accent` or `warning`. Icons: `line-added`, `line-removed`, `warning` and the
/// pillar.
pub struct CodeView<Msg> {
    source: Arc<Source>,
    line_numbers: bool,
    marks: Vec<LineMark>,
    /// The number each source line carries, when the caller gave them outright.
    numbers: Option<Vec<Option<usize>>>,
    highlights: Vec<(usize, usize, LineTone)>,
    reveal: Option<usize>,
    reveal_number: Option<usize>,
    on_copy: Option<Msg>,
}

impl<Msg: 'static> CodeView<Msg> {
    /// Shows `code` in `language`, reusing its layout when this thread showed the same code a
    /// moment ago.
    #[must_use]
    pub fn new(code: impl Into<String>, language: Language) -> Self {
        Self {
            source: Source::cached(code.into(), language),
            line_numbers: true,
            marks: Vec::new(),
            numbers: None,
            highlights: Vec::new(),
            reveal: None,
            reveal_number: None,
            on_copy: None,
        }
    }

    /// Marks lines as a diff: the first mark belongs to line 1, the next to line 2, and lines
    /// past the last mark are unchanged. Added lines are tinted with the success colour and
    /// signed `+`, removed ones with the danger colour and `−`, in the sign column at the left
    /// edge.
    ///
    /// The line numbers then follow the files rather than the text: a diff puts the lines of two
    /// versions one after another, so counting from the top would number neither file. A removed
    /// line carries the old file's number, an added line the new file's, and a line in both
    /// carries the new file's, which is the one a finding such as `PKGBUILD:22` means. Give
    /// [`line_numbers_from`](Self::line_numbers_from) instead when the diff starts part way into
    /// the file, and reach a line by its number with [`reveal_number`](Self::reveal_number).
    #[must_use]
    pub fn line_marks(mut self, marks: impl IntoIterator<Item = LineMark>) -> Self {
        self.marks = marks.into_iter().collect();
        self
    }

    /// Tints `lines` (counted from 1, like the line numbers) in `tone`, over any diff mark, and
    /// puts the tone's sign in the sign column. Call it again for more lines; where ranges meet,
    /// the later call wins. The tint is separate from a text selection, which draws over it.
    #[must_use]
    pub fn highlight_lines(mut self, lines: impl RangeBounds<usize>, tone: LineTone) -> Self {
        let first = match lines.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n.saturating_add(1),
            Bound::Unbounded => 1,
        };
        let last = match lines.end_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n.saturating_sub(1),
            Bound::Unbounded => usize::MAX,
        };
        self.highlights.push((first.max(1), last, tone));
        self
    }

    /// Scrolls the enclosing [`ScrollView`](crate::widgets::ScrollView) just enough to show
    /// `line` (counted from 1; past the end, the last line) with two rows of context, gliding
    /// there unless motion is reduced. It happens when the revealed line changes, so the user
    /// can scroll away afterwards; outside a scroll view it does nothing.
    #[must_use]
    pub fn reveal(mut self, line: usize) -> Self {
        self.reveal = Some(line);
        self
    }

    /// Shows or hides line numbers; shown by default.
    #[must_use]
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Gives each line its own number outright: the first number belongs to the first line of the
    /// code, and `None` leaves that line's column blank, as a hunk header has no number of its
    /// own. Lines past the last number are blank too.
    ///
    /// This is for a diff that starts part way into a file, where nothing in the text says the
    /// hunk began at line 120. A whole-file diff needs only [`line_marks`](Self::line_marks),
    /// which numbers the lines from the marks. Numbers given here win over that.
    #[must_use]
    pub fn line_numbers_from(mut self, numbers: impl IntoIterator<Item = Option<usize>>) -> Self {
        self.numbers = Some(numbers.into_iter().collect());
        self
    }

    /// Scrolls to the line whose number is `number`, the way [`reveal`](Self::reveal) scrolls to
    /// a line of the text. In a diff the two are not the same line, so this is what a finding
    /// that names a file and a line asks for.
    ///
    /// Two lines can carry one number — the line a version lost and the line that took its place.
    /// The line the new file numbers that way is the one reached, because that is the file a
    /// finding is about; a number only a removed line carries reaches that line. A number no line
    /// carries scrolls nowhere. Given as well as [`reveal`](Self::reveal), this wins.
    #[must_use]
    pub fn reveal_number(mut self, number: usize) -> Self {
        self.reveal_number = Some(number);
        self
    }

    /// The number each source line is drawn with: the ones given outright, else the numbers the
    /// diff marks imply, else the line's own place in the text.
    fn numbers(&self) -> Vec<Option<usize>> {
        let lines = self.source.lines;
        if let Some(given) = &self.numbers {
            return (0..lines).map(|index| given.get(index).copied().flatten()).collect();
        }
        if self.marks.is_empty() {
            return (1..=lines).map(Some).collect();
        }
        let (mut old, mut new) = (0, 0);
        (0..lines)
            .map(|index| match self.marks.get(index) {
                Some(LineMark::Removed) => {
                    old += 1;
                    Some(old)
                }
                Some(LineMark::Added) => {
                    new += 1;
                    Some(new)
                }
                Some(LineMark::Unchanged) | None => {
                    old += 1;
                    new += 1;
                    Some(new)
                }
            })
            .collect()
    }

    /// The source line `number` names, preferring the line the new file numbers that way over one
    /// the old file lost.
    fn line_of_number(&self, number: usize) -> Option<usize> {
        let numbers = self.numbers();
        let carries = |index: &usize| numbers.get(*index).copied().flatten() == Some(number);
        let kept = |index: &usize| !matches!(self.marks.get(*index), Some(LineMark::Removed));
        let index = (0..numbers.len())
            .find(|index| carries(index) && kept(index))
            .or_else(|| (0..numbers.len()).find(carries))?;
        Some(index + 1)
    }

    /// Message sent after the code was copied with `c`.
    #[must_use]
    pub fn on_copy(mut self, message: Msg) -> Self {
        self.on_copy = Some(message);
        self
    }

    fn gutter(&self) -> u16 {
        if !self.line_numbers {
            return 0;
        }
        if self.numbers.is_none() && self.marks.is_empty() {
            return gutter_for(self.source.lines);
        }
        // A diff's numbers are the files' own, which can be wider than the count of lines shown.
        let widest = self.numbers().into_iter().flatten().max().unwrap_or(1);
        text::width(&widest.to_string()).saturating_add(2)
    }

    /// Width of the sign column: a sign and a space when lines are marked or highlighted.
    fn signs(&self) -> u16 {
        if self.marks.is_empty() && self.highlights.is_empty() { 0 } else { 2 }
    }

    /// How `line` is drawn: the `code-line` variant of its tint and its sign, if any.
    fn look(&self, line: usize) -> Option<(&'static str, Sign)> {
        if let Some((_, _, tone)) =
            self.highlights.iter().rev().find(|(first, last, _)| (*first..=*last).contains(&line))
        {
            let sign = match tone {
                LineTone::Accent => Sign::Pillar,
                LineTone::Warning => Sign::Icon("warning"),
            };
            return Some((tone.variant(), sign));
        }
        match self.marks.get(line.checked_sub(1)?) {
            Some(LineMark::Added) => Some(("added", Sign::Icon("line-added"))),
            Some(LineMark::Removed) => Some(("removed", Sign::Icon("line-removed"))),
            Some(LineMark::Unchanged) | None => None,
        }
    }

    /// `rows` with each first row of a source line carrying the number that line is drawn with,
    /// when that is not its place in the text.
    fn renumbered(&self, rows: &[CodeRow]) -> Option<Vec<CodeRow>> {
        if self.numbers.is_none() && self.marks.is_empty() {
            return None;
        }
        let numbers = self.numbers();
        let mut rows = rows.to_vec();
        for row in &mut rows {
            if row.number.is_some() {
                row.number = row.line.checked_sub(1).and_then(|index| numbers.get(index).copied().flatten());
            }
        }
        Some(rows)
    }

    /// Tints marked and highlighted rows across `area` and draws their signs at `x`.
    fn paint_looks(&self, cx: &mut PaintCx<'_>, area: Rect, x: i32, top: i32, rows: &[CodeRow]) {
        let visible = visible_rows(cx, Rect::new(area.x, top, area.width, clamp_u16(area.bottom() - top)), rows.len());
        for (index, row) in rows.iter().enumerate().skip(visible.start).take(visible.len()) {
            let Some((variant, sign)) = self.look(row.line) else { continue };
            // A wrapped line signs only its first row, and a line without a number of its own —
            // a hunk header — still signs.
            let first_row = index == 0 || rows[index - 1].line != row.line;
            let y = top + i32::try_from(index).unwrap_or(i32::MAX);
            let style = cx.style("code-line", Some(variant), &[]);
            if let Some(bg) = style.color("bg") {
                cx.fill(Rect::new(area.x, y, area.width, 1), bg);
            }
            let color = style.color("fg").unwrap_or_else(|| cx.color("text"));
            match sign {
                Sign::Pillar => cx.pillar(x, y, color),
                Sign::Icon(icon) if first_row => {
                    let glyph = cx.env().icons().glyph(icon).into_owned();
                    cx.text(x, y, &glyph, CellStyle::fg(color), 1);
                }
                Sign::Icon(_) => {}
            }
        }
    }

    /// Asks the enclosing scroll view to show the revealed line, once per line.
    fn request_reveal(&self, cx: &mut PaintCx<'_>, area: Rect, top: i32, rows: &[CodeRow]) {
        let asked = match self.reveal_number {
            Some(number) => self.line_of_number(number),
            None => self.reveal,
        };
        let wanted = asked.map(|line| line.clamp(1, rows.last().map_or(1, |row| row.line)));
        let memory = cx.memory::<CodeMemory>();
        if memory.revealed == wanted {
            return;
        }
        memory.revealed = wanted;
        let Some(line) = wanted else { return };
        let Some(first) = rows.iter().position(|row| row.line == line) else { return };
        let count = rows[first..].iter().take_while(|row| row.line == line).count();
        let context = i32::from(REVEAL_CONTEXT);
        let y = (top + i32::try_from(first).unwrap_or(i32::MAX) - context).max(area.y);
        let bottom = (top + i32::try_from(first + count).unwrap_or(i32::MAX) + context).min(area.bottom());
        cx.reveal(Rect::new(area.x, y, area.width, clamp_u16(bottom - y)));
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for CodeView<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let padding = cx.env().theme().style("code", None, &[]).pair("padding").unwrap_or((1, 2));
        let content_width =
            available.width.saturating_sub(cells::sum([padding.1.saturating_mul(2), self.signs(), self.gutter()]));
        let layout = self.source.layout(content_width.max(1));
        Size::new(
            cells::sum([layout.widest, self.signs(), self.gutter(), padding.1.saturating_mul(2)]),
            clamp_u16(i32::try_from(layout.rows.len()).unwrap_or(i32::MAX)).saturating_add(padding.0.saturating_mul(2)),
        )
        .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let mut states = cx.states();
        states.retain(|state| *state != State::Hover);
        let style = cx.style("code", None, &states);
        if let Some(bg) = style.text().bg {
            cx.clear(area, bg);
        }
        cx.register_hit(area);
        let inner = area.inset(style.padding());
        // The padding is surface, not code: a selection starts and stays inside it.
        cx.selectable(inner);
        let (signs, gutter) = (self.signs(), self.gutter());
        let width = inner.width.saturating_sub(signs).saturating_sub(gutter).max(1);
        let layout = self.source.layout(width);
        let renumbered = self.renumbered(&layout.rows);
        let rows = renumbered.as_deref().unwrap_or(&layout.rows);
        if signs > 0 {
            // Signs say how a line changed; copies of the code leave them out like line numbers.
            cx.decoration(Rect::new(inner.x, inner.y, signs, inner.height));
            self.paint_looks(cx, area, inner.x, inner.y, rows);
        }
        paint_rows(
            cx,
            Rect::new(inner.x + i32::from(signs), inner.y, inner.width.saturating_sub(signs), inner.height),
            rows,
            gutter,
        );
        self.request_reveal(cx, area, inner.y, rows);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let Event::Key(key) = event else {
            return false;
        };
        if !key.is_plain(Key::Char('c')) {
            return false;
        }
        cx.copy(self.source.code.clone());
        cx.flash();
        if let Some(message) = &self.on_copy {
            cx.emit(message.clone());
        }
        true
    }

    fn focusable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests;
