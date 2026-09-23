//! Streaming log lines: follow the tail, filter by level, search with highlights, copy lines.

use std::collections::VecDeque;

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, Modifiers};
use crate::text;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::edit_menu::{self, TextMenu};
use super::log_buffer::{LogBuffer, LogLevel, LogLine};
use super::row::LEAD;
use super::rows::{self, RowScroll};
use crate::runtime::CopyKind;

/// Width of the level column.
const LEVEL_WIDTH: u16 = 5;

/// Builds a message from a number of copied lines.
type CopyMessage<Msg> = Box<dyn Fn(usize) -> Msg>;

/// A view of a [`LogBuffer`] that follows new lines as they arrive.
///
/// While the view is at the bottom it follows the tail; scrolling up (wheel, keys, scrollbar)
/// stops following and a faint note at the bottom counts the lines below; reaching the bottom
/// again, End or a click on the note resumes. Each line shows its faint timestamp, its level as
/// a word in the level's status colour and the message. [`LogView::min_level`] hides less
/// important lines and [`LogView::search`] keeps only lines containing the query, with the
/// matches highlighted; a lowercase query ignores case. Only the lines on screen are drawn and
/// filtering is incremental, so large buffers stay fast.
///
/// Keys while focused: ↑/↓ or k/j move a line cursor (shift extends a selection), PgUp/PgDn
/// page, Home jumps to the oldest line, End follows the tail again, Esc clears the cursor, `c`
/// copies the selected lines or the cursor line. A click places the cursor, shift-click or
/// dragging extends the selection.
///
/// A right click on the selected lines (elsewhere it first places the cursor on that line), or
/// Shift+F10 and the menu key, opens a menu with Copy and Raw copy. Copy writes each line as
/// `time level message` with single spaces, like `c`; Raw copy keeps the columns lined up as
/// they are shown.
///
/// Style keys: `list-item` (`hover`, `selected`, `focus`) for the rows like
/// [`List`](super::List); `log-time`; `log-level.<level>` for `trace`, `debug`, `info`, `warn`,
/// `error`; `log-match` for search matches; `log-more` for the lines-below note; `list-header`
/// for the empty text; `scrollbar`. Framework strings: `quvyta.log.below`,
/// `quvyta.log.no-match`.
pub struct LogView<Msg> {
    buffer: LogBuffer,
    min_level: LogLevel,
    query: String,
    empty: String,
    on_copy: Option<CopyMessage<Msg>>,
}

/// The lines that pass the filters, kept up to date as lines arrive.
#[derive(Debug, Default)]
struct Filter {
    key: Option<(u64, LogLevel, String)>,
    /// Permanent number of the next line to look at.
    scanned: u64,
    /// Permanent numbers of passing lines, oldest first.
    lines: VecDeque<u64>,
}

#[derive(Debug, Default)]
struct LogMemory {
    filter: Filter,
    /// Whether the user scrolled away from the tail.
    detached: bool,
    cursor: Option<u64>,
    anchor: Option<u64>,
    selecting: bool,
    /// Where the lines-below note was painted in the last frame; a click on it follows the tail.
    note: Option<Rect>,
}

