//! Highlighted code.

use unicode_segmentation::UnicodeSegmentation;

use super::cells;
use super::highlight::{Language, Token, highlight};
use crate::event::Event;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// One visual row of code: a line number on the first row of a source line, and coloured pieces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeRow {
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
        let mut row = CodeRow { number: Some(index + 1), pieces: Vec::new() };
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
                        CodeRow { number: None, pieces: vec![("  ".to_owned(), Token::Plain)] },
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

/// Code with syntax colours, line numbers and wrapping of long lines.
///
/// While focused, `c` copies the code to the clipboard and flashes. The code is a text selection
/// region: a mouse drag selects inside it (turn it off with
/// [`NodeMut::selectable`](crate::widget::NodeMut::selectable)), and clean copies leave out the
/// line numbers. Style keys: `code` (`bg`,
/// `padding`) with `focus` and `pressed`; `code-line-number`; `code-token.<kind>` where kind is
/// `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`,
/// `lifetime`, `punctuation`, `table`, `key` or `plain`.
pub struct CodeView<Msg> {
    code: String,
    language: Language,
    line_numbers: bool,
    on_copy: Option<Msg>,
}

impl<Msg: 'static> CodeView<Msg> {
    /// Shows `code` in `language`.
    #[must_use]
    pub fn new(code: impl Into<String>, language: Language) -> Self {
        Self { code: code.into(), language, line_numbers: true, on_copy: None }
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
}

impl<Msg: Clone + 'static> Widget<Msg> for CodeView<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let padding = cx.env().theme().style("code", None, &[]).pair("padding").unwrap_or((1, 2));
        let content_width = available.width.saturating_sub(cells::sum([padding.1.saturating_mul(2), self.gutter()]));
        let rows = code_rows(&self.code, self.language, content_width.max(1));
        let widest = rows
            .iter()
            .map(|row| cells::sum(row.pieces.iter().map(|(piece, _)| text::width(piece))))
            .max()
            .unwrap_or(0);
        Size::new(
            cells::sum([widest, self.gutter(), padding.1.saturating_mul(2)]),
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
        let gutter = self.gutter();
        let rows = code_rows(&self.code, self.language, inner.width.saturating_sub(gutter).max(1));
        paint_rows(cx, inner, &rows, gutter);
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
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

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

    #[test]
    fn copies_on_c() {
        let mut h = Harness::new(Demo { copies: 0 }, 40, 6);
        h.press("tab").press("c");
        assert_eq!(h.app().copies, 1);
        assert!(h.copied()[0].starts_with("fn main()"));
    }
}
