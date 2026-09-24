//! Multi-line text entry.

use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use super::cells;
use super::edit_menu::{self, EditAction, TextMenu};
use super::editor::Editor;
use super::rows::WHEEL_ROWS;
use super::scrollbar::{self, ScrollMetrics};
use super::text_input::{Blink, draw_cursor};
use super::text_rows;
use crate::env::Env;
use crate::event::{Event, KeyEvent, MouseButton, MouseEvent, MouseKind};
use crate::geometry::{Padding, Rect, Size, clamp_u16};
use crate::keymap::{Key, Modifiers, Scope};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Rows a text area measures at least, so it reads as a place for several lines.
const MIN_ROWS: usize = 3;

/// Rows a text area grows to with its content before it scrolls.
const MAX_ROWS: usize = 8;

/// Digits reserved for line numbers at least, so the gutter does not widen at line 10.
const MIN_NUMBER_DIGITS: u16 = 2;

type TextMessage<Msg> = Box<dyn Fn(String) -> Msg>;

/// A multi-line text field with word wrap, scrolling and real editing.
///
/// Long lines wrap at word boundaries and the area scrolls vertically with a scrollbar once the
/// text is taller than the area. The area measures three to eight rows, growing with its
/// content; give the node a height for a fixed size. The application owns the value and
/// receives every change through `on_change`; cursor, selection, scroll and undo history live
/// in the runtime.
///
/// Keys: Enter inserts a line break. ←/→ move (with Ctrl by word), ↑/↓ move between rows
/// keeping the column, Page Up/Page Down move a page, Home/End go to the row's start and end
/// (with Ctrl to the text's), and Shift extends the selection with any of them. Without Shift an
/// arrow clears a selection and moves one step on from its end in that direction. Backspace and
/// Delete, Ctrl+W deletes a word, Ctrl+U deletes to the line start, Ctrl+A selects all,
/// Ctrl+Z undo, Ctrl+Y or Ctrl+Shift+Z redo, Ctrl+C and Ctrl+X copy and cut. Ctrl+Enter
/// submits when [`TextArea::on_submit`] is set; terminals without the kitty keyboard protocol
/// may not report it, so offer a button as well. Pasting keeps line breaks. Clicking places the
/// cursor, dragging selects, the wheel and the scrollbar scroll.
///
/// A right click (or Shift+F10 and the menu key) opens the edit menu of
/// [`TextInput`](super::TextInput): Cut, Copy, Paste and Select all, with the same rules.
///
/// [`TextArea::variant`] with `"plain"` draws the area like paper: no field surface of its own,
/// only the tone of whatever it sits on, with focus shown by the pillar.
///
/// Style keys: `text-area` (`bg`, `fg`, `padding`, and the `see-through` flag that leaves the
/// ground unpainted) with `hover`, `focus`, `invalid`, `disabled` and the `plain` variant; `text-area-line-number` (`fg`) with `selected` on the cursor's line;
/// `text-area-counter` (`fg`); and the text input's `text-input-placeholder`,
/// `text-input-selection`, `text-input-cursor` and `scrollbar`.
pub struct TextArea<Msg> {
    value: String,
    placeholder: String,
    variant: Option<String>,
    invalid: bool,
    disabled: bool,
    max_length: Option<usize>,
    line_numbers: bool,
    counter: bool,
    on_change: Option<TextMessage<Msg>>,
    on_submit: Option<TextMessage<Msg>>,
}

#[derive(Debug, Default)]
struct AreaMemory {
    editor: Editor,
    synced: Option<String>,
    /// First visible row.
    scroll: usize,
    /// The column ↑ and ↓ keep while moving through shorter rows.
    goal: Option<u16>,
    /// Whether the next paint scrolls the cursor into view.
    follow: bool,
    last_edit: Duration,
    selecting: bool,
    dragging_bar: bool,
}

/// Where the parts of a text area go inside its area.
struct Layout {
    rows: Vec<std::ops::Range<usize>>,
    /// The text cells: one cell wider than the wrap width, for a cursor after a full row.
    text: Rect,
    /// Cells of the line number column, including its gap.
    gutter: u16,
    bar: Option<Rect>,
    counter: Option<Rect>,
}

impl Layout {
    fn visible(&self) -> usize {
        usize::from(self.text.height.max(1))
    }

    fn metrics(&self, offset: usize) -> ScrollMetrics {
        ScrollMetrics { total: self.rows.len(), visible: self.visible(), offset }
    }
}

