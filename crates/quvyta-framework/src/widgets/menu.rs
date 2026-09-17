//! Navigation menus for application sidebars.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::cells;
use super::popup_menu::type_ahead;
use super::row::{self, LEAD};
use super::rows::{self, RowScroll};

/// Builds a message from an item key.
type KeyMessage<Msg> = Box<dyn Fn(&str) -> Msg>;

/// Builds a message from a group key and whether the group should be open.
type GroupMessage<Msg> = Box<dyn Fn(&str, bool) -> Msg>;

/// One destination in a [`Menu`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    key: String,
    label: String,
    icon: Option<(String, Option<String>)>,
    badge: Option<String>,
}

impl MenuItem {
    /// An item the application knows as `key`, showing `label`.
    #[must_use]
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self { key: key.into(), label: label.into(), icon: None, badge: None }
    }

    /// Icon key drawn before the label, optionally in theme colour `color`.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>, color: Option<&str>) -> Self {
        self.icon = Some((key.into(), color.map(str::to_owned)));
        self
    }

    /// Short faint text at the right, such as a count of unread entries.
    #[must_use]
    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }
}

/// A group of items in a [`Menu`], with an optional faint heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuGroup {
    key: String,
    title: Option<String>,
    items: Vec<MenuItem>,
}

impl MenuGroup {
    /// A group the application knows as `key`, holding `items`.
    #[must_use]
    pub fn new(key: impl Into<String>, items: impl IntoIterator<Item = MenuItem>) -> Self {
        Self { key: key.into(), title: None, items: items.into_iter().collect() }
    }

    /// The faint heading above the items; a collapsible menu toggles the group from it.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// A row of the flattened menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Gap,
    Heading(usize),
    Item(usize, usize),
}

#[derive(Debug, Default)]
struct MenuMemory {
    cursor: Option<Row>,
    followed: Option<Row>,
    flashed: Option<Row>,
    /// Where the pointer was in the last frame; moving it carries the cursor.
    pointer: Option<(i32, i32)>,
}

/// The navigation column of an application: groups of destinations under faint headings.
///
/// With no options it is the plainest menu: the selected item is raised with the accent
/// pillar, a hovered item rises softly, and the icon and label of both slide one cell while
/// badges stay anchored. It is usually the content of an [`AppShell`](super::AppShell) sidebar.
///
/// Keys while focused move a cursor, drawn like hover, without leaving the current page: ↑/↓,
/// Home/End, and typing a letter jumps to the next item starting with it. Enter or Space opens
/// the item under the cursor; a click opens at once. The application owns the selection. There
/// is only ever one such highlight: moving the pointer onto a row moves the cursor there, and
/// the keyboard continues from it.
///
/// [`collapsible`](Self::collapsible) lets groups with a title fold: the heading gets a chevron,
/// a click or Enter on it toggles, → opens and ← closes it, and ← on an item goes to its heading.
/// A foldable heading rises and slides its title like an item; its chevron stays anchored.
///
/// Style keys: `menu-item` with `hover`, `selected`, `focus`, `pressed`; `menu-badge` with the
/// same states; `menu-heading` with `hover`; `scrollbar`.
pub struct Menu<Msg> {
    groups: Vec<MenuGroup>,
    selected: Option<String>,
    collapsed: Vec<String>,
    on_select: Option<KeyMessage<Msg>>,
    on_toggle: Option<GroupMessage<Msg>>,
}

