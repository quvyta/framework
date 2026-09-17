//! Context menus: actions for what is under the pointer, opened with a right click.

use std::time::Duration;

use super::context_item::{self as menu, ContextItem};
use super::placement::{self, Placement};
use crate::event::{Event, KeyEvent, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, Modifiers};
use crate::motion::Easing;
use crate::widget::{Axis, Container, EventCx, Flex, Length, MeasureCx, Node, PaintCx, Widget};

/// Wraps an area and opens a menu of [`ContextItem`]s at the pointer on a right click, or next to
/// the focused widget on Shift+F10 or the menu key.
///
/// The menu is a layer on the overlay surface that unfolds over `motion.enter`. Hovered and
/// highlighted rows show the pillar and slide their label one cell while shortcuts stay at the
/// right edge. ↑/↓ move (skipping gaps and disabled rows), Home/End jump, typing a letter jumps to
/// the next row starting with it, → or Enter opens a submenu, ← or Esc closes it, Enter or Space
/// chooses. Choosing sends the item's message. A press anywhere outside the menu closes it and
/// still reaches what it landed on; a right click inside the area reopens it there.
///
/// Style keys: see [`ContextItem`].
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::{ContextMenu, ContextItem};
///
/// struct Files { deleted: bool }
///
/// #[derive(Clone)]
/// enum Msg { Delete }
///
/// impl App for Files {
///     type Msg = Msg;
///     fn update(&mut self, Msg::Delete: Msg) -> Command<Msg> {
///         self.deleted = true;
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         let items = [ContextItem::new("Delete", Msg::Delete).danger(true)];
///         ui.add_with(ContextMenu::new(items), |ui| {
///             ui.add(Text::new("report.pdf"));
///         });
///     }
/// }
///
/// let mut app = Harness::new(Files { deleted: false }, 30, 5);
/// app.set_reduced_motion(true);
/// app.mouse(qframe::event::MouseKind::Down(qframe::event::MouseButton::Right), 2, 0);
/// app.press("down").press("enter");
/// assert!(app.app().deleted);
/// ```
pub struct ContextMenu<Msg> {
    items: Vec<ContextItem<Msg>>,
    body: Vec<Node<Msg>>,
}

#[derive(Debug, Default)]
struct ContextMenuMemory {
    open: bool,
    anchor: Rect,
    /// The highlighted row of every open level; the last level has the keyboard.
    levels: Vec<Option<usize>>,
    opened_at: Vec<Duration>,
    rects: Vec<Rect>,
    last_pointer: Option<(i32, i32)>,
}

