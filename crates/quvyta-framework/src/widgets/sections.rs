//! Titled sections that open and close: the one model behind [`Accordion`](super::Accordion)
//! and [`WidgetDock`](super::WidgetDock).
//!
//! The model owns what both views share: the title row and its states, the keyboard cursor,
//! clicking and pressing Enter to toggle, the height animation, and dragging a title to a new
//! place. The views only decide how much height an open body gets.

use std::time::Duration;

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, Modifiers};
use crate::motion::{Easing, Tween, steps};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, Node, PaintCx};

use super::cells;
use super::row;

/// The title of one section of an [`Accordion`](super::Accordion) or a
/// [`WidgetDock`](super::WidgetDock): a label with an optional icon and a faint detail on the
/// right, such as a count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    title: String,
    icon: Option<String>,
    detail: Option<String>,
}

impl Section {
    /// A section titled `title`.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), icon: None, detail: None }
    }

    /// Icon key drawn before the title.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>) -> Self {
        self.icon = Some(key.into());
        self
    }

    /// Faint text at the right end of the title row, e.g. `3 running`.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl From<&str> for Section {
    fn from(title: &str) -> Self {
        Self::new(title)
    }
}

impl From<String> for Section {
    fn from(title: String) -> Self {
        Self::new(title)
    }
}

/// Builds a message from a section index and its new open state.
pub(crate) type ToggleSection<Msg> = Box<dyn Fn(usize, bool) -> Msg>;

/// Builds a message from a section's old and new index.
pub(crate) type MoveSection<Msg> = Box<dyn Fn(usize, usize) -> Msg>;

/// How open bodies get their height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    /// Every open body takes its natural height; the whole widget grows (accordion).
    Natural,
    /// Open bodies share the area the widget was given (dock).
    Share,
}

/// Rows a drag must travel before it starts, so a slightly shaky click still toggles.
const DRAG_THRESHOLD: i32 = 1;

pub(crate) struct Sections<Msg> {
    pub(crate) sections: Vec<Section>,
    pub(crate) open: Vec<bool>,
    pub(crate) bodies: Vec<Node<Msg>>,
    pub(crate) on_toggle: Option<ToggleSection<Msg>>,
    pub(crate) on_move: Option<MoveSection<Msg>>,
    pub(crate) single: bool,
}

#[derive(Debug, Default)]
struct SectionsMemory {
    cursor: usize,
    /// Title rows as last painted, by section index.
    titles: Vec<Rect>,
    /// Top and bottom row of every section as last painted outside a drag.
    blocks: Vec<(i32, i32)>,
    pressed: Option<(usize, i32)>,
    drag: Option<Drag>,
    /// Opening progress of every section, by title so it follows a section that moves.
    reveal: Vec<(String, Tween)>,
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    from: usize,
    pointer: i32,
    target: usize,
}

/// Where one entry of the painted stack goes.
enum Entry {
    Section(usize),
    Drop,
}

