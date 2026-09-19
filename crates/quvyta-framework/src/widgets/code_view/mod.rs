//! Highlighted code.

use std::ops::{Bound, RangeBounds};

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
    for (index, line) in code.split('\n').enumerate() {
        let line_end = line_start + line.len();
        let mut row = CodeRow { line: index + 1, number: Some(index + 1), pieces: Vec::new() };
        let mut used = 0u16;
        for (range, token) in &tokens {
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
    for (y, row) in rows.iter().enumerate() {
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
    let lines = code.split('\n').count();
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

/// Code with syntax colours, line numbers and wrapping of long lines.
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
    code: String,
    language: Language,
    line_numbers: bool,
    marks: Vec<LineMark>,
    highlights: Vec<(usize, usize, LineTone)>,
    reveal: Option<usize>,
    on_copy: Option<Msg>,
}

impl<Msg: 'static> CodeView<Msg> {
    /// Shows `code` in `language`.
    #[must_use]
    pub fn new(code: impl Into<String>, language: Language) -> Self {
        Self {
            code: code.into(),
            language,
            line_numbers: true,
            marks: Vec::new(),
            highlights: Vec::new(),
            reveal: None,
            on_copy: None,
        }
    }

    /// Marks lines as a diff: the first mark belongs to line 1, the next to line 2, and lines
    /// past the last mark are unchanged. Added lines are tinted with the success colour and
    /// signed `+`, removed ones with the danger colour and `−`, in the sign column at the left
    /// edge.
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

    /// Message sent after the code was copied with `c`.
    #[must_use]
    pub fn on_copy(mut self, message: Msg) -> Self {
        self.on_copy = Some(message);
        self
    }

    fn gutter(&self) -> u16 {
        if self.line_numbers { gutter_width(&self.code) } else { 0 }
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

    /// Tints marked and highlighted rows across `area` and draws their signs at `x`.
    fn paint_looks(&self, cx: &mut PaintCx<'_>, area: Rect, x: i32, top: i32, rows: &[CodeRow]) {
        for (index, row) in rows.iter().enumerate() {
            let Some((variant, sign)) = self.look(row.line) else { continue };
            let y = top + i32::try_from(index).unwrap_or(i32::MAX);
            let style = cx.style("code-line", Some(variant), &[]);
            if let Some(bg) = style.color("bg") {
                cx.fill(Rect::new(area.x, y, area.width, 1), bg);
            }
            let color = style.color("fg").unwrap_or_else(|| cx.color("text"));
            match sign {
                Sign::Pillar => cx.pillar(x, y, color),
                Sign::Icon(icon) if row.number.is_some() => {
                    let glyph = cx.env().icons().glyph(icon).into_owned();
                    cx.text(x, y, &glyph, CellStyle::fg(color), 1);
                }
                Sign::Icon(_) => {}
            }
        }
    }

    /// Asks the enclosing scroll view to show the revealed line, once per line.
    fn request_reveal(&self, cx: &mut PaintCx<'_>, area: Rect, top: i32, rows: &[CodeRow]) {
        let wanted = self.reveal.map(|line| line.clamp(1, rows.last().map_or(1, |row| row.line)));
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
        let rows = code_rows(&self.code, self.language, content_width.max(1));
        let widest = rows
            .iter()
            .map(|row| cells::sum(row.pieces.iter().map(|(piece, _)| text::width(piece))))
            .max()
            .unwrap_or(0);
        Size::new(
            cells::sum([widest, self.signs(), self.gutter(), padding.1.saturating_mul(2)]),
            clamp_u16(i32::try_from(rows.len()).unwrap_or(i32::MAX)).saturating_add(padding.0.saturating_mul(2)),
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
        let rows = code_rows(&self.code, self.language, width);
        if signs > 0 {
            // Signs say how a line changed; copies of the code leave them out like line numbers.
            cx.decoration(Rect::new(inner.x, inner.y, signs, inner.height));
            self.paint_looks(cx, area, inner.x, inner.y, &rows);
        }
        paint_rows(
            cx,
            Rect::new(inner.x + i32::from(signs), inner.y, inner.width.saturating_sub(signs), inner.height),
            &rows,
            gutter,
        );
        self.request_reveal(cx, area, inner.y, &rows);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let Event::Key(key) = event else {
            return false;
        };
        if !key.is_plain(Key::Char('c')) {
            return false;
        }
        cx.copy(self.code.clone());
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
