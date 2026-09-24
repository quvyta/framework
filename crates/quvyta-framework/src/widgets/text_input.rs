//! Single-line text entry.

use std::ops::Range;
use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use super::cells;
use super::edit_menu::{self, EditAction, TextMenu};
use super::editor::Editor;
use crate::event::{Event, KeyEvent, MouseButton, MouseKind};
use crate::geometry::{Padding, Rect, Size, clamp_u16};
use crate::keymap::{Key, Modifiers, Scope};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

type TextMessage<Msg> = Box<dyn Fn(String) -> Msg>;

/// What an event did to a field, before any message is sent.
pub(crate) struct Edit {
    /// Whether the event was used.
    pub(crate) handled: bool,
    /// The new text, when the text changed.
    pub(crate) changed: Option<String>,
    /// Whether Enter asked to submit.
    pub(crate) submit: bool,
}

/// A text field with real editing: cursor, selection, word jumps, undo and clipboard.
///
/// The application owns the value and receives every change through `on_change`; cursor,
/// selection and undo history live in the runtime.
///
/// Keys: ←/→ move (with Ctrl by word, with Shift selecting), Home/End, Backspace/Delete,
/// Ctrl+W deletes a word, Ctrl+U deletes to the start, Ctrl+A selects all, Ctrl+Z undo,
/// Ctrl+Y or Ctrl+Shift+Z redo, Ctrl+C and Ctrl+X copy and cut the selection, Enter submits.
/// With a selection, ← and → clear it and move one step on from its left or right end. Pasting
/// inserts text. Clicking places the cursor; dragging selects.
///
/// [`select_on_focus`](Self::select_on_focus) opens the field with part of its text selected,
/// such as the name without its extension in a rename dialog; typing then replaces just that part.
///
/// A right click (or Shift+F10 and the menu key) opens an edit menu with Cut, Copy, Paste and
/// Select all. A right click inside the selection keeps it; elsewhere it first places the
/// cursor there. Cut and Copy are disabled without a selection, and always in password fields,
/// which never copy; Paste is disabled while there is nothing to paste. Paste reads the system
/// clipboard, then the terminal's, then the text the application copied last. Entry names are
/// `quvyta.edit.*`.
///
/// Style keys: `text-input` (`bg`, `fg`, `padding`) with `hover`, `focus`, `invalid`,
/// `disabled`; `text-input-prompt`, `text-input-placeholder`, `text-input-selection`,
/// `text-input-cursor`.
pub struct TextInput<Msg> {
    value: String,
    placeholder: String,
    password: bool,
    invalid: bool,
    disabled: bool,
    max_length: Option<usize>,
    on_change: Option<TextMessage<Msg>>,
    on_submit: Option<TextMessage<Msg>>,
    accept: Option<Box<dyn Fn(char) -> bool>>,
    select_on_focus: Option<Range<usize>>,
}

#[derive(Debug, Default)]
struct InputMemory {
    editor: Editor,
    synced: Option<String>,
    scroll: usize,
    last_edit: Duration,
    dragging: bool,
    /// Whether the field had focus when last seen, so the focus selection is applied only as it
    /// gains focus.
    focused: bool,
}

