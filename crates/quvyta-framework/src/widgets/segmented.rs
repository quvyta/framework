//! Segmented controls.

use super::IndexMessage;
use super::press::{self, Press};
use crate::env::Env;
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Side-by-side segments of one surface, of which the filled one is chosen: for two to five
/// short, mutually exclusive choices such as a view mode, where the choice should read at a
/// glance.
///
/// The control takes focus as one: Left and Right choose the neighbouring segment, Home and End
/// the first and last, and a click chooses the segment under the pointer. The application owns
/// the choice.
///
/// Style keys: `segment` (`bg`, `fg`, `bold`, `padding`, `pillar`) with states `hover`, `focus`,
/// `checked`, `disabled`. The pillar stands in the first cell of the hovered segment, or of the
/// chosen one while the keyboard focuses the control.
pub struct Segmented<Msg> {
    options: Vec<String>,
    selected: usize,
    disabled: bool,
    on_select: Option<IndexMessage<Msg>>,
}

impl<Msg> Segmented<Msg> {
    /// Segments for `options` with the first chosen.
    #[must_use]
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { options: options.into_iter().map(Into::into).collect(), selected: 0, disabled: false, on_select: None }
    }

    /// The chosen segment.
    #[must_use]
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Greys the control out; it cannot be focused or changed.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for choosing segment `index`.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_select.is_some() && !self.options.is_empty()
    }

    /// Left edge and width of every segment, relative to the control.
    fn spans(&self, horizontal_padding: u16) -> Vec<(u16, u16)> {
        let mut x = 0u16;
        self.options
            .iter()
            .map(|label| {
                let width = text::width(label).saturating_add(horizontal_padding.saturating_mul(2));
                let span = (x, width);
                x = x.saturating_add(width);
                span
            })
            .collect()
    }

    fn choose(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        let index = index.min(self.options.len() - 1);
        if index != self.selected
            && let Some(message) = &self.on_select
        {
            cx.emit(message(index));
        }
    }
}

/// The horizontal padding of a segment from the theme.
fn padding(env: &Env) -> u16 {
    env.theme().style("segment", None, &[]).pair("padding").map_or(2, |(_, horizontal)| horizontal)
}

impl<Msg: 'static> Widget<Msg> for Segmented<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let pad = padding(cx.env());
        let width = self.spans(pad).last().map_or(0, |(x, width)| x.saturating_add(*width));
        Size::new(width, 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let active = self.active();
        let focused = active && cx.is_focus_visible();
        let pointer = if active { cx.pointer() } else { None };
        let pad = padding(cx.env());
        for (index, (x, width)) in self.spans(pad).into_iter().enumerate() {
            let rect = Rect::new(area.x + i32::from(x), area.y, width, 1).intersect(area);
            if rect.is_empty() {
                continue;
            }
            let chosen = index == self.selected;
            let mut states = Vec::new();
            if pointer.is_some_and(|(px, py)| rect.contains(px, py)) {
                states.push(State::Hover);
            }
            if focused && chosen {
                states.push(State::Focus);
            }
            if chosen {
                states.push(State::Checked);
            }
            if self.disabled {
                states.push(State::Disabled);
            }
            let segment_style = cx.style("segment", None, &states);
            let style = segment_style.text();
            cx.clear(rect, style.bg.unwrap_or_else(|| cx.color("raised")));
            // The pillar stands beside the segment itself, never at the far left of the control.
            if let Some(color) = segment_style.color("pillar").filter(|_| pad >= 1) {
                cx.pillar(rect.x, rect.y, color);
            }
            let budget = rect.width.saturating_sub(pad);
            let label = text::truncate(&self.options[index], budget).into_owned();
            cx.text(rect.x + i32::from(pad), rect.y, &label, CellStyle { bg: None, ..style }, budget);
        }
        if active {
            cx.register_hit(area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        if let Event::Key(key) = event {
            let last = self.options.len() - 1;
            let target = if key.is_plain(Key::Left) {
                Some(self.selected.saturating_sub(1))
            } else if key.is_plain(Key::Right) {
                Some(self.selected + 1)
            } else if key.is_plain(Key::Home) {
                Some(0)
            } else if key.is_plain(Key::End) {
                Some(last)
            } else {
                None
            };
            if let Some(index) = target {
                self.choose(cx, index);
                return true;
            }
        }
        match press::read(cx, event) {
            Press::Ignored | Press::Key => false,
            Press::Used => true,
            Press::Click(x, _) => {
                let offset = x - cx.area().x;
                let pad = padding(cx.env());
                if let Some(index) = self
                    .spans(pad)
                    .iter()
                    .position(|(start, width)| (i32::from(*start)..i32::from(start + width)).contains(&offset))
                {
                    self.choose(cx, index);
                }
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo {
        chosen: usize,
    }

    impl App for Demo {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.chosen = index;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(Segmented::new(["List", "Grid", "Tree"]).selected(self.chosen).on_select(|i| i)).id("view");
        }
    }

    #[test]
    fn chosen_segment_is_filled_and_changes_by_keys_and_clicks() {
        let mut h = Harness::new(Demo { chosen: 0 }, 30, 1);
        assert_eq!(h.screen(), "  List    Grid    Tree\n");
        let theme = h.env().theme();
        // The chosen segment is a tint of the accent, calm enough to leave room for hover.
        assert!(h.bg(2, 0) != theme.color("raised") && h.bg(2, 0) != theme.color("accent"));
        assert_eq!(h.bg(10, 0), theme.color("raised"));
        h.press("tab").press("right");
        assert_eq!(h.app().chosen, 1);
        h.press("end");
        assert_eq!(h.app().chosen, 2);
        h.click_text("List");
        assert_eq!(h.app().chosen, 0);
    }

    #[test]
    fn every_cell_of_a_segment_including_its_padding_chooses_it() {
        let mut h = Harness::new(Demo { chosen: 0 }, 30, 1);
        for x in 8..16 {
            h.click(x, 0);
            assert_eq!(h.app().chosen, 1, "column {x} belongs to Grid");
            h.click(0, 0);
            assert_eq!(h.app().chosen, 0, "column 0 belongs to List");
        }
    }

    #[test]
    fn the_pillar_marks_the_hovered_segment_in_its_own_first_cell() {
        let mut h = Harness::new(Demo { chosen: 0 }, 30, 1);
        h.hover(12, 0);
        assert_eq!(h.screen(), "  List  ▌ Grid    Tree\n");
    }
}
