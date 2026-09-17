//! Side panels: a surface docked to one side of a body that opens, closes and resizes.

mod strip;
#[cfg(test)]
mod tests;

use std::rc::Rc;
use std::time::Duration;

use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::keymap::Scope;
use crate::motion::{Easing, steps};
use crate::widget::{Axis as FlexAxis, EventCx, Flex, Length, MeasureCx, Node, NodeMut, PaintCx, View, Widget};

use super::boundary::{self, Axis, Change, Toggle};
use strip::{OpenMessage, STRIP, Strip, ViewMessage};

type Part<'a, Msg> = Box<dyn FnOnce(&mut View<'_, Msg>) + 'a>;

/// Which side a [`SidePanel`] is docked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    /// The left edge.
    #[default]
    Left,
    /// The right edge.
    Right,
}

/// What closing a [`SidePanel`] leaves behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Closed {
    /// The panel folds into its icon strip, which stays at the edge; without a strip nothing but
    /// the edge column is left.
    #[default]
    Collapse,
    /// The panel and its icon strip both go; the body takes the whole width but the one edge
    /// column, which looks like the body until the pointer reaches it.
    Hide,
}

/// A panel docked to the left or right of a body, on its own surface.
///
/// The application owns whether the panel is open, how wide it is and which view it shows. The
/// plain panel is just a surface next to the body; capabilities are opt-in:
///
/// - [`SidePanel::on_toggle`] makes the edge between panel and body interactive. The edge is a
///   tone difference, never a line: on hover it brightens one step and a two-cell toggle appears
///   at its middle, a `‹` or `›` on a raised surface reaching one cell into the body. Pointing at
///   the toggle raises it a step more and shows the pillar `▌` in its first cell, pressing it
///   raises it one more; clicking toggles.
///   Tab reaches the edge and shows the same toggle, where Enter or Space toggles, and the global
///   action `toggle-panel` (`alt+b` by default) toggles from anywhere inside the panel or the
///   body. Opening and closing slide over twice `motion.enter`. Dragging a closed edge half the
///   minimum width into the body opens the panel.
/// - [`SidePanel::on_resize`] lets the edge be dragged, and moved with ←/→ while focused, within
///   [`SidePanel::limits`].
/// - [`SidePanel::strip`] adds an activity bar: a column of view icons at the outer edge, beside
///   the open panel and left behind when it collapses. [`SidePanel::active_view`] marks the shown
///   view's icon, raised with the pillar. Clicking another icon switches to its view (opening
///   the panel if it is closed); clicking the shown view's icon closes the panel. Tab reaches
///   the strip, where ↑/↓ move and Enter or Space act like a click.
/// - [`SidePanel::closed`] chooses what closing leaves: the strip ([`Closed::Collapse`], the
///   default) or only an invisible edge column ([`Closed::Hide`]).
///
/// Style keys: `side-panel` (`bg`), `side-strip` (`bg`), `side-strip-item` (`fg`, `bg`, `pillar`)
/// with `hover`, `selected`, `focus` and `pressed`, `split-handle` for the edge and `side-toggle`
/// (`bg`, `fg`, `pillar`) with `hover`, `focus` and `pressed` for the toggle. Icons: `edge-left`,
/// `edge-right`.
pub struct SidePanel<'a, Msg> {
    side: Side,
    width: u16,
    open: bool,
    closed: Closed,
    min: u16,
    max: Option<u16>,
    on_toggle: Option<OpenMessage<Msg>>,
    on_resize: Option<Box<dyn Fn(u16) -> Msg>>,
    strip: Vec<String>,
    on_strip: Option<ViewMessage<Msg>>,
    active_view: Option<u16>,
    panel: Option<Part<'a, Msg>>,
    body: Option<Part<'a, Msg>>,
}

