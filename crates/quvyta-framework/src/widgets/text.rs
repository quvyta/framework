//! Styled, wrapping text.

use std::ops::Range;

use crate::geometry::{Rect, Size, clamp_u16};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{Align, MeasureCx, PaintCx, Widget};

/// A run of text with its own look inside a [`Text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    text: String,
    role: Option<String>,
    color: Option<String>,
    background: Option<String>,
    bold: bool,
}

impl Span {
    /// A span that looks like the rest of its text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), role: None, color: None, background: None, bold: false }
    }

    /// Uses typography role `role` (`title`, `body`, `secondary`, `faint`) for this span.
    #[must_use]
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Colours this span with theme token `token`, e.g. `"accent"` or `"danger"`.
    #[must_use]
    pub fn color(mut self, token: impl Into<String>) -> Self {
        self.color = Some(token.into());
        self
    }

    /// Paints the span's background with theme token `token`, e.g. to mark a word or show a
    /// colour swatch.
    #[must_use]
    pub fn on(mut self, token: impl Into<String>) -> Self {
        self.background = Some(token.into());
        self
    }

    /// Makes this span bold.
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

/// Text that wraps to its width, or is cut with `…` when wrapping is off.
///
/// Its look comes from a typography role of the theme (`body` by default); spans can use
/// other roles, colour tokens and bold.
///
/// Text is not selectable with the mouse by itself: titles, labels and hints are not content to
/// copy. Make it selectable where it is, e.g. a message or an address, with
/// `ui.add(Text::new(url)).selectable(true)` ([`NodeMut::selectable`](crate::widget::NodeMut::selectable)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    spans: Vec<Span>,
    role: String,
    wrap: bool,
    align: Align,
}

impl Text {
    /// Plain text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self::rich([Span::new(text)])
    }

    /// Text made of differently styled spans.
    #[must_use]
    pub fn rich(spans: impl IntoIterator<Item = Span>) -> Self {
        Self { spans: spans.into_iter().collect(), role: "body".to_owned(), wrap: true, align: Align::Start }
    }

    /// Uses typography role `role` for the whole text.
    #[must_use]
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    /// Colours every span with theme token `token`.
    #[must_use]
    pub fn color(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        for span in &mut self.spans {
            span.color.get_or_insert_with(|| token.clone());
        }
        self
    }

    /// Makes every span bold.
    #[must_use]
    pub fn bold(mut self) -> Self {
        for span in &mut self.spans {
            span.bold = true;
        }
        self
    }

    /// Cuts the text with `…` instead of wrapping it.
    #[must_use]
    pub fn no_wrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    /// Aligns every line.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    fn joined(&self) -> (String, Vec<Range<usize>>) {
        let mut joined = String::new();
        let mut ranges = Vec::new();
        for span in &self.spans {
            let start = joined.len();
            joined.push_str(&span.text);
            ranges.push(start..joined.len());
        }
        (joined, ranges)
    }

    fn lines(&self, joined: &str, max: u16) -> Vec<Range<usize>> {
        if self.wrap {
            return text::wrap_ranges(joined, max);
        }
        let mut lines = Vec::new();
        let mut start = 0;
        for line in joined.split('\n') {
            lines.push(start..start + line.len());
            start += line.len() + 1;
        }
        lines
    }
}

impl<Msg: 'static> Widget<Msg> for Text {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let (joined, _) = self.joined();
        let lines = self.lines(&joined, available.width);
        let width = lines.iter().map(|line| text::width(&joined[line.clone()])).max().unwrap_or(0);
        Size::new(
            width.min(available.width),
            clamp_u16(i32::try_from(lines.len()).unwrap_or(i32::MAX)).min(available.height),
        )
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let (joined, ranges) = self.joined();
        let base = typography(cx, &self.role);
        let styles: Vec<CellStyle> = self
            .spans
            .iter()
            .map(|span| {
                let mut style = span.role.as_deref().map_or(base, |role| typography(cx, role));
                if let Some(token) = &span.color {
                    style.fg = Some(cx.color(token));
                }
                if let Some(token) = &span.background {
                    style.bg = Some(cx.color(token));
                }
                style.bold |= span.bold;
                style
            })
            .collect();

        for (row, line) in self.lines(&joined, area.width).into_iter().enumerate() {
            let Ok(row) = u16::try_from(row) else { break };
            if row >= area.height {
                break;
            }
            let line_text = &joined[line.clone()];
            let full_width = text::width(line_text);
            let (visible, cut) = if full_width > area.width {
                (text::truncate(line_text, area.width).into_owned(), true)
            } else {
                (line_text.to_owned(), false)
            };
            let visible_width = text::width(&visible);
            let offset = match self.align {
                Align::Start => 0,
                Align::Center => area.width.saturating_sub(visible_width) / 2,
                Align::End => area.width.saturating_sub(visible_width),
            };
            let y = area.y + i32::from(row);
            let mut x = area.x + i32::from(offset);
            let mut remaining = if cut { area.width.saturating_sub(1) } else { visible_width };
            for (range, style) in ranges.iter().zip(&styles) {
                let start = range.start.max(line.start);
                let end = range.end.min(line.end);
                if start >= end || remaining == 0 {
                    continue;
                }
                let piece = &joined[start..end];
                let drawn = cx.text(x, y, piece, *style, remaining).min(remaining);
                x += i32::from(drawn);
                remaining -= drawn;
            }
            if cut {
                let last_style = styles.last().copied().unwrap_or(base);
                cx.text(x, y, text::ELLIPSIS, last_style, 1);
            }
        }
    }
}

/// The text style of typography role `role`.
pub(crate) fn typography(cx: &PaintCx<'_>, role: &str) -> CellStyle {
    let Some(props) = cx.env().theme().typography(role) else {
        return CellStyle::fg(cx.color("text"));
    };
    let style = crate::style::WidgetStyle::new(props.clone(), cx.pulse_phase());
    let mut text_style = style.text();
    text_style.bg = None;
    text_style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(Text);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(self.0.clone()).fill_width();
        }
    }

    #[test]
    fn wraps_and_truncates() {
        let wrapped = Harness::new(Demo(Text::new("terminal interfaces with taste")), 12, 4);
        assert_eq!(wrapped.screen(), "terminal\ninterfaces\nwith taste\n\n");
        let cut = Harness::new(Demo(Text::new("terminal interfaces").no_wrap()), 12, 2);
        assert_eq!(cut.screen(), "terminal in…\n\n");
    }

    #[test]
    fn spans_keep_their_styles_and_alignment() {
        let text =
            Text::rich([Span::new("ok "), Span::new("done").color("success").on("raised").bold()]).align(Align::End);
        let h = Harness::new(Demo(text), 10, 1);
        assert_eq!(h.screen(), "   ok done\n");
        let theme = h.env().theme();
        assert_eq!(h.fg(3, 0), theme.color("text"));
        assert_eq!(h.fg(6, 0), theme.color("success"));
        assert!(h.is_bold(6, 0) && !h.is_bold(3, 0));
        assert_eq!(h.bg(6, 0), theme.color("raised"));
    }
}