impl<Msg: 'static> TextInput<Msg> {
    /// A field showing `value`.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: String::new(),
            password: false,
            invalid: false,
            disabled: false,
            max_length: None,
            on_change: None,
            on_submit: None,
            accept: None,
            select_on_focus: None,
        }
    }

    /// Faint text shown while the field is empty.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Masks every character.
    #[must_use]
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Marks the value as failing validation.
    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Makes the field read-only and unfocusable.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Limits the value to `max` characters.
    #[must_use]
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Message carrying the new value after every edit.
    #[must_use]
    pub fn on_change(mut self, message: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(message));
        self
    }

    /// Message carrying the value when Enter is pressed.
    #[must_use]
    pub fn on_submit(mut self, message: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_submit = Some(Box::new(message));
        self
    }

    /// Selects the characters in `range` each time the field gains focus, with the cursor at the
    /// range's end, e.g. `0..4` to select `main` in `main.rs`. The range counts characters, not
    /// bytes, and is cut to the text. From then on the selection is the user's: typing replaces
    /// it, the arrows drop it, and a value changed from outside does not bring it back. A click
    /// that gives the field focus places the cursor instead.
    #[must_use]
    pub fn select_on_focus(mut self, range: Range<usize>) -> Self {
        self.select_on_focus = Some(range);
        self
    }

    /// Selects the whole text each time the field gains focus, like
    /// [`select_on_focus`](Self::select_on_focus) with a range covering every character.
    #[must_use]
    pub fn select_all_on_focus(self) -> Self {
        self.select_on_focus(0..usize::MAX)
    }

    /// Only characters for which `accept` is true can be typed or pasted.
    pub(crate) fn accept(mut self, accept: impl Fn(char) -> bool + 'static) -> Self {
        self.accept = Some(Box::new(accept));
        self
    }

    /// `text` without the characters the field does not accept.
    fn accepted(&self, text: &str) -> String {
        text.chars().filter(|c| self.accept.as_ref().is_none_or(|accept| accept(*c))).collect()
    }

    fn sync<'m>(&self, memory: &'m mut InputMemory) -> &'m mut InputMemory {
        if memory.synced.as_deref() != Some(self.value.as_str()) {
            if memory.editor.text() != self.value {
                memory.editor.replace_all(&self.value);
            }
            memory.synced = Some(self.value.clone());
        }
        memory
    }

    /// Notes whether the field has focus and, as it gains focus, selects the focus range.
    fn follow_focus(&self, memory: &mut InputMemory, focused: bool) {
        let gained = focused && !memory.focused;
        memory.focused = focused;
        let Some(range) = self.select_on_focus.clone().filter(|_| gained) else {
            return;
        };
        let editor = &mut memory.editor;
        let text = editor.text();
        let chars = text.chars().count();
        let (start, end) = (range.start.min(chars), range.end.min(chars));
        let (start, end) = (grapheme_at_char(text, start), grapheme_at_char(text, end.max(start)));
        editor.move_to_grapheme(start, false);
        editor.move_to_grapheme(end, true);
    }

    /// The glyphs drawn for the text: the text itself or a mask.
    fn shown(&self, text: &str, mask: &str) -> Vec<String> {
        if self.password {
            text.graphemes(true).map(|_| mask.to_owned()).collect()
        } else {
            text.graphemes(true).map(str::to_owned).collect()
        }
    }

    /// The row the text uses inside `area`: after the left padding and the prompt, and short of
    /// as much padding again on the right.
    fn text_row(prompt_width: u16, area: Rect, padding: Padding) -> Rect {
        let left = padding.left.saturating_add(prompt_width);
        Rect::new(
            area.x + i32::from(left),
            area.y + i32::from(padding.top),
            area.width.saturating_sub(left.saturating_add(padding.left)),
            1,
        )
    }
}

/// The index of the grapheme that starts at or after character `index` of `text`, so a range
/// counted in characters never splits a character built from several.
fn grapheme_at_char(text: &str, index: usize) -> usize {
    let byte = text.char_indices().nth(index).map_or(text.len(), |(byte, _)| byte);
    text.grapheme_indices(true).take_while(|(start, _)| *start < byte).count()
}

/// The grapheme index of byte offset `byte` in `text`.
fn grapheme_index(text: &str, byte: usize) -> usize {
    text[..byte].graphemes(true).count()
}

