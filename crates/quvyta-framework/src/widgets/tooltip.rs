//! Tooltips: a short explanation that appears next to a widget the pointer rests on.

use std::time::Duration;

use super::placement::{self, Placement};
use crate::geometry::{Rect, Size};
use crate::motion::Easing;
use crate::style::CellStyle;
use crate::text;
use crate::widget::{Axis, Container, Flex, Length, MeasureCx, Node, PaintCx, Widget};

/// Wraps widgets and shows a short text next to them after the pointer rests on them for the
/// theme's `motion.hover-delay`.
///
/// The text is one row on the overlay surface, below the widgets by default. It flips to the
/// other side when there is no room, and moves to another side when it would cover the pointer.
/// It disappears as soon as the pointer leaves. With [`on_focus`](Tooltip::on_focus) it also
/// shows, at once, while keyboard focus is inside. The text fades in over `motion.enter`.
///
/// Style keys: `tooltip` (`bg`, `fg`, `padding`).
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::Tooltip;
///
/// struct Toolbar;
///
/// impl App for Toolbar {
///     type Msg = ();
///     fn update(&mut self, (): ()) -> Command<()> {
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, ()>) {
///         ui.add_with(Tooltip::new("Restart every container"), |ui| {
///             ui.add(Button::new("Restart").on_press(()));
///         });
///     }
/// }
///
/// let mut app = Harness::new(Toolbar, 40, 4);
/// app.hover(3, 0).advance(std::time::Duration::from_secs(1));
/// assert!(app.screen().contains("Restart every container"));
/// ```
pub struct Tooltip<Msg> {
    text: String,
    placement: Placement,
    on_focus: bool,
    body: Vec<Node<Msg>>,
}

#[derive(Debug, Default)]
struct TooltipMemory {
    hovered_since: Option<Duration>,
    shown_since: Option<Duration>,
}

impl<Msg: 'static> Tooltip<Msg> {
    /// A tooltip saying `text`. Add the widgets it explains with
    /// [`View::add_with`](crate::widget::View::add_with).
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            placement: Placement::Below,
            on_focus: false,
            body: vec![Node::new(Flex::new(Axis::Column, Vec::new()), 0)],
        }
    }

    /// The preferred side; [`Placement::Below`] by default.
    #[must_use]
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Also shows the tooltip while keyboard focus is inside the wrapped widgets.
    #[must_use]
    pub fn on_focus(mut self, on_focus: bool) -> Self {
        self.on_focus = on_focus;
        self
    }
}

impl<Msg: 'static> Container<Msg> for Tooltip<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.width = Length::Fill(1);
        column.layout.height = Length::Fill(1);
        self.body = vec![column];
    }
}

