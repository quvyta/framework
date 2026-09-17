//! Progress through a sequence of steps.

use super::IndexMessage;
use super::press::{self, Press};
use crate::event::Event;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::Icons;
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Cells between steps laid out in a row.
const ROW_GAP: u16 = 3;

/// Cells between a step's marker and its label.
const MARKER_GAP: u16 = 2;

/// Where a sequence of steps stands: finished steps carry a check in the success colour, the
/// current step is bold with an accent marker, and upcoming steps are faint. Steps are set apart
/// by space and tone; nothing connects them.
///
/// Steps sit in a row by default. When the row does not fit, only the markers stay, followed by
/// the current step's label. [`Steps::vertical`] puts one step on each line.
///
/// With [`Steps::on_select`] finished steps can be chosen to go back to them: by click, or by
/// focusing the steps, moving with the arrow keys and pressing Enter or Space. The application
/// owns the current step.
///
/// Style keys: `step` (`bg`, `pillar`) with `hover` and `focus` for choosable steps; `step-marker` and
/// `step-label` (`fg`, `bold`) with `checked` for finished steps and `active` for the current
/// one, and variants `running` and `failed` for the current step. Icons: `check`, `dot`,
/// `dot-outline`, `error`.
pub struct Steps<Msg> {
    labels: Vec<String>,
    current: usize,
    vertical: bool,
    running: bool,
    failed: bool,
    on_select: Option<IndexMessage<Msg>>,
}

/// Which finished step the keyboard is on.
#[derive(Debug, Default)]
struct StepsMemory {
    cursor: Option<usize>,
}

/// Where one step is drawn.
struct Slot {
    rect: Rect,
    index: usize,
    /// Whether the marker is drawn.
    marker: bool,
    /// Whether the label is drawn, after the marker when there is one.
    label: bool,
}

