//! Virtualised lists with keyboard and mouse selection.

use std::ops::Deref;
use std::sync::Arc;

use crate::env::Env;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::text;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::IndexMessage;
use super::cells;
use super::row;
use super::rows::{self, RowScroll};
use super::scrollbar::ScrollbarStyle;

/// What kind of row an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A selectable row.
    Normal,
    /// A selectable row drawn faint, e.g. something not available yet.
    Faint,
    /// A section heading; never selected, skipped by the keyboard.
    Header,
    /// An empty row between sections; never selected.
    Gap,
}

/// One row of a [`List`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    label: String,
    icon: Option<String>,
    icon_color: Option<String>,
    detail: Option<String>,
    kind: ItemKind,
}

impl ListItem {
    /// A selectable row.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), icon: None, icon_color: None, detail: None, kind: ItemKind::Normal }
    }

    /// A section heading.
    #[must_use]
    pub fn header(label: impl Into<String>) -> Self {
        Self { kind: ItemKind::Header, ..Self::new(label) }
    }

    /// An empty separating row.
    #[must_use]
    pub fn gap() -> Self {
        Self { kind: ItemKind::Gap, ..Self::new("") }
    }

    /// Icon key drawn before the label, optionally in theme colour `color`.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>, color: Option<&str>) -> Self {
        self.icon = Some(key.into());
        self.icon_color = color.map(str::to_owned);
        self
    }

    /// Faint text aligned right, e.g. a status or a count.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Draws the row faint while keeping it selectable.
    #[must_use]
    pub fn faint(mut self, faint: bool) -> Self {
        if faint {
            self.kind = ItemKind::Faint;
        }
        self
    }

    fn selectable(&self) -> bool {
        matches!(self.kind, ItemKind::Normal | ItemKind::Faint)
    }
}

/// A vertical list that only draws the rows it shows, so it stays fast with any number of items.
/// For a very long list, keep the items as an `Arc<[ListItem]>` in your state and pass it to
/// [`List::shared`], so they are not built again every frame.
///
/// The application owns the selection and the checked rows; the list reports changes through
/// messages. Hovered and selected rows raise their surface, show the accent pillar and slide
/// their icon and label one cell right. The pillar, the check mark of a multi-select list and the
/// detail column never move, so a mark is always where the pointer left it.
///
/// Keys while focused: ↑/↓ or k/j move, Home/End and PgUp/PgDn jump, Enter activates, Space
/// toggles in multi-select lists and activates otherwise. A click on a row selects and activates
/// it; in a multi-select list a click on the check mark (or the cell after it) only toggles.
/// Style keys: `list-item` with `hover`, `selected`, `focus`, `pressed`; `list-item.faint`,
/// `list-header`, `list-detail`, `scrollbar`.
pub struct List<Msg> {
    items: Items,
    selected: Option<usize>,
    checked: Option<Vec<bool>>,
    empty: String,
    on_select: Option<IndexMessage<Msg>>,
    on_activate: Option<IndexMessage<Msg>>,
    on_toggle: Option<IndexMessage<Msg>>,
    scrollbar: Option<ScrollbarStyle>,
}

/// The rows of a list: built for this frame, or shared with the application's state.
enum Items {
    Owned(Vec<ListItem>),
    Shared(Arc<[ListItem]>),
}

impl Deref for Items {
    type Target = [ListItem];

    fn deref(&self) -> &[ListItem] {
        match self {
            Self::Owned(items) => items,
            Self::Shared(items) => items,
        }
    }
}

