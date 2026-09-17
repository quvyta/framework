//! Breadcrumbs: the path to the current place, one clickable segment per level.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::IndexMessage;
use super::popup_menu::{PopupAction, PopupMenu};

/// Cells of padding on each side of a segment's label.
const PAD: u16 = 1;

/// A piece of a laid-out breadcrumb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    /// Segment `index`.
    Segment(usize),
    /// The `…` standing for the hidden middle.
    More,
}

#[derive(Debug, Default)]
struct CrumbMemory {
    cursor: Option<Part>,
    hidden: Vec<usize>,
}

/// The path to the current place, such as `workspace › quvyta › crates › src`.
///
/// Every segment but the last opens its level; the last is the current place, bold and not
/// clickable. Segments are bare text that rises on hover; the separator is a faint chevron icon.
/// When the path does not fit, the middle collapses into `…`, which lists the hidden levels.
///
/// Keys while focused: ←/→ move between segments, Home/End jump to the ends, Enter or Space
/// opens the segment (or the list behind `…`).
///
/// Style keys: `crumb` with `hover`, `focus`; `crumb.current`; `crumb-separator`;
/// `popup-menu`, `popup-item`, `popup-check` for the hidden levels. Icon: `crumb-separator`.
pub struct Breadcrumb<Msg> {
    segments: Vec<String>,
    on_select: Option<IndexMessage<Msg>>,
}