impl<Msg: 'static> Widget<Msg> for TextInput<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let style = cx.env().theme().style("text-input", None, &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((0, 1));
        let prompt = text::width(&cx.env().icons().glyph("prompt")) + 1;
        let content = text::width(&self.value).max(text::width(&self.placeholder)).max(12).saturating_add(1);
        Size::new(
            cells::sum([content, prompt, horizontal.saturating_mul(2)]),
            vertical.saturating_mul(2).saturating_add(1),
        )
        .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.takes_text();
        let mut states = if self.disabled { vec![State::Disabled] } else { cx.states() };
        if self.invalid {
            states.push(State::Invalid);
        }
        let focused = states.contains(&State::Focus);
        let style = cx.style("text-input", None, &states);
        let surface = style.text();
        cx.clear(area, surface.bg.unwrap_or_else(|| cx.color("raised")));
        if !self.disabled {
            cx.register_hit(area);
            edit_menu::request_overlay(cx, area);
        }
        let padding = style.padding();
        // Hover and focus raise the pillar in the left padding; the text never slides, so typing
        // and clicking stay where they are.
        if let Some(color) = style.color("pillar").filter(|_| padding.left >= 1) {
            cx.pillar(area.x, area.y + i32::from(padding.top), color);
        }
        let prompt_glyph = cx.env().icons().glyph("prompt").into_owned();
        let prompt_style = cx.style("text-input-prompt", None, &states).text();
        let prompt_x = area.x + i32::from(padding.left);
        let y = area.y + i32::from(padding.top);
        let prompt_width = cx.text(prompt_x, y, &prompt_glyph, prompt_style, area.width) + 1;
        let field = Self::text_row(prompt_width, area, padding);

        let mask = cx.env().icons().glyph("mask").into_owned();
        let now = cx.now();
        let blink = cx.env().theme().motion().cursor_blink;
        let has_focus = cx.is_focused();
        let (glyphs, cursor_index, selection, last_edit) = {
            let memory = self.sync(cx.memory::<InputMemory>());
            self.follow_focus(memory, has_focus);
            let text = memory.editor.text();
            let cursor_index = grapheme_index(text, memory.editor.cursor());
            let selection =
                memory.editor.selection().map(|r| grapheme_index(text, r.start)..grapheme_index(text, r.end));
            (self.shown(text, &mask), cursor_index, selection, memory.last_edit)
        };

        if glyphs.is_empty() && !focused {
            let placeholder = cx.style("text-input-placeholder", None, &states).text();
            let shown = text::truncate(&self.placeholder, field.width).into_owned();
            cx.text(field.x, field.y, &shown, placeholder, field.width);
            return;
        }
        // An empty focused field keeps its placeholder in place; the cursor sits on its first
        // letter in inverted colours instead of pushing it one cell aside.
        let placeholder_head = if glyphs.is_empty() && !self.placeholder.is_empty() {
            let placeholder = cx.style("text-input-placeholder", None, &states).text();
            let shown = text::truncate(&self.placeholder, field.width).into_owned();
            cx.text(field.x, field.y, &shown, placeholder, field.width);
            shown.graphemes(true).next().map(str::to_owned)
        } else {
            None
        };

        let widths: Vec<u16> = glyphs.iter().map(|g| text::grapheme_width(g).max(1)).collect();
        let scroll = {
            let memory = cx.memory::<InputMemory>();
            let mut scroll = memory.scroll.min(cursor_index);
            let cells = |from: usize, to: usize| widths[from..to].iter().map(|w| u32::from(*w)).sum::<u32>();
            while scroll < cursor_index && cells(scroll, cursor_index) + 1 > u32::from(field.width) {
                scroll += 1;
            }
            memory.scroll = scroll;
            scroll
        };

        let selection_style = cx.style("text-input-selection", None, &states).text();
        let mut text_style = surface;
        text_style.bg = None;
        let mut x = field.x;
        for (index, glyph) in glyphs.iter().enumerate().skip(scroll) {
            let width = widths[index];
            if x + i32::from(width) > field.right() {
                break;
            }
            let style = if selection.as_ref().is_some_and(|r| r.contains(&index)) {
                CellStyle { fg: selection_style.fg.or(text_style.fg), bg: selection_style.bg, ..text_style }
            } else {
                text_style
            };
            cx.text(x, field.y, glyph, style, width);
            if index == cursor_index && focused {
                draw_cursor(cx, x, field.y, glyph, Blink { now, last_edit, period: blink }, &states);
            }
            x += i32::from(width);
        }
        if focused && cursor_index == glyphs.len() && x < field.right() {
            let glyph = placeholder_head.as_deref().unwrap_or(" ");
            draw_cursor(cx, x, field.y, glyph, Blink { now, last_edit, period: blink }, &states);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let selection = self.sync(cx.memory::<InputMemory>()).editor.selection().is_some();
        self.menu(cx.env(), selection, cx.can_paste()).paint_overlay(cx, anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let edit = self.edit(cx, event);
        if let (Some(value), Some(message)) = (edit.changed, &self.on_change) {
            cx.emit(message(value));
        }
        if edit.submit
            && let Some(message) = &self.on_submit
        {
            let value = cx.memory::<InputMemory>().editor.text().to_owned();
            cx.emit(message(value));
        }
        edit.handled
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }
}

