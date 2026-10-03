//! Multi-line text entry.

use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use super::cells;
use super::code_view::token_style;
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
use crate::text::{self, Language, Token, highlight};
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget, WidgetKey};

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
/// [`TextArea::language`] paints the text in the `code-token` colours of a
/// [`CodeView`](super::CodeView) while it is edited; nothing else about the area changes.
///
/// Style keys: `text-area` (`bg`, `fg`, `padding`, and the `see-through` flag that leaves the
/// ground unpainted) with `hover`, `focus`, `invalid`, `disabled` and the `plain` variant; `text-area-line-number` (`fg`) with `selected` on the cursor's line;
/// `text-area-counter` (`fg`); `code-token.<kind>` for the text of a language, where kind is
/// what [`Token::style_variant`] names; and the text input's `text-input-placeholder`,
/// `text-input-selection`, `text-input-cursor` and `scrollbar`.
pub struct TextArea<Msg> {
    value: String,
    placeholder: String,
    variant: Option<String>,
    language: Language,
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
    coloured: Coloured,
}

/// The tokens the text was last painted with, and what they were worked out from, so a frame
/// whose text and language are both the ones already coloured takes them again instead of
/// colouring the whole text again: typing a character into a file of thousands of lines then
/// colours it once, not once a frame.
#[derive(Debug, Default)]
struct Coloured {
    /// A hash of the text and the language below.
    stamp: u64,
    /// Behind a handle, because painting walks the tokens while the memory is lent out.
    tokens: Rc<Vec<(Range<usize>, Token)>>,
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
            language: Language::Plain,
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

    /// Colours the text as `language` while it is edited, in the same `code-token` colours a
    /// [`CodeView`](super::CodeView) gives a file of that language: a keyword, a string, a
    /// comment and a number each take their own. Only the colour changes. The cursor, the
    /// selection, the undo, the scrolling and the line numbers are the area's own, a selection
    /// covers the colours under it, the placeholder stays uncoloured and a disabled area keeps
    /// its muted text. The tokens are worked out when the text or the language changes rather
    /// than on every frame, so typing in a long file stays as quick as typing in a field.
    ///
    /// [`Language::Plain`], the default, leaves the text in the area's own colour.
    /// [`Language::from_file_name`] picks the language of the file being edited.
    #[must_use]
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
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