impl<Msg: Clone + 'static> ContextMenu<Msg> {
    /// A context menu of `items` over the widgets added with
    /// [`View::add_with`](crate::widget::View::add_with).
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = ContextItem<Msg>>) -> Self {
        Self { items: items.into_iter().collect(), body: vec![Node::new(Flex::new(Axis::Column, Vec::new()), 0)] }
    }

    /// The items shown at `depth` for the open levels in `levels`.
    fn level<'s>(&'s self, levels: &[Option<usize>], depth: usize) -> &'s [ContextItem<Msg>] {
        let mut items: &[ContextItem<Msg>] = &self.items;
        for highlight in levels.iter().take(depth) {
            match highlight.and_then(|row| items.get(row)) {
                Some(item) => items = item.children(),
                None => return &[],
            }
        }
        items
    }

    /// Opens the menu below `anchor`, with the first choosable row highlighted when
    /// `highlight_first` (for menus opened from the keyboard), and takes the keys.
    pub(crate) fn open(&self, cx: &mut EventCx<'_, Msg>, anchor: Rect, highlight_first: bool) {
        let now = cx.now();
        let first = if highlight_first { menu::edge(&self.items, false) } else { None };
        let memory = cx.memory::<ContextMenuMemory>();
        memory.open = true;
        memory.anchor = anchor;
        memory.levels = vec![first];
        memory.opened_at = vec![now];
        memory.rects.clear();
        memory.last_pointer = None;
        cx.capture_keys(true);
    }

    /// Closes the menu and gives the keys back.
    pub(crate) fn close(cx: &mut EventCx<'_, Msg>) {
        let memory = cx.memory::<ContextMenuMemory>();
        memory.open = false;
        memory.levels.clear();
        memory.opened_at.clear();
        memory.rects.clear();
        cx.capture_keys(false);
    }

    /// Opens the submenu of the highlighted row of the last level, if it has one.
    fn enter_submenu(&self, cx: &mut EventCx<'_, Msg>) -> bool {
        let now = cx.now();
        let levels = cx.memory::<ContextMenuMemory>().levels.clone();
        let depth = levels.len() - 1;
        let items = self.level(&levels, depth);
        let Some(item) = levels[depth].and_then(|row| items.get(row)).filter(|item| item.has_submenu()) else {
            return false;
        };
        let first = menu::edge(item.children(), false);
        let memory = cx.memory::<ContextMenuMemory>();
        memory.levels.push(first);
        memory.opened_at.push(now);
        true
    }

    fn activate(&self, cx: &mut EventCx<'_, Msg>) {
        if self.enter_submenu(cx) {
            return;
        }
        let levels = cx.memory::<ContextMenuMemory>().levels.clone();
        let depth = levels.len() - 1;
        let items = self.level(&levels, depth);
        if let Some(message) = levels[depth].and_then(|row| items.get(row)).and_then(ContextItem::message) {
            let message = message.clone();
            Self::close(cx);
            cx.emit(message);
        }
    }

    fn set_highlight(cx: &mut EventCx<'_, Msg>, row: Option<usize>) {
        if let Some(last) = cx.memory::<ContextMenuMemory>().levels.last_mut() {
            *last = row;
        }
    }

    fn key(&self, cx: &mut EventCx<'_, Msg>, key: &KeyEvent) -> bool {
        let levels = cx.memory::<ContextMenuMemory>().levels.clone();
        let depth = levels.len() - 1;
        let items = self.level(&levels, depth);
        let highlight = levels[depth];
        if key.is_plain(Key::Esc) || (key.is_plain(Key::Left) && depth > 0) {
            if depth == 0 {
                Self::close(cx);
            } else {
                let memory = cx.memory::<ContextMenuMemory>();
                memory.levels.pop();
                memory.opened_at.pop();
            }
        } else if key.is_plain(Key::Tab) {
            Self::close(cx);
            return false;
        } else if key.is_plain(Key::Up) {
            Self::set_highlight(cx, menu::step(items, highlight, false));
        } else if key.is_plain(Key::Down) {
            Self::set_highlight(cx, menu::step(items, highlight, true));
        } else if key.is_plain(Key::Home) {
            Self::set_highlight(cx, menu::edge(items, false));
        } else if key.is_plain(Key::End) {
            Self::set_highlight(cx, menu::edge(items, true));
        } else if key.is_plain(Key::Right) {
            self.enter_submenu(cx);
        } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
            self.activate(cx);
        } else if let (Some(typed), false) = (key.text, key.chord.mods.ctrl || key.chord.mods.alt)
            && let Some(row) = menu::type_ahead(items, highlight, typed)
        {
            Self::set_highlight(cx, Some(row));
        }
        true
    }

    /// Handles a press while open. Returns whether the press was used; a left press on the area
    /// outside the menu only closes it and goes on to what lies there.
    fn press(&self, cx: &mut EventCx<'_, Msg>, x: i32, y: i32, button: MouseButton) -> bool {
        let (rects, levels) = open_rects(cx.memory::<ContextMenuMemory>());
        // A press outside the menu closes it and is not swallowed.
        let Some(depth) = rects.iter().rposition(|rect| rect.contains(x, y)) else {
            Self::close(cx);
            if button != MouseButton::Right || !cx.area().contains(x, y) {
                return false;
            }
            cx.capture_pointer();
            self.open(cx, Rect::new(x, y, 1, 1), false);
            return true;
        };
        // The release after this press belongs to the menu, not to what lies under it.
        cx.capture_pointer();
        let row = usize::try_from(y - rects[depth].y).unwrap_or(0);
        let items = self.level(&levels, depth);
        if !items.get(row).is_some_and(ContextItem::selectable) {
            return true;
        }
        let memory = cx.memory::<ContextMenuMemory>();
        memory.levels.truncate(depth + 1);
        memory.opened_at.truncate(depth + 1);
        memory.levels[depth] = Some(row);
        self.activate(cx);
        true
    }

    /// Follows the pointer: the row under it is highlighted and a submenu under it opens. Only
    /// when the pointer moved, so a resting pointer does not undo keyboard moves.
    fn follow_pointer(&self, cx: &mut PaintCx<'_>) {
        let pointer = cx.pointer();
        let now = cx.now();
        let (moved, rects, mut levels) = {
            let memory = cx.memory::<ContextMenuMemory>();
            let moved = pointer.is_some() && pointer != memory.last_pointer;
            memory.last_pointer = pointer;
            let (rects, levels) = open_rects(memory);
            (moved, rects, levels)
        };
        let Some((x, y)) = pointer.filter(|_| moved) else {
            return;
        };
        let Some(depth) = rects.iter().rposition(|rect| rect.contains(x, y)) else {
            return;
        };
        let row = usize::try_from(y - rects[depth].y).unwrap_or(0);
        let item = self.level(&levels, depth).get(row).filter(|item| item.selectable());
        let opens = item.is_some_and(ContextItem::has_submenu);
        let unchanged = levels.get(depth) == Some(&item.map(|_| row)) && (levels.len() == depth + 2) == opens;
        if unchanged {
            return;
        }
        levels.truncate(depth + 1);
        levels[depth] = item.map(|_| row);
        let memory = cx.memory::<ContextMenuMemory>();
        memory.opened_at.truncate(depth + 1);
        if opens {
            levels.push(None);
            memory.opened_at.push(now);
        }
        memory.levels = levels;
    }
}

