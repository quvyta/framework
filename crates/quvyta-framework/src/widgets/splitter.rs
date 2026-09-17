//! Splitters: two panes side by side or stacked, divided by a boundary that is never a line.

use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::widget::{Axis as FlexAxis, EventCx, Flex, Length, MeasureCx, Node, NodeMut, PaintCx, View, Widget};

use super::boundary::{self, Axis, Change};

type Pane<'a, Msg> = Box<dyn FnOnce(&mut View<'_, Msg>) + 'a>;

/// Builds a message from a new size in cells.
type SizeMessage<Msg> = Box<dyn Fn(u16) -> Msg>;

/// Two panes that share an area: side by side with [`Splitter::columns`], stacked with
/// [`Splitter::rows`]. The application owns the size of the first pane; the second takes the
/// rest.
///
/// Without [`Splitter::on_resize`] the panes simply sit next to each other. With it, a one-cell
/// boundary separates them. The boundary is invisible until the pointer reaches it: then it
/// brightens one step, and while dragged it takes the accent. It can also be focused with Tab
/// and moved with the arrow keys (Shift moves five cells, Home and End jump to the limits).
///
/// Style keys: `split-handle` (`bg`, `fg`) with `hover`, `focus` and `active` (dragging).
pub struct Splitter<'a, Msg> {
    axis: Axis,
    size: u16,
    min: u16,
    max: Option<u16>,
    on_resize: Option<SizeMessage<Msg>>,
    first: Option<Pane<'a, Msg>>,
    second: Option<Pane<'a, Msg>>,
}

impl<'a, Msg: 'static> Splitter<'a, Msg> {
    fn new(axis: Axis, size: u16) -> Self {
        Self { axis, size, min: 1, max: None, on_resize: None, first: None, second: None }
    }

    /// Panes side by side; the first (left) pane is `width` columns wide.
    #[must_use]
    pub fn columns(width: u16) -> Self {
        Self::new(Axis::Columns, width)
    }

    /// Stacked panes; the first (top) pane is `height` rows tall.
    #[must_use]
    pub fn rows(height: u16) -> Self {
        Self::new(Axis::Rows, height)
    }

    /// The smallest and largest size of the first pane, in cells: `limits(16, 60)` for a range, or
    /// `limits(16, None)` for no upper limit. By default 1 and no upper limit; whatever the limits,
    /// the second pane keeps at least one cell. A `max` below `min` counts as `min`.
    #[must_use]
    pub fn limits(mut self, min: u16, max: impl Into<Option<u16>>) -> Self {
        self.min = min;
        self.max = max.into();
        self
    }

    /// Makes the boundary draggable and focusable; the message carries the first pane's new
    /// size, already within the limits.
    #[must_use]
    pub fn on_resize(mut self, message: impl Fn(u16) -> Msg + 'static) -> Self {
        self.on_resize = Some(Box::new(message));
        self
    }

    /// The left or top pane.
    #[must_use]
    pub fn first(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.first = Some(Box::new(build));
        self
    }

    /// The right or bottom pane.
    #[must_use]
    pub fn second(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.second = Some(Box::new(build));
        self
    }

    /// Adds the splitter to `ui`, filling the space it gets.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let env = ui.env();
        let build = |pane: Option<Pane<'a, Msg>>| {
            let mut children = Vec::new();
            if let Some(pane) = pane {
                pane(&mut View::new(&mut children, env));
            }
            let mut node = Node::new(Flex::new(FlexAxis::Column, children), 0);
            node.layout.width = Length::Fill(1);
            node.layout.height = Length::Fill(1);
            node
        };
        let split = Split {
            axis: self.axis,
            size: self.size,
            min: self.min,
            max: self.max,
            on_resize: self.on_resize,
            panes: vec![build(self.first), build(self.second)],
        };
        ui.add(split).fill()
    }
}

struct Split<Msg> {
    axis: Axis,
    size: u16,
    min: u16,
    /// The largest first pane the application allows; `None` for no limit.
    max: Option<u16>,
    on_resize: Option<SizeMessage<Msg>>,
    panes: Vec<Node<Msg>>,
}