impl<Msg: 'static> Widget<Msg> for Tooltip<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.body.first().map_or(Size::default(), |body| cx.measure_child(body, available))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        // Registered first so plain content still reports hover; interactive children sit on top.
        cx.register_hit(area);
        if let Some(body) = self.body.first() {
            cx.paint_child(body, area);
        }
        let now = cx.now();
        let hovered = cx.pointer_within().is_some();
        let focused = self.on_focus && cx.has_focus_within();
        let delay = cx.env().theme().motion().hover_delay;
        let memory = cx.memory::<TooltipMemory>();
        memory.hovered_since = if hovered { Some(memory.hovered_since.unwrap_or(now)) } else { None };
        let due = memory.hovered_since.map(|since| since + delay);
        let visible = focused || due.is_some_and(|due| now >= due);
        memory.shown_since = if visible { Some(memory.shown_since.unwrap_or(now)) } else { None };
        if visible {
            cx.request_overlay(area);
        } else if let Some(due) = due {
            cx.request_frame_in(due.saturating_sub(now));
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let style = cx.style("tooltip", None, &[]);
        let padding = style.padding();
        let text_style = style.text();
        let background = text_style.bg.unwrap_or_else(|| cx.color("overlay"));
        let foreground = text_style.fg.unwrap_or_else(|| cx.color("text"));
        let screen = cx.clip();
        let size = Size::new(
            text::width(&self.text).saturating_add(padding.horizontal()),
            padding.vertical().saturating_add(1),
        );
        let pointer = cx.pointer_anywhere();
        let covers_pointer = |rect: Rect| pointer.is_some_and(|(x, y)| rect.contains(x, y));
        let sides = [self.placement, self.placement.opposite(), Placement::Right, Placement::Left];
        let candidates = sides.map(|side| placement::place(anchor, size, screen, side).0);
        // Beside the anchor if possible; on a crowded screen over it, but never under the pointer.
        let Some(rect) = candidates
            .iter()
            .find(|rect| !covers_pointer(**rect) && rect.intersect(anchor).is_empty())
            .or_else(|| candidates.iter().find(|rect| !covers_pointer(**rect)))
            .copied()
        else {
            return;
        };

        let shown_since = cx.memory::<TooltipMemory>().shown_since.unwrap_or_default();
        let enter = cx.env().theme().motion().enter;
        let progress = cx.progress_since(shown_since, enter, Easing::EaseOut);
        let grounds = cx.grounds_around(rect);
        // The text fades in from the surface as it will show, lifted or not.
        let lifted = cx.lift_for(rect, &grounds, Some(background));
        let surface = lifted.map_or(background, |lift| lift.apply(background));
        cx.clear(rect, background);
        if let Some(lift) = lifted {
            cx.lift(rect, lift);
        }
        let inner = rect.inset(padding);
        let shown = text::truncate(&self.text, inner.width).into_owned();
        let fg = surface.mix(foreground, progress);
        cx.text(inner.x, inner.y, &shown, CellStyle { fg: Some(fg), bg: None, ..text_style }, inner.width);
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.body
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    struct Demo {
        on_focus: bool,
    }

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.column(|ui| {
                ui.spacer().height(Length::Cells(2));
                ui.row(|ui| {
                    ui.add_with(Tooltip::new("Restart all").on_focus(self.on_focus), |ui| {
                        ui.add(Button::new("Restart").on_press(())).id("restart");
                    });
                    ui.add_with(Tooltip::new("Nothing to click").placement(Placement::Above), |ui| {
                        ui.add(Text::new("status"));
                    });
                })
                .gap(2);
            });
        }
    }

    #[test]
    fn appears_after_the_hover_delay_and_leaves_with_the_pointer() {
        let mut h = Harness::new(Demo { on_focus: false }, 40, 5);
        h.hover(3, 2);
        assert!(!h.screen().contains("Restart all"));
        h.advance(Duration::from_millis(300));
        assert!(!h.screen().contains("Restart all"));
        h.advance(Duration::from_millis(200));
        // The hovered button shows its pillar; the tooltip sits below it.
        assert_eq!(h.screen(), "\n\n▌ Restart    status\n Restart all\n\n");
        assert_eq!(h.bg(1, 3), h.env().theme().color("overlay"));
        h.advance(Duration::from_millis(200));
        assert_eq!(h.fg(2, 3), h.env().theme().color("text"));
        h.hover(30, 4);
        assert!(!h.screen().contains("Restart all"));
    }

    #[test]
    fn plain_content_gets_a_tooltip_above() {
        let mut h = Harness::new(Demo { on_focus: false }, 40, 5);
        h.hover(15, 2).advance(Duration::from_secs(1));
        assert_eq!(h.screen(), "\n              Nothing to click\n  Restart    status\n\n\n");
    }

    #[test]
    fn keyboard_focus_shows_it_at_once_when_asked() {
        let mut quiet = Harness::new(Demo { on_focus: false }, 40, 5);
        quiet.press("tab");
        assert!(!quiet.screen().contains("Restart all"));
        let mut h = Harness::new(Demo { on_focus: true }, 40, 5);
        h.set_reduced_motion(true).press("tab");
        assert!(h.screen().contains("Restart all"));
        assert_eq!(h.fg(2, 3), h.env().theme().color("text"));
    }

    #[test]
    fn never_covers_the_pointer_on_a_crowded_screen() {
        struct Crowded;
        impl App for Crowded {
            type Msg = ();
            fn update(&mut self, (): ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                ui.add_with(Tooltip::new("Explained"), |ui| {
                    ui.add(Text::new("first row"));
                    ui.add(Text::new("second row"));
                });
            }
        }
        // No side has room, so the tooltip goes over the widget, on the row the pointer is not on.
        let mut h = Harness::new(Crowded, 20, 2);
        h.hover(0, 1).advance(Duration::from_secs(1));
        assert_eq!(h.screen(), " Explained\nsecond row\n");
        h.hover(0, 0).advance(Duration::from_secs(1));
        assert_eq!(h.screen(), "first row\n Explained\n");
    }
}
