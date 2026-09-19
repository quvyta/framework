//! Drawing a [`Tabs`] strip: the tabs, the dragged ghost, the scroll arrows and the menu control.

use crate::geometry::{Rect, clamp_u16};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::PaintCx;

use super::super::tab_model::{self, Drag};
use super::strip::Strip;
use super::{Arrow, PAD, PopupMenu, Tabs, TabsMemory, close_mark};

impl<Msg: 'static> Tabs<Msg> {
    pub(super) fn paint_tab(&self, cx: &mut PaintCx<'_>, rect: Rect, index: usize, position: usize, states: &[State]) {
        let theme_style = cx.style("tab", None, states);
        let style = theme_style.text();
        if let Some(bg) = style.bg {
            cx.clear(rect, bg);
        }
        // Hovered and open tabs raise the pillar.
        let pillar = theme_style.color("pillar");
        // With slide on, a resting tab draws its label one cell to the left; raised, it sits at its
        // natural place. Widths never depend on slide, so turning it on moves no tab.
        let rest = i32::from(pillar.is_none() && cx.env().slide());
        if let Some(color) = pillar {
            cx.pillar(rect.x, rect.y, color);
        }
        let number_style = if self.numbered { cx.style("tab-index", None, states).text() } else { style };
        let badge_style = cx.style("tab-badge", None, states).text();
        let label = LabelStyles { number: number_style, name: style, badge: CellStyle { bg: None, ..badge_style } };
        self.paint_label(cx, rect, index, position, rest, label);
        if self.model.closable(index) {
            close_mark::paint(cx, Self::close_x(rect), rect.y, !states.is_empty());
        }
    }

    /// Paints the ghost of the tab being dragged, `width` cells wide, at the pointer. It floats over
    /// the tabs only, never over the arrows or the menu control; `order` is the preview order, which
    /// numbers it.
    pub(super) fn paint_ghost(
        &self,
        cx: &mut PaintCx<'_>,
        area: Rect,
        strip: &Strip,
        drag: Drag,
        width: u16,
        order: &[usize],
    ) {
        let lane = strip.lane(area);
        let x = (drag.pointer.0 - drag.grab.0).clamp(lane.x, (lane.right() - i32::from(width)).max(lane.x));
        let rect = Rect::new(x, area.y, width, 1);
        let position = order.iter().position(|i| *i == drag.index).unwrap_or(drag.index);
        let ghost = tab_model::paint_ghost_surface(cx, rect);
        self.paint_label(cx, rect, drag.index, position, 0, LabelStyles { number: ghost, name: ghost, badge: ghost });
    }

    /// Paints the number (when numbered), the label and the count of tab `index` at `position` in
    /// `rect`. The number and the label sit `rest` cells left of their natural place; the count
    /// keeps its place one cell after the label as it sits raised, so it does not slide. The label
    /// is cut first, so the count and the close mark always keep their cells.
    fn paint_label(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        index: usize,
        position: usize,
        rest: i32,
        styles: LabelStyles,
    ) {
        let end = rect.right() - i32::from(PAD + self.close_width(index));
        let mut x = rect.x + i32::from(PAD);
        if self.numbered {
            let number = (position + 1).to_string();
            x += i32::from(cx.text(x - rest, rect.y, &number, styles.number, 2)) + 1;
        }
        let badge = self.badge_text(index);
        let budget = clamp_u16(end - x - i32::from(self.badge_width(index)));
        let label = text::truncate(&self.labels[index], budget).into_owned();
        let drawn = cx.text(x - rest, rect.y, &label, styles.name, budget);
        if let Some(badge) = badge {
            let at = x + i32::from(drawn) + 1;
            cx.text(at, rect.y, &badge, styles.badge, clamp_u16(end - at));
        }
    }

    /// Paints the arrows of an overflowing strip, if it has them. They are small buttons: the
    /// `pointer` raises the pillar in their first cell and a press flashes them one tone brighter;
    /// an arrow with nothing more to show fades and takes no hover. The arrow towards `held`, the end
    /// a dragged tab rests against, is lit as if hovered.
    pub(super) fn paint_arrows(
        &self,
        cx: &mut PaintCx<'_>,
        strip: &Strip,
        offset: usize,
        pointer: Option<(i32, i32)>,
        held: Option<Arrow>,
    ) {
        let Some((back, forward)) = strip.arrows else {
            return;
        };
        let pressed = cx.memory::<TabsMemory>().pressed;
        let (now, flash) = (cx.now(), cx.env().theme().motion().flash);
        for (arrow, rect, glyph_key) in
            [(Arrow::Back, back, "chevron-left"), (Arrow::Forward, forward, "chevron-right")]
        {
            let mut states = Vec::new();
            if self.can_scroll(strip, offset, arrow) {
                if held == Some(arrow) || pointer.is_some_and(|(x, y)| rect.contains(x, y)) {
                    states.push(State::Hover);
                }
                if let Some((last, at)) = pressed
                    && last == arrow
                    && now < at + flash
                {
                    states.push(State::Pressed);
                    cx.request_frame_in(at + flash - now);
                }
            } else {
                states.push(State::Disabled);
            }
            let glyph = cx.env().icons().glyph(glyph_key).into_owned();
            paint_control(cx, "tab-arrow", rect, &states, &glyph, 1);
        }
    }

    /// Paints the control of a strip that lists its hidden tabs in a menu, if it has one, and
    /// remembers which tabs are hidden. Like the arrows it is a small button: the `pointer` raises
    /// the pillar in its first cell, and while its menu is open it stays lit with a steady pillar.
    pub(super) fn paint_menu_control(&self, cx: &mut PaintCx<'_>, strip: &Strip, pointer: Option<(i32, i32)>) {
        let hidden = Self::hidden(self.labels.len(), strip);
        let open = PopupMenu::is_open_paint(cx);
        if let Some(rect) = strip.menu {
            let mut states = Vec::new();
            if pointer.is_some_and(|(x, y)| rect.contains(x, y)) {
                states.push(State::Hover);
            }
            if open {
                states.push(State::Active);
            }
            let glyph = cx.env().icons().glyph("chevron-down").into_owned();
            let label = format!("{glyph} {}", hidden.len());
            paint_control(cx, "tab-menu", rect, &states, &label, rect.width.saturating_sub(1));
            if open {
                cx.request_overlay(rect);
            }
        }
        cx.memory::<TabsMemory>().hidden = hidden;
    }
}

/// The styles of the parts of a tab's label.
#[derive(Clone, Copy)]
struct LabelStyles {
    number: CellStyle,
    name: CellStyle,
    badge: CellStyle,
}

/// Paints a small strip control: its surface from style `key` in `states`, the pillar the style
/// raises in its first cell and `label` after it, cut to `budget` cells.
pub(super) fn paint_control(cx: &mut PaintCx<'_>, key: &str, rect: Rect, states: &[State], label: &str, budget: u16) {
    let theme_style = cx.style(key, None, states);
    let style = theme_style.text();
    cx.clear(rect, style.bg.unwrap_or_else(|| cx.color("raised")));
    if let Some(color) = theme_style.color("pillar") {
        cx.pillar(rect.x, rect.y, color);
    }
    cx.text(rect.x + 1, rect.y, label, CellStyle { bg: None, ..style }, budget);
}