impl<'a, Msg: 'static> SidePanel<'a, Msg> {
    /// An open panel `width` columns wide, docked left.
    #[must_use]
    pub fn new(width: u16) -> Self {
        Self {
            side: Side::Left,
            width,
            open: true,
            closed: Closed::Collapse,
            min: 8,
            max: None,
            on_toggle: None,
            on_resize: None,
            strip: Vec::new(),
            on_strip: None,
            active_view: None,
            panel: None,
            body: None,
        }
    }

    /// Docks the panel to `side`.
    #[must_use]
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Whether the panel is open; `true` by default.
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// What closing leaves behind: [`Closed::Collapse`] (the default) keeps the icon strip,
    /// [`Closed::Hide`] hides the strip too and leaves only the edge column.
    #[must_use]
    pub fn closed(mut self, closed: Closed) -> Self {
        self.closed = closed;
        self
    }

    /// Makes the edge toggle the panel; the message carries the new open state.
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Rc::new(message));
        self
    }

    /// Makes the edge resize the panel; the message carries the new width within the limits.
    #[must_use]
    pub fn on_resize(mut self, message: impl Fn(u16) -> Msg + 'static) -> Self {
        self.on_resize = Some(Box::new(message));
        self
    }

    /// The narrowest and widest the panel can be, in columns: `limits(18, 48)` for a range, or
    /// `limits(18, None)` for no upper limit. By default 8 and no upper limit; whatever the
    /// limits, the body keeps at least a quarter of the area. A `max` below `min` counts as `min`.
    #[must_use]
    pub fn limits(mut self, min: u16, max: impl Into<Option<u16>>) -> Self {
        self.min = min;
        self.max = max.into();
        self
    }

    /// Adds an activity bar of these view icons at the outer edge. Clicking an icon sends its
    /// index, meaning "show this view": set the view and open the panel in `update`. The icon of
    /// the [`active_view`](Self::active_view) instead closes the open panel through
    /// [`on_toggle`](Self::on_toggle).
    #[must_use]
    pub fn strip(
        mut self,
        icons: impl IntoIterator<Item = impl Into<String>>,
        message: impl Fn(u16) -> Msg + 'static,
    ) -> Self {
        self.strip = icons.into_iter().map(Into::into).collect();
        self.on_strip = Some(Rc::new(message));
        self
    }

    /// The index of the strip icon whose view the panel shows; it is raised with the pillar
    /// while the panel is open.
    #[must_use]
    pub fn active_view(mut self, index: u16) -> Self {
        self.active_view = Some(index);
        self
    }

    /// The panel's content.
    #[must_use]
    pub fn panel(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.panel = Some(Box::new(build));
        self
    }

    /// The body beside the panel.
    #[must_use]
    pub fn body(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.body = Some(Box::new(build));
        self
    }

    /// Adds the panel and its body to `ui`, filling the space they get.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let env = ui.env();
        let build = |part: Option<Part<'a, Msg>>| {
            let mut children = Vec::new();
            if let Some(part) = part {
                part(&mut View::new(&mut children, env));
            }
            let mut node = Node::new(Flex::new(FlexAxis::Column, children), 0);
            node.layout.width = Length::Fill(1);
            node.layout.height = Length::Fill(1);
            node
        };
        let mut parts = vec![build(self.panel), build(self.body)];
        let has_strip = match self.on_strip {
            Some(on_select) if !self.strip.is_empty() => {
                let strip = Strip {
                    side: self.side,
                    icons: self.strip,
                    active: self.active_view,
                    open: self.open,
                    on_select,
                    on_toggle: self.on_toggle.clone(),
                };
                parts.push(Node::new(strip, 2));
                true
            }
            _ => false,
        };
        let dock = Dock {
            side: self.side,
            width: self.width,
            open: self.open,
            closed: self.closed,
            min: self.min,
            max: self.max,
            on_toggle: self.on_toggle,
            on_resize: self.on_resize,
            has_strip,
            parts,
        };
        ui.add(dock).fill()
    }
}

/// Index of the panel content, the body and the strip in [`Dock::parts`].
const PANEL: usize = 0;
const BODY: usize = 1;
const STRIP_PART: usize = 2;

struct Dock<Msg> {
    side: Side,
    width: u16,
    open: bool,
    closed: Closed,
    min: u16,
    /// The widest the application allows the panel to be; `None` for no limit.
    max: Option<u16>,
    on_toggle: Option<OpenMessage<Msg>>,
    on_resize: Option<Box<dyn Fn(u16) -> Msg>>,
    /// Whether `parts` holds a strip after the panel and the body.
    has_strip: bool,
    parts: Vec<Node<Msg>>,
}