impl<Msg: 'static> TextInput<Msg> {
    /// The edit menu; a password field never offers to copy.
    fn menu(&self, env: &crate::env::Env, selection: bool, can_paste: bool) -> TextMenu<EditAction> {
        TextMenu::edit(env, selection && !self.password, can_paste)
    }

    /// The grapheme index a press at column `x` lands before.
    fn index_at(memory: &InputMemory, area: Rect, text_left: u16, x: i32) -> usize {
        let column = usize::from(clamp_u16(x - area.x - i32::from(text_left)));
        let mut cells = 0usize;
        let mut index = memory.scroll;
        for grapheme in memory.editor.text().graphemes(true).skip(memory.scroll) {
            let width = usize::from(text::grapheme_width(grapheme).max(1));
            if cells + width / 2 >= column {
                break;
            }
            cells += width;
            index += 1;
        }
        index
    }

    /// Offers `event` to the edit menu, which opens on a right press or its keys and takes every
    /// event while open. Returns `None` when the menu did not use the event.
    fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event, text_left: u16) -> Option<Edit> {
        let open = edit_menu::is_open(cx);
        if !open && !edit_menu::asks(event) {
            return None;
        }
        let area = cx.area();
        if let Event::Mouse(mouse) = event
            && !open
        {
            // A right press inside the selection keeps it; elsewhere it places the cursor first.
            let memory = self.sync(cx.memory::<InputMemory>());
            let index = Self::index_at(memory, area, text_left, mouse.x);
            let text = memory.editor.text();
            let inside = memory
                .editor
                .selection()
                .is_some_and(|r| (grapheme_index(text, r.start)..grapheme_index(text, r.end)).contains(&index));
            if !inside {
                memory.editor.move_to_grapheme(index, false);
            }
        }
        let selection = self.sync(cx.memory::<InputMemory>()).editor.selection().is_some();
        let (used, chosen) = self.menu(cx.env(), selection, cx.can_paste()).event(cx, event);
        if used && !open {
            cx.probe_clipboard();
        }
        let mut edit = Edit { handled: used, changed: None, submit: false };
        let Some(action) = chosen else {
            return used.then_some(edit);
        };
        let now = cx.now();
        let memory = self.sync(cx.memory::<InputMemory>());
        memory.last_edit = now;
        let editor = &mut memory.editor;
        let mut copy = None;
        match action {
            EditAction::Cut | EditAction::Copy if !self.password => {
                copy = editor.selected_text().map(str::to_owned);
                if action == EditAction::Cut && copy.is_some() && editor.backspace() {
                    edit.changed = Some(editor.text().to_owned());
                    memory.synced = edit.changed.clone();
                }
            }
            EditAction::Cut | EditAction::Copy | EditAction::Paste => {}
            EditAction::SelectAll => editor.select_all(),
        }
        if action == EditAction::Paste {
            cx.run_action(Scope::Global, "paste");
        }
        if let Some(text) = copy {
            cx.copy(text);
        }
        edit.handled = true;
        Some(edit)
    }

    /// Applies a key to `editor`: editing, moving, the clipboard chords and Enter.
    fn key(&self, editor: &mut Editor, key: &KeyEvent) -> KeyEdit {
        let mods = key.chord.mods;
        let ctrl = mods.ctrl && !mods.alt;
        let mut edit = KeyEdit { handled: true, ..KeyEdit::default() };
        match key.chord.key {
            Key::Char(c) if ctrl => match (c, mods.shift) {
                ('a', false) => editor.select_all(),
                ('z', false) => edit.changed = editor.undo(),
                ('y', false) | ('z', true) => edit.changed = editor.redo(),
                ('w', false) => edit.changed = editor.delete_word_back(),
                ('u', false) => edit.changed = editor.delete_to_start(),
                // A password field never copies.
                ('c', false) | ('x', false) if !self.password => {
                    edit.copy = editor.selected_text().map(str::to_owned);
                    if c == 'x' && edit.copy.is_some() {
                        edit.changed = editor.backspace();
                    }
                    edit.handled = edit.copy.is_some();
                }
                _ => edit.handled = false,
            },
            Key::Left => editor.move_left(mods.shift, mods.ctrl),
            Key::Right => editor.move_right(mods.shift, mods.ctrl),
            Key::Home => editor.move_home(mods.shift),
            Key::End => editor.move_end(mods.shift),
            Key::Backspace if mods == Modifiers::default() || mods.ctrl => {
                edit.changed = if mods.ctrl { editor.delete_word_back() } else { editor.backspace() };
            }
            Key::Delete => edit.changed = editor.delete(),
            Key::Enter if mods == Modifiers::default() => {
                edit.submit = self.on_submit.is_some();
                edit.handled = edit.submit;
            }
            _ => match key.text {
                Some(c) if !mods.ctrl && !mods.alt => {
                    edit.changed = editor.insert(&self.accepted(&c.to_string()), self.max_length);
                }
                _ => edit.handled = false,
            },
        }
        edit
    }

    /// Applies `event` to the text, cursor and selection and copies to the clipboard, without
    /// sending messages; fields built on a text input decide what the change means.
    pub(crate) fn edit(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> Edit {
        let unused = Edit { handled: false, changed: None, submit: false };
        if self.disabled {
            return unused;
        }
        let max = self.max_length;
        let now = cx.now();
        let area = cx.area();
        let text_left = {
            let padding = cx.env().theme().style("text-input", None, &[]).pair("padding").unwrap_or((0, 1)).1;
            cells::sum([padding, text::width(&cx.env().icons().glyph("prompt")), 1])
        };
        // A click that brings focus places the cursor itself, so it counts as the focus having
        // been seen already; any other event applies the focus selection first if no frame has.
        let has_focus = cx.is_focused();
        {
            let memory = self.sync(cx.memory::<InputMemory>());
            if matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::Down(_))) {
                memory.focused = has_focus;
            } else {
                self.follow_focus(memory, has_focus);
            }
        }
        if let Some(edit) = self.menu_event(cx, event, text_left) {
            return edit;
        }
        let (handled, changed, submit, copy) = {
            let memory = self.sync(cx.memory::<InputMemory>());
            let editor = &mut memory.editor;
            let mut changed = false;
            let mut submit = false;
            let mut copy = None;
            let handled = match event {
                Event::Paste(text) => {
                    changed = editor.insert(&self.accepted(&text.replace(['\n', '\r'], " ")), max);
                    true
                }
                Event::Key(key) => {
                    let edit = self.key(editor, key);
                    (changed, submit, copy) = (edit.changed, edit.submit, edit.copy);
                    edit.handled
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseKind::Down(MouseButton::Left) | MouseKind::Drag(MouseButton::Left) => {
                        let dragging = matches!(mouse.kind, MouseKind::Drag(_));
                        let index = Self::index_at(memory, area, text_left, mouse.x);
                        memory.editor.move_to_grapheme(index, dragging);
                        memory.dragging = true;
                        true
                    }
                    MouseKind::Up(MouseButton::Left) => {
                        memory.dragging = false;
                        true
                    }
                    _ => false,
                },
                Event::PointerOutside => false,
            };
            if handled {
                memory.last_edit = now;
            }
            if changed {
                memory.synced = Some(memory.editor.text().to_owned());
            }
            (handled, changed.then(|| memory.editor.text().to_owned()), submit, copy)
        };
        if let Event::Mouse(mouse) = event
            && mouse.kind == MouseKind::Down(MouseButton::Left)
        {
            cx.capture_pointer();
        }
        if let Some(text) = copy {
            cx.copy(text);
        }
        Edit { handled, changed, submit }
    }
}

