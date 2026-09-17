//! Context menu items and the rows that draw them.

use crate::geometry::{Rect, Size, clamp_u16};
use crate::text;
use crate::theme::State;
use crate::widget::PaintCx;

use super::cells;

/// One row of a menu: an action, a submenu or a gap between groups.
///
/// Style keys: `context-menu` (`bg`), `context-item` with `hover` and `disabled`,
/// `context-item.danger`, `context-item-shortcut`, `context-item-chevron`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem<Msg> {
    label: String,
    icon: Option<String>,
    shortcut: Option<String>,
    message: Option<Msg>,
    submenu: Vec<ContextItem<Msg>>,
    disabled: bool,
    danger: bool,
    gap: bool,
}

impl<Msg> ContextItem<Msg> {
    /// A row labelled `label` with no message, submenu or option yet.
    fn plain(label: String) -> Self {
        Self {
            label,
            icon: None,
            shortcut: None,
            message: None,
            submenu: Vec::new(),
            disabled: false,
            danger: false,
            gap: false,
        }
    }

    /// An action labelled `label` that sends `message` when chosen.
    #[must_use]
    pub fn new(label: impl Into<String>, message: Msg) -> Self {
        Self { message: Some(message), ..Self::plain(label.into()) }
    }

    /// A row that opens `items` beside the menu.
    #[must_use]
    pub fn submenu(label: impl Into<String>, items: impl IntoIterator<Item = Self>) -> Self {
        Self { submenu: items.into_iter().collect(), ..Self::plain(label.into()) }
    }

    /// An empty row that separates groups; menus never draw lines.
    #[must_use]
    pub fn gap() -> Self {
        Self { gap: true, ..Self::plain(String::new()) }
    }

    /// Icon key drawn before the label.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>) -> Self {
        self.icon = Some(key.into());
        self
    }

    /// Faint key label kept at the right edge, e.g. `"ctrl r"`. It only describes the key; bind
    /// the key itself in the keymap.
    #[must_use]
    pub fn shortcut(mut self, label: impl Into<String>) -> Self {
        self.shortcut = Some(label.into());
        self
    }

    /// Greys the row out; it cannot be chosen or highlighted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Draws the row in the danger colour, for destructive actions such as delete.
    #[must_use]
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub(crate) fn selectable(&self) -> bool {
        !self.gap && !self.disabled
    }

    pub(crate) fn children(&self) -> &[Self] {
        &self.submenu
    }

    pub(crate) fn has_submenu(&self) -> bool {
        !self.submenu.is_empty()
    }

    pub(crate) fn message(&self) -> Option<&Msg> {
        self.message.as_ref()
    }
}

/// Replaces the message of every row in `items`, submenus included, with its position in the
/// returned list of messages. A widget can run a menu on the positions and send the chosen message
/// by value, so its application's messages need not be `Clone`.
pub(crate) fn keyed<Msg>(items: Vec<ContextItem<Msg>>) -> (Vec<ContextItem<usize>>, Vec<Msg>) {
    fn key<Msg>(items: Vec<ContextItem<Msg>>, messages: &mut Vec<Msg>) -> Vec<ContextItem<usize>> {
        items
            .into_iter()
            .map(|item| {
                let message = item.message.map(|message| {
                    messages.push(message);
                    messages.len() - 1
                });
                ContextItem {
                    label: item.label,
                    icon: item.icon,
                    shortcut: item.shortcut,
                    message,
                    submenu: key(item.submenu, messages),
                    disabled: item.disabled,
                    danger: item.danger,
                    gap: item.gap,
                }
            })
            .collect()
    }
    let mut messages = Vec::new();
    let items = key(items, &mut messages);
    (items, messages)
}

/// Cells of the icon column of `items`: the widest icon and a space, or nothing when no row has
/// an icon. Labels line up after it, so rows without an icon keep the column empty.
fn icon_column<Msg>(cx: &PaintCx<'_>, items: &[ContextItem<Msg>]) -> u16 {
    let icons = cx.env().icons();
    items
        .iter()
        .filter_map(|item| item.icon.as_deref())
        .map(|key| text::width(&icons.glyph(key)).saturating_add(1))
        .max()
        .unwrap_or(0)
}

