//! The command palette: find and run any command by typing a few letters of its name.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, KeyChord, Scope};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::cells;
use super::editor::Editor;
use super::filter;
use super::layer::{self, Backdrop, BarDrag, PointerGate, SurfacePosition};
use super::rows::WHEEL_ROWS;
use super::scrollbar::{self, ScrollMetrics};

/// Width of the palette when none is set, in cells.
const DEFAULT_WIDTH: u16 = 72;

/// Rows of commands shown before the list scrolls.
const MAX_ROWS: u16 = 10;

/// Global actions that make no sense as commands: moving focus inside the palette, or opening
/// the palette itself.
const HIDDEN_ACTIONS: [&str; 3] = ["focus-next", "focus-prev", "palette"];

/// Builds a message from the id of a command.
type IdMessage<Msg> = Box<dyn Fn(&str) -> Msg>;

/// One command of a [`CommandPalette`].
pub struct PaletteCommand<Msg> {
    id: String,
    label: String,
    chord: Option<String>,
    message: Msg,
}

impl<Msg> PaletteCommand<Msg> {
    /// A command named `label` that sends `message`; `id` identifies it for
    /// [`CommandPalette::recent`] and [`CommandPalette::on_run`].
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>, message: Msg) -> Self {
        Self { id: id.into(), label: label.into(), chord: None, message }
    }

    /// The key that runs the command outside the palette, shown on the right, e.g. `"ctrl r"`.
    #[must_use]
    pub fn chord(mut self, label: impl Into<String>) -> Self {
        self.chord = Some(label.into());
        self
    }
}

/// What running an entry does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// Sends the message of the application command at this index.
    Command(usize),
    /// Runs the keymap action at this index of the entries built from the keymap.
    Action(usize),
}

/// A row of the palette list.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    Header(String),
    Entry { target: Target, positions: Vec<usize> },
}

/// A keymap action offered as a command.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ActionEntry {
    scope: Scope,
    action: String,
    label: String,
    chord: Option<String>,
}

#[derive(Debug, Default)]
struct PaletteMemory {
    editor: Editor,
    /// The highlighted entry, counted among entries only.
    highlight: usize,
    offset: usize,
    visible: usize,
    list: Rect,
    /// The pointer moves the highlight only once it moves after the palette opened.
    pointer: PointerGate,
    bar: BarDrag,
}

/// A layer with a filter field over a list of commands, for running anything from the
/// keyboard.
///
/// Typing filters the commands with fuzzy matching, best matches first, matched characters in
/// the match colour. ↑/↓ (or Ctrl+P/Ctrl+N), PgUp/PgDn move the highlight, Enter runs the
/// highlighted command. The mouse does the same: moving the pointer over a row moves the one
/// highlight (a pointer resting where the palette opened waits until it moves), a click runs
/// the clicked command and the wheel scrolls. It is dismissable by default: Esc, the close
/// mark `×` at the top right and a click on the dimmed screen close it;
/// [`dismissable(false)`](CommandPalette::dismissable) turns all three off. Running sends the
/// close message first, then the command's message. Only visible rows are drawn, so thousands of
/// commands stay fast. Add it to the view while it should be shown; it opens as a modal layer
/// in the upper part of the screen. Applications usually open it from the global `palette`
/// action, bound to Ctrl+P.
///
/// With no options it lists the given commands. `keymap` adds the keymap's actions with
/// their keys; `recent` lists the given command ids first under "Recent" while nothing is
/// typed; `on_run` reports which command ran, e.g. to remember recent ones.
///
/// Style keys: `modal` (with its `pillar` down the left edge), `close-mark`, `layer-backdrop`,
/// `layer-filter`, `layer-filter-mark`, `layer-filter-placeholder`, `layer-filter-cursor`,
/// `layer-match`, `palette-item` with `hover` (`bg`, `fg`, `pillar`), `palette-chord`,
/// `palette-header`, `layer-hint-key`, `layer-hint-label`. Text: `quvyta.palette.*`,
/// `quvyta.layer.*`.
pub struct CommandPalette<Msg> {
    commands: Vec<PaletteCommand<Msg>>,
    keymap: bool,
    recent: Vec<String>,
    placeholder: Option<String>,
    width: u16,
    dismissable: bool,
    on_close: Msg,
    on_run: Option<IdMessage<Msg>>,
}