impl<Msg: 'static> Split<Msg> {
    fn extent(&self, area: Rect) -> u16 {
        match self.axis {
            Axis::Columns => area.width,
            Axis::Rows => area.height,
        }
    }

    /// The largest first-pane size within the limits that leaves the handle and one cell of the
    /// second pane.
    fn max_size(&self, area: Rect) -> u16 {
        let handle = u16::from(self.on_resize.is_some());
        let room = self.extent(area).saturating_sub(handle + 1);
        self.max.map_or(room, |max| max.min(room))
    }

    fn first_size(&self, area: Rect) -> u16 {
        boundary::clamp_size(i32::from(self.size), self.min, self.max_size(area)).min(self.extent(area))
    }

    fn resize(&self, cx: &mut EventCx<'_, Msg>, target: i32) {
        let area = cx.area();
        let size = boundary::clamp_size(target, self.min, self.max_size(area));
        if let Some(message) = &self.on_resize
            && size != self.first_size(area)
        {
            cx.emit(message(size));
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Split<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let first = self.first_size(area);
        let handle = u16::from(self.on_resize.is_some());
        let rest = self.extent(area).saturating_sub(first + handle);
        let offset = |cells: u16| i32::from(cells);
        let (first_rect, handle_rect, second_rect) = match self.axis {
            Axis::Columns => (
                Rect::new(area.x, area.y, first, area.height),
                Rect::new(area.x + offset(first), area.y, handle, area.height),
                Rect::new(area.x + offset(first + handle), area.y, rest, area.height),
            ),
            Axis::Rows => (
                Rect::new(area.x, area.y, area.width, first),
                Rect::new(area.x, area.y + offset(first), area.width, handle),
                Rect::new(area.x, area.y + offset(first + handle), area.width, rest),
            ),
        };
        cx.paint_child(&self.panes[0], first_rect);
        cx.paint_child(&self.panes[1], second_rect);
        if self.on_resize.is_some() {
            boundary::paint(cx, handle_rect, None);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.on_resize.is_none() {
            return false;
        }
        let area = cx.area();
        let current = i32::from(self.first_size(area));
        match boundary::event(cx, event, self.axis) {
            Change::Ignored => false,
            Change::Used | Change::Activate => true,
            Change::DragTo(at) => {
                let start = match self.axis {
                    Axis::Columns => area.x,
                    Axis::Rows => area.y,
                };
                self.resize(cx, at - start);
                true
            }
            Change::Nudge(cells) => {
                self.resize(cx, current + cells);
                true
            }
            Change::Jump(end) => {
                self.resize(cx, if end { i32::MAX } else { 0 });
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.on_resize.is_some()
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.panes
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.panes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{MouseButton, MouseKind};
    use crate::runtime::{App, Command, Harness};
    use crate::theme::State;
    use crate::widgets::Text;

    struct Demo {
        size: u16,
        rows: bool,
        resizable: bool,
        max: Option<u16>,
    }

    impl App for Demo {
        type Msg = u16;
        fn update(&mut self, size: u16) -> Command<u16> {
            self.size = size;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, u16>) {
            let split = if self.rows { Splitter::rows(self.size) } else { Splitter::columns(self.size) };
            let split = if self.resizable { split.on_resize(|size| size) } else { split };
            split
                .limits(3, self.max)
                .first(|ui| {
                    ui.add(Text::new("left"));
                })
                .second(|ui| {
                    ui.add(Text::new("right"));
                })
                .show(ui)
                .id("split");
        }
    }

    #[test]
    fn plain_panes_sit_side_by_side_without_a_boundary() {
        let h = Harness::new(Demo { size: 6, rows: false, resizable: false, max: Some(12) }, 20, 2);
        assert_eq!(h.screen(), "left  right\n\n");
    }

    #[test]
    fn boundary_is_invisible_until_hovered_and_takes_the_accent_while_dragged() {
        let mut h = Harness::new(Demo { size: 6, rows: false, resizable: true, max: Some(12) }, 20, 3);
        assert_eq!(h.screen(), "left   right\n\n\n");
        let theme = h.env().theme();
        let canvas = theme.color("canvas");
        let hover = theme.style("split-handle", None, &[State::Hover]).paint("bg").map(|p| p.at(0.0));
        let active = theme.style("split-handle", None, &[State::Active]).paint("bg").map(|p| p.at(0.0));
        assert_eq!(h.bg(6, 1), canvas);
        h.hover(6, 1);
        assert_eq!(h.bg(6, 1), hover);
        assert_ne!(hover, canvas);
        h.mouse(MouseKind::Down(MouseButton::Left), 6, 1);
        h.mouse(MouseKind::Drag(MouseButton::Left), 9, 1);
        assert_eq!(h.app().size, 9);
        assert_eq!(h.bg(9, 0), active);
        h.mouse(MouseKind::Drag(MouseButton::Left), 30, 1);
        assert_eq!(h.app().size, 12, "limited by max");
        h.mouse(MouseKind::Up(MouseButton::Left), 30, 1);
        assert_eq!(h.screen(), "left         right\n\n\n");
    }

    #[test]
    fn keyboard_moves_the_focused_boundary() {
        let mut h = Harness::new(Demo { size: 6, rows: true, resizable: true, max: Some(12) }, 20, 20);
        h.press("tab");
        assert!(h.is_focused("split"));
        h.press("down");
        assert_eq!(h.app().size, 7);
        h.press("shift+up");
        assert_eq!(h.app().size, 3, "limited by min");
        h.press("end");
        assert_eq!(h.app().size, 12);
        assert_eq!(h.screen().lines().nth(13), Some("right"));
    }

    #[test]
    fn without_an_upper_limit_only_the_second_pane_stops_the_boundary() {
        let mut h = Harness::new(Demo { size: 6, rows: false, resizable: true, max: None }, 20, 3);
        h.mouse(MouseKind::Down(MouseButton::Left), 6, 1);
        h.mouse(MouseKind::Drag(MouseButton::Left), 30, 1);
        assert_eq!(h.app().size, 18, "20 columns: the handle and one cell of the second pane stay");
        h.mouse(MouseKind::Drag(MouseButton::Left), 0, 1);
        assert_eq!(h.app().size, 3, "the lower limit still holds");
        h.mouse(MouseKind::Up(MouseButton::Left), 0, 1);
        h.press("end");
        assert_eq!(h.app().size, 18);
        assert_eq!(h.screen(), "left               r\n                   i\n                   g\n");
        h.press("home");
        assert_eq!(h.app().size, 3);
    }

    #[test]
    fn a_max_below_min_counts_as_min() {
        let mut h = Harness::new(Demo { size: 6, rows: false, resizable: true, max: Some(2) }, 20, 1);
        assert_eq!(h.screen(), "lef right\n", "the first pane is held at the minimum of 3");
        h.press("tab").press("right");
        assert_eq!(h.app().size, 6, "already at the only allowed size: nothing to send");
    }

    #[test]
    fn narrow_area_keeps_the_second_pane() {
        let h = Harness::new(Demo { size: 12, rows: false, resizable: true, max: Some(12) }, 8, 1);
        assert_eq!(h.screen(), "left   r\n");
    }
}