impl<Msg: 'static> Sections<Msg> {
    pub(crate) fn new(sections: Vec<Section>) -> Self {
        Self { sections, open: Vec::new(), bodies: Vec::new(), on_toggle: None, on_move: None, single: false }
    }

    fn is_open(&self, index: usize) -> bool {
        self.open.get(index).copied().unwrap_or(false)
    }

    fn body(&self, index: usize) -> Option<&Node<Msg>> {
        self.bodies.get(index)
    }

    /// Rows between sections.
    fn gap(env: &crate::env::Env) -> u16 {
        env.theme().style("section", None, &[]).cells("gap").unwrap_or(1)
    }

    /// Height of `index`'s body with padding when open.
    fn natural(&self, cx: &mut MeasureCx<'_>, index: usize, width: u16) -> u16 {
        let padding = crate::style::WidgetStyle::new(cx.env().theme().style("section-body", None, &[]), 0.0).padding();
        let inner = Size::new(width.saturating_sub(padding.horizontal()), u16::MAX);
        let content = self.body(index).map_or(0, |body| cx.measure_child(body, inner).height);
        content.saturating_add(padding.vertical())
    }

    pub(crate) fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.sections.is_empty() {
            return Size::default();
        }
        let count = u16::try_from(self.sections.len()).unwrap_or(u16::MAX);
        let mut height = count.saturating_add(Self::gap(cx.env()).saturating_mul(count - 1));
        let mut width = 0;
        for (index, section) in self.sections.iter().enumerate() {
            let detail = section.detail.as_deref().map_or(0, |d| text::width(d).saturating_add(2));
            let icon = if section.icon.is_some() { 2 } else { 0 };
            width = width.max(cells::sum([text::width(&section.title), detail, icon, 7]));
            if self.is_open(index) {
                height = height.saturating_add(self.natural(cx, index, available.width));
                if let Some(body) = self.body(index) {
                    width = width.max(cx.measure_child(body, available).width + 4);
                }
            }
        }
        Size::new(width, height).min(available)
    }

    /// The opening progress of `index`, moving towards its open state. With `Flow::Natural`
    /// closing is immediate, because the widget has already given its rows back to the layout.
    fn progress(&self, cx: &mut PaintCx<'_>, index: usize, flow: Flow) -> f32 {
        let target = if self.is_open(index) { 1.0 } else { 0.0 };
        if cx.reduced_motion() {
            return target;
        }
        let now = cx.now();
        let duration = cx.env().theme().motion().enter * 2;
        let title = self.sections[index].title.clone();
        let memory = cx.memory::<SectionsMemory>();
        let tween = match memory.reveal.iter_mut().find(|(key, _)| *key == title) {
            Some((_, tween)) => tween,
            None => {
                memory.reveal.push((title, Tween::settled(target)));
                &mut memory.reveal.last_mut().expect("a tween was just pushed").1
            }
        };
        if (tween.target() - target).abs() > f32::EPSILON {
            if flow == Flow::Natural && target < 0.5 {
                *tween = Tween::settled(0.0);
            } else {
                tween.retarget(target, now, duration, Easing::EaseOut);
            }
        }
        let (value, running) = (tween.value(now), tween.is_running(now));
        if running {
            cx.request_frame_in(Duration::from_millis(16));
        }
        value
    }

    pub(crate) fn paint(&self, cx: &mut PaintCx<'_>, area: Rect, flow: Flow) {
        let count = self.sections.len();
        let gap = Self::gap(cx.env());
        let body_style = cx.style("section-body", None, &[]);
        let padding = body_style.padding();
        let body_bg = body_style.text().bg;
        let (cursor, drag) = {
            let memory = cx.memory::<SectionsMemory>();
            forget_gone(&mut memory.reveal, &self.sections);
            memory.cursor = memory.cursor.min(count.saturating_sub(1));
            (memory.cursor, memory.drag.filter(|drag| drag.from < count))
        };

        let progress: Vec<f32> = (0..count).map(|index| self.progress(cx, index, flow)).collect();
        let naturals: Vec<u16> = (0..count)
            .map(|index| {
                let showing = self.is_open(index) || progress[index] > 0.0;
                if showing { self.natural(&mut MeasureCx::new(cx.env()), index, area.width) } else { 0 }
            })
            .collect();
        let allotted = match flow {
            Flow::Natural => naturals.clone(),
            Flow::Share => {
                let titles = u16::try_from(count).unwrap_or(u16::MAX);
                let gaps = gap.saturating_mul(titles.saturating_sub(1));
                // While dragging, the dragged body is folded away and its title becomes the ghost.
                let mut wants = naturals.clone();
                if let Some(drag) = drag {
                    wants[drag.from] = 0;
                }
                share(&wants, area.height.saturating_sub(titles + gaps))
            }
        };

        let entries: Vec<Entry> = match drag {
            Some(drag) => {
                let mut order: Vec<Entry> = (0..count).filter(|i| *i != drag.from).map(Entry::Section).collect();
                order.insert(drag.target.min(order.len()), Entry::Drop);
                order
            }
            None => (0..count).map(Entry::Section).collect(),
        };

        let focused = cx.is_focused();
        let pointer = cx.pointer();
        let mut titles = vec![Rect::default(); count];
        let mut blocks = vec![(0, 0); count];
        let mut y = area.y;
        for (position, entry) in entries.iter().enumerate() {
            if position > 0 {
                y += i32::from(gap);
            }
            let row = Rect::new(area.x, y, area.width, 1);
            let index = match entry {
                Entry::Drop => {
                    let color = cx.style("section-drop", None, &[]).text().bg.unwrap_or_else(|| cx.color("active"));
                    cx.clear(row, color);
                    y += 1;
                    continue;
                }
                Entry::Section(index) => *index,
            };
            let hovered = drag.is_none() && pointer.is_some_and(|(px, py)| row.contains(px, py));
            self.paint_title(cx, row, index, hovered, focused && cursor == index, false);
            cx.register_hit(row);
            titles[index] = row;
            let visible = steps(progress[index], allotted[index]);
            let top = y;
            y += 1;
            if visible > 0 {
                let body = Rect::new(area.x, y, area.width, visible);
                if let Some(bg) = body_bg {
                    cx.clear(body, bg);
                }
                if let Some(node) = self.body(index) {
                    let natural = naturals[index].max(allotted[index]);
                    let full = Rect::new(area.x, y, area.width, natural).inset(padding);
                    let inner_height = allotted[index].saturating_sub(padding.vertical());
                    let content = Rect::new(full.x, full.y, full.width, inner_height);
                    cx.with_clip(body, |cx| cx.paint_child(node, content));
                }
                y += i32::from(visible);
            }
            blocks[index] = (top, y);
        }

        if let Some(drag) = drag {
            let ghost_y = drag.pointer.clamp(area.y, last_row(area));
            self.paint_title(cx, Rect::new(area.x, ghost_y, area.width, 1), drag.from, false, false, true);
        } else {
            let memory = cx.memory::<SectionsMemory>();
            memory.titles = titles;
            memory.blocks = blocks;
        }
    }

    fn paint_title(&self, cx: &mut PaintCx<'_>, row: Rect, index: usize, hovered: bool, cursor: bool, ghost: bool) {
        let section = &self.sections[index];
        let open = self.is_open(index);
        let mut states = Vec::new();
        if hovered {
            states.push(State::Hover);
        }
        if cursor {
            states.push(State::Focus);
        }
        if open {
            states.push(State::Checked);
        }
        let variant = ghost.then_some("ghost");
        let style = cx.style("section-title", variant, &states);
        let title_style = CellStyle { bg: None, ..style.text() };
        // A title row always has a surface, even in a theme that gives it no background.
        cx.clear(row, style.text().bg.unwrap_or_else(|| cx.color("raised")));
        let slide = cx.env().slide() && (hovered || cursor || ghost);
        let detail_width = section.detail.as_deref().map_or(0, |d| text::width(d).saturating_add(2));
        // The detail keeps a margin of two cells on the right; it gives way on narrow rows.
        let show_detail = detail_width > 0 && row.width.saturating_sub(5) > detail_width.saturating_add(8);
        let chevron_key = if open { "section-open" } else { "section-closed" };
        let chevron = cx.env().icons().glyph(chevron_key).into_owned();
        let chevron_style = CellStyle { bg: None, ..cx.style("section-chevron", None, &states).text() };
        let icon: Vec<row::Mark> =
            section.icon.iter().map(|icon| (cx.env().icons().glyph(icon).into_owned(), title_style)).collect();
        // The chevron is a fixed mark: only the icon and the title slide.
        let parts = row::Parts {
            fixed: &[(chevron, chevron_style)],
            sliding: &icon,
            label: &section.title,
            trailing: 1 + if show_detail { detail_width } else { 0 },
            indent: 0,
        };
        row::paint_parts(cx, row, &style, slide, &parts);
        if let (true, Some(detail)) = (show_detail, &section.detail) {
            let detail_style = cx.style("section-detail", None, &states).text();
            let width = text::width(detail);
            cx.text(row.right() - 2 - i32::from(width), row.y, detail, CellStyle { bg: None, ..detail_style }, width);
        }
    }

    fn toggle(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        let Some(message) = &self.on_toggle else {
            return;
        };
        let open = !self.is_open(index);
        if open && self.single {
            for other in (0..self.sections.len()).filter(|i| *i != index && self.is_open(*i)) {
                cx.emit(message(other, false));
            }
        }
        cx.emit(message(index, open));
    }

    fn move_section(&self, cx: &mut EventCx<'_, Msg>, from: usize, to: usize) {
        if let Some(message) = &self.on_move
            && from != to
            && to < self.sections.len()
        {
            cx.memory::<SectionsMemory>().cursor = to;
            cx.emit(message(from, to));
        }
    }

    pub(crate) fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let count = self.sections.len();
        if count == 0 {
            return false;
        }
        match event {
            Event::Key(key) => {
                let cursor = cx.memory::<SectionsMemory>().cursor.min(count - 1);
                let reorder = Modifiers { ctrl: true, shift: true, alt: false };
                if self.on_move.is_some() && key.chord.mods == reorder {
                    let to = match key.chord.key {
                        Key::Up => cursor.checked_sub(1),
                        Key::Down => Some(cursor + 1).filter(|to| *to < count),
                        _ => return false,
                    };
                    if let Some(to) = to {
                        self.move_section(cx, cursor, to);
                    }
                    return true;
                }
                let target = if key.is_plain(Key::Up) {
                    cursor.checked_sub(1)
                } else if key.is_plain(Key::Down) {
                    Some(cursor + 1).filter(|next| *next < count)
                } else if key.is_plain(Key::Home) {
                    Some(0)
                } else if key.is_plain(Key::End) {
                    Some(count - 1)
                } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                    if self.on_toggle.is_none() {
                        return false;
                    }
                    self.toggle(cx, cursor);
                    return true;
                } else {
                    return false;
                };
                match target {
                    Some(target) => {
                        cx.memory::<SectionsMemory>().cursor = target;
                        true
                    }
                    None => false,
                }
            }
            Event::Mouse(mouse) => self.mouse(cx, mouse.kind, mouse.x, mouse.y),
            _ => false,
        }
    }

    fn mouse(&self, cx: &mut EventCx<'_, Msg>, kind: MouseKind, x: i32, y: i32) -> bool {
        let area = cx.area();
        let memory = cx.memory::<SectionsMemory>();
        let under = memory.titles.iter().position(|row| row.contains(x, y));
        match kind {
            MouseKind::Down(MouseButton::Left) => {
                let Some(index) = under else {
                    return false;
                };
                memory.cursor = index;
                memory.pressed = Some((index, y));
                cx.capture_pointer();
                true
            }
            MouseKind::Drag(MouseButton::Left) => {
                let Some((from, start)) = memory.pressed else {
                    return false;
                };
                if self.on_move.is_none() {
                    return true;
                }
                if memory.drag.is_none() && (y - start).abs() < DRAG_THRESHOLD {
                    return true;
                }
                let target = memory
                    .blocks
                    .iter()
                    .enumerate()
                    .filter(|(index, (top, bottom))| *index != from && (top + bottom) / 2 < y)
                    .count();
                memory.drag = Some(Drag { from, pointer: y.clamp(area.y, last_row(area)), target });
                true
            }
            MouseKind::Up(MouseButton::Left) => {
                let pressed = memory.pressed.take();
                let drag = memory.drag.take();
                match (pressed, drag) {
                    (_, Some(drag)) => self.move_section(cx, drag.from, drag.target),
                    (Some((index, _)), None) if under == Some(index) => self.toggle(cx, index),
                    _ => {}
                }
                pressed.is_some()
            }
            _ => false,
        }
    }
}