impl<Msg: Clone + 'static> CommandPalette<Msg> {
    /// A palette of `commands`; dismissing it (Esc, the close mark, a click outside) and running a
    /// command send `on_close`.
    #[must_use]
    pub fn new(commands: impl IntoIterator<Item = PaletteCommand<Msg>>, on_close: Msg) -> Self {
        Self {
            commands: commands.into_iter().collect(),
            keymap: false,
            recent: Vec::new(),
            placeholder: None,
            width: DEFAULT_WIDTH,
            dismissable: true,
            on_close,
            on_run: None,
        }
    }

    /// Also lists the keymap's actions (except moving focus and the palette itself), labelled
    /// from the language files and showing their first key. Running one runs the action as if
    /// its key had been pressed. Their ids are `action:<name>`.
    #[must_use]
    pub fn keymap(mut self, include: bool) -> Self {
        self.keymap = include;
        self
    }

    /// Ids of recently run commands, most recent first, listed on top while nothing is typed.
    #[must_use]
    pub fn recent(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.recent = ids.into_iter().map(Into::into).collect();
        self
    }

    /// The faint text of the empty filter; `quvyta.palette.placeholder` by default.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// Width in cells, padding included; 72 by default. Narrow screens shrink it.
    #[must_use]
    pub fn width(mut self, cells: u16) -> Self {
        self.width = cells;
        self
    }

    /// Whether Esc, the close mark and a click on the dimmed screen close the palette; `true` by
    /// default. With `false` none of them works and the mark is not drawn; running a command
    /// still sends the close message.
    #[must_use]
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
        self
    }

    /// The message sent with the id of every command that runs, after the command's own.
    #[must_use]
    pub fn on_run(mut self, message: impl Fn(&str) -> Msg + 'static) -> Self {
        self.on_run = Some(Box::new(message));
        self
    }

    fn actions(&self, env: &crate::env::Env) -> Vec<ActionEntry> {
        if !self.keymap {
            return Vec::new();
        }
        env.keymap()
            .iter()
            .filter(|(scope, action, _)| !(*scope == Scope::Global && HIDDEN_ACTIONS.contains(action)))
            .map(|(scope, action, chords)| ActionEntry {
                scope,
                action: action.to_owned(),
                label: env.i18n().translate(&scope.label_key(action), &[]),
                chord: chords.first().map(KeyChord::label),
            })
            .collect()
    }

    fn id(&self, target: Target, actions: &[ActionEntry]) -> String {
        match target {
            Target::Command(index) => self.commands[index].id.clone(),
            Target::Action(index) => format!("action:{}", actions[index].action),
        }
    }

    fn label<'a>(&'a self, target: Target, actions: &'a [ActionEntry]) -> &'a str {
        match target {
            Target::Command(index) => &self.commands[index].label,
            Target::Action(index) => &actions[index].label,
        }
    }

    /// The rows for `query`: best matches first, or recent commands and then all in order while
    /// the query is empty.
    fn rows(&self, env: &crate::env::Env, actions: &[ActionEntry], query: &str) -> Vec<Row> {
        let i18n = env.i18n();
        let targets: Vec<Target> =
            (0..self.commands.len()).map(Target::Command).chain((0..actions.len()).map(Target::Action)).collect();
        if !query.trim().is_empty() {
            let mut matched: Vec<(i32, usize, Row)> = targets
                .iter()
                .enumerate()
                .filter_map(|(order, target)| {
                    let found = filter::fuzzy(query, self.label(*target, actions))?;
                    Some((found.score, order, Row::Entry { target: *target, positions: found.positions }))
                })
                .collect();
            matched.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            return matched.into_iter().map(|(_, _, row)| row).collect();
        }
        let entry = |target: Target| Row::Entry { target, positions: Vec::new() };
        let recent: Vec<Target> = self
            .recent
            .iter()
            .filter_map(|id| targets.iter().find(|target| self.id(**target, actions) == *id).copied())
            .collect();
        if recent.is_empty() {
            return targets.into_iter().map(entry).collect();
        }
        let mut rows = vec![Row::Header(i18n.translate("quvyta.palette.recent", &[]))];
        rows.extend(recent.iter().copied().map(entry));
        rows.push(Row::Header(i18n.translate("quvyta.palette.all", &[])));
        rows.extend(targets.into_iter().filter(|target| !recent.contains(target)).map(entry));
        rows
    }

    /// The row index of entry number `entry`.
    fn row_of_entry(rows: &[Row], entry: usize) -> Option<usize> {
        rows.iter().enumerate().filter(|(_, row)| matches!(row, Row::Entry { .. })).nth(entry).map(|(index, _)| index)
    }

    fn entries(rows: &[Row]) -> usize {
        rows.iter().filter(|row| matches!(row, Row::Entry { .. })).count()
    }

    fn run(&self, cx: &mut EventCx<'_, Msg>, target: Target, actions: &[ActionEntry]) {
        cx.emit(self.on_close.clone());
        match target {
            Target::Command(index) => cx.emit(self.commands[index].message.clone()),
            Target::Action(index) => cx.run_action(actions[index].scope, actions[index].action.clone()),
        }
        if let Some(on_run) = &self.on_run {
            let id = self.id(target, actions);
            cx.emit(on_run(&id));
        }
    }

    /// Moves the highlight to entry `entry` and scrolls it into view.
    fn highlight(cx: &mut EventCx<'_, Msg>, rows: &[Row], entry: usize) {
        let count = Self::entries(rows);
        if count == 0 {
            return;
        }
        let entry = entry.min(count - 1);
        let row = Self::row_of_entry(rows, entry).unwrap_or(0);
        let memory = cx.memory::<PaletteMemory>();
        memory.highlight = entry;
        let visible = memory.visible.max(1);
        // Keep a header right above the first entry of its group in view too.
        let top = if row > 0 && matches!(rows[row - 1], Row::Header(_)) { row - 1 } else { row };
        if top < memory.offset {
            memory.offset = top;
        } else if row >= memory.offset + visible {
            memory.offset = row + 1 - visible;
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for CommandPalette<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, _available: Size) -> Size {
        Size::default()
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.request_overlay(area);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, _anchor: Rect) {
        let screen = cx.clip();
        let actions = self.actions(cx.env());
        let query = cx.memory::<PaletteMemory>().editor.text().to_owned();
        let rows = self.rows(cx.env(), &actions, &query);
        let padding = layer::padding(cx, "modal", self.dismissable);
        // The filter and the hint line, each with a blank row.
        let chrome = padding.vertical().saturating_add(4);
        let room = screen.height.saturating_sub(chrome.saturating_add(2)).clamp(1, MAX_ROWS);
        let visible = clamp_u16(i32::try_from(rows.len()).unwrap_or(i32::MAX)).clamp(1, room);
        let width = self.width.min(screen.width.saturating_sub(2));
        let look = layer::Look { style: "modal", variant: None, dismissable: self.dismissable };
        let surface = layer::open(cx, Size::new(width, chrome.saturating_add(visible)), SurfacePosition::Top, look);
        let inner = surface.inner;
        let list =
            Rect::new(inner.x - i32::from(padding.left), inner.y + 2, inner.width + padding.horizontal(), visible);
        // The scrollbar sits in the list's right padding, one cell in from the surface's edge.
        let bar = Rect::new(list.right() - i32::from(padding.right.max(2)) + 1, list.y, 1, list.height);
        let pointer = cx.pointer_anywhere();
        let (metrics, highlight, editor) = {
            let memory = cx.memory::<PaletteMemory>();
            if surface.fresh {
                *memory = PaletteMemory { pointer: PointerGate::new(pointer), ..PaletteMemory::default() };
            }
            memory.highlight = memory.highlight.min(Self::entries(&rows).saturating_sub(1));
            let total = rows.len();
            memory.offset = memory.offset.min(total.saturating_sub(usize::from(visible)));
            memory.visible = usize::from(visible);
            memory.list = list;
            // The pointer carries the one highlight when it moves onto an entry.
            let moved = memory.pointer.moved(pointer);
            // The scrollbar column is not a row: dragging it must not move the highlight.
            let rows_end = if total > usize::from(visible) { bar.x } else { list.right() };
            let hovered = pointer
                .filter(|(x, y)| moved && list.contains(*x, *y) && *x < rows_end)
                .and_then(|(_, y)| usize::try_from(y - list.y).ok())
                .map(|row| memory.offset + row)
                .filter(|row| matches!(rows.get(*row), Some(Row::Entry { .. })))
                .map(|row| rows[..row].iter().filter(|r| matches!(r, Row::Entry { .. })).count());
            if let Some(entry) = hovered {
                memory.highlight = entry;
            }
            (
                ScrollMetrics { total, visible: usize::from(visible), offset: memory.offset },
                memory.highlight,
                memory.editor.clone(),
            )
        };
        let highlighted_row = Self::row_of_entry(&rows, highlight);
        let slide = cx.env().slide();
        cx.with_clip(surface.shown, |cx| {
            let placeholder = self
                .placeholder
                .clone()
                .unwrap_or_else(|| cx.env().i18n().translate("quvyta.palette.placeholder", &[]));
            filter::paint(cx, Rect::new(inner.x, inner.y, inner.width, 1), &editor, &placeholder);
            let content_width =
                if metrics.overflows() { list.width.saturating_sub(padding.right.max(2)) } else { list.width };
            if rows.is_empty() {
                let empty = cx.env().i18n().translate("quvyta.palette.empty", &[]);
                let style = cx.style("palette-header", None, &[]).text();
                cx.text(inner.x, list.y, &empty, style, inner.width);
            }
            for (line, row) in rows.iter().enumerate().skip(metrics.offset).take(usize::from(visible)) {
                let y = list.y + i32::try_from(line - metrics.offset).unwrap_or(0);
                let rect = Rect::new(list.x, y, content_width, 1);
                match row {
                    Row::Header(title) => {
                        let style = cx.style("palette-header", None, &[]).text();
                        cx.text(inner.x, y, title, style, inner.width);
                    }
                    Row::Entry { target, positions } => {
                        // One highlight, moved by the keyboard and by the pointer alike.
                        let states = if highlighted_row == Some(line) { vec![State::Hover] } else { Vec::new() };
                        let style = cx.style("palette-item", None, &states);
                        let text_style = style.text();
                        if let Some(bg) = text_style.bg {
                            cx.fill(rect, bg);
                        }
                        if let Some(color) = style.color("pillar") {
                            cx.pillar(rect.x, y, color);
                        }
                        let chord = match target {
                            Target::Command(index) => self.commands[*index].chord.clone(),
                            Target::Action(index) => actions[*index].chord.clone(),
                        };
                        let chord_width = chord.as_deref().map_or(0, text::width);
                        if let Some(chord) = &chord {
                            let chord_style = cx.style("palette-chord", None, &states).text();
                            let x = inner.right() - i32::from(chord_width);
                            cx.text(x, y, chord, CellStyle { bg: None, ..chord_style }, chord_width);
                        }
                        let shift = u16::from(slide && !states.is_empty());
                        let label_end = inner.right() - i32::from(chord_width) - if chord_width > 0 { 2 } else { 0 };
                        // The label is cut at the same place resting and raised, one cell short of its
                        // room, so the sliding label never touches the chord.
                        let budget = clamp_u16(label_end - inner.x - 1);
                        let label = self.label(*target, &actions);
                        filter::paint_matched(cx, inner.x + i32::from(shift), y, label, budget, positions, text_style);
                    }
                }
            }
            if metrics.overflows() {
                let active = {
                    let memory = cx.memory::<PaletteMemory>();
                    memory.bar.place(Some(bar));
                    memory.bar
                }
                .active(cx.pointer_anywhere());
                scrollbar::paint(cx, bar, metrics, active, None);
            } else {
                cx.memory::<PaletteMemory>().bar.place(None);
            }
            let mut hints = Vec::new();
            if self.dismissable {
                hints.push(layer::hint(cx, "esc", "close"));
            }
            hints.extend([layer::hint(cx, "↑↓", "move"), layer::hint(cx, "⏎", "run")]);
            layer::paint_hints(cx, inner.x, inner.bottom() - 1, inner.width, &hints);
            let count = format!("{} / {}", Self::entries(&rows), self.commands.len() + actions.len());
            let count_style = cx.style("layer-hint-label", None, &[]).text();
            let count_width = text::width(&count);
            if cells::sum([layer::hints_width(&hints), count_width, 3]) <= inner.width {
                cx.text(inner.right() - i32::from(count_width), inner.bottom() - 1, &count, count_style, count_width);
            }
        });
        layer::finish(cx, &surface);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let actions = self.actions(cx.env());
        let query = cx.memory::<PaletteMemory>().editor.text().to_owned();
        let rows = self.rows(cx.env(), &actions, &query);
        let highlight = cx.memory::<PaletteMemory>().highlight;
        match layer::backdrop_event(cx, event, self.dismissable, true) {
            Backdrop::Close => {
                cx.emit(self.on_close.clone());
                return true;
            }
            Backdrop::Swallowed | Backdrop::Inside => {
                let Event::Mouse(mouse) = event else {
                    return true;
                };
                let (list, offset, mut bar, visible) = {
                    let memory = cx.memory::<PaletteMemory>();
                    (memory.list, memory.offset, memory.bar, memory.visible)
                };
                let metrics = ScrollMetrics { total: rows.len(), visible, offset };
                let dragged = bar.event(cx, mouse, metrics);
                let memory = cx.memory::<PaletteMemory>();
                memory.bar = bar;
                if let Some(offset) = dragged {
                    memory.offset = offset.min(metrics.max_offset());
                    return true;
                }
                if !list.contains(mouse.x, mouse.y) {
                    return true;
                }
                match mouse.kind {
                    MouseKind::ScrollUp | MouseKind::ScrollDown => {
                        let memory = cx.memory::<PaletteMemory>();
                        memory.offset = if mouse.kind == MouseKind::ScrollUp {
                            memory.offset.saturating_sub(usize::from(WHEEL_ROWS))
                        } else {
                            memory.offset.saturating_add(usize::from(WHEEL_ROWS)).min(metrics.max_offset())
                        };
                    }
                    MouseKind::Down(MouseButton::Left) => {
                        let line = offset + usize::try_from(mouse.y - list.y).unwrap_or(0);
                        if let Some(Row::Entry { target, .. }) = rows.get(line) {
                            self.run(cx, *target, &actions);
                        }
                    }
                    _ => {}
                }
                return true;
            }
            Backdrop::Ignored => {}
        }
        if let Event::Key(key) = event {
            let page = cx.memory::<PaletteMemory>().visible.max(1);
            let ctrl = |c: char| key.chord.key == Key::Char(c) && key.chord.mods.ctrl && !key.chord.mods.alt;
            if key.is_plain(Key::Up) || ctrl('p') {
                Self::highlight(cx, &rows, highlight.saturating_sub(1));
                return true;
            }
            if key.is_plain(Key::Down) || ctrl('n') {
                Self::highlight(cx, &rows, highlight + 1);
                return true;
            }
            if key.is_plain(Key::PageUp) {
                Self::highlight(cx, &rows, highlight.saturating_sub(page));
                return true;
            }
            if key.is_plain(Key::PageDown) {
                Self::highlight(cx, &rows, highlight + page);
                return true;
            }
            if key.is_plain(Key::Enter) {
                if let Some(Row::Entry { target, .. }) = Self::row_of_entry(&rows, highlight).map(|row| &rows[row]) {
                    self.run(cx, *target, &actions);
                }
                return true;
            }
        }
        let memory = cx.memory::<PaletteMemory>();
        let before = memory.editor.text().to_owned();
        let used = filter::edit(&mut memory.editor, event);
        if memory.editor.text() != before {
            memory.highlight = 0;
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
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::Text;

    #[derive(Default)]
    struct Demo {
        open: bool,
        log: Vec<String>,
        recent: Vec<String>,
        count: usize,
        firm: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Open,
        Close,
        Run(String),
        Ran(String),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Open => self.open = true,
                Msg::Close => self.open = false,
                Msg::Run(name) => self.log.push(name),
                Msg::Ran(id) => {
                    self.recent.retain(|r| *r != id);
                    self.recent.insert(0, id);
                }
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.add(Text::new("Deploys"));
                if self.open {
                    let mut commands = vec![
                        PaletteCommand::new("restart", "Restart container", Msg::Run("restart".into())).chord("ctrl r"),
                        PaletteCommand::new("logs", "Open logs", Msg::Run("logs".into())),
                        PaletteCommand::new("deploy", "Deploy to staging", Msg::Run("deploy".into())),
                    ];
                    commands.extend((0..self.count).map(|i| {
                        PaletteCommand::new(
                            format!("image-{i}"),
                            format!("Pull image {i}"),
                            Msg::Run(format!("pull {i}")),
                        )
                    }));
                    ui.add(
                        CommandPalette::new(commands, Msg::Close)
                            .dismissable(!self.firm)
                            .keymap(true)
                            .recent(self.recent.clone())
                            .on_run(|id| Msg::Ran(id.to_owned())),
                    );
                }
            });
        }
        fn action(&self, name: &str) -> Option<Msg> {
            match name {
                "palette" => Some(Msg::Open),
                "help" => Some(Msg::Run("help".into())),
                _ => None,
            }
        }
    }

    fn opened(demo: Demo) -> Harness<Demo> {
        let mut h = Harness::new(demo, 70, 24);
        h.press("ctrl+p").advance(Duration::from_millis(200));
        h
    }

    #[test]
    fn lists_commands_and_keymap_actions_with_chords_right_aligned() {
        let h = opened(Demo::default());
        let screen = h.screen();
        for expected in ["Type a command", "Restart container", "ctrl r", "Open logs", "quit", "ctrl q", "keys"] {
            assert!(screen.contains(expected), "{expected}:\n{screen}");
        }
        assert!(!screen.contains("next"), "moving focus is not a command");
        let line = screen.lines().find(|line| line.contains("Restart container")).unwrap_or_default();
        assert!(line.starts_with("       ▌ ") || line.contains("▌"), "the first entry is highlighted: {line}");
    }

    #[test]
    fn filters_fuzzily_and_runs_with_enter() {
        let mut h = opened(Demo::default());
        h.type_text("dstg");
        let screen = h.screen();
        assert!(screen.contains("Deploy to staging") && !screen.contains("Open logs"), "{screen}");
        let (x, y) = h.find("Deploy to").expect("row");
        assert_eq!(h.fg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0)), h.env().theme().color("accent"));
        h.press("enter");
        assert_eq!(h.app().log, ["deploy"]);
        assert!(!h.app().open);
        assert_eq!(h.app().recent, ["deploy"]);
    }

    #[test]
    fn arrows_move_and_keymap_actions_run_like_their_keys() {
        let mut h = opened(Demo::default());
        h.type_text("keys").press("enter");
        assert_eq!(h.app().log, ["help"], "the help action ran through App::action");
        let mut h = opened(Demo::default());
        h.press("down").press("enter");
        assert_eq!(h.app().log, ["logs"]);
        let mut h = opened(Demo::default());
        h.type_text("zzzz");
        assert!(h.screen().contains("No matching commands"));
        h.press("enter").press("esc");
        assert!(!h.app().open && h.app().log.is_empty());
    }

    #[test]
    fn recent_commands_come_first_and_clicks_run() {
        let mut h = opened(Demo { recent: vec!["logs".into()], ..Demo::default() });
        let screen = h.screen();
        assert!(screen.find("Recent") < screen.find("Open logs"), "{screen}");
        assert!(screen.find("Open logs") < screen.find("All commands"));
        h.click_text("Deploy to staging");
        assert_eq!(h.app().log, ["deploy"]);
        let mut h = opened(Demo::default());
        h.click(1, 22);
        assert!(!h.app().open, "a click outside closes the palette");
    }

    #[test]
    fn thousands_of_commands_scroll_with_the_highlight() {
        let mut h = opened(Demo { count: 5000, ..Demo::default() });
        h.type_text("image 4999");
        assert!(h.screen().contains("Pull image 4999"), "{}", h.screen());
        h.press("ctrl+u");
        for _ in 0..30 {
            h.press("pgdn");
        }
        assert!(h.screen().contains("Pull image"), "{}", h.screen());
        h.press("enter");
        assert_eq!(h.app().log.len(), 1);
    }

    // The palette with the mouse.

    /// The rows raised by the highlight, told by their surface: every row carries the dialog's
    /// pillar, so the glyph alone says nothing.
    fn lit(h: &Harness<Demo>) -> Vec<String> {
        let active = h.env().theme().color("active");
        h.screen()
            .lines()
            .enumerate()
            .filter(|(y, _)| u16::try_from(*y).is_ok_and(|y| h.bg(10, y) == active))
            .map(|(_, line)| line.replace('▌', "").split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    }

    #[test]
    fn moving_the_pointer_moves_the_one_highlight_and_keys_continue_from_it() {
        let mut h = opened(Demo::default());
        assert_eq!(lit(&h), ["Restart container ctrl r"], "{}", h.screen());
        let (x, y) = h.find("Deploy to staging").expect("row");
        h.hover(x + 2, y);
        assert_eq!(lit(&h), ["Deploy to staging"], "only the hovered row is lit:\n{}", h.screen());
        h.press("up");
        assert_eq!(lit(&h), ["Open logs"], "the key moves on from the hovered row and the resting pointer waits");
        h.press("enter");
        assert_eq!(h.app().log, ["logs"]);
    }

    #[test]
    fn a_pointer_resting_where_the_palette_opens_changes_nothing_until_it_moves() {
        let mut probe = opened(Demo::default());
        let (x, y) = probe.find("Open logs").expect("row");
        let mut h = Harness::new(Demo::default(), 70, 24);
        h.hover(x, y).press("ctrl+p").advance(Duration::from_millis(200));
        assert_eq!(lit(&h), ["Restart container ctrl r"], "{}", h.screen());
        h.hover(x + 1, y);
        assert_eq!(lit(&h), ["Open logs"], "{}", h.screen());
        probe.press("esc");
    }

    #[test]
    fn a_click_runs_the_clicked_row_and_the_wheel_scrolls() {
        let mut h = opened(Demo { count: 40, ..Demo::default() });
        assert!(!h.screen().contains("Pull image 12"), "{}", h.screen());
        let (x, y) = h.find("Open logs").expect("row");
        for _ in 0..4 {
            h.mouse(MouseKind::ScrollDown, x, y);
        }
        assert!(h.screen().contains("Pull image 12") && !h.screen().contains("Open logs"), "{}", h.screen());
        h.mouse(MouseKind::ScrollUp, x, y);
        let (x, y) = h.find("Pull image 9").expect("scrolled row");
        h.click(x, y);
        assert_eq!(h.app().log, ["pull 9"]);
        assert!(!h.app().open);
    }

    #[test]
    fn escape_the_close_mark_and_the_dimmed_screen_close_only_while_dismissable() {
        let mut h = opened(Demo::default());
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let filter = lines.iter().position(|line| line.contains("Type a command")).unwrap_or_default();
        assert!(lines[filter - 1].ends_with('×'), "the mark sits on the surface's first row: {screen}");
        let (x, y) = h.find("×").expect("mark");
        h.hover(x, y);
        let (column, row) = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
        let lit = h.bg(column, row);
        assert_ne!(lit, h.bg(column - 2, row), "the mark lights up");
        assert_eq!((h.bg(column - 1, row), h.bg(column + 1, row)), (lit, lit), "all three cells light up");
        h.click(x, y);
        assert!(!h.app().open, "the mark closes");
        let mut h = opened(Demo { firm: true, ..Demo::default() });
        assert!(!h.screen().contains('×') && !h.screen().contains("esc close"), "{}", h.screen());
        h.press("esc").click(1, 22);
        assert!(h.app().open, "neither Esc nor the dimmed screen closes it");
        h.click_text("Open logs");
        assert_eq!((h.app().open, h.app().log.as_slice()), (false, &["logs".to_owned()][..]), "running still closes");
    }

    #[test]
    fn the_scrollbar_can_be_dragged_without_moving_the_highlight() {
        let mut h = opened(Demo { count: 40, ..Demo::default() });
        let (mark_x, _) = h.find("×").expect("close mark");
        let (_, top) = h.find("Restart container").expect("first row");
        // The bar sits in the list's right padding, under the close mark's glyph.
        let x = mark_x;
        h.mouse(MouseKind::Down(MouseButton::Left), x, top);
        h.mouse(MouseKind::Drag(MouseButton::Left), x, top + 30);
        assert!(h.screen().contains("Pull image 39"), "dragged to the end:\n{}", h.screen());
        h.mouse(MouseKind::Up(MouseButton::Left), x, top + 30);
        assert!(h.app().open && h.app().log.is_empty(), "nothing ran and the palette stayed");
        h.press("enter");
        assert_eq!(h.app().log, ["restart"], "the highlight stayed on the first entry");
    }

    #[test]
    fn a_raised_entry_is_cut_at_the_same_place_as_a_resting_one() {
        let mut h = Harness::new(Demo::default(), 28, 24);
        h.press("ctrl+p").advance(Duration::from_millis(200));
        let row = |h: &Harness<Demo>| {
            let screen = h.screen();
            let line = screen.lines().find(|line| line.contains("Restart c")).unwrap_or_default().replace('▌', " ");
            line.split("ctrl").next().unwrap_or_default().trim().to_owned()
        };
        let raised = row(&h);
        h.press("down");
        let resting = row(&h);
        assert!(resting.contains('…'), "{}", h.screen());
        assert_eq!(raised, resting, "{}", h.screen());
    }
}
