//! The key binding overview opened with `?`.

use crate::event::{Event, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, Scope};
use crate::text;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::cells;
use super::editor::Editor;
use super::filter;
use super::layer::{self, Backdrop, BarDrag, SurfacePosition};
use super::rows::WHEEL_ROWS;
use super::scrollbar::{self, ScrollMetrics};

/// Width of the layer when none is set, in cells.
const DEFAULT_WIDTH: u16 = 64;

/// Most rows of bindings shown before the list scrolls.
const MAX_ROWS: u16 = 18;

/// Widest key column, in cells; longer chord lists are cut.
const MAX_KEYS_WIDTH: u16 = 24;

/// A layer listing every key binding: the keys of the focused widget and the hints of the current
/// screen first, then the application's keymap actions, then the framework's. Labels come from the
/// language files (`keys.<action>`, `quvyta.keys.<action>` and the widget's own
/// `quvyta.widget.<label>`), keys from the keymap, so rebinding a key or switching the language
/// updates the list.
///
/// Typing filters the list with fuzzy matching (matched characters take the match colour);
/// ↑/↓, PgUp/PgDn and the wheel scroll it. It is dismissable by default: Esc and the close mark
/// `×` at the top right send the close message; [`dismissable(false)`](HelpLayer::dismissable)
/// turns both off. Add it to the view while it should be shown; it opens as a modal layer (see
/// [`Modal`](super::Modal)), with the same pillar down its left edge. Applications usually open it
/// from the global `help` action, bound to `?`.
///
/// A widget that takes keys of its own declares them with [`Widget::keys`], so a screen needs no
/// hint for them and the list cannot fall behind the widget; [`hint`](Self::hint) stays for the
/// keys of a screen that are not a widget's, such as the `r` of an application that restarts the
/// selected row. The focused widget is the one whose keys are listed, which is why the widget
/// under the layer is asked and not the layer itself.
///
/// Style keys: `modal`, `modal-title`, `close-mark`, `layer-backdrop`, `layer-filter`, `layer-filter-mark`,
/// `layer-filter-placeholder`, `layer-filter-cursor`, `layer-match`, `help-group`, `help-key`,
/// `help-label`, `layer-hint-key`, `layer-hint-label`. Text: `quvyta.help.*`,
/// `quvyta.layer.*`, `quvyta.widget.*`.
pub struct HelpLayer<Msg> {
    hints: Vec<(String, String)>,
    width: u16,
    dismissable: bool,
    on_close: Msg,
}

#[derive(Debug, Default)]
struct HelpMemory {
    editor: Editor,
    offset: usize,
    /// Rows shown and the largest offset in the last frame, for scrolling between frames.
    visible: usize,
    max_offset: usize,
    list: Rect,
    bar: BarDrag,
}

/// The keys of a binding and its label.
type Binding = (Vec<String>, String);

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    Group(String),
    Binding { keys: Vec<String>, label: String, positions: Vec<usize> },
}