impl<Msg: 'static> Dock<Msg> {
    fn interactive(&self) -> bool {
        self.on_toggle.is_some() || self.on_resize.is_some()
    }

    /// Columns the strip takes beside the open panel.
    fn strip_width(&self) -> u16 {
        if self.has_strip { STRIP } else { 0 }
    }

    /// Columns left at the edge when the panel is closed, not counting the edge column the
    /// body gives up when nothing is left.
    fn collapsed_width(&self) -> u16 {
        match self.closed {
            Closed::Collapse => self.strip_width(),
            Closed::Hide => 0,
        }
    }

    /// Room for the panel in `area`: the body keeps a quarter of it, and the strip its columns.
    fn room(&self, area: Rect) -> u16 {
        area.width.saturating_sub(area.width / 4).saturating_sub(self.strip_width())
    }

    /// The widest the panel can be in `area`: its limit, within the room.
    fn max_width(&self, area: Rect) -> u16 {
        let room = self.room(area);
        self.max.map_or(room, |max| max.min(room))
    }

    /// The width the panel has in `area` when open: the application's width within the limits,
    /// cut to the room even when that is below the minimum.
    fn open_width(&self, area: Rect) -> u16 {
        boundary::clamp_size(i32::from(self.width), self.min, self.max_width(area)).min(self.room(area))
    }

    /// Screen rectangle of a `width`-column strip on the docked side.
    fn edge_rect(&self, area: Rect, width: u16) -> Rect {
        match self.side {
            Side::Left => Rect::new(area.x, area.y, width, area.height),
            Side::Right => Rect::new(area.right() - i32::from(width), area.y, width, area.height),
        }
    }

    /// The boundary column of a docked area `width` columns wide: its inner edge, or the body's
    /// edge when nothing is docked.
    fn handle_rect(&self, area: Rect, width: u16) -> Rect {
        let panel = self.edge_rect(area, width.max(1));
        match self.side {
            Side::Left => Rect::new(panel.right() - 1, area.y, 1, area.height),
            Side::Right => Rect::new(panel.x, area.y, 1, area.height),
        }
    }

    fn toggle(&self, cx: &mut EventCx<'_, Msg>) {
        if let Some(message) = &self.on_toggle {
            cx.emit(message(!self.open));
        }
    }

    fn resize(&self, cx: &mut EventCx<'_, Msg>, target: i32) {
        let area = cx.area();
        let width = boundary::clamp_size(target, self.min, self.max_width(area));
        if let Some(message) = &self.on_resize
            && self.open
            && width != self.open_width(area)
        {
            cx.emit(message(width));
        }
    }

    /// A drag on the closed edge, with the pointer at column `x` asking for a `target`-column
    /// panel. Once the pointer is half the minimum width past the closed edge the panel opens,
    /// at the dragged width when it can be resized; nearer, the drag is only held.
    fn drag_open(&self, cx: &mut EventCx<'_, Msg>, x: i32, target: i32) {
        let Some(toggle) = &self.on_toggle else { return };
        let area = cx.area();
        let edge = self.handle_rect(area, self.collapsed_width().min(area.width)).x;
        let past_edge = match self.side {
            Side::Left => x - edge,
            Side::Right => edge - x,
        };
        if past_edge < i32::from((self.min / 2).max(1)) {
            return;
        }
        cx.emit(toggle(true));
        if let Some(message) = &self.on_resize {
            let width = boundary::clamp_size(target, self.min, self.max_width(area));
            if width != self.open_width(area) {
                cx.emit(message(width));
            }
        }
    }

    /// Paints what moves while the panel opens and closes: the panel, and the strip too when it
    /// hides with it. It is laid out at its open width with its inner edge on the visible inner
    /// edge, and shows only its visible part, so its content never reflows while it slides.
    fn paint_sliding(&self, cx: &mut PaintCx<'_>, area: Rect, width: u16, open_width: u16) {
        let collapsed = self.collapsed_width();
        let visible = self.edge_rect(area, width);
        let sliding = width.saturating_sub(collapsed);
        let strip = self.strip_width().saturating_sub(collapsed);
        let layer_width = strip + open_width;
        let (moving, layer_x) = match self.side {
            Side::Left => (
                Rect::new(visible.x + i32::from(collapsed), area.y, sliding, area.height),
                visible.right() - i32::from(layer_width),
            ),
            Side::Right => (Rect::new(visible.x, area.y, sliding, area.height), visible.x),
        };
        let (strip_x, panel_x) = match self.side {
            Side::Left => (layer_x, layer_x + i32::from(strip)),
            Side::Right => (layer_x + i32::from(open_width), layer_x),
        };
        let panel = Rect::new(panel_x, area.y, open_width, area.height);
        let background = cx.style("side-panel", None, &[]).text().bg.unwrap_or_else(|| cx.color("surface"));
        // The panel's inner column is the edge, painted by the boundary.
        let handle = u16::from(self.interactive());
        let (content, inside) = match self.side {
            Side::Left => (
                Rect::new(panel.x, area.y, open_width.saturating_sub(handle), area.height),
                Rect::new(moving.x, area.y, sliding.saturating_sub(handle), area.height),
            ),
            Side::Right => (
                Rect::new(panel.x + i32::from(handle), area.y, open_width.saturating_sub(handle), area.height),
                Rect::new(moving.x + i32::from(handle), area.y, sliding.saturating_sub(handle), area.height),
            ),
        };
        cx.with_clip(moving, |cx| cx.clear(panel, background));
        cx.with_clip(inside, |cx| {
            cx.paint_child(&self.parts[PANEL], content);
            if strip > 0 {
                cx.paint_child(&self.parts[STRIP_PART], Rect::new(strip_x, area.y, strip, area.height));
            }
        });
    }
}