impl<Msg: 'static> Menu<Msg> {
    /// A menu of `groups`.
    #[must_use]
    pub fn new(groups: impl IntoIterator<Item = MenuGroup>) -> Self {
        Self {
            groups: groups.into_iter().collect(),
            selected: None,
            collapsed: Vec::new(),
            on_select: None,
            on_toggle: None,
        }
    }

    /// The key of the current item.
    #[must_use]
    pub fn selected(mut self, key: Option<&str>) -> Self {
        self.selected = key.map(str::to_owned);
        self
    }

    /// Message for opening the item with `key`.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(&str) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// Lets titled groups fold: `message(group, open)` asks the application to open or close a
    /// group. Pass the closed groups with [`collapsed`](Self::collapsed).
    #[must_use]
    pub fn collapsible(mut self, message: impl Fn(&str, bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(message));
        self
    }

    /// Keys of the groups that are closed in a collapsible menu.
    #[must_use]
    pub fn collapsed(mut self, groups: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.collapsed = groups.into_iter().map(Into::into).collect();
        self
    }

    fn is_closed(&self, group: usize) -> bool {
        self.on_toggle.is_some() && self.collapsed.contains(&self.groups[group].key)
    }

    fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for (index, group) in self.groups.iter().enumerate() {
            if index > 0 {
                rows.push(Row::Gap);
            }
            if group.title.is_some() {
                rows.push(Row::Heading(index));
            }
            if !self.is_closed(index) {
                rows.extend((0..group.items.len()).map(|item| Row::Item(index, item)));
            }
        }
        rows
    }

    /// Whether the keyboard cursor can rest on `row`.
    fn navigable(&self, row: Row) -> bool {
        match row {
            Row::Gap => false,
            Row::Heading(_) => self.on_toggle.is_some(),
            Row::Item(..) => true,
        }
    }

    fn selected_row(&self) -> Option<Row> {
        let key = self.selected.as_deref()?;
        self.groups
            .iter()
            .enumerate()
            .find_map(|(g, group)| group.items.iter().position(|item| item.key == key).map(|i| Row::Item(g, i)))
    }

    /// The cursor in `rows`: the remembered one while it is visible, else the selection, else
    /// the first navigable row.
    fn cursor(&self, remembered: Option<Row>, rows: &[Row]) -> Option<usize> {
        let find = |row: Option<Row>| row.and_then(|row| rows.iter().position(|r| *r == row));
        find(remembered)
            .or_else(|| find(self.selected_row()))
            .or_else(|| rows.iter().position(|row| self.navigable(*row)))
    }

    fn item(&self, group: usize, item: usize) -> &MenuItem {
        &self.groups[group].items[item]
    }

    fn open(&self, cx: &mut EventCx<'_, Msg>, row: Row) {
        match row {
            Row::Item(group, item) => {
                let key = &self.item(group, item).key;
                if let Some(message) = &self.on_select {
                    cx.memory::<MenuMemory>().flashed = Some(row);
                    cx.flash();
                    if self.selected.as_deref() != Some(key) {
                        cx.emit(message(key));
                    }
                }
            }
            Row::Heading(group) => self.toggle(cx, group, self.is_closed(group)),
            Row::Gap => {}
        }
    }

    fn toggle(&self, cx: &mut EventCx<'_, Msg>, group: usize, open: bool) {
        if let Some(message) = &self.on_toggle
            && open == self.is_closed(group)
        {
            cx.emit(message(&self.groups[group].key, open));
        }
    }

    fn step(&self, rows: &[Row], from: usize, forward: bool) -> usize {
        let mut index = from;
        loop {
            let next = if forward { index + 1 } else { index.wrapping_sub(1) };
            match rows.get(next) {
                Some(row) if self.navigable(*row) => return next,
                Some(_) => index = next,
                None => return from,
            }
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Menu<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let rows = self.rows();
        let widest = rows
            .iter()
            .map(|row| match *row {
                Row::Gap => 0,
                Row::Heading(group) => {
                    cells::sum([text::width(self.groups[group].title.as_deref().unwrap_or_default()), LEAD, 3])
                }
                Row::Item(group, item) => {
                    let item = self.item(group, item);
                    cells::sum([
                        LEAD,
                        text::width(&item.label),
                        item.icon.as_ref().map_or(0, |_| 2),
                        item.badge.as_deref().map_or(0, |badge| text::width(badge).saturating_add(2)),
                        3,
                    ])
                }
            })
            .max()
            .unwrap_or(0);
        Size::new(widest, clamp_u16(i32::try_from(rows.len()).unwrap_or(i32::MAX))).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        let rows = self.rows();
        if rows.is_empty() || area.is_empty() {
            return;
        }
        let focused = cx.is_focused();
        let pressed = cx.is_pressed();
        let pointer = cx.pointer();
        let slide = cx.env().slide();
        let visible = usize::from(area.height);
        // The first visible row and the scrollbar drag live in the shared row scroll state.
        let mut offset = cx.memory::<RowScroll>().offset;
        let (cursor, flashed) = {
            let memory = cx.memory::<MenuMemory>();
            // The pointer moves the one highlight: a row it moves onto becomes the cursor, so the
            // keyboard continues from there and never a second row is raised.
            if pointer != memory.pointer {
                memory.pointer = pointer;
                let bar = rows.len() > visible && pointer.is_some_and(|(x, _)| x == area.right() - 1);
                let under = pointer
                    .filter(|_| !bar)
                    .and_then(|(_, y)| usize::try_from(y - area.y).ok())
                    .and_then(|line| rows.get(offset + line))
                    .filter(|row| self.navigable(**row));
                if let Some(row) = under {
                    memory.cursor = Some(*row);
                }
            }
            let cursor = self.cursor(memory.cursor, &rows);
            let follow = if focused { cursor.map(|c| rows[c]) } else { self.selected_row() };
            if follow != memory.followed {
                if let Some(index) = follow.and_then(|row| rows.iter().position(|r| *r == row)) {
                    if index < offset {
                        offset = index;
                    } else if index >= offset + visible {
                        offset = index + 1 - visible;
                    }
                }
                memory.followed = follow;
            }
            (cursor, memory.flashed)
        };
        offset = offset.min(rows.len().saturating_sub(visible));
        cx.memory::<RowScroll>().offset = offset;
        let width = if rows.len() > visible { area.width.saturating_sub(1) } else { area.width };
        let selected = self.selected_row();

        for (line, index) in (offset..rows.len()).take(visible).enumerate() {
            let rect = Rect::new(area.x, area.y + i32::try_from(line).unwrap_or(0), width, 1);
            let row = rows[index];
            let mut states = Vec::new();
            let pointed = pointer.is_some_and(|(x, y)| rect.contains(x, y));
            let lit = if focused { cursor == Some(index) } else { pointed };
            if self.navigable(row) && lit {
                states.push(State::Hover);
            }
            match row {
                Row::Gap => {}
                Row::Heading(group) => {
                    let style = cx.style("menu-heading", None, &states);
                    let title = self.groups[group].title.as_deref().unwrap_or_default();
                    // A foldable heading is a row like any other: it rises and its title slides,
                    // while the chevron at the right stays anchored.
                    let chevron = self.on_toggle.is_some().then(|| {
                        let key = if self.is_closed(group) { "chevron-right" } else { "chevron-down" };
                        text::truncate(&cx.env().icons().glyph(key), 1).into_owned()
                    });
                    let trailing = if chevron.is_some() { 2 } else { 0 };
                    let raised = states.contains(&State::Hover);
                    row::paint(cx, rect, &style, slide && raised, &[], title, trailing);
                    if let Some(glyph) = chevron {
                        let plain = CellStyle { bg: None, ..style.text() };
                        cx.text(rect.right() - 2, rect.y, &glyph, plain, 1);
                    }
                }
                Row::Item(group, item_index) => {
                    let item = self.item(group, item_index);
                    if selected == Some(row) {
                        states.push(State::Selected);
                        if focused {
                            states.push(State::Focus);
                        }
                    }
                    if pressed && flashed == Some(row) {
                        states.push(State::Pressed);
                    }
                    let style = cx.style("menu-item", None, &states);
                    let text_style = style.text();
                    let raised = states.contains(&State::Hover) || states.contains(&State::Selected);
                    let badge_width = item.badge.as_deref().map_or(0, |badge| text::width(badge).saturating_add(2));
                    let marks: Vec<row::Mark> = item
                        .icon
                        .iter()
                        .map(|(key, color)| row::icon(cx, key, color.as_deref(), text_style.fg))
                        .collect();
                    row::paint(cx, rect, &style, slide && raised, &marks, &item.label, badge_width);
                    if let Some(badge) = &item.badge {
                        let badge_style = cx.style("menu-badge", None, &states).text();
                        row::paint_trailing(cx, rect, badge, CellStyle { bg: None, ..badge_style });
                    }
                }
            }
        }
        rows::paint_scrollbar(cx, area, rows.len(), offset, None);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let rows = self.rows();
        if rows.is_empty() {
            return false;
        }
        let area = cx.area();
        match event {
            Event::Key(key) => {
                let remembered = cx.memory::<MenuMemory>().cursor;
                let Some(cursor) = self.cursor(remembered, &rows) else {
                    return false;
                };
                let row = rows[cursor];
                let target = if key.is_plain(Key::Up) {
                    self.step(&rows, cursor, false)
                } else if key.is_plain(Key::Down) {
                    self.step(&rows, cursor, true)
                } else if key.is_plain(Key::Home) {
                    rows.iter().position(|r| self.navigable(*r)).unwrap_or(cursor)
                } else if key.is_plain(Key::End) {
                    rows.iter().rposition(|r| self.navigable(*r)).unwrap_or(cursor)
                } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                    self.open(cx, row);
                    return true;
                } else if key.is_plain(Key::Right) {
                    let Row::Heading(group) = row else { return false };
                    self.toggle(cx, group, true);
                    return self.on_toggle.is_some();
                } else if key.is_plain(Key::Left) {
                    match row {
                        Row::Heading(group) => self.toggle(cx, group, false),
                        Row::Item(group, _) if self.on_toggle.is_some() && self.groups[group].title.is_some() => {
                            cx.memory::<MenuMemory>().cursor = Some(Row::Heading(group));
                        }
                        _ => return false,
                    }
                    return self.on_toggle.is_some();
                } else if let (Some(typed), false) = (key.text, key.chord.mods.ctrl || key.chord.mods.alt) {
                    let items: Vec<usize> = (0..rows.len()).filter(|i| matches!(rows[*i], Row::Item(..))).collect();
                    let labels: Vec<String> = items
                        .iter()
                        .map(|i| match rows[*i] {
                            Row::Item(g, it) => self.item(g, it).label.clone(),
                            _ => String::new(),
                        })
                        .collect();
                    let from = items.iter().rposition(|i| *i <= cursor).unwrap_or(items.len().saturating_sub(1));
                    match type_ahead(&labels, from, typed) {
                        Some(found) => items[found],
                        None => return !labels.is_empty(),
                    }
                } else {
                    return false;
                };
                cx.memory::<MenuMemory>().cursor = Some(rows[target]);
                true
            }
            Event::Mouse(mouse) => {
                // The wheel scrolls, and the scrollbar is dragged like a list's.
                if rows::scroll_mouse(cx, mouse, area, rows.len()) {
                    return true;
                }
                let offset = cx.memory::<RowScroll>().offset;
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) => {
                        let index = usize::try_from(mouse.y - area.y).ok().map(|line| offset + line);
                        let Some(row) = index.and_then(|i| rows.get(i).copied()).filter(|r| self.navigable(*r)) else {
                            return false;
                        };
                        cx.memory::<MenuMemory>().cursor = Some(row);
                        self.open(cx, row);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.groups.iter().any(|group| !group.items.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Sidebar {
        page: String,
        closed: Vec<String>,
        collapsible: bool,
    }

    #[derive(Debug, Clone)]
    enum Msg {
        Go(String),
        Group(String, bool),
    }

    impl App for Sidebar {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Go(page) => self.page = page,
                Msg::Group(group, open) => {
                    self.closed.retain(|g| *g != group);
                    if !open {
                        self.closed.push(group);
                    }
                }
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let groups = [
                MenuGroup::new(
                    "work",
                    [MenuItem::new("overview", "Overview"), MenuItem::new("deploys", "Deploys").badge("3")],
                )
                .title("WORKSPACE"),
                MenuGroup::new(
                    "infra",
                    [
                        MenuItem::new("containers", "Containers").icon("dot", Some("success")),
                        MenuItem::new("volumes", "Volumes"),
                    ],
                )
                .title("INFRASTRUCTURE"),
            ];
            let mut menu = Menu::new(groups).selected(Some(&self.page)).on_select(|key| Msg::Go(key.to_owned()));
            if self.collapsible {
                menu =
                    menu.collapsible(|group, open| Msg::Group(group.to_owned(), open)).collapsed(self.closed.clone());
            }
            ui.add(menu).fill().id("menu");
        }
    }

    fn sidebar(collapsible: bool) -> Sidebar {
        Sidebar { page: "overview".into(), closed: Vec::new(), collapsible }
    }

    #[test]
    fn groups_headings_badges_and_selection() {
        let h = Harness::new(sidebar(false), 24, 7);
        assert_eq!(
            h.screen(),
            "  WORKSPACE\n▌  Overview\n  Deploys             3\n\n  INFRASTRUCTURE\n  ● Containers\n  Volumes\n"
        );
        assert_eq!(h.bg(5, 1), h.env().theme().color("active"));
    }

    #[test]
    fn keyboard_moves_a_cursor_and_enter_opens() {
        let mut h = Harness::new(sidebar(false), 24, 7);
        h.press("tab").press("down");
        assert_eq!(h.app().page, "overview", "moving does not navigate");
        assert!(h.screen().contains("▌  Deploys            3"), "{}", h.screen());
        h.press("down").press("enter");
        assert_eq!(h.app().page, "containers");
        h.press("v");
        h.press("space");
        assert_eq!(h.app().page, "volumes");
        h.press("home").press("enter");
        assert_eq!(h.app().page, "overview");
        h.click_text("Deploys");
        assert_eq!(h.app().page, "deploys");
    }

    #[test]
    fn the_scrollbar_is_dragged_and_never_opens_the_row_beside_it() {
        let mut h = Harness::new(sidebar(false), 24, 4);
        assert!(h.screen().starts_with("  WORKSPACE"), "{}", h.screen());
        h.mouse(MouseKind::Down(MouseButton::Left), 23, 2);
        h.mouse(MouseKind::Drag(MouseButton::Left), 23, 3);
        h.mouse(MouseKind::Up(MouseButton::Left), 23, 3);
        assert_eq!(h.app().page, "overview", "a press on the scrollbar opens nothing");
        let screen = h.screen();
        assert!(screen.contains("Volumes") && !screen.contains("WORKSPACE"), "dragged to the end:\n{screen}");
        h.mouse(MouseKind::ScrollUp, 5, 1);
        assert!(h.screen().starts_with("  WORKSPACE"), "{}", h.screen());
    }

    #[test]
    fn the_pointer_carries_the_one_highlight() {
        let mut h = Harness::new(sidebar(false), 24, 7);
        h.press("tab").press("down");
        assert!(h.screen().contains("▌  Deploys"), "{}", h.screen());
        let (x, y) = h.find("Volumes").expect("volumes row");
        h.hover(x + 2, y);
        let screen = h.screen();
        assert_eq!(
            screen,
            "  WORKSPACE\n▌  Overview\n  Deploys             3\n\n  INFRASTRUCTURE\n  ● Containers\n▌  Volumes\n",
            "the keyboard's row gives way to the pointer's"
        );
        h.press("up");
        let screen = h.screen();
        assert!(screen.contains("▌  ● Containers") && screen.contains("\n  Volumes"), "{screen}");
        h.press("enter");
        assert_eq!(h.app().page, "containers", "the keyboard continued from the pointer's row");
    }

    #[test]
    fn a_foldable_heading_rises_and_slides_while_its_chevron_stays() {
        let mut h = Harness::new(sidebar(true), 24, 7);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        h.hover(6, 0);
        assert_eq!(h.screen().lines().next(), Some("▌  WORKSPACE          ▾"), "{}", h.screen());
        assert_eq!(h.bg(10, 0), h.env().theme().color("raised"));
        let mut env = crate::env::Env::builtin();
        env.set_slide(false);
        let mut h = Harness::with_env(sidebar(true), env, 24, 7);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        h.hover(6, 0);
        assert_eq!(h.screen().lines().next(), Some("▌ WORKSPACE           ▾"), "{}", h.screen());
        let mut plain = Harness::new(sidebar(false), 24, 7);
        plain.hover(6, 0);
        assert_eq!(plain.screen().lines().next(), Some("  WORKSPACE"), "a heading that cannot fold never rises");
    }

    #[test]
    fn collapsible_groups_fold_by_click_and_arrows() {
        let mut h = Harness::new(sidebar(true), 24, 7);
        assert!(h.screen().starts_with("  WORKSPACE           ▾\n"), "{}", h.screen());
        h.click_text("INFRASTRUCTURE");
        assert_eq!(h.app().closed, vec!["infra".to_owned()]);
        assert!(!h.screen().contains("Volumes") && h.screen().contains('▶'), "{}", h.screen());
        h.press("home").press("down").press("left");
        assert_eq!(h.app().closed, vec!["infra".to_owned()], "left on an item goes to its heading");
        h.press("left");
        assert!(h.app().closed.contains(&"work".to_owned()), "{:?}", h.app().closed);
        h.press("right");
        assert!(!h.app().closed.contains(&"work".to_owned()));
        let plain = Harness::new(sidebar(false), 24, 7);
        assert!(!plain.screen().contains('▾'), "plain menus have no chevrons");
    }
}