/// Whether `key` opens a context menu from the keyboard: Shift+F10 or the menu key.
pub(crate) fn is_menu_key(key: &KeyEvent) -> bool {
    let shift = Modifiers { shift: true, ..Modifiers::default() };
    (key.chord.key == Key::F(10) && key.chord.mods == shift) || key.is_plain(Key::Menu)
}

/// Whether `(x, y)` is on a level of the context menu kept in the memory of the widget handling an
/// event, as painted last frame.
pub(crate) fn contains<Msg>(cx: &mut EventCx<'_, Msg>, x: i32, y: i32) -> bool {
    let (rects, _) = open_rects(cx.memory::<ContextMenuMemory>());
    rects.iter().any(|rect| rect.contains(x, y))
}

/// The rects of the levels painted last frame that are still open, and the open levels. Events
/// arrive in batches between frames, so a key that closed a submenu can come before a press on
/// where that submenu was drawn.
fn open_rects(memory: &ContextMenuMemory) -> (Vec<Rect>, Vec<Option<usize>>) {
    let open = memory.rects.len().min(memory.levels.len());
    (memory.rects[..open].to_vec(), memory.levels.clone())
}

// Menus driven by the widget that owns them, for the edit menus of text widgets.
/// Whether the context menu kept in the memory of the widget being painted is open.
pub(crate) fn is_open(cx: &mut PaintCx<'_>) -> bool {
    cx.memory::<ContextMenuMemory>().open
}

/// Whether the context menu kept in the memory of the widget handling an event is open.
pub(crate) fn is_open_in<Msg>(cx: &mut EventCx<'_, Msg>) -> bool {
    cx.memory::<ContextMenuMemory>().open
}

