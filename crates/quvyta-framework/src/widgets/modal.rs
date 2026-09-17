//! Modal dialogs.

use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::widget::{Axis, Container, EventCx, Flex, Length, MeasureCx, Node, PaintCx, Widget};

use super::Button;
use super::cells;
use super::layer::{self, Backdrop, SurfacePosition};

/// Width of a dialog when none is set, in cells.
const DEFAULT_WIDTH: u16 = 56;

/// Cells between two action buttons.
const ACTION_GAP: u16 = 2;

/// A dialog over a dimmed screen: a title, any content and action buttons.
///
/// Add it to the view while it should be shown, anywhere in the tree: it takes no room where it
/// is added and is drawn over everything as a layer, centred on the screen. It enters with a
/// short pop over the theme's `motion.enter` (at once with reduced motion). While it is open,
/// focus stays inside (the first focusable widget is focused), Tab cycles through its widgets,
/// nothing beneath reacts to keys or the pointer and application shortcuts are paused; when it
/// is removed, focus returns to where it was. Dialogs stack: one added inside another, or later
/// in the view, is on top.
///
/// A pillar runs down the whole left edge of the surface, accent-muted, in the danger colour for
/// `variant("danger")`.
///
/// With no options it is a plain surface with its content that only the application closes.
/// `on_close` makes it dismissable: Esc and the close mark `×` at the top right send the message,
/// always together, and `close_on_click_outside` adds a click on the dimmed screen.
/// `dismissable(false)` turns all of them off at once (and hides the mark) while keeping the
/// message, e.g. while the dialog is busy. `title` and `variant("danger")` mark it, `action` adds
/// buttons at the bottom right. A faint hint line at the bottom left names Esc and Tab when they
/// do something.
///
/// Style keys: `modal` (`bg`, `padding`, `pillar`) with variants such as `modal.danger`,
/// `modal-title` (`fg`, `bold`), `close-mark`, `layer-backdrop` (`scrim`, `strength` in percent),
/// `layer-hint-key`, `layer-hint-label`. Hint labels: `quvyta.layer.close`,
/// `quvyta.layer.switch`.
pub struct Modal<Msg> {
    title: Option<String>,
    variant: Option<String>,
    width: u16,
    on_close: Option<Msg>,
    dismissable: bool,
    click_outside_closes: bool,
    /// The body column first, then one node per action.
    parts: Vec<Node<Msg>>,
}

impl<Msg: Clone + 'static> Modal<Msg> {
    /// An empty dialog. Add its content with [`View::add_with`](crate::widget::View::add_with).
    #[must_use]
    pub fn new() -> Self {
        Self {
            title: None,
            variant: None,
            width: DEFAULT_WIDTH,
            on_close: None,
            dismissable: true,
            click_outside_closes: false,
            parts: vec![body(Vec::new())],
        }
    }

    /// A bold heading in the first row.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Theme variant; `"danger"` gives a destructive dialog a danger pillar.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Width in cells, padding included; 56 by default. Narrow screens shrink it.
    #[must_use]
    pub fn width(mut self, cells: u16) -> Self {
        self.width = cells;
        self
    }

    /// The message sent when the dialog is dismissed: Esc inside it or a click on its close mark.
    /// Without it neither exists and the dialog closes only through its own buttons.
    #[must_use]
    pub fn on_close(mut self, message: Msg) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Whether the dialog can be dismissed; `true` by default. `false` turns off Esc, the close
    /// mark and the click outside together, and hides the mark, without removing `on_close`.
    #[must_use]
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
        self
    }

    /// Also sends the close message when the dimmed screen around the dialog is clicked, while
    /// the dialog is dismissable.
    #[must_use]
    pub fn close_on_click_outside(mut self, closes: bool) -> Self {
        self.click_outside_closes = closes;
        self
    }

    /// Adds a button to the action row at the bottom right, after the ones added before. Put
    /// the safe action first: the first focusable widget has focus when the dialog opens.
    #[must_use]
    pub fn action(mut self, button: Button<Msg>) -> Self {
        let index = self.parts.len();
        self.parts.push(Node::new(button, index));
        self
    }
}

impl<Msg> Modal<Msg> {
    fn is_dismissable(&self) -> bool {
        self.dismissable && self.on_close.is_some()
    }
}

fn body<Msg: 'static>(children: Vec<Node<Msg>>) -> Node<Msg> {
    let mut column = Node::new(Flex::new(Axis::Column, children), 0);
    column.layout.width = Length::Fill(1);
    column
}

fn count_focusable<M: 'static>(node: &Node<M>) -> usize {
    usize::from(node.widget.focusable()) + node.widget.children().iter().map(count_focusable).sum::<usize>()
}