/// Drops the opening progress of sections that are gone, so renaming or removing sections does
/// not grow the memory for as long as the widget lives.
fn forget_gone(reveal: &mut Vec<(String, Tween)>, sections: &[Section]) {
    reveal.retain(|(title, _)| sections.iter().any(|section| section.title == *title));
}

/// The last row of `area`, or its top row when it has no height: a dock can lose its rows while a
/// title is being dragged.
fn last_row(area: Rect) -> i32 {
    (area.bottom() - 1).max(area.y)
}

/// Shares `available` rows between bodies wanting `wants` rows: small bodies get all they
/// want, the rest split what is left evenly.
fn share(wants: &[u16], available: u16) -> Vec<u16> {
    let mut given = vec![0; wants.len()];
    let mut order: Vec<usize> = (0..wants.len()).filter(|i| wants[*i] > 0).collect();
    order.sort_by_key(|i| wants[*i]);
    let mut left = available;
    let mut remaining = clamp_u16(i32::try_from(order.len()).unwrap_or(i32::MAX));
    for index in order {
        let fair = left / remaining.max(1);
        given[index] = wants[index].min(fair);
        left -= given[index];
        remaining -= 1;
    }
    given
}

#[cfg(test)]
mod tests {
    use super::{Section, Tween, forget_gone, share};

    #[test]
    fn small_bodies_keep_their_height_and_large_ones_split_the_rest() {
        assert_eq!(share(&[3, 0, 20, 20], 21), vec![3, 0, 9, 9]);
        assert_eq!(share(&[3, 4], 30), vec![3, 4]);
        assert_eq!(share(&[10, 10, 10], 10), vec![3, 3, 4]);
        assert_eq!(share(&[], 10), Vec::<u16>::new());
    }

    #[test]
    fn progress_of_sections_that_are_gone_is_forgotten() {
        let mut reveal = vec![("Git".to_owned(), Tween::settled(1.0)), ("Old name".to_owned(), Tween::settled(0.0))];
        forget_gone(&mut reveal, &[Section::new("Git"), Section::new("Ports")]);
        let titles: Vec<&str> = reveal.iter().map(|(title, _)| title.as_str()).collect();
        assert_eq!(titles, ["Git"]);
    }
}
