//! Widget docks: a stack of titled widgets that open, close and are reordered by dragging.

use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::widget::{Container, EventCx, MeasureCx, Node, PaintCx, Widget};

use super::sections::{Flow, Section, Sections};

/// A vertical stack of titled widgets filling the area it is given, like the side panel of an
/// IDE: each widget opens and closes, and open widgets share the height between them.
///
/// Add one child per widget with [`View::add_with`](crate::widget::View::add_with), in the
/// order they are shown. A widget that wants less than its share keeps its natural height; a
/// taller one, such as a [`List`](super::List), gets an even share of what is left and scrolls
/// inside it. The application owns the order and the open state, so both can be saved and
/// restored: [`WidgetDock::on_toggle`] and [`WidgetDock::on_move`] report changes.
///
/// With `on_move`, dragging a title picks the widget up: its title follows the pointer as a
/// ghost and a tinted row shows where it will land. Keys while focused: ↑/↓, Home/End move
/// between titles, Enter or Space opens or closes, Ctrl+Shift+↑/↓ moves the widget.
///
/// Style keys are those of [`Accordion`](super::Accordion) plus `section-title.ghost` for the
/// dragged title, `section-drop` (`bg`) for the landing row and `section-empty` (`fg`).
pub struct WidgetDock<Msg> {
    model: Sections<Msg>,
    empty: String,
}

impl<Msg: 'static> WidgetDock<Msg> {
    /// A dock of `sections` in this order, all closed.
    #[must_use]
    pub fn new(sections: impl IntoIterator<Item = impl Into<Section>>) -> Self {
        Self { model: Sections::new(sections.into_iter().map(Into::into).collect()), empty: String::new() }
    }

    /// Which widgets are open; `open[i]` for widget `i`, missing entries are closed.
    #[must_use]
    pub fn open(mut self, open: impl IntoIterator<Item = bool>) -> Self {
        self.model.open = open.into_iter().collect();
        self
    }

    /// Message for opening (`true`) or closing (`false`) widget `index`.
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(usize, bool) -> Msg + 'static) -> Self {
        self.model.on_toggle = Some(Box::new(message));
        self
    }

    /// Lets the user reorder widgets. The message carries the widget's index and the index it
    /// should have afterwards: remove it at `from`, then insert it at `to`.
    #[must_use]
    pub fn on_move(mut self, message: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.model.on_move = Some(Box::new(message));
        self
    }

    /// Text shown when the dock has no widgets.
    #[must_use]
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }
}

impl<Msg: 'static> Container<Msg> for WidgetDock<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        self.model.bodies = children;
    }
}