/// What an event did.
#[derive(Default)]
struct Outcome {
    handled: bool,
    changed: bool,
    submit: bool,
    copy: Option<String>,
    capture: bool,
    /// Whether the cursor should be scrolled into view.
    follow: bool,
}

impl<Msg: 'static> TextArea<Msg> {
    /// An area showing `value`.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: String::new(),
            variant: None,
            invalid: false,
            disabled: false,
            max_length: None,
            line_numbers: false,
            counter: false,
            on_change: None,
            on_submit: None,
        }
    }

    /// Faint text shown while the area is empty.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Theme variant. The built-in themes have `"plain"`: the area takes the tone of the surface
    /// it sits on, like paper, instead of a raised field; focus shows as the pillar, and the
    /// cursor and the selection look as they always do.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Marks the value as failing validation.
    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Makes the area read-only and unfocusable.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Limits the value to `max` characters; a line break counts as one.
    #[must_use]
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Numbers every line in a faint column on the left; wrapped rows are not numbered.
    #[must_use]
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// Shows the character count on a row below the text, with the limit when there is one.
    #[must_use]
    pub fn counter(mut self, on: bool) -> Self {
        self.counter = on;
        self
    }

    /// Message carrying the new value after every edit.
    #[must_use]
    pub fn on_change(mut self, message: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(message));
        self
    }

    /// Message carrying the value when Ctrl+Enter is pressed.
    #[must_use]
    pub fn on_submit(mut self, message: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_submit = Some(Box::new(message));
        self
    }

    fn sync<'m>(&self, memory: &'m mut AreaMemory) -> &'m mut AreaMemory {
        if memory.synced.as_deref() != Some(self.value.as_str()) {
            if memory.editor.text() != self.value {
                memory.editor.replace_all(&self.value);
                memory.goal = None;
            }
            memory.synced = Some(self.value.clone());
        }
        memory
    }

    fn gutter(&self, text: &str) -> u16 {
        if !self.line_numbers {
            return 0;
        }
        let lines = text.matches('\n').count() + 1;
        let digits = clamp_u16(i32::try_from(lines.to_string().len()).unwrap_or(i32::MAX));
        digits.max(MIN_NUMBER_DIGITS) + 1
    }

    fn layout(&self, env: &Env, area: Rect, text: &str) -> Layout {
        let inner = area.inset(padding(env, self.variant.as_deref()));
        let counter =
            (self.counter && inner.height >= 2).then(|| Rect::new(inner.x, inner.bottom() - 1, inner.width, 1));
        let height = inner.height - u16::from(counter.is_some());
        let gutter = self.gutter(text).min(inner.width);
        let full = inner.width - gutter;
        let x = inner.x + i32::from(gutter);
        let rows = text_rows::wrap(text, full.saturating_sub(1));
        if rows.len() > usize::from(height) && full > 2 {
            let bar = Rect::new(inner.right() - 1, inner.y, 1, height);
            let rows = text_rows::wrap(text, full - 2);
            return Layout { rows, text: Rect::new(x, inner.y, full - 1, height), gutter, bar: Some(bar), counter };
        }
        Layout { rows, text: Rect::new(x, inner.y, full, height), gutter, bar: None, counter }
    }

    fn key(&self, memory: &mut AreaMemory, key: &KeyEvent, layout: &Layout) -> Outcome {
        let mut out = Outcome { handled: true, follow: true, ..Outcome::default() };
        let max = self.max_length;
        let mods = key.chord.mods;
        let ctrl = mods.ctrl && !mods.alt;
        let text = memory.editor.text().to_owned();
        let cursor = memory.editor.cursor();
        // ↑ and ↓ without Shift clear a selection and move a row on from its end in that direction.
        let from = match (memory.editor.selection(), key.chord.key) {
            (Some(range), Key::Up | Key::PageUp) if !mods.shift => range.start,
            (Some(range), Key::Down | Key::PageDown) if !mods.shift => range.end,
            _ => cursor,
        };
        let (row, column) = text_rows::locate(&text, &layout.rows, from);
        let vertical = |memory: &mut AreaMemory, delta: isize| {
            let goal = memory.goal.unwrap_or(column);
            let target = row.checked_add_signed(delta).filter(|target| *target < layout.rows.len());
            let offset = match target {
                Some(target) => text_rows::offset_at(&text, &layout.rows, target, goal),
                None if delta < 0 => 0,
                None => text.len(),
            };
            memory.editor.move_to_offset(offset, mods.shift);
            memory.goal = Some(goal);
        };
        let page = isize::try_from(layout.visible()).unwrap_or(1);
        match key.chord.key {
            Key::Up | Key::Down | Key::PageUp | Key::PageDown if !ctrl => {
                let delta = match key.chord.key {
                    Key::Up => -1,
                    Key::Down => 1,
                    Key::PageUp => -page,
                    _ => page,
                };
                vertical(memory, delta);
                return out;
            }
            _ => memory.goal = None,
        }
        let editor = &mut memory.editor;
        match key.chord.key {
            Key::Char(c) if ctrl => match (c, mods.shift) {
                ('a', false) => editor.select_all(),
                ('z', false) => out.changed = editor.undo(),
                ('y', false) | ('z', true) => out.changed = editor.redo(),
                ('w', false) => out.changed = editor.delete_word_back(),
                ('u', false) => {
                    let line_start = text[..cursor].rfind('\n').map_or(0, |index| index + 1);
                    out.changed = editor.delete_range(line_start..cursor);
                }
                ('c', false) | ('x', false) => {
                    out.copy = editor.selected_text().map(str::to_owned);
                    if c == 'x' && out.copy.is_some() {
                        out.changed = editor.backspace();
                    }
                    out.handled = out.copy.is_some();
                }
                _ => out.handled = false,
            },
            Key::Enter if ctrl && !mods.shift => {
                out.submit = self.on_submit.is_some();
                out.handled = out.submit;
            }
            Key::Enter if mods == Modifiers::default() => out.changed = editor.insert_lines("\n", max),
            Key::Left => editor.move_left(mods.shift, mods.ctrl),
            Key::Right => editor.move_right(mods.shift, mods.ctrl),
            Key::Home if mods.ctrl => editor.move_home(mods.shift),
            Key::End if mods.ctrl => editor.move_end(mods.shift),
            Key::Home => editor.move_to_offset(layout.rows.get(row).map_or(0, |r| r.start), mods.shift),
            Key::End => editor.move_to_offset(text_rows::offset_at(&text, &layout.rows, row, u16::MAX), mods.shift),
            Key::Backspace if mods == Modifiers::default() || mods.ctrl => {
                out.changed = if mods.ctrl { editor.delete_word_back() } else { editor.backspace() };
            }
            Key::Delete => out.changed = editor.delete(),
            _ => match key.text {
                Some(c) if !mods.ctrl && !mods.alt => out.changed = editor.insert_lines(&c.to_string(), max),
                _ => out.handled = false,
            },
        }
        out
    }

    /// The byte offset of the text under the pointer.
    fn offset_under(memory: &AreaMemory, mouse: &MouseEvent, layout: &Layout) -> usize {
        let relative = isize::try_from(mouse.y - layout.text.y).unwrap_or(0);
        let last = layout.rows.len().saturating_sub(1);
        let row = memory.scroll.checked_add_signed(relative).unwrap_or(0).min(last);
        let column = clamp_u16(mouse.x - layout.text.x);
        text_rows::offset_at(memory.editor.text(), &layout.rows, row, column)
    }

    /// Offers `event` to the edit menu, which opens on a right press or its keys and takes every
    /// event while open. Returns `None` when the menu did not use the event.
    fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event, layout: &Layout) -> Option<Outcome> {
        let open = edit_menu::is_open(cx);
        if !open && !edit_menu::asks(event) {
            return None;
        }
        if let Event::Mouse(mouse) = event
            && !open
        {
            // A right press inside the selection keeps it; elsewhere it places the cursor first.
            let memory = self.sync(cx.memory::<AreaMemory>());
            let offset = Self::offset_under(memory, mouse, layout);
            if !memory.editor.selection().is_some_and(|range| range.contains(&offset)) {
                memory.editor.move_to_offset(offset, false);
                memory.goal = None;
            }
        }
        let selection = self.sync(cx.memory::<AreaMemory>()).editor.selection().is_some();
        let (used, chosen) = TextMenu::edit(cx.env(), selection, cx.can_paste()).event(cx, event);
        if used && !open {
            cx.probe_clipboard();
        }
        let mut out = Outcome { handled: used, ..Outcome::default() };
        let Some(action) = chosen else {
            return used.then_some(out);
        };
        let editor = &mut self.sync(cx.memory::<AreaMemory>()).editor;
        match action {
            EditAction::Cut | EditAction::Copy => {
                out.copy = editor.selected_text().map(str::to_owned);
                out.changed = action == EditAction::Cut && out.copy.is_some() && editor.backspace();
            }
            EditAction::Paste => cx.run_action(Scope::Global, "paste"),
            EditAction::SelectAll => editor.select_all(),
        }
        out.handled = true;
        out.follow = true;
        Some(out)
    }

    fn mouse(&self, memory: &mut AreaMemory, mouse: &MouseEvent, layout: &Layout) -> Outcome {
        let mut out = Outcome { handled: true, ..Outcome::default() };
        let metrics = layout.metrics(memory.scroll);
        let bar_row = |bar: Rect| clamp_u16(mouse.y - bar.y);
        let on_bar = layout.bar.is_some_and(|bar| bar.contains(mouse.x, mouse.y));
        match mouse.kind {
            MouseKind::ScrollUp => memory.scroll = memory.scroll.saturating_sub(usize::from(WHEEL_ROWS)),
            MouseKind::ScrollDown => {
                memory.scroll = (memory.scroll + usize::from(WHEEL_ROWS)).min(metrics.max_offset());
            }
            MouseKind::Down(MouseButton::Left) if on_bar => {
                if let Some(bar) = layout.bar {
                    memory.scroll = metrics.offset_at(bar_row(bar), bar.height);
                }
                memory.dragging_bar = true;
                out.capture = true;
            }
            MouseKind::Drag(MouseButton::Left) if memory.dragging_bar => {
                if let Some(bar) = layout.bar {
                    memory.scroll = metrics.offset_at(bar_row(bar), bar.height);
                }
            }
            MouseKind::Down(MouseButton::Left) | MouseKind::Drag(MouseButton::Left) => {
                let dragging = matches!(mouse.kind, MouseKind::Drag(_));
                if dragging && !memory.selecting {
                    return Outcome::default();
                }
                let offset = Self::offset_under(memory, mouse, layout);
                memory.editor.move_to_offset(offset, dragging);
                memory.goal = None;
                memory.selecting = true;
                out.capture = !dragging;
                out.follow = true;
            }
            MouseKind::Up(MouseButton::Left) => {
                memory.selecting = false;
                memory.dragging_bar = false;
            }
            _ => out.handled = false,
        }
        out
    }
}