impl<Msg: Clone + 'static> Container<Msg> for ContextMenu<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.width = Length::Fill(1);
        column.layout.height = Length::Fill(1);
        self.body = vec![column];
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ContextMenu<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.body.first().map_or(Size::default(), |body| cx.measure_child(body, available))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        // Plain content still receives the right click; interactive children sit on top.
        cx.register_hit(area);
        if let Some(body) = self.body.first() {
            cx.paint_child(body, area);
        }
        if cx.memory::<ContextMenuMemory>().open {
            cx.request_overlay(area);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, _area: Rect) {
        let screen = cx.clip();
        // Only the menu rows take presses (`menu::paint` registers them); the runtime closes the
        // menu on a press anywhere else and passes that press on.
        self.follow_pointer(cx);
        let (anchor, levels, opened_at) = {
            let memory = cx.memory::<ContextMenuMemory>();
            (memory.anchor, memory.levels.clone(), memory.opened_at.clone())
        };
        let enter = cx.env().theme().motion().enter;
        let mut rects = Vec::new();
        let mut parent_row = anchor;
        for depth in 0..levels.len() {
            let items = self.level(&levels, depth);
            if items.is_empty() {
                break;
            }
            let preferred = if depth == 0 { Placement::Below } else { Placement::Right };
            let (full, side) = placement::place(parent_row, menu::size(cx, items), screen, preferred);
            let started = opened_at.get(depth).copied().unwrap_or_default();
            let progress = cx.progress_since(started, enter, Easing::EaseOut);
            // Submenus beside their row unfold downwards too.
            let shown =
                placement::unfold(full, if side == Placement::Above { side } else { Placement::Below }, progress);
            menu::paint(cx, items, full, shown, levels[depth]);
            rects.push(full);
            let Some(row) = levels[depth] else {
                break;
            };
            parent_row = full.row(clamp_u16(i32::try_from(row).unwrap_or(i32::MAX)));
        }
        cx.memory::<ContextMenuMemory>().rects = rects;
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let open = cx.memory::<ContextMenuMemory>().open;
        match event {
            Event::PointerOutside => {
                Self::close(cx);
                true
            }
            Event::Key(key) if open => self.key(cx, key),
            Event::Key(key) => {
                if !is_menu_key(key) || self.items.is_empty() {
                    return false;
                }
                let area = cx.area();
                let anchor =
                    cx.focused_area().filter(|rect| !rect.is_empty()).unwrap_or(Rect::new(area.x, area.y, 1, 1));
                self.open(cx, anchor, true);
                true
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseKind::Down(button) if open => self.press(cx, mouse.x, mouse.y, button),
                MouseKind::Down(MouseButton::Right) if !self.items.is_empty() => {
                    self.open(cx, Rect::new(mouse.x, mouse.y, 1, 1), false);
                    true
                }
                MouseKind::Up(_) | MouseKind::Drag(_) | MouseKind::ScrollUp | MouseKind::ScrollDown => open,
                _ => false,
            },
            Event::Paste(_) => false,
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.body
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    #[derive(Default)]
    struct Demo {
        chosen: Vec<&'static str>,
        pressed: u32,
    }

    #[derive(Clone)]
    enum Msg {
        Chose(&'static str),
        Press,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Chose(name) => self.chosen.push(name),
                Msg::Press => self.pressed += 1,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let items = [
                ContextItem::new("Restart", Msg::Chose("restart")).shortcut("ctrl r"),
                ContextItem::new("Pause", Msg::Chose("pause")).disabled(true),
                ContextItem::submenu("Move to", [ContextItem::new("Staging", Msg::Chose("staging"))]),
                ContextItem::gap(),
                ContextItem::new("Delete", Msg::Chose("delete")).danger(true),
            ];
            ui.column(|ui| {
                ui.add_with(ContextMenu::new(items), |ui| {
                    ui.add(Text::new("api-gateway"));
                    ui.add(Button::new("Logs").on_press(Msg::Press)).id("logs");
                })
                .height(Length::Cells(10))
                .fill_width();
            })
            .fill_width();
        }
    }

    fn right_click(h: &mut Harness<Demo>, x: i32, y: i32) {
        h.mouse(MouseKind::Down(MouseButton::Right), x, y);
        h.mouse(MouseKind::Up(MouseButton::Right), x, y);
    }

    #[test]
    fn right_click_opens_at_the_pointer_with_shortcuts_anchored_right() {
        let mut h = Harness::new(Demo::default(), 40, 12);
        right_click(&mut h, 3, 0);
        h.advance(Duration::from_millis(200));
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        // The menu opens one row under the pointer and covers what is below it.
        assert_eq!(lines[1], "  L  Restart    ctrl r", "{screen}");
        assert_eq!(lines[2], "     Pause");
        assert_eq!(lines[3], "     Move to         ▶");
        assert_eq!(lines[4], "");
        assert_eq!(lines[5], "     Delete");
        let theme = h.env().theme();
        assert_eq!(h.bg(3, 1), theme.color("overlay"));
        assert_eq!(h.fg(5, 5), theme.color("danger"));
        assert_eq!(h.fg(5, 2), theme.color("muted"));
        assert_eq!(h.fg(16, 1), theme.color("muted"));
    }

    #[test]
    fn keyboard_skips_disabled_rows_and_gaps_and_chooses() {
        let mut h = Harness::new(Demo::default(), 40, 12);
        h.set_reduced_motion(true);
        right_click(&mut h, 3, 0);
        h.press("down").press("down");
        let screen = h.screen();
        assert!(screen.lines().nth(3).is_some_and(|line| line.starts_with("   ▌  Move to")), "{screen}");
        h.press("down").press("enter");
        assert_eq!(h.app().chosen, vec!["delete"]);
        assert!(!h.screen().contains("Restart"));
    }

    #[test]
    fn submenus_open_with_right_and_close_with_left() {
        let mut h = Harness::new(Demo::default(), 40, 12);
        h.set_reduced_motion(true);
        right_click(&mut h, 3, 0);
        h.press("m").press("right");
        assert!(h.screen().contains("Staging"), "{}", h.screen());
        h.press("left");
        assert!(!h.screen().contains("Staging"));
        h.press("enter").press("enter");
        assert_eq!(h.app().chosen, vec!["staging"]);
    }

    #[test]
    fn hovering_a_submenu_opens_it_and_clicks_choose() {
        let mut h = Harness::new(Demo::default(), 40, 12);
        h.set_reduced_motion(true);
        right_click(&mut h, 3, 0);
        h.hover(8, 3);
        let (x, y) = h.find("Staging").expect("submenu opened on hover");
        h.click(x, y);
        assert_eq!(h.app().chosen, vec!["staging"]);
    }

    #[test]
    fn outside_press_closes_and_still_reaches_the_button() {
        let mut h = Harness::new(Demo::default(), 40, 12);
        h.set_reduced_motion(true);
        right_click(&mut h, 20, 8);
        assert!(h.screen().contains("Restart"));
        h.click_text("Logs");
        assert_eq!(h.app().pressed, 1, "one press closes the menu and presses the button");
        assert!(!h.screen().contains("Restart"));
        assert!(h.app().chosen.is_empty());
    }

    #[test]
    fn shift_f10_opens_next_to_the_focused_widget() {
        let mut h = Harness::new(Demo::default(), 40, 12);
        h.set_reduced_motion(true);
        h.press("tab").press("shift+f10");
        let screen = h.screen();
        assert!(screen.lines().nth(2).is_some_and(|line| line.starts_with("▌  Restart   ctrl r")), "{screen}");
        h.press("esc");
        assert!(!h.screen().contains("Restart"));
        assert!(h.is_focused("logs"));
    }

    #[test]
    fn flips_above_near_the_bottom() {
        let mut h = Harness::new(Demo::default(), 40, 12);
        h.set_reduced_motion(true);
        right_click(&mut h, 3, 9);
        let (_, y) = h.find("Delete").expect("open");
        assert_eq!(y, 8);
    }
}