impl<Msg: 'static> Breadcrumb<Msg> {
    /// A path of `segments` from the root to the current place.
    #[must_use]
    pub fn new(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { segments: segments.into_iter().map(Into::into).collect(), on_select: None }
    }

    /// Message for opening segment `index`.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    fn segment_width(&self, index: usize) -> u16 {
        text::width(&self.segments[index]).saturating_add(PAD * 2)
    }

    fn width_of(&self, parts: &[Part]) -> u16 {
        let count = u16::try_from(parts.len()).unwrap_or(u16::MAX);
        parts
            .iter()
            .map(|part| match part {
                Part::Segment(index) => self.segment_width(*index),
                Part::More => 1 + PAD * 2,
            })
            .fold(0u16, u16::saturating_add)
            .saturating_add(count.saturating_sub(1))
    }

    /// The parts that fit in `width`: everything, or the root, `…` and as many of the last
    /// levels as fit, or at worst `…` and the current place.
    fn parts(&self, width: u16) -> Vec<Part> {
        let count = self.segments.len();
        let all: Vec<Part> = (0..count).map(Part::Segment).collect();
        if count <= 2 || self.width_of(&all) <= width {
            return all;
        }
        let last = count - 1;
        let mut tail = vec![Part::Segment(last)];
        for index in (1..last).rev() {
            let mut candidate = vec![Part::Segment(0), Part::More, Part::Segment(index)];
            candidate.extend(&tail);
            if self.width_of(&candidate) > width {
                break;
            }
            tail.insert(0, Part::Segment(index));
        }
        let mut with_root = vec![Part::Segment(0), Part::More];
        with_root.extend(&tail);
        if self.width_of(&with_root) <= width {
            return with_root;
        }
        vec![Part::More, Part::Segment(last)]
    }

    /// Screen rectangles of `parts` laid out from `area`'s left edge.
    fn layout(&self, parts: &[Part], area: Rect) -> Vec<(Part, Rect)> {
        let mut x = area.x;
        parts
            .iter()
            .map(|part| {
                let width = match part {
                    Part::Segment(index) => self.segment_width(*index),
                    Part::More => 1 + PAD * 2,
                };
                let rect = Rect::new(x, area.y, width, 1).intersect(area);
                x += i32::from(width) + 1;
                (*part, rect)
            })
            .collect()
    }

    fn hidden(parts: &[Part], count: usize) -> Vec<usize> {
        (0..count).filter(|index| !parts.contains(&Part::Segment(*index))).collect()
    }

    fn is_current(&self, part: Part) -> bool {
        part == Part::Segment(self.segments.len().saturating_sub(1))
    }

    fn active(&self) -> bool {
        self.on_select.is_some() && self.segments.len() > 1
    }

    /// The parts the keyboard can rest on, in order.
    fn stops(&self, parts: &[Part]) -> Vec<Part> {
        parts.iter().copied().filter(|part| !self.is_current(*part)).collect()
    }

    fn activate(&self, cx: &mut EventCx<'_, Msg>, part: Part, parts: &[Part]) {
        match part {
            Part::Segment(index) => {
                if let Some(message) = &self.on_select
                    && !self.is_current(part)
                {
                    cx.flash();
                    cx.emit(message(index));
                }
            }
            Part::More => {
                let hidden = Self::hidden(parts, self.segments.len());
                let highlight = hidden.len().saturating_sub(1);
                cx.memory::<CrumbMemory>().hidden = hidden;
                PopupMenu::open(cx, highlight);
            }
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Breadcrumb<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let all: Vec<Part> = (0..self.segments.len()).map(Part::Segment).collect();
        Size::new(self.width_of(&all), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if self.segments.is_empty() || area.is_empty() {
            return;
        }
        let active = self.active();
        if active {
            cx.register_hit(area);
        }
        let focused = active && cx.is_focused();
        let pointer = if active { cx.pointer() } else { None };
        let parts = self.parts(area.width);
        let stops = self.stops(&parts);
        let cursor = {
            let memory = cx.memory::<CrumbMemory>();
            memory.cursor.filter(|part| stops.contains(part)).or_else(|| stops.last().copied())
        };
        let separator = cx.env().icons().glyph("crumb-separator").into_owned();
        let separator_style = cx.style("crumb-separator", None, &[]).text();
        for (position, (part, rect)) in self.layout(&parts, area).into_iter().enumerate() {
            if position > 0 {
                cx.text(rect.x - 1, rect.y, &separator, separator_style, 1);
            }
            if rect.is_empty() {
                continue;
            }
            let current = self.is_current(part);
            let mut states = Vec::new();
            if !current && pointer.is_some_and(|(x, y)| rect.contains(x, y)) {
                states.push(State::Hover);
            }
            if focused && cursor == Some(part) {
                states.push(State::Focus);
            }
            if part == Part::More && PopupMenu::is_open_paint(cx) {
                states.push(State::Active);
                cx.request_overlay(rect);
            }
            let style = cx.style("crumb", current.then_some("current"), &states).text();
            if let Some(bg) = style.bg {
                cx.clear(rect, bg);
            }
            let label = match part {
                Part::Segment(index) => self.segments[index].as_str(),
                Part::More => text::ELLIPSIS,
            };
            let budget = rect.width.saturating_sub(PAD * 2);
            let shown = text::truncate(label, budget).into_owned();
            cx.text(rect.x + i32::from(PAD), rect.y, &shown, CellStyle { bg: None, ..style }, budget);
        }
        cx.memory::<CrumbMemory>().hidden = Self::hidden(&parts, self.segments.len());
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let hidden = cx.memory::<CrumbMemory>().hidden.clone();
        let labels: Vec<String> = hidden.iter().map(|index| self.segments[*index].clone()).collect();
        PopupMenu::paint(cx, anchor, &labels, None);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        if PopupMenu::is_open(cx) {
            let hidden = cx.memory::<CrumbMemory>().hidden.clone();
            let labels: Vec<String> = hidden.iter().map(|index| self.segments[*index].clone()).collect();
            match PopupMenu::event(cx, event, &labels) {
                PopupAction::Chosen(row) => {
                    if let Some(message) = &self.on_select {
                        cx.emit(message(hidden[row]));
                    }
                    return true;
                }
                PopupAction::Used | PopupAction::Closed => return true,
                PopupAction::Ignored => {}
            }
        }
        let area = cx.area();
        let parts = self.parts(area.width);
        let stops = self.stops(&parts);
        match event {
            Event::Key(key) => {
                let remembered = cx.memory::<CrumbMemory>().cursor;
                let Some(position) = remembered
                    .and_then(|part| stops.iter().position(|stop| *stop == part))
                    .or_else(|| stops.len().checked_sub(1))
                else {
                    return false;
                };
                let target = if key.is_plain(Key::Left) {
                    position.saturating_sub(1)
                } else if key.is_plain(Key::Right) {
                    (position + 1).min(stops.len() - 1)
                } else if key.is_plain(Key::Home) {
                    0
                } else if key.is_plain(Key::End) {
                    stops.len() - 1
                } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                    self.activate(cx, stops[position], &parts);
                    return true;
                } else {
                    return false;
                };
                cx.memory::<CrumbMemory>().cursor = Some(stops[target]);
                true
            }
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                let hit = self.layout(&parts, area).into_iter().find(|(_, rect)| rect.contains(mouse.x, mouse.y));
                match hit {
                    Some((part, _)) if !self.is_current(part) => {
                        cx.memory::<CrumbMemory>().cursor = Some(part);
                        self.activate(cx, part, &parts);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    struct Files {
        path: Vec<&'static str>,
        width: u16,
    }

    impl App for Files {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.path.truncate(index + 1);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(Breadcrumb::new(self.path.clone()).on_select(|i| i)).width(Length::Cells(self.width)).id("path");
        }
    }

    fn files(width: u16) -> Files {
        Files { path: vec!["workspace", "quvyta", "crates", "framework", "src", "widgets"], width }
    }

    #[test]
    fn segments_open_levels_and_the_current_one_is_bold() {
        let mut h = Harness::new(files(80), 80, 6);
        assert_eq!(h.screen().lines().next(), Some(" workspace › quvyta › crates › framework › src › widgets"));
        assert!(h.is_bold(50, 0), "the current place is bold");
        assert!(!h.is_bold(2, 0));
        h.click_text("crates");
        assert_eq!(h.app().path, vec!["workspace", "quvyta", "crates"]);
        h.click_text("crates");
        assert_eq!(h.app().path.len(), 3, "the current place is not a link");
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen().lines().next(), Some(" workspace : quvyta : crates"));
    }

    #[test]
    fn narrow_paths_collapse_the_middle_and_list_it() {
        let mut h = Harness::new(files(34), 34, 8);
        assert_eq!(h.screen().lines().next(), Some(" workspace › … › src › widgets"));
        h.click_text("…").advance(std::time::Duration::from_millis(300));
        let screen = h.screen();
        assert!(screen.contains("quvyta") && screen.contains("framework"), "{screen}");
        h.click_text("framework");
        assert_eq!(h.app().path.last(), Some(&"framework"));
        let tiny = Harness::new(files(14), 14, 1);
        assert_eq!(tiny.screen(), " … › widgets\n");
    }

    #[test]
    fn keyboard_moves_between_segments() {
        let mut h = Harness::new(files(80), 80, 6);
        h.press("tab").press("left").press("left").press("enter");
        assert_eq!(h.app().path, vec!["workspace", "quvyta", "crates"]);
        h.press("home").press("enter");
        assert_eq!(h.app().path, vec!["workspace"]);
        assert!(!h.is_focused("path"), "a single segment has nothing to open");
    }

    #[test]
    fn segments_wider_than_any_screen_do_not_overflow() {
        let long: &'static str = "d".repeat(70_000).leak();
        let h = Harness::new(Files { path: vec!["workspace", long, long, "src"], width: 24 }, 24, 1);
        assert_eq!(h.screen(), " workspace › … › src\n");
    }
}