impl<Msg: 'static> Widget<Msg> for Dock<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let open_width = self.open_width(area);
        let full = (self.strip_width() + open_width).min(area.width);
        let collapsed = self.collapsed_width().min(full);
        let duration = cx.env().theme().motion().enter * 2;
        let progress = cx.animate("open", if self.open { 1.0 } else { 0.0 }, duration, Easing::EaseOut);
        let width = collapsed + steps(progress, full - collapsed);
        // A handle over the body's edge column when nothing is docked.
        let gutter = u16::from(self.interactive() && width == 0);

        let body_rect = match self.side {
            Side::Left => Rect::new(
                area.x + i32::from(width + gutter),
                area.y,
                area.width.saturating_sub(width + gutter),
                area.height,
            ),
            Side::Right => Rect::new(area.x, area.y, area.width.saturating_sub(width + gutter), area.height),
        };
        // The strip of a collapsing panel stays put at the outer edge.
        if collapsed > 0 {
            cx.paint_child(&self.parts[STRIP_PART], self.edge_rect(area, collapsed));
        }
        if width > collapsed {
            self.paint_sliding(cx, area, width, open_width);
        } else if self.has_strip
            && collapsed == 0
            && self.interactive()
            && cx.focused() == Some(self.parts[STRIP_PART].id())
        {
            // A hidden strip cannot keep focus: the edge, which reopens it, takes over.
            cx.request_focus(cx.id());
            cx.request_frame_in(Duration::ZERO);
        }
        // Painted after the strip and the panel, so Tab goes from the edge to the strip, the
        // panel's content and then the body.
        cx.paint_child(&self.parts[BODY], body_rect);
        if self.interactive() {
            let arrow = match (self.side, self.open) {
                (Side::Left, true) | (Side::Right, false) => "edge-left",
                (Side::Left, false) | (Side::Right, true) => "edge-right",
            };
            // The toggle reaches into the body, never over the panel's own content.
            let toggle = self.on_toggle.as_ref().map(|_| Toggle { arrow, reach_after: self.side == Side::Left });
            boundary::paint(cx, self.handle_rect(area, width), toggle);
        }
        if progress > 0.0 && progress < 1.0 {
            cx.request_frame_in(Duration::from_millis(16));
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if let Event::Key(key) = event
            && self.on_toggle.is_some()
            && cx.env().keymap().chords_for(Scope::Global, "toggle-panel").contains(&key.chord)
        {
            self.toggle(cx);
            return true;
        }
        if !self.interactive() {
            return false;
        }
        let area = cx.area();
        let current = i32::from(self.open_width(area));
        let outward = match self.side {
            Side::Left => 1,
            Side::Right => -1,
        };
        match boundary::event(cx, event, Axis::Columns) {
            Change::Ignored => false,
            Change::Used => true,
            Change::Activate => {
                self.toggle(cx);
                true
            }
            Change::DragTo(x) => {
                // The panel's width with its edge under the pointer, beside the strip.
                let target = match self.side {
                    Side::Left => x - area.x + 1,
                    Side::Right => area.right() - x,
                } - i32::from(self.strip_width());
                if self.open {
                    self.resize(cx, target);
                } else {
                    self.drag_open(cx, x, target);
                }
                true
            }
            Change::Nudge(cells) => {
                self.resize(cx, current + cells * outward);
                true
            }
            Change::Jump(end) => {
                let wide = (end && outward > 0) || (!end && outward < 0);
                self.resize(cx, if wide { i32::MAX } else { 0 });
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.interactive()
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.parts
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.parts
    }
}