/// The padding of the text area from the theme.
fn padding(env: &Env, variant: Option<&str>) -> Padding {
    let (vertical, horizontal) = env.theme().style("text-area", variant, &[]).pair("padding").unwrap_or((0, 1));
    Padding::symmetric(vertical, horizontal)
}

impl<Msg: 'static> Widget<Msg> for TextArea<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let padding = padding(cx.env(), self.variant.as_deref());
        let width = available.width.saturating_sub(cells::sum([padding.horizontal(), self.gutter(&self.value), 1]));
        let rows = text_rows::wrap(&self.value, width).len().clamp(MIN_ROWS, MAX_ROWS);
        let height = clamp_u16(i32::try_from(rows).unwrap_or(i32::MAX)) + u16::from(self.counter);
        Size::new(available.width, height.saturating_add(padding.vertical())).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.takes_text();
        let mut states = if self.disabled { vec![State::Disabled] } else { cx.states() };
        if self.invalid {
            states.push(State::Invalid);
        }
        let focused = states.contains(&State::Focus);
        let area_style = cx.style("text-area", self.variant.as_deref(), &states);
        let surface = area_style.text();
        // A see-through area keeps the ground its parent painted, in every state.
        if !area_style.flag("see-through") {
            cx.clear(area, surface.bg.unwrap_or_else(|| cx.color("raised")));
        }
        // The pillar runs down the whole left padding column; the text never slides.
        if let Some(color) = area_style.color("pillar").filter(|_| area_style.padding().left >= 1) {
            for row in 0..area.height {
                cx.pillar(area.x, area.y + i32::from(row), color);
            }
        }
        if !self.disabled {
            cx.register_hit(area);
            edit_menu::request_overlay(cx, area);
        }
        let (text, cursor, selection, last_edit) = {
            let memory = self.sync(cx.memory::<AreaMemory>());
            let editor = &memory.editor;
            (editor.text().to_owned(), editor.cursor(), editor.selection(), memory.last_edit)
        };
        let layout = self.layout(cx.env(), area, &text);
        let visible = layout.visible();
        let (cursor_row, cursor_column) = text_rows::locate(&text, &layout.rows, cursor);
        let scroll = {
            let memory = cx.memory::<AreaMemory>();
            if memory.follow {
                if cursor_row < memory.scroll {
                    memory.scroll = cursor_row;
                } else if cursor_row >= memory.scroll + visible {
                    memory.scroll = cursor_row + 1 - visible;
                }
                memory.follow = false;
            }
            memory.scroll = memory.scroll.min(layout.rows.len().saturating_sub(visible));
            memory.scroll
        };

        if let Some(counter) = layout.counter {
            let count = text.graphemes(true).count();
            let label = self.max_length.map_or_else(|| count.to_string(), |max| format!("{count} / {max}"));
            let style = cx.style("text-area-counter", None, &states).text();
            let width = text::width(&label).min(counter.width);
            cx.text(counter.right() - i32::from(width), counter.y, &label, style, width);
        }

        let text_style = CellStyle { bg: None, ..surface };
        // An empty area keeps its placeholder in place; when focused the cursor sits on its first
        // letter in inverted colours instead of pushing it aside.
        let mut placeholder_head = None;
        if text.is_empty() && !self.placeholder.is_empty() {
            let placeholder = cx.style("text-input-placeholder", None, &states).text();
            let budget = layout.text.width;
            let shown = text::truncate(&self.placeholder, budget).into_owned();
            cx.text(layout.text.x, layout.text.y, &shown, placeholder, budget);
            placeholder_head = shown.graphemes(true).next().map(str::to_owned);
        }
        let selection_style = cx.style("text-input-selection", None, &states).text();
        let cursor_line = text[..cursor].matches('\n').count();
        for (index, range) in layout.rows.iter().enumerate().skip(scroll).take(visible) {
            let y = layout.text.y + i32::try_from(index - scroll).unwrap_or(0);
            if self.line_numbers && text_rows::starts_line(&text, range) {
                let line = text[..range.start].matches('\n').count();
                let number_states = if line == cursor_line && focused { vec![State::Selected] } else { Vec::new() };
                let style = cx.style("text-area-line-number", None, &number_states).text();
                let number = (line + 1).to_string();
                let width = text::width(&number).min(layout.gutter.saturating_sub(1));
                let x = layout.text.x - 1 - i32::from(width);
                cx.text(x, y, &number, style, width);
            }
            let mut x = layout.text.x;
            for (offset, grapheme) in text[range.clone()].grapheme_indices(true) {
                let start = range.start + offset;
                let width = text::grapheme_width(grapheme).max(1);
                if x + i32::from(width) > layout.text.right() {
                    break;
                }
                let selected = selection.as_ref().is_some_and(|selection| selection.contains(&start));
                let style = if selected {
                    CellStyle { fg: selection_style.fg.or(text_style.fg), bg: selection_style.bg, ..text_style }
                } else {
                    text_style
                };
                cx.text(x, y, grapheme, style, width);
                x += i32::from(width);
            }
        }
        if focused && (scroll..scroll + visible).contains(&cursor_row) {
            let y = layout.text.y + i32::try_from(cursor_row - scroll).unwrap_or(0);
            let x = layout.text.x + i32::from(cursor_column.min(layout.text.width.saturating_sub(1)));
            let row_end = layout.rows.get(cursor_row).map_or(cursor, |row| row.end);
            let glyph = text[cursor..row_end]
                .graphemes(true)
                .next()
                .map(str::to_owned)
                .or(placeholder_head)
                .unwrap_or_else(|| " ".to_owned());
            let blink = Blink { now: cx.now(), last_edit, period: cx.env().theme().motion().cursor_blink };
            draw_cursor(cx, x, y, &glyph, blink, &states);
        }
        if let Some(bar) = layout.bar {
            let dragging = cx.memory::<AreaMemory>().dragging_bar;
            let active = dragging || cx.pointer().is_some_and(|(x, _)| x == bar.x);
            scrollbar::paint(cx, bar, layout.metrics(scroll), active, None);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.disabled {
            return false;
        }
        let now = cx.now();
        let area = cx.area();
        let text = self.sync(cx.memory::<AreaMemory>()).editor.text().to_owned();
        let layout = self.layout(cx.env(), area, &text);
        let menu = self.menu_event(cx, event, &layout);
        let (out, value) = {
            let memory = self.sync(cx.memory::<AreaMemory>());
            let out = match event {
                _ if let Some(out) = menu => out,
                Event::Paste(pasted) => Outcome {
                    handled: true,
                    changed: memory.editor.insert_lines(pasted, self.max_length),
                    follow: true,
                    ..Outcome::default()
                },
                Event::Key(key) => self.key(memory, key, &layout),
                Event::Mouse(mouse) => self.mouse(memory, mouse, &layout),
                Event::PointerOutside => Outcome::default(),
            };
            if out.handled {
                memory.last_edit = now;
                memory.follow |= out.follow;
            }
            if out.changed {
                memory.synced = Some(memory.editor.text().to_owned());
            }
            (out, memory.editor.text().to_owned())
        };
        if out.capture {
            cx.capture_pointer();
        }
        if let Some(copied) = out.copy {
            cx.copy(copied);
        }
        if out.changed
            && let Some(message) = &self.on_change
        {
            cx.emit(message(value.clone()));
        }
        if out.submit
            && let Some(message) = &self.on_submit
        {
            cx.emit(message(value));
        }
        out.handled
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let selection = self.sync(cx.memory::<AreaMemory>()).editor.selection().is_some();
        TextMenu::edit(cx.env(), selection, cx.can_paste()).paint_overlay(cx, anchor);
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    #[derive(Default)]
    struct Demo {
        value: String,
        submitted: Option<String>,
        numbers: bool,
        counter: bool,
        disabled: bool,
        /// Extra columns beyond the usual 16.
        wider: u16,
    }

    #[derive(Clone)]
    enum Msg {
        Changed(String),
        Submitted(String),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Changed(value) => self.value = value,
                Msg::Submitted(value) => self.submitted = Some(value),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add(
                TextArea::new(&self.value)
                    .placeholder("Release notes")
                    .max_length(60)
                    .line_numbers(self.numbers)
                    .counter(self.counter)
                    .disabled(self.disabled)
                    .on_change(Msg::Changed)
                    .on_submit(Msg::Submitted),
            )
            .width(Length::Cells(16 + self.wider))
            .id("notes");
        }
    }

    fn harness(value: &str) -> Harness<Demo> {
        let mut h = Harness::new(Demo { value: value.into(), ..Demo::default() }, 16, 4);
        h.set_reduced_motion(true);
        h
    }

    #[test]
    fn placeholder_then_typing_with_line_breaks_and_submit_on_ctrl_enter() {
        let mut h = harness("");
        assert_eq!(h.screen(), "  Release not…\n\n\n\n");
        h.press("tab").type_text("fixed").press("enter").type_text("faster");
        assert_eq!(h.app().value, "fixed\nfaster");
        assert_eq!(h.screen(), "▌ fixed\n▌ faster\n▌\n\n");
        h.press("ctrl+enter");
        assert_eq!(h.app().submitted.as_deref(), Some("fixed\nfaster"));
        h.paste("\r\nlast");
        assert_eq!(h.app().value, "fixed\nfaster\nlast");
    }

    #[test]
    fn wraps_words_and_up_down_keep_the_column() {
        // Two cells wider than the other tests: typing `Y` must not rewrap the middle row.
        let demo = Demo { value: "the canary deploy went well".into(), wider: 2, ..Demo::default() };
        let mut h = Harness::new(demo, 18, 4);
        h.set_reduced_motion(true);
        assert_eq!(h.screen(), "  the canary\n  deploy went\n  well\n\n");
        h.press("tab").press("ctrl+home").press("down");
        for _ in 0..9 {
            h.press("right");
        }
        // Down through the short last row to the end of the text and back up: column 9 is kept.
        h.press("down").press("down").press("up").type_text("Y");
        assert_eq!(h.app().value, "the canary deploy weYnt well");
        h.press("home").type_text("Z").press("end").type_text("!");
        assert_eq!(h.app().value, "the canary Zdeploy weYnt! well", "end stops before the wrapped space");
    }

    #[test]
    fn selection_copy_cut_and_line_deletion() {
        let mut h = harness("alpha\nbeta");
        h.press("tab").press("ctrl+end").press("shift+up").press("ctrl+c");
        assert_eq!(h.copied(), &["a\nbeta".to_owned()]);
        h.press("ctrl+x");
        assert_eq!(h.app().value, "alph");
        h.press("ctrl+z").press("ctrl+end").press("ctrl+u");
        assert_eq!(h.app().value, "alpha\n");
        let mut h = harness("alpha\nbeta");
        h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 4, 1);
        h.mouse(MouseKind::Up(MouseButton::Left), 4, 1).press("ctrl+c");
        assert_eq!(h.copied(), &["ha\nbe".to_owned()], "dragging across a line break selects it");
    }

    #[test]
    fn scrolls_to_the_cursor_with_a_scrollbar_and_the_wheel() {
        let text = "one\ntwo\nthree\nfour\nfive\nsix";
        let mut h = harness(text);
        let screen = h.screen();
        assert!(screen.starts_with("  one"), "{screen}");
        assert!(super::super::scrollbar::column(&h, 13).starts_with("##"), "{screen}");
        h.press("tab").press("ctrl+end");
        assert!(h.screen().contains("six"), "{}", h.screen());
        assert!(!h.screen().contains("one"));
        h.mouse(MouseKind::ScrollUp, 3, 1);
        assert!(h.screen().contains("one"));
        h.click(4, 1).type_text("X");
        assert_eq!(h.app().value, "one\ntwXo\nthree\nfour\nfive\nsix");
    }

    #[test]
    fn line_numbers_counter_limit_and_disabled() {
        let mut h = Harness::new(Demo { value: "a\nb".into(), numbers: true, counter: true, ..Demo::default() }, 16, 4);
        assert_eq!(h.screen(), "   1 a\n   2 b\n\n        3 / 60\n");
        h.press("tab").press("ctrl+end").paste(&"x".repeat(80));
        assert_eq!(h.app().value.chars().count(), 60);
        let mut h = Harness::new(Demo { value: "a".into(), disabled: true, ..Demo::default() }, 16, 4);
        h.press("tab").type_text("b");
        assert_eq!(h.app().value, "a");
        assert_eq!(h.fg(2, 0), h.env().theme().color("muted"));
    }

    /// A text area on a panel, as a note tool puts it.
    struct Paper {
        value: String,
        variant: Option<&'static str>,
        invalid: bool,
    }

    impl App for Paper {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            if let Msg::Changed(value) = msg {
                self.value = value;
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add_with(crate::widgets::Panel::new(), |ui| {
                let mut area = TextArea::new(&self.value).invalid(self.invalid).on_change(Msg::Changed);
                if let Some(variant) = self.variant {
                    area = area.variant(variant);
                }
                ui.add(area).height(Length::Cells(3));
            })
            .fill();
        }
    }

    /// What a text area on a panel looks like at rest, after a click into it, and with its
    /// text selected: the ground under a letter at rest and focused, the glyph and colour left of
    /// the text when focused, the cursor's cell and a selected letter's ground.
    #[derive(Debug, PartialEq)]
    struct Looks {
        surface: Option<crate::color::Rgb>,
        rest: Option<crate::color::Rgb>,
        focused: Option<crate::color::Rgb>,
        pillar: (Option<char>, Option<crate::color::Rgb>),
        cursor: Option<crate::color::Rgb>,
        selected: Option<crate::color::Rgb>,
    }

    fn looks(variant: Option<&'static str>) -> (Looks, String) {
        let mut h = Harness::new(Paper { value: "hello paper".into(), variant, invalid: false }, 30, 7);
        h.set_reduced_motion(true);
        let (x, y) = h.find("hello").expect("drawn");
        let cell = |x: i32| u16::try_from(x).expect("on screen");
        let (row, far) = (cell(y), cell(x + 13));
        let rest = h.bg(far, row);
        h.click(x + 2, y);
        let focused = h.bg(far, row);
        let glyph =
            h.screen().lines().nth(usize::from(row)).and_then(|line| line.chars().nth(usize::from(cell(x - 2))));
        let pillar = (glyph, h.fg(cell(x - 2), row));
        let cursor = h.bg(cell(x + 2), row);
        h.type_text("!");
        h.press("ctrl+a");
        let selected = h.bg(cell(x + 4), row);
        let surface = h.env().theme().color("surface");
        (Looks { surface, rest, focused, pillar, cursor, selected }, h.app().value.clone())
    }

    #[test]
    fn a_plain_area_takes_the_panel_tone_and_still_shows_focus() {
        let (plain, typed) = looks(Some("plain"));
        let surface = plain.surface;
        assert_eq!(typed, "he!llo paper", "the click placed the cursor");
        assert_eq!((plain.rest, plain.focused), (surface, surface), "{plain:?}");
        assert_eq!(plain.pillar.0, Some('▌'), "focus is shown by the pillar");
        let (field, _) = looks(None);
        assert_ne!(field.rest, surface, "the default area is a raised field");
        assert_ne!(field.focused, surface);
        assert_eq!(plain.pillar, field.pillar, "the same pillar");
        assert_eq!(
            (plain.cursor, plain.selected),
            (field.cursor, field.selected),
            "cursor and selection keep their look"
        );
    }

    #[test]
    fn a_plain_area_shows_invalid_text_with_a_danger_pillar_at_rest() {
        let h = Harness::new(Paper { value: "hello".into(), variant: Some("plain"), invalid: true }, 30, 7);
        let (x, y) = h.find("hello").expect("drawn");
        let (column, row) = (u16::try_from(x - 2).expect("on screen"), u16::try_from(y).expect("on screen"));
        assert_eq!(h.fg(column, row), h.env().theme().color("danger"));
        assert_eq!(h.bg(column + 10, row), h.env().theme().color("surface"));
    }

    // Selection, copying, the edit menu, paste sources and arrows with a selection.

    fn roomy(value: &str) -> Harness<Demo> {
        let mut h = Harness::new(Demo { value: value.into(), ..Demo::default() }, 30, 9);
        h.set_reduced_motion(true);
        h
    }

    fn right_click(h: &mut Harness<Demo>, x: i32, y: i32) {
        h.mouse(MouseKind::Down(MouseButton::Right), x, y);
        h.mouse(MouseKind::Up(MouseButton::Right), x, y);
    }

    #[test]
    fn dragging_selects_its_own_text_and_releasing_copies_nothing() {
        let mut h = roomy("alpha\nbeta");
        let plain = h.bg(3, 1);
        h.drag((2, 0), (4, 1));
        assert!(h.copied().is_empty(), "releasing copies nothing");
        assert_ne!(h.bg(3, 1), plain, "the selection is shown");
        h.press("ctrl+c");
        assert_eq!(h.copied(), ["alpha\nbe"]);
    }

    #[test]
    fn right_click_opens_the_edit_menu() {
        let mut h = roomy("alpha\nbeta");
        right_click(&mut h, 3, 1);
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(
            &lines[2..6],
            [
                "▌    Cut           ctrl x",
                "     Copy          ctrl c",
                "     Paste         ctrl v",
                "     Select all    ctrl a"
            ],
            "{screen}"
        );
        let muted = h.env().theme().color("muted");
        assert_eq!((h.fg(5, 2), h.fg(5, 3), h.fg(5, 4)), (muted, muted, muted));
        h.click_text("Select all");
        right_click(&mut h, 3, 0);
        assert_ne!(h.fg(5, 1), muted, "a right click inside the selection keeps it");
        h.click_text("Cut");
        assert_eq!((h.app().value.as_str(), h.clipboard()), ("", Some("alpha\nbeta")));
        right_click(&mut h, 3, 0);
        h.click_text("Paste");
        assert_eq!(h.app().value, "alpha\nbeta", "line breaks survive the round trip");
        right_click(&mut h, 4, 1);
        h.press("esc").type_text("X");
        assert_eq!(h.app().value, "alpha\nbeXta", "a right click elsewhere placed the cursor");
    }

    #[test]
    fn paste_reads_the_system_clipboard_first() {
        let mut h = roomy("");
        h.set_system_clipboard(Some("one\ntwo")).press("tab").press("ctrl+v");
        assert_eq!(h.app().value, "one\ntwo");
    }

    #[test]
    fn arrows_with_a_selection_clear_it_and_move_on_from_its_end() {
        let mut h = roomy("alpha\nbeta\ngamma");
        h.press("tab").press("ctrl+home").press("shift+right").press("shift+right").press("right").type_text("R");
        assert_eq!(h.app().value, "alpRha\nbeta\ngamma", "one past the right end");
        h.press("ctrl+home").press("down").press("shift+right").press("shift+right").press("left").type_text("L");
        assert_eq!(h.app().value, "alpRhaL\nbeta\ngamma", "one before the left end, across the line break");
        h.press("ctrl+home").press("down").press("shift+right").press("shift+right").press("up").type_text("U");
        assert_eq!(h.app().value, "UalpRhaL\nbeta\ngamma", "a row up from the upper end");
        h.press("ctrl+home").press("down").press("shift+right").press("shift+right").press("down").type_text("D");
        assert_eq!(h.app().value, "UalpRhaL\nbeta\ngaDmma", "a row down from the lower end");
    }
}