impl<Msg: 'static> Steps<Msg> {
    /// Steps named `labels`, the first one current.
    #[must_use]
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            current: 0,
            vertical: false,
            running: false,
            failed: false,
            on_select: None,
        }
    }

    /// The current step. Steps before it are finished; a value past the last step shows every
    /// step finished.
    #[must_use]
    pub fn current(mut self, index: usize) -> Self {
        self.current = index;
        self
    }

    /// One step per line instead of a row.
    #[must_use]
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// The current step is being worked on: its marker breathes.
    #[must_use]
    pub fn running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    /// The current step failed: its marker becomes the error icon in the danger colour.
    #[must_use]
    pub fn failed(mut self, failed: bool) -> Self {
        self.failed = failed;
        self
    }

    /// Makes finished steps choosable; the message carries the chosen step.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// How many steps are finished and therefore choosable.
    fn finished(&self) -> usize {
        self.current.min(self.labels.len())
    }

    fn padding(&self) -> u16 {
        u16::from(self.on_select.is_some())
    }

    /// Cells every marker takes: the widest of the marker icons, so steps line up in any state.
    fn marker_width(icons: &Icons) -> u16 {
        ["check", "dot", "dot-outline", "error"]
            .into_iter()
            .map(|key| text::width(&icons.glyph(key)))
            .max()
            .unwrap_or(1)
    }

    fn item_width(&self, index: usize, marker: u16) -> u16 {
        (self.padding() * 2 + marker + MARKER_GAP).saturating_add(text::width(&self.labels[index]))
    }

    /// Cells of every step in a row, with the gaps between them.
    fn row_width(&self, marker: u16) -> u16 {
        let count = u16::try_from(self.labels.len()).unwrap_or(u16::MAX);
        let gaps = ROW_GAP.saturating_mul(count.saturating_sub(1));
        (0..self.labels.len()).fold(gaps, |sum, index| sum.saturating_add(self.item_width(index, marker)))
    }

    /// Every step's place in `area`.
    fn slots(&self, area: Rect, marker: u16) -> Vec<Slot> {
        let pad = self.padding();
        let count = self.labels.len();
        if self.vertical {
            return (0..count)
                .map(|index| Slot {
                    rect: Rect::new(area.x, area.y + i32::try_from(index).unwrap_or(i32::MAX), area.width, 1),
                    index,
                    marker: true,
                    label: true,
                })
                .collect();
        }
        let mut x = area.x;
        if self.row_width(marker) <= area.width {
            return (0..count)
                .map(|index| {
                    let width = self.item_width(index, marker);
                    let slot = Slot { rect: Rect::new(x, area.y, width, 1), index, marker: true, label: true };
                    x += i32::from(width) + i32::from(ROW_GAP);
                    slot
                })
                .collect();
        }
        // Compact: every marker, then the label of the current step only.
        let mut slots: Vec<Slot> = (0..count)
            .map(|index| {
                let width = pad * 2 + marker;
                let slot = Slot { rect: Rect::new(x, area.y, width, 1), index, marker: true, label: false };
                x += i32::from(width + 1);
                slot
            })
            .collect();
        if self.current < count {
            let label_x = x + 1;
            let rect = Rect::new(label_x, area.y, clamp_u16(area.right() - label_x), 1);
            slots.push(Slot { rect, index: self.current, marker: false, label: true });
        }
        slots
    }

    fn states(&self, index: usize) -> Vec<State> {
        match index.cmp(&self.current) {
            std::cmp::Ordering::Less => vec![State::Checked],
            std::cmp::Ordering::Equal => vec![State::Active],
            std::cmp::Ordering::Greater => Vec::new(),
        }
    }

    fn marker_key(&self, index: usize) -> &'static str {
        match index.cmp(&self.current) {
            std::cmp::Ordering::Less => "check",
            std::cmp::Ordering::Equal if self.failed => "error",
            std::cmp::Ordering::Equal => "dot",
            std::cmp::Ordering::Greater => "dot-outline",
        }
    }

    fn variant(&self, index: usize) -> Option<&'static str> {
        if index != self.current {
            return None;
        }
        if self.failed {
            Some("failed")
        } else if self.running {
            Some("running")
        } else {
            None
        }
    }

    /// The finished step the keyboard is on: the `remembered` one, else the last finished step.
    fn cursor(&self, remembered: Option<usize>) -> usize {
        let last = self.finished().saturating_sub(1);
        remembered.unwrap_or(last).min(last)
    }

    fn choose(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        if let Some(message) = &self.on_select {
            cx.memory::<StepsMemory>().cursor = Some(index);
            cx.flash();
            cx.emit(message(index));
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Steps<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let marker = Self::marker_width(cx.env().icons());
        let count = clamp_u16(i32::try_from(self.labels.len()).unwrap_or(i32::MAX));
        if self.vertical {
            let widest = (0..self.labels.len()).map(|index| self.item_width(index, marker)).max().unwrap_or(0);
            return Size::new(widest, count).min(available);
        }
        Size::new(self.row_width(marker), u16::from(count > 0)).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let marker = Self::marker_width(cx.env().icons());
        let pad = self.padding();
        let choosable = self.on_select.is_some() && self.finished() > 0;
        let focused = choosable && cx.is_focused();
        let pointer = if choosable { cx.pointer() } else { None };
        let cursor = self.cursor(cx.memory::<StepsMemory>().cursor);
        for slot in self.slots(area, marker) {
            let rect = slot.rect.intersect(area);
            if rect.is_empty() {
                continue;
            }
            let mut states = self.states(slot.index);
            let finished = slot.index < self.finished();
            if finished && choosable {
                if pointer.is_some_and(|(x, y)| rect.contains(x, y)) {
                    states.push(State::Hover);
                }
                if focused && slot.index == cursor {
                    states.push(State::Focus);
                }
            }
            let variant = self.variant(slot.index);
            let surface = cx.style("step", variant, &states);
            if slot.marker {
                if let Some(bg) = surface.text().bg {
                    cx.clear(rect, bg);
                }
                // A choosable step is pressable: it rises with the pillar in its first cell.
                if let Some(pillar) = surface.color("pillar") {
                    cx.pillar(rect.x, rect.y, pillar);
                }
            }
            let mut x = rect.x;
            if slot.marker {
                let glyph = cx.env().icons().glyph(self.marker_key(slot.index)).into_owned();
                let marker_style = cx.style("step-marker", variant, &states).text();
                x += i32::from(pad);
                cx.text(x, rect.y, &glyph, CellStyle { bg: None, ..marker_style }, marker);
                x += i32::from(marker + MARKER_GAP);
            }
            if slot.label {
                let label_style = cx.style("step-label", variant, &states).text();
                let label_x = x;
                let budget = clamp_u16(rect.right() - label_x - i32::from(pad));
                let shown = text::truncate(&self.labels[slot.index], budget).into_owned();
                cx.text(label_x, rect.y, &shown, CellStyle { bg: None, ..label_style }, budget);
            }
        }
        if choosable {
            cx.register_hit(area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let finished = self.finished();
        if self.on_select.is_none() || finished == 0 {
            return false;
        }
        let cursor = self.cursor(cx.memory::<StepsMemory>().cursor);
        if let Event::Key(key) = event {
            let (back, forward) = if self.vertical { (Key::Up, Key::Down) } else { (Key::Left, Key::Right) };
            let target = if key.is_plain(back) {
                Some(cursor.saturating_sub(1))
            } else if key.is_plain(forward) {
                Some((cursor + 1).min(finished - 1))
            } else if key.is_plain(Key::Home) {
                Some(0)
            } else if key.is_plain(Key::End) {
                Some(finished - 1)
            } else {
                None
            };
            if let Some(target) = target {
                cx.memory::<StepsMemory>().cursor = Some(target);
                return true;
            }
        }
        match press::read(cx, event) {
            Press::Ignored => false,
            Press::Used => true,
            Press::Key => {
                self.choose(cx, cursor);
                true
            }
            Press::Click(x, y) => {
                let area = cx.area();
                let marker = Self::marker_width(cx.env().icons());
                let hit = self.slots(area, marker).into_iter().find(|slot| slot.marker && slot.rect.contains(x, y));
                if let Some(slot) = hit.filter(|slot| slot.index < finished) {
                    self.choose(cx, slot.index);
                }
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.on_select.is_some() && self.finished() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    const LABELS: [&str; 4] = ["Project", "Engine", "Theme", "Summary"];

    struct Demo {
        current: usize,
        vertical: bool,
        choosable: bool,
        failed: bool,
    }

    impl App for Demo {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.current = index;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            let mut steps = Steps::new(LABELS).current(self.current).vertical(self.vertical).failed(self.failed);
            if self.choosable {
                steps = steps.on_select(|index| index);
            }
            ui.add(steps).id("steps");
        }
    }

    fn demo(current: usize) -> Demo {
        Demo { current, vertical: false, choosable: false, failed: false }
    }

    #[test]
    fn finished_current_and_upcoming_steps_read_by_marker_and_tone() {
        let h = Harness::new(demo(2), 60, 1);
        assert_eq!(h.screen(), "✓  Project   ✓  Engine   ●  Theme   ○  Summary\n");
        let theme = h.env().theme();
        assert_eq!(h.fg(0, 0), theme.color("success"));
        assert_eq!(h.fg(25, 0), theme.color("accent"));
        assert!(h.is_bold(28, 0));
        assert_eq!(h.fg(36, 0), theme.color("muted"));
    }

    #[test]
    fn narrow_rows_keep_markers_and_the_current_label() {
        let h = Harness::new(demo(2), 24, 1);
        assert_eq!(h.screen(), "✓ ✓ ● ○  Theme\n");
    }

    #[test]
    fn vertical_and_failed() {
        let mut app = demo(1);
        app.vertical = true;
        app.failed = true;
        let h = Harness::new(app, 20, 4);
        assert_eq!(h.screen(), "✓  Project\n✕  Engine\n○  Theme\n○  Summary\n");
        assert_eq!(h.fg(0, 1), h.env().theme().color("danger"));
    }

    #[test]
    fn finished_steps_are_chosen_by_click_and_keyboard_only_when_enabled() {
        let mut h = Harness::new(demo(2), 60, 1);
        h.click_text("Project");
        assert_eq!(h.app().current, 2, "plain steps ignore clicks");
        let mut app = demo(3);
        app.choosable = true;
        let mut h = Harness::new(app, 60, 1);
        h.click_text("Summary");
        assert_eq!(h.app().current, 3, "upcoming and current steps are not choosable");
        h.press("tab").press("left").press("enter");
        assert_eq!(h.app().current, 1);
        h.click_text("Project");
        assert_eq!(h.app().current, 0);
    }

    #[test]
    fn choosable_steps_rise_with_the_pillar_under_the_pointer_and_the_keyboard() {
        let mut app = demo(2);
        app.choosable = true;
        let mut h = Harness::new(app, 60, 1);
        assert_eq!(h.screen(), " ✓  Project     ✓  Engine     ●  Theme     ○  Summary\n");
        h.hover(5, 0);
        assert_eq!(h.screen(), "▌✓  Project     ✓  Engine     ●  Theme     ○  Summary\n");
        assert_eq!(h.bg(5, 0), h.env().theme().color("raised"));
        h.hover(33, 0);
        assert_eq!(
            h.screen(),
            " ✓  Project     ✓  Engine     ●  Theme     ○  Summary\n",
            "the current step is not pressable"
        );
        h.press("tab");
        assert_eq!(
            h.screen(),
            " ✓  Project    ▌✓  Engine     ●  Theme     ○  Summary\n",
            "the keyboard starts on the last finished step"
        );
        assert_eq!(h.bg(20, 0), h.env().theme().color("active"));
        let plain = Harness::new(demo(2), 60, 1);
        assert!(!plain.screen().contains('▌'), "steps that cannot be chosen never rise");
    }

    #[test]
    fn a_running_step_breathes_unless_motion_is_reduced() {
        struct Running;
        impl App for Running {
            type Msg = ();
            fn update(&mut self, _: ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                ui.add(Steps::new(LABELS).current(1).running(true));
            }
        }
        let mut h = Harness::new(Running, 60, 1);
        let start = h.fg(13, 0);
        let half = h.env().theme().motion().pulse_period / 2;
        h.advance(half);
        assert_ne!(h.fg(13, 0), start);
        h.set_reduced_motion(true);
        assert_eq!(h.fg(13, 0), start);
    }

    #[test]
    fn ascii_markers_are_not_bracketed() {
        let mut h = Harness::new(demo(1), 60, 1);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "v  Project   *  Engine   o  Theme   o  Summary\n");
    }
}