impl<Msg: Clone + 'static> HelpLayer<Msg> {
    /// A help layer; Esc and its close mark send `on_close`.
    #[must_use]
    pub fn new(on_close: Msg) -> Self {
        Self { hints: Vec::new(), width: DEFAULT_WIDTH, dismissable: true, on_close }
    }

    /// Whether Esc and the close mark close the layer; `true` by default. With `false` neither
    /// works, the mark is not drawn and the application closes the layer itself.
    #[must_use]
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
        self
    }

    /// Adds a key of the current screen that no widget declares, e.g. `("↑↓", "move")` for the keys
    /// an application reads itself. These come under "This screen", after the keys of the focused
    /// widget.
    #[must_use]
    pub fn hint(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.hints.push((key.into(), label.into()));
        self
    }

    /// Width in cells, padding included; 64 by default. Narrow screens shrink it.
    #[must_use]
    pub fn width(mut self, cells: u16) -> Self {
        self.width = cells;
        self
    }

    /// The rows for `query`: groups with at least one matching binding.
    fn rows(&self, cx: &PaintCx<'_>, query: &str) -> Vec<Row> {
        let env = cx.env();
        let i18n = env.i18n();
        let mut groups: Vec<(String, Vec<Binding>)> = Vec::new();
        // The widget the keyboard is with knows its own keys; hints are for the rest of the screen.
        let screen: Vec<Binding> = cx
            .declared_keys()
            .iter()
            .map(|key| (vec![key.keys.clone()], key.label.clone()))
            .chain(self.hints.iter().map(|(key, label)| (vec![key.clone()], label.clone())))
            .collect();
        groups.push((i18n.translate("quvyta.help.screen", &[]), screen));
        for (scope, group) in [(Scope::App, "quvyta.help.app"), (Scope::Global, "quvyta.help.global")] {
            let bindings = env
                .keymap()
                .iter()
                .filter(|(s, _, chords)| *s == scope && !chords.is_empty())
                .map(|(_, action, chords)| {
                    (
                        chords.iter().map(crate::keymap::KeyChord::label).collect(),
                        i18n.translate(&scope.label_key(action), &[]),
                    )
                })
                .collect();
            groups.push((i18n.translate(group, &[]), bindings));
        }
        let mut rows = Vec::new();
        for (title, bindings) in groups {
            let matched: Vec<Row> = bindings
                .into_iter()
                .filter_map(|(keys, label)| {
                    let positions = match filter::fuzzy(query, &label) {
                        Some(found) => found.positions,
                        None => {
                            filter::fuzzy(query, &keys.join(" "))?;
                            Vec::new()
                        }
                    };
                    Some(Row::Binding { keys, label, positions })
                })
                .collect();
            if !matched.is_empty() {
                rows.push(Row::Group(title));
                rows.extend(matched);
            }
        }
        rows
    }

    fn scroll(cx: &mut EventCx<'_, Msg>, to: impl FnOnce(usize, usize) -> usize) {
        let memory = cx.memory::<HelpMemory>();
        memory.offset = to(memory.offset, memory.visible.max(1)).min(memory.max_offset);
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for HelpLayer<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, _available: Size) -> Size {
        Size::default()
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.request_overlay(area);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, _anchor: Rect) {
        let screen = cx.clip();
        let query = cx.memory::<HelpMemory>().editor.text().to_owned();
        let rows = self.rows(cx, &query);
        let padding = layer::padding(cx, "modal", self.dismissable);
        // Title, filter and hint line, each with a blank row after or before it.
        let chrome = padding.vertical().saturating_add(6);
        let room = screen.height.saturating_sub(chrome.saturating_add(2)).clamp(1, MAX_ROWS);
        let visible = clamp_u16(i32::try_from(rows.len()).unwrap_or(i32::MAX)).clamp(1, room);
        let width = self.width.min(screen.width.saturating_sub(2));
        let look = layer::Look { style: "modal", variant: None, dismissable: self.dismissable };
        let surface = layer::open(cx, Size::new(width, chrome.saturating_add(visible)), SurfacePosition::Center, look);
        let inner = surface.inner;
        let list = Rect::new(inner.x, inner.y + 4, inner.width, visible);
        let bar = Rect::new(list.right() - 1, list.y, 1, list.height);
        let metrics = {
            let memory = cx.memory::<HelpMemory>();
            if surface.fresh {
                *memory = HelpMemory::default();
            }
            let metrics = ScrollMetrics { total: rows.len(), visible: usize::from(visible), offset: memory.offset };
            memory.max_offset = metrics.max_offset();
            memory.offset = memory.offset.min(memory.max_offset);
            memory.visible = usize::from(visible);
            memory.list = list;
            memory.bar.place(metrics.overflows().then_some(bar));
            ScrollMetrics { offset: memory.offset, ..metrics }
        };
        let editor = cx.memory::<HelpMemory>().editor.clone();
        let bar_active = cx.memory::<HelpMemory>().bar.active(cx.pointer_anywhere());
        cx.with_clip(surface.shown, |cx| {
            let title = cx.env().i18n().translate("quvyta.help.title", &[]);
            layer::title(cx, inner.x, inner.y, inner.width, &title);
            let placeholder = cx.env().i18n().translate("quvyta.layer.filter", &[]);
            filter::paint(cx, Rect::new(inner.x, inner.y + 2, inner.width, 1), &editor, &placeholder);
            let content_width = if metrics.overflows() { list.width.saturating_sub(2) } else { list.width };
            if rows.is_empty() {
                let empty = cx.env().i18n().translate("quvyta.help.empty", &[]);
                let style = cx.style("help-label", None, &[]).text();
                cx.text(list.x, list.y, &empty, style, content_width);
            }
            let keys_width = rows
                .iter()
                .map(|row| match row {
                    Row::Binding { keys, .. } => cells::sum(keys.iter().map(|key| text::width(key).saturating_add(3))),
                    Row::Group(_) => 0,
                })
                .max()
                .unwrap_or(0)
                .min(MAX_KEYS_WIDTH);
            let group_style = cx.style("help-group", None, &[]).text();
            let key_style = cx.style("help-key", None, &[]).text();
            let label_style = cx.style("help-label", None, &[]).text();
            for (line, row) in rows.iter().skip(metrics.offset).take(usize::from(visible)).enumerate() {
                let y = list.y + i32::try_from(line).unwrap_or(0);
                match row {
                    Row::Group(title) => {
                        cx.text(list.x, y, title, group_style, content_width);
                    }
                    Row::Binding { keys, label, positions } => {
                        let mut x = list.x + 2;
                        let keys_end = x + i32::from(keys_width);
                        for key in keys {
                            let chip = format!(" {key} ");
                            let chip_width = text::width(&chip);
                            if x + i32::from(chip_width) > keys_end {
                                break;
                            }
                            x += i32::from(cx.text(x, y, &chip, key_style, chip_width)) + 1;
                        }
                        let label_x = keys_end + 2;
                        let budget = clamp_u16(list.x + i32::from(content_width) - label_x);
                        filter::paint_matched(cx, label_x, y, label, budget, positions, label_style);
                    }
                }
            }
            if metrics.overflows() {
                scrollbar::paint(cx, bar, metrics, bar_active, None);
            }
            let mut hints = Vec::new();
            if self.dismissable {
                hints.push(layer::hint(cx, "esc", "close"));
            }
            if metrics.overflows() {
                hints.push(layer::hint(cx, &layer::arrows(cx), "scroll"));
            }
            layer::paint_hints(cx, inner.x, inner.bottom() - 1, inner.width, &hints);
        });
        layer::finish(cx, &surface);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        match layer::backdrop_event(cx, event, self.dismissable, false) {
            Backdrop::Close => {
                cx.emit(self.on_close.clone());
                return true;
            }
            Backdrop::Swallowed | Backdrop::Inside => {
                let Event::Mouse(mouse) = event else {
                    return true;
                };
                let (mut bar, metrics) = {
                    let memory = cx.memory::<HelpMemory>();
                    let total = memory.max_offset + memory.visible;
                    (memory.bar, ScrollMetrics { total, visible: memory.visible, offset: memory.offset })
                };
                let dragged = bar.event(cx, mouse, metrics);
                let memory = cx.memory::<HelpMemory>();
                memory.bar = bar;
                if let Some(offset) = dragged {
                    memory.offset = offset.min(memory.max_offset);
                } else if memory.list.contains(mouse.x, mouse.y) {
                    const WHEEL: usize = WHEEL_ROWS as usize;
                    match mouse.kind {
                        MouseKind::ScrollUp => Self::scroll(cx, |offset, _| offset.saturating_sub(WHEEL)),
                        MouseKind::ScrollDown => Self::scroll(cx, |offset, _| offset + WHEEL),
                        _ => {}
                    }
                }
                return true;
            }
            Backdrop::Ignored => {}
        }
        if let Event::Key(key) = event {
            if key.is_plain(Key::Up) {
                Self::scroll(cx, |offset, _| offset.saturating_sub(1));
                return true;
            }
            if key.is_plain(Key::Down) {
                Self::scroll(cx, |offset, _| offset + 1);
                return true;
            }
            if key.is_plain(Key::PageUp) {
                Self::scroll(cx, usize::saturating_sub);
                return true;
            }
            if key.is_plain(Key::PageDown) {
                Self::scroll(cx, |offset, page| offset + page);
                return true;
            }
        }
        let memory = cx.memory::<HelpMemory>();
        let before = memory.editor.text().len();
        let used = filter::edit(&mut memory.editor, event);
        if used && memory.editor.text().len() != before {
            memory.offset = 0;
        }
        used
    }

    fn focusable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Column, Table, TableRow, Text};

    #[derive(Default)]
    struct Demo {
        open: bool,
        saved: u32,
        firm: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Help,
        Close,
        Save,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Help => self.open = true,
                Msg::Close => self.open = false,
                Msg::Save => self.saved += 1,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.add(Text::new("Deploys"));
                ui.add(Button::new("Save").on_press(Msg::Save)).id("save");
                if self.open {
                    ui.add(HelpLayer::new(Msg::Close).dismissable(!self.firm).hint("↑↓", "move between deploys"));
                }
            });
        }
        fn action(&self, name: &str) -> Option<Msg> {
            match name {
                "help" => Some(Msg::Help),
                "save" => Some(Msg::Save),
                _ => None,
            }
        }
    }

    fn opened(height: u16) -> Harness<Demo> {
        opened_with(Demo::default(), height)
    }

    fn opened_with(demo: Demo, height: u16) -> Harness<Demo> {
        let mut env = crate::env::Env::builtin();
        env.keymap_mut().bind(Scope::App, "save", &["ctrl+s".parse().expect("chord")]);
        let mut h = Harness::with_env(demo, env, 70, height);
        h.press("?").advance(Duration::from_millis(200));
        h
    }

    #[test]
    fn lists_screen_hints_app_and_framework_bindings_in_groups() {
        let h = opened(30);
        let screen = h.screen();
        assert!(h.app().open, "the global help action reaches the application");
        for expected in [
            "Keyboard shortcuts",
            "This screen",
            "↑↓",
            "move between deploys",
            "Application",
            "ctrl s",
            "General",
            "ctrl q",
            "quit",
        ] {
            assert!(screen.contains(expected), "{expected} missing:\n{screen}");
        }
        assert!(screen.find("This screen") < screen.find("Application"));
        assert!(screen.find("Application") < screen.find("General"));
    }

    #[test]
    fn typing_filters_and_escape_closes() {
        let mut h = opened(30);
        h.type_text("qt");
        let screen = h.screen();
        assert!(screen.contains("quit") && !screen.contains("ctrl s"), "{screen}");
        assert!(!screen.contains("Application"), "empty groups are hidden");
        let (x, y) = h.find("quit").expect("quit");
        let accent = h.env().theme().color("accent");
        assert_eq!(h.fg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0)), accent, "q matched");
        h.type_text("zz");
        assert!(h.screen().contains("No matching keys"));
        h.press("ctrl+s");
        assert_eq!(h.app().saved, 0, "application shortcuts pause while help is open");
        h.press("esc");
        assert!(!h.app().open);
        assert!(!h.screen().contains("Keyboard shortcuts"));
    }

    #[test]
    fn long_lists_scroll_and_reopen_fresh() {
        let mut h = opened(14);
        let screen = h.screen();
        assert!(screen.contains("This screen") && !screen.contains("General"), "{screen}");
        for _ in 0..6 {
            h.press("pgdn");
        }
        assert!(h.screen().contains("alt b"), "the last binding comes into view: {}", h.screen());
        h.type_text("q").press("esc").press("?").advance(Duration::from_millis(200));
        assert!(h.screen().contains("This screen"), "reopening starts unfiltered at the top");
    }

    #[test]
    fn the_close_mark_sits_in_the_top_right_corner_lights_three_cells_and_closes() {
        let mut h = opened(30);
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let title = lines.iter().position(|line| line.contains("Keyboard shortcuts")).unwrap_or_default();
        assert!(lines[title - 1].ends_with('×') && !lines[title].contains('×'), "the row above the title: {screen}");
        let (x, y) = h.find("×").expect("close mark");
        let (column, row) = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
        let resting = h.bg(column, row);
        h.hover(x + 1, y);
        let lit = h.bg(column, row);
        assert_ne!(lit, resting);
        assert_eq!((h.bg(column - 1, row), h.bg(column + 1, row)), (lit, lit), "three cells light up");
        let pillar = h.env().theme().style("modal", None, &[]);
        assert!(pillar.get("pillar").is_some(), "the theme gives the surface a pillar");
        let (left, _) = h.find("▌").expect("pillar");
        assert!(
            screen.lines().filter(|line| line.chars().nth(usize::try_from(left).unwrap_or(0)) == Some('▌')).count()
                > 10
        );
        h.click(x - 1, y);
        assert!(!h.app().open, "a click on the mark closes");
    }

    #[test]
    fn a_help_layer_that_is_not_dismissable_ignores_escape_and_draws_no_mark() {
        let mut h = opened_with(Demo { firm: true, ..Demo::default() }, 30);
        let screen = h.screen();
        assert!(!screen.contains('×') && !screen.contains("esc close"), "{screen}");
        h.press("esc");
        assert!(h.app().open);
    }

    #[test]
    fn the_scrollbar_can_be_pressed_and_dragged() {
        let mut h = opened(14);
        assert!(!h.screen().contains("alt b"), "{}", h.screen());
        let (x, top) = scrollbar_column(&h);
        let bottom = top + 20;
        h.mouse(MouseKind::Down(crate::event::MouseButton::Left), x, top);
        h.mouse(MouseKind::Drag(crate::event::MouseButton::Left), x, bottom);
        assert!(h.screen().contains("alt b"), "dragging to the end shows the last binding:\n{}", h.screen());
        h.mouse(MouseKind::Up(crate::event::MouseButton::Left), x, bottom);
        assert!(h.app().open, "the drag released outside the surface does not close the layer");
        h.mouse(MouseKind::Down(crate::event::MouseButton::Left), x, top);
        h.mouse(MouseKind::Up(crate::event::MouseButton::Left), x, top);
        assert!(h.screen().contains("This screen"), "a press at the top scrolls back:\n{}", h.screen());
    }

    /// The scrollbar column (the list's last column, just left of the close mark's padding) and
    /// the list's first row, four rows under the title.
    fn scrollbar_column(h: &Harness<Demo>) -> (i32, i32) {
        let (_, title_y) = h.find("Keyboard shortcuts").expect("title");
        let (mark_x, _) = h.find("×").expect("close mark");
        (mark_x - 2, title_y + 4)
    }

    /// A screen whose keys belong to a table, with one key of its own that no widget takes.
    #[derive(Debug)]
    struct Rows {
        open: bool,
        /// Whether a row can be opened, which is what puts `enter` in the help.
        activates: bool,
        /// Check marks on the rows, and whether Space checks them (`Some(false)` leaves it to the
        /// application); `None` shows no marks.
        marks: Option<bool>,
        selected: Option<usize>,
        opened: Vec<usize>,
    }

    impl Default for Rows {
        fn default() -> Self {
            Self { open: false, activates: false, marks: None, selected: Some(0), opened: Vec::new() }
        }
    }

    #[derive(Clone)]
    enum RowMsg {
        Help,
        Close,
        Select(usize),
        Open(usize),
    }

    impl App for Rows {
        type Msg = RowMsg;
        fn update(&mut self, msg: RowMsg) -> Command<RowMsg> {
            match msg {
                RowMsg::Help => self.open = true,
                RowMsg::Close => self.open = false,
                RowMsg::Select(row) => self.selected = Some(row),
                RowMsg::Open(row) => self.opened.push(row),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, RowMsg>) {
            let rows = ["web", "api"].map(|name| TableRow::new([name]));
            let table = Table::new([Column::new("Deploy")], rows).selected(self.selected).on_select(RowMsg::Select);
            let table = if self.activates { table.on_activate(RowMsg::Open) } else { table };
            let table = match self.marks {
                Some(space) => table.checked(vec![false; 2]).on_toggle(RowMsg::Select).space_activates(space),
                None => table,
            };
            ui.column(|ui| {
                ui.add(table).id("rows");
                ui.add(Button::new("Save").on_press(RowMsg::Close)).id("save");
                if self.open {
                    ui.add(HelpLayer::new(RowMsg::Close).hint("r", "restart the deploy"));
                }
            });
        }
        fn action(&self, name: &str) -> Option<RowMsg> {
            (name == "help").then_some(RowMsg::Help)
        }
    }

    /// The lines of the "This screen" group, from its title to the next group.
    fn screen_group(h: &Harness<Rows>) -> String {
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let Some(start) = lines.iter().position(|line| line.contains("This screen")) else {
            return String::new();
        };
        let end = lines[start + 1..]
            .iter()
            .position(|line| line.contains("Application") || line.contains("General"))
            .map_or(lines.len(), |next| start + 1 + next);
        lines[start..end].join("\n")
    }

    /// A harness with the help open over the table, which Tab has focused.
    fn rows_with_help(activates: bool) -> Harness<Rows> {
        let mut h = Harness::new(Rows { activates, ..Rows::default() }, 70, 26);
        h.press("tab");
        assert!(h.is_focused("rows"), "Tab reaches the table");
        h.press("?").advance(Duration::from_millis(200));
        h
    }

    #[test]
    fn the_focused_widget_lists_its_own_keys_first() {
        let h = rows_with_help(true);
        assert!(h.app().open, "the global help action reaches the application");
        let group = screen_group(&h);
        for expected in ["↑↓", "move", "pgup pgdn", "scroll", "enter", "open"] {
            assert!(group.contains(expected), "{expected} missing from the focused widget's keys:\n{group}");
        }
        // The application wrote a hint for its own key only; the table's keys are there without one.
        assert!(group.contains("restart the deploy"), "the hint the application wrote is listed too:\n{group}");
        assert!(group.find("↑↓") < group.find("restart the deploy"), "the widget's keys come first:\n{group}");
    }

    #[test]
    fn a_table_that_cannot_open_a_row_has_no_enter_line() {
        let h = rows_with_help(false);
        let group = screen_group(&h);
        assert!(group.contains("↑↓"), "{group}");
        assert!(!group.contains("enter"), "a table with nothing to open says nothing about Enter:\n{group}");
    }

    #[test]
    fn a_table_that_leaves_space_to_the_application_lists_no_space_line() {
        let help = |marks| {
            let mut h = Harness::new(Rows { marks: Some(marks), ..Rows::default() }, 70, 26);
            h.press("tab").press("?").advance(Duration::from_millis(200));
            screen_group(&h)
        };
        let checking = help(true);
        assert!(checking.contains("space") && checking.contains("check"), "Space checks a row:\n{checking}");
        let leaving = help(false);
        assert!(leaving.contains("↑↓"), "the table's other keys are listed:\n{leaving}");
        assert!(!leaving.contains("space"), "Space is the application's, not the table's:\n{leaving}");
    }

    #[test]
    fn the_keys_of_the_widget_under_the_layer_stay_while_it_is_open() {
        let mut h = rows_with_help(true);
        // Focus moves into the layer once it is painted; the keys of the table under it are what
        // the person opened the help to read, so the filter narrows those and not the layer's own.
        h.press("pgdn").type_text("sc");
        let group = screen_group(&h);
        assert!(group.contains("scroll"), "{group}");
        assert!(!group.contains("move"), "the filter narrows the table's own keys too:\n{group}");
    }

    #[test]
    fn a_screen_with_the_focus_elsewhere_lists_no_widget_keys() {
        let mut h = Harness::new(Rows::default(), 70, 26);
        h.click_text("Save");
        assert!(h.is_focused("save"), "the button has the focus, not the table");
        h.press("?").advance(Duration::from_millis(200));
        let group = screen_group(&h);
        assert!(group.contains("restart the deploy"), "the hint the application wrote is listed:\n{group}");
        assert!(!group.contains("↑↓") && !group.contains("enter"), "a button takes no keys of its own:\n{group}");
    }

    #[test]
    fn the_keys_the_help_lists_are_the_keys_the_table_takes() {
        let mut h = Harness::new(Rows { activates: true, ..Rows::default() }, 70, 10);
        h.press("tab").press("down").press("enter");
        assert_eq!(h.app().selected, Some(1), "the row the help says `↑↓ move` reaches");
        assert_eq!(h.app().opened, [1], "the row the help says `enter open` opens");
    }

    #[test]
    fn a_terminal_that_shows_no_arrows_is_told_in_carets() {
        let mut h = rows_with_help(true);
        h.set_glyph_mode(GlyphMode::Ascii);
        let group = screen_group(&h);
        assert!(group.contains("^v") && group.contains("move"), "the arrows of the icon set in use:\n{group}");
        assert!(!group.contains('↑'), "a terminal that cannot draw an arrow is not shown one:\n{group}");
    }
}
