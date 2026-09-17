//! Drawing the rows of a [`TabRail`]: tabs, the collapsed strip and the add row.
//!
//! A row is painted into its block: the surface and the pillar cover every line of the block and
//! the content sits on its middle line, so a one-line block is exactly a list row.

use crate::geometry::Rect;
use crate::style::{CellStyle, WidgetStyle};
use crate::text;
use crate::theme::State;
use crate::widget::PaintCx;

use super::super::close_mark;
use super::super::row;
use super::{CollapsedMarker, TabRail};
use unicode_segmentation::UnicodeSegmentation;

/// The variant of blocks taller than one line: resting rows are raised blocks too.
const TALL: &str = "tall";

/// The first character of `name` as a marker: a whole grapheme (a letter with its accents), upper
/// case when that keeps it one character per character, so `ß` stays `ß` rather than growing to
/// `SS`.
fn initial(name: &str, locale: &str) -> String {
    let first = name.graphemes(true).next().unwrap_or_default();
    // Turkish and Azerbaijani pair a dotted and a dotless i: `i` rises to `İ`, not `I`.
    let dotted = matches!(locale.split(['-', '_']).next(), Some("tr" | "az"));
    let upper = match first {
        "i" if dotted => "İ".to_owned(),
        _ => first.to_uppercase(),
    };
    if upper.chars().count() == first.chars().count() { upper } else { first.to_owned() }
}

impl<Msg: 'static> TabRail<Msg> {
    /// The glyph of tab `index` in the collapsed strip, following its marker.
    fn marker_glyph(&self, cx: &PaintCx<'_>, index: usize) -> String {
        let tab = &self.tabs[index];
        match (tab.marker.unwrap_or(self.marker), &tab.icon) {
            (CollapsedMarker::Icon, Some(key)) => cx.env().icons().glyph(key).into_owned(),
            (CollapsedMarker::Icon | CollapsedMarker::Initial, _) => initial(&tab.name, cx.env().i18n().active()),
            (CollapsedMarker::Number, _) => (index + 1).to_string(),
        }
    }

    /// The style variant of plain rows: `tall` for blocks taller than a line, none otherwise.
    fn block_variant(&self) -> Option<&'static str> {
        (self.row_height > 1).then_some(TALL)
    }

    /// Paints `style`'s surface and pillar over every line of `block` above and below its middle
    /// line, and returns the middle line, which the row itself paints.
    fn paint_block(cx: &mut PaintCx<'_>, block: Rect, style: &WidgetStyle) -> Rect {
        let middle = Self::middle(block);
        if block.height > 1 {
            let bg = style.text().bg;
            let pillar = style.color("pillar");
            for y in (block.y..block.bottom()).filter(|y| *y != middle) {
                if let Some(bg) = bg {
                    cx.fill(Rect::new(block.x, y, block.width, 1), bg);
                }
                if let Some(pillar) = pillar {
                    cx.pillar(block.x, y, pillar);
                }
            }
        }
        Rect::new(block.x, middle, block.width, 1)
    }

    /// Paints tab `index` into `block`. `variant` is `ghost` for the tab following the pointer.
    pub(super) fn paint_row(
        &self,
        cx: &mut PaintCx<'_>,
        block: Rect,
        index: usize,
        states: &[State],
        variant: Option<&str>,
    ) {
        let tab = &self.tabs[index];
        let ghost = variant.is_some();
        let style = cx.style("rail-tab", variant.or(self.block_variant()), states);
        let text_style = style.text();
        let slide = cx.env().slide() && !ghost && (states.contains(&State::Hover) || states.contains(&State::Selected));
        let rect = Self::paint_block(cx, block, &style);
        if self.collapsed {
            let glyph = self.marker_glyph(cx, index);
            let fg = tab.status.as_deref().map_or(text_style.fg, |token| Some(cx.color(token)));
            Self::paint_strip_row(cx, rect, &style, &glyph, CellStyle { bg: None, fg, ..text_style });
            return;
        }
        let icon = tab.icon.as_deref().map(|key| cx.env().icons().glyph(key).into_owned());
        let marks: Vec<(String, CellStyle)> =
            icon.map(|glyph| (glyph, CellStyle { bg: None, ..text_style })).into_iter().collect();
        let trailing = self.trailing_width(index, block.height);
        row::paint(cx, rect, &style, slide, &marks, &tab.name, trailing);

        // Trailing content is anchored to the right edge and never slides. The close mark takes the
        // block's top right corner, which in a one-line row is the end of the row itself.
        let mut x = rect.right() - 1;
        if let Some(close) = self.close_rect(index, block) {
            close_mark::paint(cx, close.x, close.y, !states.is_empty());
            if close.y == rect.y {
                // The mark's first cell is the space before it.
                x = close.x;
            }
        }
        // A rail too narrow for a mark leaves it out rather than drawing it over the pillar.
        let lead = rect.x + i32::from(row::LEAD);
        if let Some(badge) = &tab.badge {
            let badge_style = cx.style("rail-badge", None, states).text();
            let width = text::width(badge);
            x -= i32::from(width);
            if x >= lead {
                cx.text(x, rect.y, badge, CellStyle { bg: None, ..badge_style }, width);
            }
            x -= 1;
        }
        if let Some(token) = &tab.status
            && x > lead
        {
            let dot = cx.env().icons().glyph("dot").into_owned();
            x -= 1;
            cx.text(x, rect.y, &dot, CellStyle::fg(cx.color(token)), 1);
        }
    }

    /// A row of the collapsed strip: surface, pillar and a one-cell label right after the pillar.
    ///
    /// The label never slides: in a strip this narrow a slide pushes the icon towards the scrollbar
    /// column and the strip looks wider while hovered, so the pillar alone marks a raised row. A
    /// strip too narrow for the label beside the pillar shows only the pillar.
    pub(super) fn paint_strip_row(
        cx: &mut PaintCx<'_>,
        rect: Rect,
        style: &WidgetStyle,
        glyph: &str,
        glyph_style: CellStyle,
    ) {
        row::paint(cx, rect, style, false, &[], "", 0);
        let room = rect.width.saturating_sub(1).min(1);
        let shown = text::truncate(glyph, room).into_owned();
        cx.text(rect.x + 1, rect.y, &shown, glyph_style, room);
    }

    /// The add row in `block`: `+`, and in a wide rail the faint word for a new tab.
    pub(super) fn paint_add(&self, cx: &mut PaintCx<'_>, block: Rect, hovered: bool) {
        let states = if hovered { vec![State::Hover] } else { Vec::new() };
        let style = cx.style("rail-add", self.block_variant(), &states);
        let glyph = cx.env().icons().glyph("add").into_owned();
        let text_style = CellStyle { bg: None, ..style.text() };
        let rect = Self::paint_block(cx, block, &style);
        if self.collapsed {
            Self::paint_strip_row(cx, rect, &style, &glyph, text_style);
        } else {
            let slide = hovered && cx.env().slide();
            let label = cx.env().i18n().translate("quvyta.tab-rail.add", &[]);
            row::paint(cx, rect, &style, slide, &[(glyph, text_style)], &label, 0);
        }
    }
}

#[cfg(test)]
mod initial_tests {
    use super::initial;

    #[test]
    fn a_turkish_initial_keeps_the_dot_on_the_capital_i() {
        assert_eq!(initial("istanbul", "tr"), "İ");
        assert_eq!(initial("istanbul", "en"), "I");
        assert_eq!(initial("ılgaz", "tr"), "I");
        assert_eq!(initial("ßeta", "en"), "ß");
    }
}