    /// The tokens the text is painted with: those of the text as it was the last time it or the
    /// language changed, taken again on a frame where neither did. A plain area has none of its
    /// own, because it wants the colour the area gives the text rather than a token's.
    fn tokens(&self, cx: &mut PaintCx<'_>, text: &str) -> Rc<Vec<(Range<usize>, Token)>> {
        // Every text area is plain unless asked otherwise, and those must not pay for hashing
        // their whole text on each frame.
        if self.language == Language::Plain {
            return Rc::default();
        }
        let memory = cx.memory::<AreaMemory>();
        let stamp = stamp(text, self.language);
        if memory.coloured.stamp == stamp {
            return Rc::clone(&memory.coloured.tokens);
        }
        let tokens = Rc::new(highlight(text, self.language));
        memory.coloured = Coloured { stamp, tokens: Rc::clone(&tokens) };
        tokens
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

/// A number that differs between two texts in two languages, as far as a hash tells: the pair is
/// what the tokens were worked out from, and nothing else can go into them.
fn stamp(text: &str, language: Language) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (text, language).hash(&mut hasher);
    hasher.finish()
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
        let tokens = self.tokens(cx, &text);
        // A disabled area is the muted text the field gives it in every state, so its tokens are
        // not read; an empty list says that in the one place the colours are chosen.
        let tokens: &[(Range<usize>, Token)] = if self.disabled { &[] } else { &tokens };
        // The tokens cover the text in order and the graphemes are walked in order too, so one
        // index forward finds the token each grapheme falls in.
        let mut at = 0;
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
                while tokens.get(at).is_some_and(|(token, _)| token.end <= start) {
                    at += 1;
                }
                let token = tokens.get(at).filter(|(token, _)| token.start <= start).map(|(_, kind)| *kind);
                let selected = selection.as_ref().is_some_and(|selection| selection.contains(&start));
                let style = if selected {
                    CellStyle { fg: selection_style.fg.or(text_style.fg), bg: selection_style.bg, ..text_style }
                } else {
                    match token {
                        Some(token) => token_style(cx, token),
                        None => text_style,
                    }
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

    fn keys(&self, env: &Env) -> Vec<WidgetKey> {
        // A plain Enter is a line break here, so the key that submits is the one worth a row.
        if self.on_submit.is_some() {
            return vec![WidgetKey::new("ctrl enter", env.i18n().translate("quvyta.widget.submit", &[]))];
        }
        Vec::new()
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

    // Syntax colours, from the language the file being edited would give.

    /// The line the playground of the text area colours.
    const SNIPPET: &str = "fn main() { let s = \"x\"; }";

    /// A text area holding code, in the language its file name would give.
    struct Code {
        value: String,
        language: Language,
        disabled: bool,
    }

    impl App for Code {
        type Msg = String;
        fn update(&mut self, value: String) -> Command<String> {
            self.value = value;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, String>) {
            ui.add(
                TextArea::new(self.value.as_str())
                    .language(self.language)
                    .disabled(self.disabled)
                    .on_change(|value| value),
            );
        }
    }

    fn code(value: &str, language: Language) -> Harness<Code> {
        let mut h = Harness::new(Code { value: value.into(), language, disabled: false }, 32, 4);
        // Iris, whose accent is not its text colour as in the monochrome theme, so that a token's
        // colour is told from the colour of the field's own text.
        h.set_theme("iris").set_reduced_motion(true);
        h
    }

    /// The column and row of `text` on the screen.
    fn cell(h: &Harness<Code>, text: &str) -> (u16, u16) {
        let (x, y) = h.find(text).unwrap_or_else(|| panic!("{text:?} is drawn:\n{}", h.screen()));
        (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0))
    }

    /// The colour the theme paints `widget.variant` with, as a cell takes it.
    fn painted(h: &Harness<Code>, widget: &str, variant: Option<&str>) -> crate::color::Rgb {
        h.env()
            .theme()
            .style(widget, variant, &[])
            .paint("fg")
            .map(|paint| paint.at(0.0))
            .expect("a style the themes give")
    }

    /// The text colour of the cell at `at`, which a drawn letter always has.
    fn fg(h: &Harness<Code>, at: (u16, u16)) -> crate::color::Rgb {
        h.fg(at.0, at.1).expect("a drawn letter has a text colour")
    }

    /// The colour of the cell at `at` to stand on.
    fn bg(h: &Harness<Code>, at: (u16, u16)) -> crate::color::Rgb {
        h.bg(at.0, at.1).expect("a cell has a ground")
    }

    #[test]
    fn a_language_paints_each_token_in_its_own_colour_and_plain_leaves_the_text_alone() {
        let h = code(SNIPPET, Language::Rust);
        let text = painted(&h, "text-area", None);
        let keyword = painted(&h, "code-token", Some("keyword"));
        assert_ne!(keyword, text, "a keyword is not the area's own text colour");
        assert_eq!(fg(&h, cell(&h, "fn")), keyword, "fn takes the keyword colour:\n{}", h.screen());
        assert_eq!(fg(&h, cell(&h, "\"x\"")), painted(&h, "code-token", Some("string")), "and the string its own");

        let plain = code(SNIPPET, Language::Plain);
        assert_eq!(
            (fg(&plain, cell(&plain, "fn")), fg(&plain, cell(&plain, "\"x\""))),
            (text, text),
            "without a language the text is the field's own colour:\n{}",
            plain.screen()
        );
    }

    #[test]
    fn typing_a_comment_recolours_the_line_and_undo_brings_the_keyword_back() {
        let mut h = code(SNIPPET, Language::Rust);
        // The cursor is moved off the word both times, because the cursor block paints over the
        // cell it is on in the colours the theme gives it.
        h.press("tab").press("ctrl+home").type_text("// ").press("ctrl+end");
        assert_eq!(fg(&h, cell(&h, "fn")), painted(&h, "code-token", Some("comment")), "{}", h.screen());
        // A word and the space that ends it are undone apart, so the comment goes in two steps.
        h.press("ctrl+z").press("ctrl+z").press("ctrl+end");
        assert_eq!(fg(&h, cell(&h, "fn")), painted(&h, "code-token", Some("keyword")), "{}", h.screen());
    }

    #[test]
    fn a_selection_covers_the_colours_under_it() {
        let mut h = code(SNIPPET, Language::Rust);
        let keyword = painted(&h, "code-token", Some("keyword"));
        let selection = h
            .env()
            .theme()
            .style("text-input-selection", None, &[])
            .paint("bg")
            .map(|paint| paint.at(0.0))
            .expect("the themes give a selection a ground");
        // `main`, four letters on from the start of the line.
        h.press("tab").press("ctrl+home");
        for _ in 0..3 {
            h.press("right");
        }
        for _ in 0..4 {
            h.press("shift+right");
        }
        let main = cell(&h, "main");
        assert_eq!(bg(&h, main), selection, "the selection has its own ground:\n{}", h.screen());
        assert_ne!(bg(&h, (main.0 - 1, main.1)), selection, "only the selected letters take it");
        // A keyword under the selection is a selected letter, not a keyword any more.
        h.press("ctrl+home").press("shift+right").press("shift+right");
        assert_ne!(fg(&h, cell(&h, "fn")), keyword, "a selected keyword loses its token colour:\n{}", h.screen());
    }

    #[test]
    fn the_colours_stay_readable_in_ascii_glyphs_and_sixteen_colours() {
        use crate::color::ColorDepth;
        use crate::icons::GlyphMode;
        use ratatui_core::style::Color;
        let mut h = code(SNIPPET, Language::Rust);
        h.set_glyph_mode(GlyphMode::Ascii);
        let screen = h.screen();
        assert!(screen.contains(SNIPPET), "ASCII glyphs draw the code as it is:\n{screen}");

        // Sixteen colours are chosen against the ground of the frame, and text against the
        // background of its own cell, so both are read before the frame is drawn in them.
        let canvas = h.env().theme().color("canvas").expect("the themes name a canvas");
        let (fn_at, string) = (cell(&h, "fn"), cell(&h, "\"x\""));
        let behind = bg(&h, fn_at);
        h.set_depth(ColorDepth::Ansi16);
        let shown = |colour: crate::color::Rgb| Color::Indexed(colour.to_ansi16_text(behind, canvas));
        let at = |cell: (u16, u16)| h.buffer()[(cell.0, cell.1)].fg;
        assert_eq!(at(fn_at), shown(painted(&h, "code-token", Some("keyword"))), "{}", h.screen());
        assert_eq!(at(string), shown(painted(&h, "code-token", Some("string"))), "{}", h.screen());
        let ground = h.buffer()[(fn_at.0, fn_at.1)].bg;
        assert_ne!(
            (at(fn_at), at(string)),
            (ground, ground),
            "both tokens are told from the ground in sixteen colours"
        );
    }

    #[test]
    fn a_disabled_area_keeps_its_muted_text() {
        let h = Harness::new(Code { value: SNIPPET.into(), language: Language::Rust, disabled: true }, 32, 4);
        let muted = h.env().theme().color("muted").expect("the themes name a muted colour");
        assert_eq!(fg(&h, cell(&h, "fn")), muted, "the field's own disabled text:\n{}", h.screen());
    }

    /// A file of thousands of lines of Rust in an area, coloured or not, as an editor holds it.
    struct File {
        code: String,
        language: Language,
    }

    impl App for File {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(TextArea::new(self.code.as_str()).language(self.language)).width(Length::Cells(100));
        }
    }

    /// Five thousand lines of Rust, of the shape a real file has.
    fn rust_file() -> String {
        (0..5_000)
            .map(|n| match n % 4 {
                0 => format!("/// Answers the request numbered {n} of the batch.\n"),
                1 => format!("pub fn answer_{n}(request: &Request) -> Result<Reply, Error> {{\n"),
                2 => format!("    Ok(Reply::new(request.field(\"name-{n}\")?, {n}u32))\n"),
                _ => "}\n".to_owned(),
            })
            .collect()
    }

    /// The fastest of a few tries at one frame after a key press, which changes the text.
    fn frame_after_a_key(code: &str, language: Language) -> std::time::Duration {
        let mut h = Harness::new(File { code: code.into(), language }, 110, 30);
        h.press("tab");
        (0..3)
            .map(|_| {
                let started = std::time::Instant::now();
                h.type_text("x");
                started.elapsed()
            })
            .min()
            .unwrap_or_default()
    }

    /// The fastest of a few tries at twenty steps that move the cursor and leave the text as it is.
    fn steps_over_the_same_text(code: &str, language: Language) -> std::time::Duration {
        let mut h = Harness::new(File { code: code.into(), language }, 110, 30);
        h.press("tab");
        (0..3)
            .map(|_| {
                let started = std::time::Instant::now();
                for step in 0..20 {
                    h.press(if step % 2 == 0 { "down" } else { "up" });
                }
                started.elapsed()
            })
            .min()
            .unwrap_or_default()
    }

    #[test]
    fn a_file_of_thousands_of_lines_is_coloured_once_rather_than_once_a_frame() {
        // A view is built anew every frame and an area wraps its whole text every frame, so a
        // frame that coloured the whole text as well would pay for both on every key. Remembering
        // the tokens means the colouring happens when the text or the language changes, and the
        // frames after it take what they took before.
        //
        // So a frame after a key costs a couple of times what the same frame without a language
        // costs, and steps that do not change the text cost the same again and again however long
        // the file is. Both are compared rather than held to a clock: a machine busy building
        // other programs slows the two sides alike. The fastest of a few tries is taken, because
        // load only ever adds time.
        let file = rust_file();
        let (plain, coloured) = (frame_after_a_key(&file, Language::Plain), frame_after_a_key(&file, Language::Rust));
        let ratio = coloured.as_secs_f64() / plain.as_secs_f64().max(1e-6);
        assert!(
            ratio < 4.0,
            "a frame after a key with a language took {ratio:.1} times as long ({plain:?} against {coloured:?})"
        );

        let (plain, coloured) =
            (steps_over_the_same_text(&file, Language::Plain), steps_over_the_same_text(&file, Language::Rust));
        let ratio = coloured.as_secs_f64() / plain.as_secs_f64().max(1e-6);
        assert!(
            ratio < 2.0,
            "steps over an unchanged text took {ratio:.1} times as long ({plain:?} against {coloured:?})"
        );
    }
}