/// What a key did to a field's editor.
#[derive(Default)]
struct KeyEdit {
    handled: bool,
    changed: bool,
    submit: bool,
    copy: Option<String>,
}

/// Timing of the cursor blink.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Blink {
    pub(crate) now: Duration,
    pub(crate) last_edit: Duration,
    pub(crate) period: Duration,
}

/// Draws the blinking block cursor over the glyph at `x`. The cursor stays solid for one blink
/// period after every edit so it never disappears while typing.
pub(crate) fn draw_cursor(cx: &mut PaintCx<'_>, x: i32, y: i32, glyph: &str, blink: Blink, states: &[State]) {
    let since = blink.now.saturating_sub(blink.last_edit);
    let period = blink.period.max(Duration::from_millis(1));
    let phase = since.as_millis() / period.as_millis();
    let next = period.saturating_mul(u32::try_from(phase + 1).unwrap_or(u32::MAX)).saturating_sub(since);
    cx.request_frame_in(next);
    if phase.is_multiple_of(2) {
        let style = cx.style("text-input-cursor", None, states).text();
        cx.text(x, y, glyph, style, text::width(glyph).max(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    #[derive(Default)]
    struct Demo {
        value: String,
        submitted: Option<String>,
        password: bool,
    }

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
                TextInput::new(&self.value)
                    .placeholder("Project name")
                    .password(self.password)
                    .max_length(20)
                    .on_change(Msg::Changed)
                    .on_submit(Msg::Submitted),
            )
            .fill_width()
            .id("name");
        }
    }

    #[test]
    fn types_edits_and_submits() {
        let mut h = Harness::new(Demo::default(), 30, 1);
        assert_eq!(h.screen(), "  ❯ Project name\n");
        h.press("tab").type_text("quvyta framework");
        assert_eq!(h.app().value, "quvyta framework");
        h.press("ctrl+w").press("ctrl+w");
        assert_eq!(h.app().value, "");
        h.press("ctrl+z");
        assert_eq!(h.app().value, "quvyta ");
        h.press("enter");
        assert_eq!(h.app().submitted.as_deref(), Some("quvyta "));
    }

    #[test]
    fn selection_copy_and_paste() {
        let mut h = Harness::new(Demo { value: "hello world".into(), ..Demo::default() }, 30, 1);
        h.press("tab").press("ctrl+shift+left").press("ctrl+c");
        assert_eq!(h.copied(), &["world".to_owned()]);
        h.paste("there");
        assert_eq!(h.app().value, "hello there");
    }

    #[test]
    fn scrolls_long_text_and_masks_passwords() {
        let mut h = Harness::new(Demo { password: true, ..Demo::default() }, 12, 1);
        h.press("tab").type_text("abcdefghijklmn");
        let screen = h.screen();
        assert_eq!(h.app().value, "abcdefghijklmn");
        assert!(screen.starts_with("▌ ❯ •••••"), "{screen}");
        assert!(!screen.contains('a'));
    }

    fn right_click(h: &mut Harness<Demo>, x: i32, y: i32) {
        h.mouse(MouseKind::Down(MouseButton::Right), x, y);
        h.mouse(MouseKind::Up(MouseButton::Right), x, y);
    }

    fn menu_harness(value: &str, password: bool) -> Harness<Demo> {
        let mut h = Harness::new(Demo { value: value.into(), password, ..Demo::default() }, 30, 7);
        h.set_reduced_motion(true);
        h
    }

    #[test]
    fn right_click_opens_an_edit_menu_whose_entries_need_a_selection_or_a_clipboard() {
        let mut h = menu_harness("hello world", false);
        right_click(&mut h, 12, 0);
        assert_eq!(
            h.screen(),
            "▌ ❯ hello world\n        Cut           ctrl x\n        Copy          ctrl c\n        Paste         ctrl v\n        Select all    ctrl a\n\n\n"
        );
        let theme = h.env().theme();
        let muted = theme.color("muted");
        assert_eq!((h.fg(8, 1), h.fg(8, 2), h.fg(8, 3)), (muted, muted, muted), "nothing selected, nothing to paste");
        assert_ne!(h.fg(8, 4), muted, "Select all always works");
        h.press("down").press("enter");
        assert!(!h.screen().contains("Cut"), "choosing closes the menu");
        right_click(&mut h, 12, 0);
        assert_ne!(h.fg(8, 1), muted, "a right click inside the selection keeps it: Cut is enabled");
        h.click_text("Copy");
        assert_eq!(h.clipboard(), Some("hello world"));
        right_click(&mut h, 12, 0);
        assert_ne!(h.fg(8, 3), muted, "now there is something to paste");
        h.click_text("Cut");
        assert_eq!(h.app().value, "");
        right_click(&mut h, 6, 0);
        h.click_text("Paste");
        assert_eq!(h.app().value, "hello world");
        assert!(h.is_focused("name"));
    }

    #[test]
    fn right_click_outside_the_selection_places_the_cursor_first() {
        let mut h = menu_harness("hello world", false);
        h.press("tab").press("ctrl+shift+left");
        right_click(&mut h, 4, 0);
        let theme = h.env().theme();
        assert_eq!(h.fg(8, 1), theme.color("muted"), "the selection is gone: Cut is disabled");
        h.press("esc").type_text("X");
        assert_eq!(h.app().value, "Xhello world");
    }

    #[test]
    fn paste_reads_the_system_clipboard_before_the_last_copy_inside_the_application() {
        let mut h = menu_harness("hello", false);
        h.press("tab").press("ctrl+a").press("ctrl+c").press("end");
        h.set_system_clipboard(Some(" from web"));
        right_click(&mut h, 20, 0);
        h.click_text("Paste");
        assert_eq!(h.app().value, "hello from web");
        h.set_system_clipboard(None).press("ctrl+v");
        assert_eq!(h.app().value, "hello from webhello", "an empty system clipboard falls back");
    }

    #[test]
    fn shift_f10_and_the_menu_key_open_it_under_the_field() {
        let mut h = menu_harness("hello", false);
        h.press("tab").press("shift+f10");
        assert!(h.screen().lines().nth(1).is_some_and(|line| line.contains("Cut")), "{}", h.screen());
        h.press("esc");
        assert!(!h.screen().contains("Cut"));
        h.press("menu").press("up").press("enter").type_text("!");
        assert_eq!(h.app().value, "!", "↑ wrapped to Select all");
    }

    #[test]
    fn the_menu_fits_a_narrow_screen_in_ascii() {
        let mut h = Harness::new(Demo { value: "hello".into(), ..Demo::default() }, 18, 6);
        h.set_reduced_motion(true);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        right_click(&mut h, 15, 0);
        // The menu moves left to stay on screen and cuts the longest entry with an ASCII mark; the pillar of the
        // focused field is a coloured cell in ASCII.
        assert_eq!(h.screen(), "  > hello\n  Cut     ctrl x\n  Copy    ctrl c\n  Paste   ctrl v\n  Selec~  ctrl a\n\n");
        assert_ne!(h.bg(0, 0), h.bg(1, 0), "the pillar cell stands out from the field");
    }

    #[test]
    fn password_fields_never_copy() {
        let mut h = menu_harness("secret", true);
        h.press("tab").press("ctrl+a").press("ctrl+c").press("ctrl+x");
        assert!(h.copied().is_empty());
        assert_eq!(h.app().value, "secret");
        right_click(&mut h, 5, 0);
        let theme = h.env().theme();
        assert_eq!((h.fg(8, 1), h.fg(8, 2)), (theme.color("muted"), theme.color("muted")));
    }

    /// A rename field that opens with part of the name selected.
    struct Rename {
        value: String,
        range: Option<Range<usize>>,
        all: bool,
    }

    impl App for Rename {
        type Msg = String;
        fn update(&mut self, value: String) -> Command<String> {
            self.value = value;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, String>) {
            let mut input = TextInput::new(&self.value).on_change(|value| value);
            if let Some(range) = self.range.clone() {
                input = input.select_on_focus(range);
            }
            if self.all {
                input = input.select_all_on_focus();
            }
            ui.add(input).fill_width().id("name");
        }
    }

    fn rename(value: &str, range: Option<Range<usize>>) -> Harness<Rename> {
        Harness::new(Rename { value: value.into(), range, all: false }, 30, 1)
    }

    #[test]
    fn typing_replaces_the_part_selected_on_focus() {
        let mut h = rename("main.rs", Some(0..4));
        h.press("tab");
        assert_eq!(h.screen(), "▌ ❯ main.rs\n");
        let (selected, plain) = (h.bg(4, 0), h.bg(9, 0));
        assert_ne!(selected, plain, "main is selected");
        assert_eq!(h.bg(7, 0), selected, "all four letters");
        h.type_text("x");
        assert_eq!(h.app().value, "x.rs");
    }

    #[test]
    fn the_cursor_stands_at_the_end_of_the_range_and_an_arrow_drops_the_selection() {
        let mut h = rename("main.rs", Some(0..4));
        h.press("tab").press("right").type_text("_");
        assert_eq!(h.app().value, "main._rs", "→ leaves the selection one step on from its end");
        let mut h = rename("main.rs", Some(0..4));
        h.press("tab").press("shift+right").type_text("x");
        assert_eq!(h.app().value, "xrs", "the cursor was at the range's end, so shift+→ grows it");
    }

    #[test]
    fn without_a_range_the_field_opens_as_before() {
        let mut h = rename("main.rs", None);
        h.press("tab").type_text("x");
        assert_eq!(h.app().value, "main.rsx");
    }

    #[test]
    fn the_range_counts_characters_and_is_cut_to_the_text() {
        let mut h = rename("şğü.txt", Some(0..3));
        h.press("tab").type_text("a");
        assert_eq!(h.app().value, "a.txt", "three Turkish letters are three characters, six bytes");
        let mut h = rename("çay", Some(1..40));
        h.press("tab").type_text("ok");
        assert_eq!(h.app().value, "çok");
        let mut h = Harness::new(Rename { value: "e\u{301}te".into(), range: Some(0..1), all: false }, 30, 1);
        h.press("tab").type_text("a");
        assert_eq!(h.app().value, "ate", "a range never splits an accented letter built from two characters");
    }

    #[test]
    fn select_all_on_focus_selects_everything() {
        let mut h = Harness::new(Rename { value: "draft".into(), range: None, all: true }, 30, 1);
        h.press("tab").type_text("final");
        assert_eq!(h.app().value, "final");
    }

    #[test]
    fn a_value_changed_from_outside_keeps_the_users_selection() {
        let mut h = rename("main.rs", Some(0..4));
        h.press("tab").press("end");
        h.send("lib.rs".to_owned()).type_text("!");
        assert_eq!(h.app().value, "lib.rs!", "the new value is not selected again");
    }

    #[test]
    fn a_click_that_brings_focus_places_the_cursor() {
        let mut h = rename("main.rs", Some(0..4));
        h.click(9, 0).type_text("X");
        assert_eq!(h.app().value, "main.Xrs", "the cursor lands where clicked, before r");
    }

    #[test]
    fn coming_back_selects_the_range_again() {
        struct Two(String);
        impl App for Two {
            type Msg = String;
            fn update(&mut self, value: String) -> Command<String> {
                self.0 = value;
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, String>) {
                ui.add(TextInput::new(&self.0).select_on_focus(0..4).on_change(|value| value)).fill_width();
                ui.add(TextInput::new("other")).fill_width();
            }
        }
        let mut h = Harness::new(Two("main.rs".into()), 30, 2);
        h.press("tab").press("end").press("tab").press("shift+tab").type_text("x");
        assert_eq!(h.app().0, "x.rs");
    }

    #[test]
    fn click_places_cursor_and_value_changes_resync() {
        let mut h = Harness::new(Demo { value: "abcd".into(), ..Demo::default() }, 30, 1);
        h.click(5, 0).type_text("X");
        assert_eq!(h.app().value, "aXbcd");
    }
}