impl<Msg: 'static> LogView<Msg> {
    /// A view of `buffer`; cloning a buffer is cheap.
    #[must_use]
    pub fn new(buffer: &LogBuffer) -> Self {
        Self {
            buffer: buffer.clone(),
            min_level: LogLevel::Trace,
            query: String::new(),
            empty: String::new(),
            on_copy: None,
        }
    }

    /// Hides lines less important than `level`.
    #[must_use]
    pub fn min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Shows only lines containing `query` and highlights it; empty shows everything.
    #[must_use]
    pub fn search(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// Text shown while the buffer is empty.
    #[must_use]
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    /// Message sent after `c` copied lines, with how many.
    #[must_use]
    pub fn on_copy(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_copy = Some(Box::new(message));
        self
    }

    fn filtering(&self) -> bool {
        self.min_level > LogLevel::Trace || !self.query.is_empty()
    }

    fn passes(&self, line: &LogLine) -> bool {
        line.level() >= self.min_level && (self.query.is_empty() || !matches(line.text(), &self.query).is_empty())
    }

    /// Brings the filter up to date with the buffer.
    fn refresh(&self, filter: &mut Filter) {
        if !self.filtering() {
            return;
        }
        let key = (self.buffer.id(), self.min_level, self.query.clone());
        let first = self.buffer.first_number();
        if filter.key.as_ref() != Some(&key) {
            *filter = Filter { key: Some(key), scanned: first, lines: VecDeque::new() };
        }
        while filter.lines.front().is_some_and(|number| *number < first) {
            filter.lines.pop_front();
        }
        let end = first + self.buffer.len() as u64;
        for number in filter.scanned.max(first)..end {
            if self.buffer.by_number(number).is_some_and(|line| self.passes(line)) {
                filter.lines.push_back(number);
            }
        }
        filter.scanned = end;
    }

    fn total(&self, filter: &Filter) -> usize {
        if self.filtering() { filter.lines.len() } else { self.buffer.len() }
    }

    /// The permanent number of the `row`-th visible line.
    fn number_at(&self, filter: &Filter, row: usize) -> Option<u64> {
        if self.filtering() {
            filter.lines.get(row).copied()
        } else {
            (row < self.buffer.len()).then(|| self.buffer.first_number() + row as u64)
        }
    }

    /// The row of line `number`, or the row of the nearest passing line after it.
    fn row_of(&self, filter: &Filter, number: u64) -> usize {
        if self.filtering() {
            filter.lines.partition_point(|n| *n < number)
        } else {
            usize::try_from(number.saturating_sub(self.buffer.first_number())).unwrap_or(usize::MAX)
        }
    }

    fn selection(memory: &LogMemory) -> Option<(u64, u64)> {
        let cursor = memory.cursor?;
        let anchor = memory.anchor.unwrap_or(cursor);
        Some((cursor.min(anchor), cursor.max(anchor)))
    }

    fn copy(&self, cx: &mut EventCx<'_, Msg>, kind: CopyKind) -> bool {
        let memory = cx.memory::<LogMemory>();
        let Some((from, to)) = Self::selection(memory) else {
            return false;
        };
        self.refresh(&mut memory.filter);
        let start = self.row_of(&memory.filter, from);
        let mut lines = Vec::new();
        let mut row = start;
        while let Some(number) = self.number_at(&memory.filter, row).filter(|n| *n <= to) {
            if let Some(line) = self.buffer.by_number(number) {
                let (gap, width) = match kind {
                    CopyKind::Clean => (" ", 0),
                    CopyKind::Raw => ("  ", usize::from(LEVEL_WIDTH)),
                };
                let mut out = String::new();
                if let Some(time) = line.timestamp() {
                    out.push_str(time);
                    out.push_str(gap);
                }
                out.push_str(&format!("{:<width$}", line.level().name()));
                out.push_str(gap);
                out.push_str(line.text());
                lines.push(out);
            }
            row += 1;
        }
        if lines.is_empty() {
            return false;
        }
        let count = lines.len();
        cx.copy(lines.join("\n"));
        cx.flash();
        if let Some(message) = &self.on_copy {
            cx.emit(message(count));
        }
        true
    }

    /// Moves the cursor `delta` rows (or to a row), keeps it in view and updates following.
    fn move_cursor(&self, cx: &mut EventCx<'_, Msg>, target: CursorTarget, extend: bool) -> bool {
        let area = cx.area();
        let visible = usize::from(area.height);
        let offset = cx.memory::<RowScroll>().offset;
        let memory = cx.memory::<LogMemory>();
        self.refresh(&mut memory.filter);
        let total = self.total(&memory.filter);
        if total == 0 {
            return false;
        }
        let current = memory.cursor.map(|number| self.row_of(&memory.filter, number).min(total - 1));
        let row = match (target, current) {
            (CursorTarget::By(delta), Some(row)) => row.saturating_add_signed(delta).min(total - 1),
            // The first move starts from the bottom line on screen.
            (CursorTarget::By(_), None) => (offset + visible).min(total).saturating_sub(1),
            (CursorTarget::First, _) => 0,
        };
        let number = self.number_at(&memory.filter, row);
        if !extend || memory.anchor.is_none() {
            memory.anchor = if extend { memory.cursor.or(number) } else { number };
        }
        memory.cursor = number;
        let scroll = cx.memory::<RowScroll>();
        if row < scroll.offset {
            scroll.offset = row;
        } else if visible > 0 && row >= scroll.offset + visible {
            scroll.offset = row + 1 - visible;
        }
        let at_bottom = scroll.offset + visible >= total;
        cx.memory::<LogMemory>().detached = !at_bottom;
        true
    }

    /// Offers `event` to the Copy and Raw copy menu, which opens on a right press on a line or
    /// its keys while lines are selected, and takes every event while open. Returns `None` when
    /// the menu did not use the event.
    fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> Option<bool> {
        let open = edit_menu::is_open(cx);
        if !open && !edit_menu::asks(event) {
            return None;
        }
        if let Event::Mouse(mouse) = event
            && !open
        {
            let area = cx.area();
            let offset = cx.memory::<RowScroll>().offset;
            let memory = cx.memory::<LogMemory>();
            self.refresh(&mut memory.filter);
            let row = usize::try_from(mouse.y - area.y).ok().map(|row| offset + row);
            let number = row.and_then(|row| self.number_at(&memory.filter, row))?;
            // A right press on the selected lines keeps them; elsewhere it selects that line.
            if !Self::selection(memory).is_some_and(|(from, to)| (from..=to).contains(&number)) {
                memory.cursor = Some(number);
                memory.anchor = Some(number);
                memory.detached = true;
            }
        }
        if !open && Self::selection(cx.memory::<LogMemory>()).is_none() {
            return None;
        }
        let (used, chosen) = TextMenu::copy(cx.env()).event(cx, event);
        if let Some(kind) = chosen {
            self.copy(cx, kind);
        }
        used.then_some(true)
    }

    fn follow_tail(cx: &mut EventCx<'_, Msg>) {
        let memory = cx.memory::<LogMemory>();
        memory.detached = false;
        memory.cursor = None;
        memory.anchor = None;
    }

    fn paint_line(&self, cx: &mut PaintCx<'_>, rect: Rect, line: &LogLine, selected: bool, focused: bool) {
        let hovered = cx.pointer().is_some_and(|(x, y)| rect.contains(x, y));
        let states = rows::row_states(hovered, selected, focused, false);
        let style = cx.style("list-item", None, &states);
        let row_style = rows::paint_row(cx, rect, &style);
        let mut x = rect.x + i32::from(LEAD + rows::slide(cx, &states));
        // One spare cell keeps the sliding text inside the row.
        let right = rect.right() - 1;
        let room = |x: i32| clamp_u16(right - x);
        if let Some(time) = line.timestamp() {
            let time_style = cx.style("log-time", None, &states).text();
            x += i32::from(cx.text(x, rect.y, time, time_style, room(x))) + 2;
        }
        let level_style = cx.style("log-level", Some(line.level().name()), &states).text();
        cx.text(x, rect.y, line.level().name(), level_style, room(x).min(LEVEL_WIDTH));
        x += i32::from(LEVEL_WIDTH) + 2;
        let budget = room(x);
        let shown = text::truncate(line.text(), budget);
        cx.text(x, rect.y, &shown, row_style, budget);
        if self.query.is_empty() {
            return;
        }
        let mut match_style = cx.style("log-match", None, &states).text();
        if match_style.fg.is_none() {
            match_style.fg = row_style.fg;
        }
        let limit = shown.len().saturating_sub(if shown.len() < line.text().len() { text::ELLIPSIS.len() } else { 0 });
        for (start, end) in matches(line.text(), &self.query) {
            if end > limit {
                break;
            }
            let dx = i32::from(text::width(&line.text()[..start]));
            cx.text(x + dx, rect.y, &line.text()[start..end], match_style, budget);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CursorTarget {
    By(isize),
    First,
}

/// Byte ranges of `query` in `text`; a query without uppercase letters ignores case.
fn matches(text: &str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let fold = !query.chars().any(char::is_uppercase);
    let needle: Vec<char> = query.chars().collect();
    let same = |a: char, b: char| if fold { a.to_lowercase().eq(b.to_lowercase()) } else { a == b };
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        if needle.iter().enumerate().all(|(k, c)| same(chars[i + k].1, *c)) {
            let end = chars.get(i + needle.len()).map_or(text.len(), |(byte, _)| *byte);
            out.push((chars[i].0, end));
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

impl<Msg: 'static> Widget<Msg> for LogView<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let rows = clamp_u16(i32::try_from(self.buffer.len().max(1)).unwrap_or(i32::MAX));
        Size::new(available.width, rows).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        edit_menu::request_overlay(cx, area);
        let visible = usize::from(area.height);
        let focused = cx.is_focused();
        let (total, detached, selection) = {
            let memory = cx.memory::<LogMemory>();
            self.refresh(&mut memory.filter);
            (self.total(&memory.filter), memory.detached, Self::selection(memory))
        };
        if total == 0 {
            cx.memory::<LogMemory>().note = None;
            let faint = cx.style("list-header", None, &[]).text();
            let text = if self.buffer.is_empty() {
                self.empty.clone()
            } else {
                crate::i18n::translate_active("quvyta.log.no-match", &[])
            };
            cx.text(area.x + i32::from(LEAD), area.y, &text, faint, area.width.saturating_sub(LEAD));
            return;
        }
        let max_offset = total.saturating_sub(visible);
        let offset = {
            let scroll = cx.memory::<RowScroll>();
            scroll.offset = if detached { scroll.offset.min(max_offset) } else { max_offset };
            scroll.offset
        };
        let width = area.width.saturating_sub(u16::from(total > visible));
        for row in 0..visible.min(total - offset) {
            let memory = cx.memory::<LogMemory>();
            let Some(number) = self.number_at(&memory.filter, offset + row) else { break };
            let Some(line) = self.buffer.by_number(number) else { continue };
            let selected = selection.is_some_and(|(from, to)| (from..=to).contains(&number));
            let rect = Rect::new(area.x, area.y + i32::try_from(row).unwrap_or(0), width, 1);
            self.paint_line(cx, rect, line, selected, focused);
        }
        rows::paint_scrollbar(cx, area, total, offset, None);
        let below = total - (offset + visible).min(total);
        let note = (detached && below > 0).then(|| {
            let note = rows::paint_below_note(cx, Rect::new(area.x, area.y, width, area.height), below);
            cx.register_hit(note);
            note
        });
        cx.memory::<LogMemory>().note = note;
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        TextMenu::copy(cx.env()).paint_overlay(cx, anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if let Some(used) = self.menu_event(cx, event) {
            return used;
        }
        let area = cx.area();
        let page = isize::try_from(area.height.max(1)).unwrap_or(1);
        match event {
            Event::Key(key) => {
                let shift = Modifiers { shift: true, ..Modifiers::default() };
                let extend = key.chord.mods == shift;
                let plain_or_shift = key.chord.mods == Modifiers::default() || extend;
                let delta = match key.chord.key {
                    Key::Up | Key::Char('k') if plain_or_shift => Some(-1),
                    Key::Down | Key::Char('j') if plain_or_shift => Some(1),
                    Key::PageUp if plain_or_shift => Some(-page),
                    Key::PageDown if plain_or_shift => Some(page),
                    _ => None,
                };
                if let Some(delta) = delta {
                    return self.move_cursor(cx, CursorTarget::By(delta), extend);
                }
                if key.is_plain(Key::Home) {
                    return self.move_cursor(cx, CursorTarget::First, false);
                }
                if key.is_plain(Key::End) {
                    Self::follow_tail(cx);
                    return true;
                }
                if key.is_plain(Key::Esc) {
                    let memory = cx.memory::<LogMemory>();
                    let had = memory.cursor.is_some();
                    memory.cursor = None;
                    memory.anchor = None;
                    return had;
                }
                if key.is_plain(Key::Char('c')) {
                    return self.copy(cx, CopyKind::Clean);
                }
                false
            }
            Event::Mouse(mouse) => {
                let total = {
                    let memory = cx.memory::<LogMemory>();
                    self.refresh(&mut memory.filter);
                    self.total(&memory.filter)
                };
                if rows::scroll_mouse(cx, mouse, area, total) {
                    let offset = cx.memory::<RowScroll>().offset;
                    cx.memory::<LogMemory>().detached = offset + usize::from(area.height) < total;
                    return true;
                }
                let offset = cx.memory::<RowScroll>().offset;
                let row = usize::try_from(mouse.y - area.y).ok().map(|r| offset + r).filter(|r| *r < total);
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) => {
                        let visible = usize::from(area.height);
                        if cx.memory::<LogMemory>().note.is_some_and(|note| note.contains(mouse.x, mouse.y)) {
                            Self::follow_tail(cx);
                            return true;
                        }
                        let Some(row) = row else { return false };
                        let memory = cx.memory::<LogMemory>();
                        let number = self.number_at(&memory.filter, row);
                        if !(mouse.mods.shift && memory.cursor.is_some()) {
                            memory.anchor = number;
                        }
                        memory.cursor = number;
                        memory.selecting = true;
                        memory.detached = offset + visible < total;
                        cx.capture_pointer();
                        true
                    }
                    MouseKind::Drag(MouseButton::Left) if cx.memory::<LogMemory>().selecting => {
                        // The view can lose its rows while a drag is on; the top row stands in.
                        let clamped = (mouse.y - area.y).clamp(0, i32::from(area.height.saturating_sub(1)));
                        let row = (offset + usize::try_from(clamped).unwrap_or(0)).min(total.saturating_sub(1));
                        let memory = cx.memory::<LogMemory>();
                        memory.cursor = self.number_at(&memory.filter, row).or(memory.cursor);
                        memory.detached = true;
                        true
                    }
                    MouseKind::Up(MouseButton::Left) if cx.memory::<LogMemory>().selecting => {
                        cx.memory::<LogMemory>().selecting = false;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo {
        buffer: LogBuffer,
        level: LogLevel,
        query: String,
        copies: Vec<usize>,
    }

    #[derive(Clone)]
    enum Msg {
        Push(LogLine),
        Copied(usize),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Push(line) => self.buffer.push(line),
                Msg::Copied(n) => self.copies.push(n),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let view = LogView::new(&self.buffer)
                .min_level(self.level)
                .search(self.query.clone())
                .empty_text("Waiting for logs")
                .on_copy(Msg::Copied);
            ui.add(view).fill().id("log");
        }
    }

    fn line(n: usize) -> LogLine {
        let level = if n % 5 == 4 { LogLevel::Warn } else { LogLevel::Info };
        LogLine::new(level, format!("request {n} served")).time(format!("12:00:{:02}", n % 60))
    }

    fn demo(count: usize) -> Harness<Demo> {
        let mut buffer = LogBuffer::new(1000);
        for n in 0..count {
            buffer.push(line(n));
        }
        let mut h =
            Harness::new(Demo { buffer, level: LogLevel::Trace, query: String::new(), copies: Vec::new() }, 44, 4);
        h.set_glyph_mode(GlyphMode::Unicode);
        h
    }

    #[test]
    fn follows_the_tail_until_scrolled_up() {
        let mut h = demo(10);
        assert_eq!(
            h.screen(),
            "  12:00:06  info   request 6 served\n  12:00:07  info   request 7 served\n  12:00:08  info   request 8 served\n  12:00:09  warn   request 9 served\n"
        );
        assert_eq!(super::super::scrollbar::column(&h, 43), "---#", "the thumb follows the tail");
        h.send(Msg::Push(line(10)));
        assert!(h.screen().contains("request 10 served"));
        h.mouse(MouseKind::ScrollUp, 5, 1);
        h.send(Msg::Push(line(11)));
        let screen = h.screen();
        assert!(!screen.contains("request 11"), "{screen}");
        assert!(screen.contains("↓ 4 lines below"), "{screen}");
        h.click_text("lines below");
        assert!(h.screen().contains("request 11 served"), "{}", h.screen());
    }

    #[test]
    fn only_a_click_on_the_note_resumes_following() {
        let mut h = demo(10);
        h.mouse(MouseKind::ScrollUp, 5, 1);
        // The note reads ` ↓ 3 lines below `: two cells before its first cell is a line.
        let (words, y) = h.find("lines below").expect("the note");
        let note = words - 5;
        h.click(note - 2, y);
        assert!(!h.screen().contains("request 9 served"), "a click beside the note places the cursor:\n{}", h.screen());
        h.click(note, y);
        assert!(h.screen().contains("request 9 served"), "a click on the note follows the tail:\n{}", h.screen());
    }

    #[test]
    fn level_colour_marks_and_faint_time() {
        let h = demo(10);
        let theme = h.env().theme();
        assert_eq!(h.fg(12, 3), theme.color("warning"), "warn is a word in the warning colour");
        assert_eq!(h.fg(12, 2), theme.color("info"));
        assert_eq!(h.fg(2, 3), theme.color("muted"), "timestamps are faint");
    }

    #[test]
    fn filters_by_level_and_highlights_search() {
        let mut h = demo(30);
        h.set_glyph_mode(GlyphMode::Unicode);
        let app_level = |h: &mut Harness<Demo>, level, query: &str| {
            let buffer = h.app().buffer.clone();
            let mut next = Harness::new(Demo { buffer, level, query: query.to_owned(), copies: Vec::new() }, 44, 4);
            next.set_glyph_mode(GlyphMode::Unicode);
            next
        };
        let warn = app_level(&mut h, LogLevel::Warn, "");
        let screen = warn.screen();
        assert!(screen.lines().all(|l| l.contains("warn")), "{screen}");
        assert!(screen.contains("request 29 served"), "{screen}");
        let search = app_level(&mut h, LogLevel::Trace, "REQUEST 2");
        assert!(search.screen().contains("No lines match"), "{}", search.screen());
        let search = app_level(&mut h, LogLevel::Trace, "request 2");
        let screen = search.screen();
        assert!(screen.contains("request 29") && !screen.contains("request 19"), "{screen}");
        let highlight = search.env().theme().color("warning");
        let x = u16::try_from(search.find("request 29").map_or(0, |(x, _)| x)).unwrap_or(0);
        assert_ne!(search.bg(x, 3), search.bg(x - 2, 3), "matches are highlighted");
        assert!(highlight.is_some());
    }

    #[test]
    fn cursor_selection_and_copy() {
        let mut h = demo(10);
        h.press("tab").press("up").press("shift+up").press("c");
        assert_eq!(
            h.copied().last().map(String::as_str),
            Some("12:00:08 info request 8 served\n12:00:09 warn request 9 served")
        );
        assert_eq!(h.app().copies, vec![2]);
        assert!(h.screen().starts_with("  12:00:06"), "{}", h.screen());
        h.press("end");
        h.send(Msg::Push(line(10)));
        assert!(h.screen().contains("request 10"));
        h.press("home");
        assert!(h.screen().contains("request 0 served"));
    }

    #[test]
    fn right_click_copies_the_selected_lines_clean_or_raw() {
        let mut h = demo(10);
        h.set_reduced_motion(true);
        h.mouse(MouseKind::Down(MouseButton::Left), 5, 2).mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
        h.mouse(MouseKind::Up(MouseButton::Left), 5, 3);
        assert!(h.copied().is_empty(), "selecting lines copies nothing");
        h.mouse(MouseKind::Down(MouseButton::Right), 20, 3).mouse(MouseKind::Up(MouseButton::Right), 20, 3);
        h.set_glyph_mode(GlyphMode::Unicode);
        let screen = h.screen();
        assert!(screen.contains("Copy") && screen.contains("Raw copy"), "{screen}");
        h.click_text("Raw copy");
        assert_eq!(
            h.copied().last().map(String::as_str),
            Some("12:00:08  info   request 8 served\n12:00:09  warn   request 9 served"),
            "the columns as shown"
        );
        h.mouse(MouseKind::Down(MouseButton::Right), 20, 0).mouse(MouseKind::Up(MouseButton::Right), 20, 0);
        h.click_text("Copy");
        assert_eq!(h.copied().last().map(String::as_str), Some("12:00:06 info request 6 served"));
        assert_eq!(h.app().copies, vec![2, 1], "a right click off the selection took that line");
    }

    /// A log whose height the application sets, to shrink it while lines are being selected.
    struct Shrinking {
        buffer: LogBuffer,
        rows: u16,
    }

    impl App for Shrinking {
        type Msg = u16;
        fn update(&mut self, rows: u16) -> Command<u16> {
            self.rows = rows;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, u16>) {
            ui.add(LogView::new(&self.buffer)).fill_width().height(crate::widget::Length::Cells(self.rows));
        }
    }

    #[test]
    fn a_selection_drag_survives_the_view_losing_its_height() {
        let mut buffer = LogBuffer::new(100);
        for n in 0..10 {
            buffer.push(line(n));
        }
        let mut h = Harness::new(Shrinking { buffer, rows: 4 }, 44, 4);
        h.mouse(MouseKind::Down(MouseButton::Left), 5, 1);
        h.send(0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
        h.mouse(MouseKind::Up(MouseButton::Left), 5, 3);
        assert_eq!(h.screen(), "\n\n\n\n", "a log with no rows paints nothing");
    }

    #[test]
    fn keeps_capacity_and_shows_empty_text() {
        let mut h = demo(0);
        assert_eq!(h.screen(), "  Waiting for logs\n\n\n\n");
        for n in 0..1500 {
            h.send(Msg::Push(line(n)));
        }
        assert_eq!(h.app().buffer.len(), 1000);
        assert!(h.screen().contains("request 1499 served"));
    }

    #[test]
    fn a_line_meant_for_a_terminal_shows_what_a_terminal_would_leave() {
        let buffer = LogBuffer::new(10);
        let app = Demo { buffer, level: LogLevel::Trace, query: String::new(), copies: Vec::new() };
        let mut h = Harness::new(app, 60, 6);
        for text in [
            "Sending build context to Docker daemon  2.048kB\r\r",
            "10%\r50%\r100%",
            "\u{1b}[1;32mok\u{1b}[0m done",
            "half \u{1b}[3",
            "lone \u{1b}",
            "a\tb\u{7}c\u{0}d",
        ] {
            h.send(Msg::Push(LogLine::new(LogLevel::Info, text)));
        }
        let screen = h.screen();
        let shown: Vec<&str> = screen.lines().map(str::trim_end).collect();
        assert_eq!(
            shown,
            [
                "  info   Sending build context to Docker daemon  2.048kB",
                "  info   100%",
                "  info   ok done",
                "  info   half",
                "  info   lone",
                "  info   a       bcd",
            ],
            "{screen}"
        );
        h.press("tab").press("up").press("c");
        assert_eq!(h.copied().last().map(String::as_str), Some("info a       bcd"), "a copy holds what is shown");
    }
}