impl<Msg: Clone + 'static> Default for Modal<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> Container<Msg> for Modal<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        self.parts[0] = body(children);
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Modal<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, _available: Size) -> Size {
        Size::default()
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.request_overlay(area);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, _anchor: Rect) {
        let screen = cx.clip();
        let dismissable = self.is_dismissable();
        let padding = layer::padding(cx, "modal", dismissable);
        let width = self.width.min(screen.width.saturating_sub(2));
        let inner_width = width.saturating_sub(padding.horizontal());
        let title_rows: u16 = if self.title.is_some() { 2 } else { 0 };
        let (body, actions) = self.parts.split_first().expect("a modal always has its body");
        let action_sizes: Vec<Size> =
            actions.iter().map(|action| cx.measure_child(action, Size::new(inner_width, 1))).collect();
        let mut hints = Vec::new();
        if dismissable {
            hints.push(layer::hint(cx, "esc", "close"));
        }
        if count_focusable(body) + actions.len() > 1 {
            hints.push(layer::hint(cx, "tab", "switch"));
        }
        let footer_rows: u16 = if actions.is_empty() && hints.is_empty() { 0 } else { 2 };
        let chrome = cells::sum([padding.vertical(), title_rows, footer_rows]);
        let available_body = screen.height.saturating_sub(chrome.saturating_add(2));
        let body_height = cx.measure_child(body, Size::new(inner_width, available_body)).height;
        let size = Size::new(width, chrome.saturating_add(body_height));

        let look = layer::Look { style: "modal", variant: self.variant.as_deref(), dismissable };
        let surface = layer::open(cx, size, SurfacePosition::Center, look);
        let inner = surface.inner;
        cx.with_clip(surface.shown, |cx| {
            if let Some(title) = &self.title {
                layer::title(cx, inner.x, inner.y, inner.width, title);
            }
            let body_rect = Rect::new(
                inner.x,
                inner.y + i32::from(title_rows),
                inner.width,
                inner.height.saturating_sub(title_rows + footer_rows),
            );
            cx.paint_child(body, body_rect);
            if footer_rows == 0 {
                return;
            }
            let row = inner.bottom() - 1;
            let gaps = ACTION_GAP * u16::try_from(actions.len().saturating_sub(1)).unwrap_or(0);
            let actions_width = action_sizes.iter().map(|size| size.width).sum::<u16>() + gaps;
            let start = inner.right() - i32::from(actions_width);
            // Painted left to right so Tab follows the order the user reads.
            let mut x = start;
            for (action, size) in actions.iter().zip(&action_sizes) {
                cx.paint_child(action, Rect::new(x, row, size.width, 1));
                x += i32::from(size.width + ACTION_GAP);
            }
            let hint_width = crate::geometry::clamp_u16(start - i32::from(ACTION_GAP) - inner.x);
            layer::paint_hints(cx, inner.x, row, hint_width, &hints);
        });
        layer::finish(cx, &surface);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        match layer::backdrop_event(cx, event, self.is_dismissable(), self.click_outside_closes) {
            Backdrop::Close => {
                if let Some(message) = &self.on_close {
                    cx.emit(message.clone());
                }
                true
            }
            Backdrop::Swallowed => true,
            Backdrop::Inside | Backdrop::Ignored => false,
        }
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
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Text, TextInput};

    #[derive(Default)]
    struct Demo {
        open: bool,
        nested: bool,
        removed: u32,
        name: String,
        outside: bool,
        busy: bool,
        plain: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Open,
        Close,
        Remove,
        Nested(bool),
        Name(String),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Open => self.open = true,
                Msg::Close => self.open = false,
                Msg::Remove => {
                    self.removed += 1;
                    self.open = false;
                }
                Msg::Nested(on) => self.nested = on,
                Msg::Name(name) => self.name = name,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.add(Text::new("Containers"));
                ui.add(Button::new("Remove web").on_press(Msg::Open)).id("open");
                ui.add(Button::new("Other").on_press(Msg::Nested(false))).id("other");
                if self.open {
                    let mut modal = Modal::new().width(40).on_close(Msg::Close).dismissable(!self.busy);
                    if !self.plain {
                        modal = modal.title("Remove web?").variant("danger");
                    }
                    let modal = modal
                        .close_on_click_outside(self.outside)
                        .action(Button::new("Cancel").on_press(Msg::Close))
                        .action(Button::new("Remove").variant("danger").on_press(Msg::Remove));
                    ui.add_with(modal, |ui| {
                        ui.add(Text::new("Its volumes go too."));
                        ui.add(TextInput::new(&self.name).on_change(Msg::Name)).id("name");
                        if self.nested {
                            ui.add_with(Modal::new().title("Sure?").on_close(Msg::Nested(false)).width(24), |ui| {
                                ui.add(Text::new("Really."));
                            });
                        }
                    });
                }
            });
        }
    }

    fn opened(demo: Demo) -> Harness<Demo> {
        let mut h = Harness::new(demo, 50, 16);
        h.press("tab").press("enter").advance(Duration::from_millis(200));
        h
    }

    #[test]
    fn draws_a_dimmed_screen_a_pillar_down_the_edge_a_close_mark_and_right_aligned_actions() {
        let h = opened(Demo::default());
        let screen = h.screen();
        // The danger pillar runs down every row of the surface, and the close mark takes the top
        // right corner, on the top padding row above the title.
        assert_eq!(
            screen,
            "Containers\n  Remove web\n  Other\n\n     ▌                                     ×\n     ▌  Remove web?\n     ▌\n     ▌  Its volumes go too.\n     ▌  ▌ ❯\n     ▌\n     ▌  esc close     Cancel      Remove\n     ▌\n\n\n\n\n",
            "{screen}"
        );
        let theme = h.env().theme();
        assert_eq!(h.bg(20, 5), theme.color("overlay"));
        for row in 4..=11 {
            assert_eq!(h.fg(5, row), theme.color("danger"), "pillar on row {row}");
        }
        assert!(h.is_bold(8, 5));
        let text = theme.color("text").expect("text colour");
        let dimmed = h.fg(0, 0).expect("dimmed text");
        assert_ne!(dimmed, text, "the screen behind is dimmed");
    }

    #[test]
    fn a_plain_dialog_has_an_accent_muted_pillar_and_its_close_mark_on_the_first_row() {
        let h = opened(Demo { plain: true, ..Demo::default() });
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(lines[5], "     ▌                                     ×", "the top padding row: {screen}");
        assert_eq!(lines[6], "     ▌  Its volumes go too.", "{screen}");
        let theme = h.env().theme();
        let muted =
            theme.color("accent").zip(theme.color("overlay")).map(|(accent, overlay)| overlay.mix(accent, 0.45));
        let pillar = h.fg(5, 6).expect("pillar colour");
        let expected = muted.expect("theme colours");
        let close = |a: u8, b: u8| a.abs_diff(b) <= 1;
        assert!(close(pillar.r, expected.r) && close(pillar.g, expected.g), "{pillar:?} vs {expected:?}");
        assert_ne!(Some(pillar), theme.color("accent"), "muted, not the full accent");
    }

    #[test]
    fn the_close_mark_lights_three_cells_under_the_pointer_and_closes_on_a_click() {
        let mut h = opened(Demo::default());
        let (x, y) = h.find("×").expect("close mark");
        let (column, row) = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
        let resting = h.bg(column, row);
        h.hover(x - 1, y);
        let lit = h.bg(column, row);
        assert_ne!(lit, resting, "the mark lights up");
        assert_eq!((h.bg(column - 1, row), h.bg(column + 1, row)), (lit, lit), "all three cells light up");
        assert_eq!(h.bg(column - 2, row), resting, "and no more");
        h.click(x + 1, y);
        assert!(!h.app().open, "a click on the mark closes");
    }

    #[test]
    fn a_dialog_that_is_not_dismissable_has_no_close_mark_and_ignores_esc_and_outside_clicks() {
        let mut h = opened(Demo { busy: true, outside: true, ..Demo::default() });
        let screen = h.screen();
        assert!(!screen.contains('×') && !screen.contains("esc close"), "{screen}");
        h.press("esc").click(1, 14).click(44, 5);
        assert!(h.app().open, "Esc, the corner and the dimmed screen do nothing");
        h.click_text("Cancel");
        assert!(!h.app().open, "its own buttons still close it");
    }

    #[test]
    fn escape_and_the_close_mark_always_come_together() {
        let mut h = opened(Demo::default());
        h.press("esc");
        assert!(!h.app().open, "Esc closes a dismissable dialog");
        let mut h = opened(Demo::default());
        h.click_text("×");
        assert!(!h.app().open, "and so does its mark");
    }

    #[test]
    fn the_pillar_and_the_close_mark_enter_with_the_surface_and_ascii_keeps_both() {
        let mut h = Harness::new(Demo::default(), 50, 16);
        h.press("tab").press("enter");
        let danger = h.env().theme().color("danger");
        let entering: Vec<_> = (4..12).filter_map(|row| h.fg(7, row)).collect();
        assert!(h.screen().contains('▌'), "{}", h.screen());
        assert!(entering.iter().all(|color| Some(*color) != danger), "the pillar fades in with the surface");
        h.advance(Duration::from_millis(200));
        assert_eq!(h.fg(5, 5), h.env().theme().color("danger"));
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        let screen = h.screen();
        assert!(screen.lines().nth(4).is_some_and(|line| line.ends_with('x')), "{screen}");
        assert_eq!(h.bg(5, 7), h.env().theme().color("danger"), "the ASCII pillar is a coloured cell");
        let mut h = Harness::new(Demo::default(), 50, 16);
        h.set_reduced_motion(true).press("tab").press("enter");
        assert_eq!(h.fg(5, 9), h.env().theme().color("danger"), "at once with reduced motion");
        assert!(h.screen().contains('×'));
    }

    #[test]
    fn narrow_screens_keep_the_close_mark_inside_the_surface() {
        let mut h = Harness::new(Demo::default(), 24, 16);
        h.set_reduced_motion(true).press("tab").press("enter");
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let mark = lines.iter().position(|line| line.contains('×')).unwrap_or_default();
        assert!(lines[mark].ends_with('×') && lines[mark].chars().count() <= 23, "{screen}");
        assert!(lines[mark + 1].contains("Remove web?"), "the mark sits on the row above the title: {screen}");
    }

    #[test]
    fn the_close_mark_takes_the_top_right_cells_of_the_surface() {
        let mut h = opened(Demo::default());
        let overlay = h.env().theme().color("overlay");
        let (x, y) = h.find("×").expect("close mark");
        let (column, row) = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
        let on_surface = |h: &Harness<Demo>, column: u16, row: u16| h.bg(column, row) == overlay;
        assert!(on_surface(&h, column - 3, row), "the mark's row belongs to the surface");
        assert!(!on_surface(&h, column, row - 1), "and it is the surface's first row");
        assert!(on_surface(&h, column + 1, row + 1), "the mark's last cell is on the surface's last column");
        assert!(!on_surface(&h, column + 2, row + 1), "and nothing of the surface lies beyond it");
        h.hover(x, y);
        let lit = h.bg(column, row);
        assert_ne!(lit, overlay, "the mark lights up");
        assert_eq!([h.bg(column - 1, row), h.bg(column + 1, row)], [lit, lit], "its three cells light together");
        assert_eq!(h.bg(column - 2, row), overlay);
    }

    #[test]
    fn focus_is_trapped_and_returns_on_close() {
        let mut h = opened(Demo::default());
        assert!(h.is_focused("name"), "the first focusable widget inside is focused");
        h.press("tab").press("tab").press("tab");
        assert!(h.is_focused("name"), "tab cycles inside the dialog");
        h.press("shift+tab");
        assert!(h.screen().contains("Remove"), "{}", h.screen());
        h.press("enter");
        assert_eq!(h.app().removed, 1, "{}", h.screen());
        assert!(h.is_focused("open"), "focus returns to the button that opened the dialog");
    }

    #[test]
    fn escape_closes_and_keys_never_reach_widgets_beneath() {
        let mut h = opened(Demo::default());
        h.type_text("db");
        assert_eq!(h.app().name, "db");
        h.press("esc");
        assert!(!h.app().open);
        assert!(!h.screen().contains("Remove web?"));
    }

    #[test]
    fn clicks_outside_are_swallowed_or_close_when_asked() {
        let mut h = opened(Demo::default());
        h.click_text("Containers");
        assert!(h.app().open, "clicks on the dimmed screen do nothing by default");
        let mut h = opened(Demo { outside: true, ..Demo::default() });
        h.click(1, 14);
        assert!(!h.app().open);
    }

    #[test]
    fn dialogs_stack_and_the_top_one_owns_the_keys() {
        let mut h = opened(Demo { nested: true, ..Demo::default() });
        let screen = h.screen();
        assert!(screen.contains("Sure?") && screen.contains("Really."), "{screen}");
        h.press("esc");
        assert!(!h.app().nested);
        assert!(h.app().open, "only the top dialog closed");
    }

    #[test]
    fn pops_in_and_appears_at_once_with_reduced_motion() {
        let mut h = Harness::new(Demo::default(), 50, 16);
        h.press("tab").press("enter");
        let entering = h.bg(7, 5);
        h.advance(Duration::from_millis(200));
        assert_ne!(entering, h.bg(7, 5), "the surface grows in over motion.enter");
        let mut h = Harness::new(Demo::default(), 50, 16);
        h.set_reduced_motion(true).press("tab").press("enter");
        assert_eq!(h.bg(7, 5), h.env().theme().color("overlay"));
    }
}