impl<Msg: 'static> List<Msg> {
    /// A list of `items`.
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = ListItem>) -> Self {
        Self::with_items(Items::Owned(items.into_iter().collect()))
    }

    /// A list of `items` kept by the application, e.g. in its state: building the list in `view`
    /// only clones the `Arc`, however many items there are.
    #[must_use]
    pub fn shared(items: Arc<[ListItem]>) -> Self {
        Self::with_items(Items::Shared(items))
    }

    fn with_items(items: Items) -> Self {
        Self {
            items,
            selected: None,
            checked: None,
            empty: String::new(),
            on_select: None,
            on_activate: None,
            on_toggle: None,
            scrollbar: None,
        }
    }

    /// Draws the scrollbar in `style` whatever the theme chooses.
    #[must_use]
    pub fn scrollbar(mut self, style: ScrollbarStyle) -> Self {
        self.scrollbar = Some(style);
        self
    }

    /// The selected row index.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Turns the list into a multi-select list; `checked[i]` tells whether row `i` is checked.
    #[must_use]
    pub fn checked(mut self, checked: Vec<bool>) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Text shown when there are no items.
    #[must_use]
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    /// Message for moving the selection to a row.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// Message for opening a row (Enter, click).
    #[must_use]
    pub fn on_activate(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_activate = Some(Box::new(message));
        self
    }

    /// Message for checking or unchecking a row in a multi-select list (Space, click on the mark).
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(message));
        self
    }

    fn next_selectable(&self, from: Option<usize>, step: isize) -> Option<usize> {
        let len = isize::try_from(self.items.len()).ok()?;
        let mut index = from.map_or(if step > 0 { -1 } else { len }, |i| isize::try_from(i).unwrap_or(0));
        loop {
            index += step;
            if index < 0 || index >= len {
                return from;
            }
            let candidate = usize::try_from(index).ok()?;
            if self.items[candidate].selectable() {
                return Some(candidate);
            }
        }
    }

    fn select(&self, cx: &mut EventCx<'_, Msg>, index: Option<usize>) {
        if let (Some(index), Some(message)) = (index, &self.on_select)
            && Some(index) != self.selected
        {
            cx.emit(message(index));
        }
    }

    fn activate(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        if let Some(message) = &self.on_activate {
            cx.memory::<RowScroll>().flashed = Some(index);
            cx.flash();
            cx.emit(message(index));
        }
    }

    fn row_at(&self, cx: &mut EventCx<'_, Msg>, y: i32) -> Option<usize> {
        let area = cx.area();
        let offset = cx.memory::<RowScroll>().offset;
        let row = usize::try_from(y - area.y).ok()?;
        let index = offset + row;
        (row < usize::from(area.height) && index < self.items.len()).then_some(index)
    }

    /// Cells from the left edge through the check mark and its air: a press there toggles.
    fn check_column(env: &Env) -> u16 {
        let widest = ["select-on", "select-off"].map(|key| text::width(&env.icons().glyph(key))).into_iter().max();
        row::LEAD + widest.unwrap_or(1) + 1
    }
}

