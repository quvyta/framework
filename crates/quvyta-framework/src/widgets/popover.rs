//! Popovers: content that opens as a layer next to the widget it belongs to.

use std::time::Duration;

use super::placement::{self, Placement};
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::motion::Easing;
use crate::widget::{Axis, EventCx, Flex, Length, MeasureCx, Node, NodeMut, PaintCx, View, Widget, WidgetId};

type Part<'a, Msg> = Box<dyn FnOnce(&mut View<'_, Msg>) + 'a>;

/// Content shown as a layer next to an anchor, such as a filter panel under a button.
///
/// The application owns whether it is open: pass it to [`Popover::new`] and close it when the
/// [`on_dismiss`](Popover::on_dismiss) message arrives. The layer sits below the anchor, flips
/// above (or to the other side) when there is no room and never leaves the screen. It unfolds
/// from the anchor over the theme's `motion.enter`. Esc or a press outside the anchor and the
/// layer sends the dismiss message, and the press still reaches what it landed on, so one press
/// on another button closes the popover and presses that button. A press on the widget that
/// opened the popover only dismisses it, even when that widget is outside the anchor.
///
/// Style keys: `popover` (`bg`, `padding`).
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::Popover;
///
/// struct Filters { open: bool }
///
/// #[derive(Clone)]
/// enum Msg { Toggle, Close }
///
/// impl App for Filters {
///     type Msg = Msg;
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         self.open = matches!(msg, Msg::Toggle) && !self.open;
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         Popover::new(self.open)
///             .on_dismiss(Msg::Close)
///             .anchor(|ui| {
///                 ui.add(Button::new("Filters").on_press(Msg::Toggle));
///             })
///             .content(|ui| {
///                 ui.add(Text::new("Only running"));
///             })
///             .show(ui);
///     }
/// }
///
/// let mut app = Harness::new(Filters { open: false }, 30, 6);
/// app.click_text("Filters").advance(std::time::Duration::from_millis(200));
/// assert!(app.screen().contains("Only running"));
/// app.press("esc");
/// assert!(!app.screen().contains("Only running"));
/// ```
pub struct Popover<'a, Msg> {
    open: bool,
    placement: Placement,
    focus_inside: bool,
    on_dismiss: Option<Msg>,
    anchor: Option<Part<'a, Msg>>,
    content: Option<Part<'a, Msg>>,
}

impl<'a, Msg: Clone + 'static> Popover<'a, Msg> {
    /// A popover that shows its content while `open`.
    #[must_use]
    pub fn new(open: bool) -> Self {
        Self { open, placement: Placement::Below, focus_inside: false, on_dismiss: None, anchor: None, content: None }
    }

    /// The widgets the layer belongs to, usually a button that toggles it.
    #[must_use]
    pub fn anchor(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.anchor = Some(Box::new(build));
        self
    }

    /// The widgets shown in the layer.
    #[must_use]
    pub fn content(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.content = Some(Box::new(build));
        self
    }

    /// The preferred side of the anchor; [`Placement::Below`] by default.
    #[must_use]
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Moves keyboard focus to the first focusable widget in the layer when it opens, and back
    /// to where it was when it closes.
    #[must_use]
    pub fn focus_inside(mut self, focus_inside: bool) -> Self {
        self.focus_inside = focus_inside;
        self
    }

    /// Message sent on Esc or a press outside; the application usually closes the popover.
    #[must_use]
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Adds the popover to `ui`. The returned node sizes the anchor.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let build = |part: Option<Part<'a, Msg>>, index: usize| {
            let mut children = Vec::new();
            if let Some(part) = part {
                part(&mut ui.nested(&mut children));
            }
            Node::new(Flex::new(Axis::Column, children), index)
        };
        let mut anchor = build(self.anchor, ANCHOR);
        anchor.layout.width = Length::Fill(1);
        anchor.layout.height = Length::Fill(1);
        let content = build(self.content, CONTENT);
        ui.add(Layer {
            parts: [anchor, content],
            open: self.open,
            placement: self.placement,
            focus_inside: self.focus_inside,
            on_dismiss: self.on_dismiss,
        })
    }
}

const ANCHOR: usize = 0;
const CONTENT: usize = 1;

struct Layer<Msg> {
    parts: [Node<Msg>; 2],
    open: bool,
    placement: Placement,
    focus_inside: bool,
    on_dismiss: Option<Msg>,
}