/// The size of a menu layer showing `items`.
pub(crate) fn size<Msg>(cx: &PaintCx<'_>, items: &[ContextItem<Msg>]) -> Size {
    let icons = cx.env().icons();
    let lead =
        cells::sum([items.iter().map(|item| text::width(&item.label)).max().unwrap_or(0), icon_column(cx, items)]);
    let trail = items
        .iter()
        .map(|item| {
            let shortcut = item.shortcut.as_deref().map_or(0, text::width);
            let chevron = if item.has_submenu() { text::width(&icons.glyph("chevron-right")) } else { 0 };
            shortcut.max(chevron)
        })
        .max()
        .unwrap_or(0);
    // Pillar and space, the label with its spare slide cell, a gap, the trailing column, a margin.
    let width = cells::sum([2, lead, 1, if trail > 0 { trail.saturating_add(3) } else { 2 }, 2]);
    Size::new(width.max(18), clamp_u16(i32::try_from(items.len()).unwrap_or(i32::MAX)))
}

/// The next selectable row after `from` in direction `step`, wrapping around.
pub(crate) fn step<Msg>(items: &[ContextItem<Msg>], from: Option<usize>, forward: bool) -> Option<usize> {
    let len = items.len();
    if len == 0 {
        return None;
    }
    let start = from.unwrap_or(if forward { len - 1 } else { 0 });
    (1..=len)
        .map(|offset| if forward { (start + offset) % len } else { (start + len * 2 - offset) % len })
        .find(|index| items[*index].selectable())
}

/// The first or last selectable row.
pub(crate) fn edge<Msg>(items: &[ContextItem<Msg>], last: bool) -> Option<usize> {
    if last { items.iter().rposition(ContextItem::selectable) } else { items.iter().position(ContextItem::selectable) }
}

/// The next selectable row after `from` whose label starts with `typed`.
pub(crate) fn type_ahead<Msg>(items: &[ContextItem<Msg>], from: Option<usize>, typed: char) -> Option<usize> {
    let start = from.unwrap_or(items.len().saturating_sub(1));
    super::popup_menu::type_ahead_by(items.len(), start, typed, |index| {
        let item = &items[index];
        item.selectable().then_some(item.label.as_str())
    })
}

/// Draws `items` into `rect` (already the visible part of the layer at `full`), with row
/// `highlight` raised.
pub(crate) fn paint<Msg>(
    cx: &mut PaintCx<'_>,
    items: &[ContextItem<Msg>],
    full: Rect,
    shown: Rect,
    highlight: Option<usize>,
) {
    let background = cx.style("context-menu", None, &[]).text().bg.unwrap_or_else(|| cx.color("overlay"));
    cx.clear(shown, background);
    cx.register_hit(shown);
    let slide = cx.env().slide();
    let chevron = cx.env().icons().glyph("chevron-right").into_owned();
    let icon_column = icon_column(cx, items);
    cx.with_clip(shown, |cx| {
        for (row, item) in items.iter().enumerate().take(usize::from(full.height)) {
            if item.gap {
                continue;
            }
            let rect = full.row(clamp_u16(i32::try_from(row).unwrap_or(i32::MAX)));
            let mut states = Vec::new();
            if item.disabled {
                states.push(State::Disabled);
            } else if highlight == Some(row) {
                states.push(State::Hover);
            }
            let variant = item.danger.then_some("danger");
            let style = cx.style("context-item", variant, &states);
            let text_style = style.text();
            if let Some(bg) = text_style.bg {
                cx.fill(rect, bg);
            }
            if let Some(color) = style.color("pillar") {
                cx.pillar(rect.x, rect.y, color);
            }

            let right = rect.right() - 2;
            let mut trail_x = right;
            if let Some(shortcut) = &item.shortcut {
                let width = text::width(shortcut);
                trail_x = right - i32::from(width);
                let shortcut_style = cx.style("context-item-shortcut", None, &states).text();
                cx.text(trail_x, rect.y, shortcut, shortcut_style, width);
            } else if item.has_submenu() {
                let width = text::width(&chevron);
                trail_x = right - i32::from(width);
                let chevron_style = cx.style("context-item-chevron", None, &states).text();
                cx.text(trail_x, rect.y, &chevron, chevron_style, width);
            }

            let shift = i32::from(slide && states.contains(&State::Hover));
            let mut x = rect.x + 2 + shift;
            // The label column keeps one spare cell for the slide and stops short of the trail.
            let limit = trail_x - 2;
            let mut label_style = text_style;
            label_style.bg = None;
            if let Some(icon) = &item.icon {
                let glyph = cx.env().icons().glyph(icon).into_owned();
                cx.text(x, rect.y, &glyph, label_style, clamp_u16(limit - x));
            }
            x += i32::from(icon_column);
            let budget = clamp_u16(limit - x);
            let label = text::truncate(&item.label, budget).into_owned();
            cx.text(x, rect.y, &label, label_style, budget);
        }
    });
}