impl<Msg: 'static> Widget<Msg> for List<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let rows = if self.items.is_empty() { 1 } else { self.items.len() };
        let widest = self
            .items
            .iter()
            .map(|item| {
                cells::sum([
                    text::width(&item.label),
                    item.detail.as_deref().map_or(0, |d| text::width(d).saturating_add(2)),
                    item.icon.as_ref().map_or(0, |_| 2),
                    if self.checked.is_some() { 2 } else { 0 },
                    5,
                ])
            })
            .max()
            .unwrap_or_else(|| text::width(&self.empty).saturating_add(3));
        Size::new(widest, clamp_u16(i32::try_from(rows).unwrap_or(i32::MAX))).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        if self.items.is_empty() {
            let faint = cx.style("list-header", None, &[]).text();
            cx.text(area.x + 2, area.y, &self.empty, faint, area.width.saturating_sub(2));
            return;
        }
        let focused = cx.is_focused();
        let pressed = cx.is_pressed();
        let pointer = cx.pointer();
        let visible = usize::from(area.height);
        let (offset, flashed) = {
            let scroll = cx.memory::<RowScroll>();
            (scroll.follow(self.selected, self.items.len(), visible), scroll.flashed)
        };
        let content_width = area.width.saturating_sub(u16::from(self.items.len() > visible));

        for (row, index) in (offset..self.items.len()).take(visible).enumerate() {
            let item = &self.items[index];
            let row_rect = Rect::new(area.x, area.y + i32::try_from(row).unwrap_or(0), content_width, 1);
            match item.kind {
                ItemKind::Gap => continue,
                ItemKind::Header => {
                    let style = cx.style("list-header", None, &[]).text();
                    cx.text(row_rect.x + 2, row_rect.y, &item.label, style, content_width.saturating_sub(3));
                    continue;
                }
                ItemKind::Normal | ItemKind::Faint => {}
            }
            let hovered = pointer.is_some_and(|(x, y)| row_rect.contains(x, y));
            let states =
                rows::row_states(hovered, Some(index) == self.selected, focused, pressed && flashed == Some(index));
            let variant = (item.kind == ItemKind::Faint).then_some("faint");
            let style = cx.style("list-item", variant, &states);
            let text_style = style.text();
            let detail_width = item.detail.as_deref().map_or(0, |d| text::width(d).saturating_add(2));
            let fixed: Vec<row::Mark> = self
                .checked
                .as_ref()
                .map(|checked| row::check(cx, checked.get(index).copied().unwrap_or(false)))
                .into_iter()
                .collect();
            let icon: Vec<row::Mark> =
                item.icon.iter().map(|key| row::icon(cx, key, item.icon_color.as_deref(), text_style.fg)).collect();
            let parts =
                row::Parts { fixed: &fixed, sliding: &icon, label: &item.label, trailing: detail_width, indent: 0 };
            row::paint_parts(cx, row_rect, &style, rows::slide(cx, &states) > 0, &parts);

            if let Some(detail) = &item.detail {
                let detail_style = cx.style("list-detail", None, &states).text();
                row::paint_trailing(cx, row_rect, detail, detail_style);
            }
        }
        rows::paint_scrollbar(cx, area, self.items.len(), offset, self.scrollbar);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let area = cx.area();
        let page = usize::from(area.height.max(1));
        match event {
            Event::Key(key) => {
                let target = if key.is_plain(Key::Up) || key.is_plain(Key::Char('k')) {
                    self.next_selectable(self.selected, -1)
                } else if key.is_plain(Key::Down) || key.is_plain(Key::Char('j')) {
                    self.next_selectable(self.selected, 1)
                } else if key.is_plain(Key::Home) {
                    self.next_selectable(None, 1)
                } else if key.is_plain(Key::End) {
                    self.next_selectable(None, -1)
                } else if key.is_plain(Key::PageUp) || key.is_plain(Key::PageDown) {
                    let down = key.is_plain(Key::PageDown);
                    let mut index = self.selected;
                    for _ in 0..page {
                        index = self.next_selectable(index, if down { 1 } else { -1 });
                    }
                    index
                } else if key.is_plain(Key::Enter) {
                    if let Some(index) = self.selected {
                        self.activate(cx, index);
                    }
                    return self.selected.is_some() && self.on_activate.is_some();
                } else if key.is_plain(Key::Space) {
                    let Some(index) = self.selected else { return false };
                    if let (Some(_), Some(toggle)) = (&self.checked, &self.on_toggle) {
                        cx.emit(toggle(index));
                        return true;
                    }
                    self.activate(cx, index);
                    return self.on_activate.is_some();
                } else {
                    return false;
                };
                if target == self.selected {
                    return target.is_some();
                }
                self.select(cx, target);
                true
            }
            Event::Mouse(mouse) => {
                if rows::scroll_mouse(cx, mouse, area, self.items.len()) {
                    return true;
                }
                if mouse.kind != MouseKind::Down(MouseButton::Left) {
                    return false;
                }
                let Some(index) = self.row_at(cx, mouse.y).filter(|i| self.items[*i].selectable()) else {
                    return false;
                };
                // The check mark never slides, so its column is the same on every row.
                if let (Some(_), Some(toggle)) = (&self.checked, &self.on_toggle)
                    && mouse.x < area.x + i32::from(Self::check_column(cx.env()))
                {
                    cx.emit(toggle(index));
                    return true;
                }
                self.select(cx, Some(index));
                self.activate(cx, index);
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.items.iter().any(ListItem::selectable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo {
        count: usize,
        selected: Option<usize>,
        opened: Vec<usize>,
        checked: Option<Vec<bool>>,
    }

    #[derive(Clone)]
    enum Msg {
        Select(usize),
        Open(usize),
        Toggle(usize),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Select(i) => self.selected = Some(i),
                Msg::Open(i) => self.opened.push(i),
                Msg::Toggle(i) => {
                    if let Some(checked) = &mut self.checked {
                        checked[i] = !checked[i];
                    }
                }
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let mut items = vec![ListItem::header("CONTAINERS")];
            items.extend((0..self.count).map(|i| ListItem::new(format!("item {i}")).detail("ready")));
            let mut list = List::new(items)
                .selected(self.selected)
                .on_select(Msg::Select)
                .on_activate(Msg::Open)
                .on_toggle(Msg::Toggle);
            if let Some(checked) = &self.checked {
                list = list.checked(checked.clone());
            }
            ui.add(list).fill().id("list");
        }
    }

    fn demo(count: usize) -> Demo {
        Demo { count, selected: None, opened: Vec::new(), checked: None }
    }

    #[test]
    fn keyboard_skips_headers_and_selected_row_slides() {
        let mut h = Harness::new(demo(3), 24, 4);
        h.press("tab").press("down");
        assert_eq!(h.app().selected, Some(1));
        let screen = h.screen();
        assert_eq!(screen, "  CONTAINERS\n▌  item 0         ready\n  item 1          ready\n  item 2          ready\n");
        h.press("up");
        assert_eq!(h.app().selected, Some(1));
        h.press("enter");
        assert_eq!(h.app().opened, vec![1]);
    }

    #[test]
    fn a_shared_list_looks_and_behaves_like_a_built_one() {
        struct Shared {
            items: Arc<[ListItem]>,
            selected: Option<usize>,
            shared: bool,
        }
        impl App for Shared {
            type Msg = usize;
            fn update(&mut self, index: usize) -> Command<usize> {
                self.selected = Some(index);
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, usize>) {
                let list =
                    if self.shared { List::shared(Arc::clone(&self.items)) } else { List::new(self.items.to_vec()) };
                ui.add(list.selected(self.selected).on_select(|index| index)).fill().id("list");
            }
        }
        let items: Arc<[ListItem]> = (0..1000).map(|i| ListItem::new(format!("deploy {i}")).detail("ready")).collect();
        let run = |shared: bool| {
            let mut h = Harness::new(Shared { items: Arc::clone(&items), selected: None, shared }, 24, 4);
            h.press("tab").press("down").press("pgdn").press("down");
            (h.app().selected, h.html("list"))
        };
        let shared = run(true);
        assert_eq!(shared.0, Some(5));
        assert_eq!(shared, run(false));
        assert_eq!(Arc::strong_count(&items), 1, "the list let go of the shared items");
    }

    #[test]
    fn scrolls_to_follow_selection_and_draws_scrollbar() {
        let mut h = Harness::new(demo(100_000), 24, 5);
        h.press("tab").press("end");
        assert_eq!(h.app().selected, Some(100_000));
        let screen = h.screen();
        assert!(screen.contains("item 99999"), "{screen}");
        assert!(!super::super::scrollbar::column(&h, 23).contains(' '), "{screen}");
    }

    #[test]
    fn click_selects_and_opens_and_wheel_scrolls() {
        let mut h = Harness::new(demo(20), 24, 5);
        h.click_text("item 2");
        assert_eq!(h.app().selected, Some(3));
        assert_eq!(h.app().opened, vec![3]);
        h.mouse(MouseKind::ScrollDown, 3, 2);
        assert!(!h.screen().contains("CONTAINERS"));
    }

    fn multi(count: usize) -> Demo {
        Demo { checked: Some(vec![false; count + 1]), ..demo(count) }
    }

    fn without_slide(app: Demo, width: u16, height: u16) -> Harness<Demo> {
        let mut env = crate::env::Env::builtin();
        env.set_slide(false);
        Harness::with_env(app, env, width, height)
    }

    #[test]
    fn multi_select_toggles_with_space() {
        let mut h = Harness::new(multi(2), 24, 3);
        h.press("tab").press("down").press("space");
        assert_eq!(h.app().checked.as_deref(), Some(&[false, true, false][..]));
        assert_eq!(h.screen(), "  CONTAINERS\n▌ ☑  item 0       ready\n  ☐ item 1        ready\n");
        assert_eq!(h.fg(2, 1), h.env().theme().color("accent"), "a checked mark takes the accent");
        assert_eq!(h.fg(2, 2), h.env().theme().color("muted"), "an unchecked mark is faint");
    }

    #[test]
    fn check_marks_stay_put_while_the_label_slides() {
        let mut h = Harness::new(multi(3), 24, 4);
        h.hover(8, 2);
        let screen = h.screen();
        assert_eq!(screen, "  CONTAINERS\n  ☐ item 0        ready\n▌ ☐  item 1       ready\n  ☐ item 2        ready\n");
        let column = |line: &str| line.chars().position(|c| c == '☐');
        let lines: Vec<&str> = screen.lines().skip(1).collect();
        assert!(lines.iter().all(|line| column(line) == Some(2)), "the mark column never moves:\n{screen}");
        assert_eq!(h.bg(2, 2), h.env().theme().color("raised"), "the mark sits on the raised row");

        let mut h = without_slide(multi(3), 24, 4);
        h.hover(8, 2);
        assert_eq!(
            h.screen(),
            "  CONTAINERS\n  ☐ item 0        ready\n▌ ☐ item 1        ready\n  ☐ item 2        ready\n"
        );
    }

    #[test]
    fn a_click_on_the_mark_toggles_and_a_click_on_the_label_opens() {
        let mut h = Harness::new(multi(3), 24, 4);
        // Row 2 of the screen is item 1, index 2 after the heading.
        h.hover(8, 2).click(2, 2);
        assert_eq!(h.app().checked.as_deref(), Some(&[false, false, true, false][..]));
        assert_eq!((h.app().selected, h.app().opened.as_slice()), (None, &[][..]), "the mark only toggles");
        h.click(3, 3);
        assert_eq!(h.app().checked.as_deref(), Some(&[false, false, true, true][..]), "its air cell counts too");
        h.click(4, 3);
        assert_eq!((h.app().selected, h.app().opened.as_slice()), (Some(3), &[3][..]), "the label opens the row");
        assert_eq!(h.app().checked.as_deref(), Some(&[false, false, true, true][..]));
    }

    #[test]
    fn a_label_is_cut_at_the_same_place_resting_and_sliding() {
        struct Long;
        impl App for Long {
            type Msg = ();
            fn update(&mut self, _: ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                let items = ["docs-preview-environment", "nightly-integration-tests"]
                    .map(|name| ListItem::new(name).icon("dot", Some("success")).detail("running"));
                ui.add(List::new(items).checked(vec![true, false])).fill();
            }
        }
        let mut h = Harness::new(Long, 24, 2);
        assert_eq!(h.screen(), "  ☑ ● docs-p…   running\n  ☐ ● nightl…   running\n");
        h.hover(10, 1);
        assert_eq!(h.screen(), "  ☑ ● docs-p…   running\n▌ ☐  ● nightl…  running\n");
    }

    #[test]
    fn ascii_marks_are_letters_not_brackets() {
        let mut h = Harness::new(multi(2), 24, 3);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        h.press("tab").press("down").press("space");
        assert_eq!(h.screen(), "  CONTAINERS\n  x  item 0       ready\n  o item 1        ready\n");
        assert_eq!(h.bg(2, 1), h.env().theme().color("active"), "the selection shows by surface alone");
    }

    #[test]
    fn empty_list_shows_empty_text() {
        struct Empty;
        impl App for Empty {
            type Msg = ();
            fn update(&mut self, _: ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                ui.add(List::new(Vec::new()).empty_text("Nothing here")).fill();
            }
        }
        assert_eq!(Harness::new(Empty, 20, 1).screen(), "  Nothing here\n");
    }
}