#[derive(Debug, Default)]
struct PopoverMemory {
    was_open: bool,
    opened_at: Duration,
    just_opened: bool,
    focus_before: Option<WidgetId>,
    focus_was_inside: bool,
}

impl<Msg: Clone + 'static> Widget<Msg> for Layer<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        cx.measure_child(&self.parts[ANCHOR], available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.paint_child(&self.parts[ANCHOR], area);
        let now = cx.now();
        let focused = cx.focused();
        let memory = cx.memory::<PopoverMemory>();
        memory.just_opened = self.open && !memory.was_open;
        if memory.just_opened {
            memory.opened_at = now;
            memory.focus_before = focused;
        }
        let closed_with_focus = !self.open && memory.was_open && memory.focus_was_inside;
        let give_back = memory.focus_before.take_if(|_| closed_with_focus);
        memory.was_open = self.open;
        if !self.open {
            memory.focus_was_inside = false;
        }
        if let Some(previous) = give_back.filter(|_| self.focus_inside) {
            cx.request_focus(previous);
        }
        if self.open {
            cx.request_overlay(area);
            cx.register_dismissable();
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let style = cx.style("popover", None, &[]);
        let padding = style.padding();
        let background = style.text().bg.unwrap_or_else(|| cx.color("overlay"));
        let screen = cx.clip();
        let available = Size::new(
            screen.width.saturating_sub(padding.horizontal()),
            screen.height.saturating_sub(padding.vertical()),
        );
        let content = cx.measure_child(&self.parts[CONTENT], available);
        let size = Size::new(
            content.width.saturating_add(padding.horizontal()),
            content.height.saturating_add(padding.vertical()),
        );
        let (full, side) = placement::place(anchor, size, screen, self.placement);

        // The layer unfolds from the anchor's side over `motion.enter`.
        let (opened_at, just_opened) = {
            let memory = cx.memory::<PopoverMemory>();
            (memory.opened_at, memory.just_opened)
        };
        let enter = cx.env().theme().motion().enter;
        let progress = cx.progress_since(opened_at, enter, Easing::EaseOut);
        let shown = placement::unfold(full, side, progress);
        cx.register_hit(shown);
        cx.floating(shown, |cx| {
            cx.clear(shown, background);
            cx.with_clip(shown, |cx| cx.paint_child(&self.parts[CONTENT], full.inset(padding)));
        });
        let content_id = self.parts[CONTENT].id();
        if self.focus_inside && just_opened {
            cx.request_focus_within(content_id);
        }
        // Focus on the anchor counts too; giving focus back to it on close changes nothing.
        let inside = cx.has_focus_within();
        cx.memory::<PopoverMemory>().focus_was_inside |= inside;
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.open {
            return false;
        }
        let dismiss = match event {
            Event::PointerOutside => true,
            Event::Key(key) => key.is_plain(Key::Esc),
            _ => false,
        };
        if dismiss && let Some(message) = &self.on_dismiss {
            cx.emit(message.clone());
        }
        dismiss
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.parts
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widgets::{Button, Text, TextInput};

    struct Demo {
        open: bool,
        focus_inside: bool,
        placement: Placement,
        name: String,
        dismissed: u32,
        other: u32,
    }

    #[derive(Clone)]
    enum Msg {
        Toggle,
        Dismiss,
        Name(String),
        Other,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Toggle => self.open = !self.open,
                Msg::Dismiss => {
                    self.open = false;
                    self.dismissed += 1;
                }
                Msg::Name(name) => self.name = name,
                Msg::Other => self.other += 1,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.row(|ui| {
                    Popover::new(self.open)
                        .placement(self.placement)
                        .focus_inside(self.focus_inside)
                        .on_dismiss(Msg::Dismiss)
                        .anchor(|ui| {
                            ui.add(Button::new("Filters").on_press(Msg::Toggle)).id("filters");
                        })
                        .content(|ui| {
                            ui.add(Text::new("Status"));
                            ui.add(TextInput::new(&self.name).on_change(Msg::Name)).width(Length::Cells(10)).id("name");
                        })
                        .show(ui);
                    ui.add(Button::new("Other").on_press(Msg::Other)).id("other");
                })
                .gap(1);
                ui.add(Text::new("content under the layer")).selectable(true);
            });
        }
    }

    fn demo() -> Demo {
        Demo {
            open: false,
            focus_inside: false,
            placement: Placement::Below,
            name: String::new(),
            dismissed: 0,
            other: 0,
        }
    }

    #[test]
    fn opens_below_unfolding_on_the_overlay_surface() {
        let mut h = Harness::new(demo(), 40, 8);
        h.click_text("Filters");
        assert!(h.app().open);
        let first = h.screen();
        assert!(!first.contains("Status"), "the layer starts folded: {first}");
        h.advance(Duration::from_millis(200));
        // The pointer still rests on the anchor button, so it shows its pillar.
        assert_eq!(h.screen(), "▌ Filters     Other\n              the layer\n  Status\n    ❯\n\n\n\n\n");
        let overlay = h.env().theme().color("overlay");
        assert_eq!(h.bg(0, 1), overlay);
        assert_eq!(h.bg(13, 4), overlay);
    }

    #[test]
    fn escape_and_outside_press_dismiss_and_the_press_still_reaches_its_target() {
        let mut h = Harness::new(Demo { open: true, ..demo() }, 40, 8);
        h.press("esc");
        assert_eq!((h.app().open, h.app().dismissed), (false, 1));
        h.click_text("Filters").advance(Duration::from_millis(200));
        h.click_text("Other");
        assert_eq!((h.app().open, h.app().dismissed, h.app().other), (false, 2, 1), "one press closes and acts");
    }

    #[test]
    fn a_press_that_closes_the_layer_follows_the_text_selection_rules_of_its_cell() {
        let mut closed = Harness::new(demo(), 40, 8);
        closed.drag((16, 1), (30, 1)).press("ctrl+c");
        assert!(closed.clipboard().is_some_and(|text| text.contains("layer")), "{:?}", closed.clipboard());
        let mut h = Harness::new(Demo { open: true, ..demo() }, 40, 8);
        h.advance(Duration::from_millis(200));
        h.drag((16, 1), (30, 1)).press("ctrl+c");
        assert_eq!((h.app().open, h.app().dismissed), (false, 1));
        assert_eq!(h.clipboard(), closed.clipboard(), "the same drag without a layer selects the same");
    }

    #[test]
    fn clicking_inside_the_layer_keeps_it_open_and_the_anchor_toggles() {
        let mut h = Harness::new(Demo { open: true, ..demo() }, 40, 8);
        h.advance(Duration::from_millis(200));
        h.click_text("Status");
        assert!(h.app().open);
        h.click_text("Filters");
        assert_eq!((h.app().open, h.app().dismissed), (false, 0));
    }

    #[test]
    fn focus_moves_inside_and_comes_back() {
        let mut h = Harness::new(Demo { focus_inside: true, ..demo() }, 40, 8);
        h.press("tab");
        assert!(h.is_focused("filters"));
        h.press("enter");
        assert!(h.is_focused("name"));
        h.type_text("web");
        assert_eq!(h.app().name, "web");
        h.press("esc");
        assert!(!h.app().open);
        assert!(h.is_focused("filters"));
    }

    #[test]
    fn flips_above_near_the_bottom_and_opens_at_once_with_reduced_motion() {
        struct Bottom(bool);
        impl App for Bottom {
            type Msg = ();
            fn update(&mut self, (): ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                ui.column(|ui| {
                    ui.spacer();
                    Popover::new(self.0)
                        .anchor(|ui| {
                            ui.add(Text::new("anchor"));
                        })
                        .content(|ui| {
                            ui.add(Text::new("menu"));
                        })
                        .show(ui);
                })
                .fill();
            }
        }
        let mut h = Harness::new(Bottom(true), 20, 6);
        h.set_reduced_motion(true);
        assert_eq!(h.screen(), "\n\n\n  menu\n\nanchor\n");
    }

    #[test]
    fn side_placement_sits_to_the_right() {
        let mut h = Harness::new(Demo { open: true, placement: Placement::Right, ..demo() }, 60, 8);
        h.advance(Duration::from_millis(200));
        let screen = h.screen();
        assert!(screen.lines().nth(1).is_some_and(|line| line.starts_with("content und  Status")), "{screen}");
        assert_eq!(h.bg(11, 0), h.env().theme().color("overlay"));
    }
}