impl<Msg: 'static> Widget<Msg> for WidgetDock<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.model.sections.is_empty() {
            return Size::new(crate::text::width(&self.empty).saturating_add(4), 1).min(available);
        }
        self.model.measure(cx, available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if self.model.sections.is_empty() {
            let style = cx.style("section-empty", None, &[]).text();
            cx.text(area.x + 2, area.y, &self.empty, CellStyle { bg: None, ..style }, area.width.saturating_sub(2));
            return;
        }
        self.model.paint(cx, area, Flow::Share);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        self.model.event(cx, event)
    }

    fn focusable(&self) -> bool {
        !self.model.sections.is_empty()
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.model.bodies
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.model.bodies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{MouseButton, MouseKind};
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{List, ListItem, Text};

    struct Demo {
        order: Vec<&'static str>,
        open: Vec<bool>,
    }

    #[derive(Clone)]
    enum Msg {
        Toggle(usize, bool),
        Move(usize, usize),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Toggle(index, open) => self.open[index] = open,
                Msg::Move(from, to) => {
                    let name = self.order.remove(from);
                    let open = self.open.remove(from);
                    self.order.insert(to, name);
                    self.open.insert(to, open);
                }
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let dock = WidgetDock::new(self.order.clone())
                .open(self.open.clone())
                .on_toggle(Msg::Toggle)
                .on_move(Msg::Move)
                .empty_text("No widgets");
            ui.add_with(dock, |ui| {
                for name in &self.order {
                    match *name {
                        "Files" => {
                            let rows = (1..=30).map(|n| ListItem::new(format!("file {n}")));
                            ui.add(List::new(rows)).fill().id("files");
                        }
                        other => {
                            ui.add(Text::new(format!("{other} body"))).id(other);
                        }
                    }
                }
            })
            .fill()
            .id("dock");
        }
    }

    fn demo(open: [bool; 3]) -> Demo {
        Demo { order: vec!["Git", "Files", "Ports"], open: open.to_vec() }
    }

    #[test]
    fn open_widgets_share_the_height() {
        let mut h = Harness::new(demo([true, true, true]), 24, 16);
        h.set_reduced_motion(true);
        let screen = h.screen();
        let rows: Vec<&str> = screen.lines().collect();
        assert_eq!(rows[0], "  ▾ Git");
        assert_eq!(rows[2], "  Git body");
        assert!(rows.iter().any(|row| row.starts_with("  ▾ Files")));
        assert!(rows.contains(&"  Ports body"), "{screen}");
        assert_eq!(rows.len(), 16, "{screen}");
        assert!(screen.contains("file 1") && !screen.contains("file 30"), "{screen}");
    }

    #[test]
    fn keyboard_reorders_and_toggles() {
        let mut h = Harness::new(demo([false; 3]), 24, 10);
        h.set_reduced_motion(true);
        h.press("tab").press("ctrl+shift+down");
        assert_eq!(h.app().order, vec!["Files", "Git", "Ports"]);
        h.press("enter");
        assert_eq!(h.app().open, vec![false, true, false]);
        h.press("ctrl+shift+up");
        assert_eq!(h.app().order, vec!["Git", "Files", "Ports"]);
        assert_eq!(h.app().open, vec![true, false, false]);
    }

    #[test]
    fn dragging_a_title_shows_a_ghost_and_drops_it_between_others() {
        let mut h = Harness::new(demo([false; 3]), 24, 10);
        h.set_reduced_motion(true);
        let theme = h.env().theme();
        let drop = theme.style("section-drop", None, &[]).paint("bg").map(|paint| paint.at(0.0));
        h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
        let screen = h.screen();
        assert!(screen.contains("Git"), "the ghost title follows the pointer:\n{screen}");
        assert!((0..10).any(|y| h.bg(20, y) == drop), "a tinted landing row:\n{screen}");
        h.mouse(MouseKind::Drag(MouseButton::Left), 5, 5);
        h.mouse(MouseKind::Up(MouseButton::Left), 5, 5);
        assert_eq!(h.app().order, vec!["Files", "Ports", "Git"]);
        assert_eq!(h.app().open, vec![false; 3], "a drag never toggles");
    }

    #[test]
    fn click_without_moving_toggles() {
        let mut h = Harness::new(demo([false; 3]), 24, 10);
        h.set_reduced_motion(true);
        h.click_text("Ports");
        assert_eq!(h.app().open, vec![false, false, true]);
        assert_eq!(h.app().order, vec!["Git", "Files", "Ports"]);
    }

    /// A dock whose height the application sets, to shrink it while a title is dragged.
    struct Shrinking {
        rows: u16,
    }

    impl App for Shrinking {
        type Msg = u16;
        fn update(&mut self, rows: u16) -> Command<u16> {
            self.rows = rows;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, u16>) {
            let dock = WidgetDock::new(["Git", "Ports"]).on_toggle(|_, _| 0).on_move(|_, _| 0);
            ui.add_with(dock, |ui| {
                ui.add(Text::new("Git body"));
                ui.add(Text::new("Ports body"));
            })
            .fill_width()
            .height(crate::widget::Length::Cells(self.rows));
        }
    }

    #[test]
    fn a_drag_survives_the_dock_losing_its_height() {
        let mut h = Harness::new(Shrinking { rows: 6 }, 24, 6);
        h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
        h.send(0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 5, 4);
        h.mouse(MouseKind::Up(MouseButton::Left), 5, 4);
        assert_eq!(h.screen(), "\n\n\n\n\n\n", "a dock with no rows paints nothing");
    }

    #[test]
    fn empty_dock_says_so() {
        let mut app = demo([false; 3]);
        app.order.clear();
        app.open.clear();
        assert_eq!(Harness::new(app, 24, 3).screen(), "  No widgets\n\n\n");
    }
}
