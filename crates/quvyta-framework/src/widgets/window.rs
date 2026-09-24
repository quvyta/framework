//! Windows: a surface placed freely in a stack, with a one-row title strip, that tells the
//! application how the pointer moves, resizes, minimizes, maximizes and closes it.

use crate::color::{ColorDepth, Rgb};
use crate::event::{Event, MouseButton, MouseEvent, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::Glyph;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{Axis, Container, EventCx, Flex, Length, MeasureCx, Node, PaintCx, Widget};

use super::{click, close_mark};

/// Cells the marks take at the right end of the title row: minimize, maximize and close.
const MARKS: u16 = close_mark::WIDTH * 3;

/// Cells between the name and the subtitle.
const TITLE_GAP: u16 = 2;

/// The fewest cells a shortened subtitle keeps; below that it is left out, since an ellipsis and
/// two letters say nothing.
const MIN_SUBTITLE: u16 = 4;

/// How far the ground under a shadow is darkened, in percent, when the theme does not say.
const DEFAULT_SHADOW: u16 = 45;

/// What the pointer did to a [`Window`], sent through [`Window::on_event`].
///
/// Deltas count cells since the last message of the same drag, so an application adds them to
/// the window's rectangle as they come. What it allows (a smallest size, staying on screen) is
/// its own decision; the window reports the pointer, not a new rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    /// A press anywhere in a window that is not [focused](Window::focused), sent before whatever
    /// else the press does: raise the window and give it focus.
    Focus,
    /// The window was dragged by its title, or with alt and the left button anywhere, by `dx`
    /// columns and `dy` rows.
    Move {
        /// Columns to the right; negative is to the left.
        dx: i32,
        /// Rows down; negative is up.
        dy: i32,
    },
    /// An edge or a corner was dragged: the right column or bottom row of the body, their corner,
    /// or with alt and the right button the edge or corner nearest to the press.
    Resize {
        /// The edge or corner that moves.
        edge: WindowEdge,
        /// Columns the edge's side moves to the right; 0 for the top and bottom edges.
        dx: i32,
        /// Rows the edge's side moves down; 0 for the left and right edges.
        dy: i32,
    },
    /// The minimize mark was clicked.
    Minimize,
    /// The maximize mark was clicked, or the title double-clicked.
    ToggleMaximize,
    /// The close mark was clicked.
    Close,
    /// A move or a resize ended: the button came up after at least one [`WindowEvent::Move`] or
    /// [`WindowEvent::Resize`]. This is where snapping to an edge and a ghost drag land, and
    /// where a size is saved.
    Dropped,
}

/// An edge or a corner of a window being resized, see [`WindowEvent::Resize`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEdge {
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The top left corner.
    TopLeft,
    /// The top right corner.
    TopRight,
    /// The bottom left corner.
    BottomLeft,
    /// The bottom right corner.
    BottomRight,
}

impl WindowEdge {
    /// Whether the left side moves: the left edge and the corners beside it. The window's `x`
    /// then moves by `dx` and its width by `-dx`.
    #[must_use]
    pub fn left(self) -> bool {
        matches!(self, Self::Left | Self::TopLeft | Self::BottomLeft)
    }

    /// Whether the right side moves; the width then changes by `dx`.
    #[must_use]
    pub fn right(self) -> bool {
        matches!(self, Self::Right | Self::TopRight | Self::BottomRight)
    }

    /// Whether the top side moves. The window's `y` then moves by `dy` and its height by `-dy`.
    #[must_use]
    pub fn top(self) -> bool {
        matches!(self, Self::Top | Self::TopLeft | Self::TopRight)
    }

    /// Whether the bottom side moves; the height then changes by `dy`.
    #[must_use]
    pub fn bottom(self) -> bool {
        matches!(self, Self::Bottom | Self::BottomLeft | Self::BottomRight)
    }
}

/// Builds a message from what the pointer did to the window.
type EventMessage<Msg> = Box<dyn Fn(WindowEvent) -> Msg>;

/// A window: a title strip one row tall above a body, with no border lines, for applications
/// that put surfaces where the user drags them, such as a desktop or a tool box. Place it with
/// [`View::place`](crate::widget::View::place) inside a stack; the body is built with
/// [`View::add_with`](crate::widget::View::add_with).
///
/// The title strip shows the icon and the name, then a faint subtitle (a program's own title, a
/// folder). A window that is [`focused`](Self::focused) is one tone raised, its name bright, and
/// the pillar `▌` runs down its whole left edge; others sit one tone lower with a quieter name.
/// On a narrow window the subtitle shortens first, then the name, each with `…`. The body keeps
/// the pillar column and one cell after it on the left, and the right column and the bottom row
/// free for the handles.
///
/// With no options the window is only a surface. [`on_event`](Self::on_event) makes it one the
/// pointer moves: the three marks at the right end of the title (minimize, maximize or restore,
/// close) light up together under the pointer like every close mark; dragging the title moves
/// the window and double-clicking it maximizes; the body's right column, bottom row and their
/// corner are handles that brighten under the pointer and take the accent while dragged, like a
/// splitter's boundary; alt with the left button drags the window from anywhere, alt with the
/// right button resizes it from the nearest edge or corner (the left and top edges too). A drag
/// belongs to the window until the button comes up, which arrives as [`WindowEvent::Dropped`],
/// wherever the pointer goes. The title, the
/// marks, the handles and alt drags are the window's even when the body holds a
/// [`Terminal`](super::Terminal) whose program reads the mouse; other presses in the body reach
/// the body. The window reports all of it through [`WindowEvent`]s and changes nothing itself:
/// stacking order, focus, size limits and snapping are the application's.
///
/// [`shadow`](Self::shadow) darkens one column right of the window and one row below it. It is
/// not drawn with reduced motion or in 16 colours.
///
/// Style keys: `window` (`bg`, `pillar`), `window-title` (`bg`, `fg`, `bold`),
/// `window-subtitle` (`fg`), all with `focus` for a focused window; `close-mark` for the marks
/// (`active` while focused, `hover`); `split-handle` for the handles (`hover`, `active` while
/// dragged); `window-shadow` (`scrim`, `strength` in percent).
pub struct Window<Msg> {
    title: String,
    subtitle: Option<String>,
    icon: Option<Glyph>,
    focused: bool,
    maximized: bool,
    shadow: bool,
    on_event: Option<EventMessage<Msg>>,
    /// The body: one column holding what [`View::add_with`](crate::widget::View::add_with) built.
    body: Vec<Node<Msg>>,
}

impl<Msg: 'static> Window<Msg> {
    /// A window named `title`, unfocused, with an empty body.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            icon: None,
            focused: false,
            maximized: false,
            shadow: false,
            on_event: None,
            body: vec![body(Vec::new())],
        }
    }

    /// A faint second title after the name, such as the title a program set or its folder.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// The glyph before the name: an icon key such as `"folder"`, or a [`Glyph::literal`].
    #[must_use]
    pub fn icon(mut self, glyph: impl Into<Glyph>) -> Self {
        self.icon = Some(glyph.into());
        self
    }

    /// Whether this is the window the user works in: raised one tone, bright name and the pillar
    /// down its left edge. A press on a window that is not focused sends [`WindowEvent::Focus`].
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Whether the window fills its desktop; the maximize mark then offers to restore it.
    #[must_use]
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Darkens one column right of the window and one row below it, as if it floated; drawn in
    /// the cell a placed child may reach past its rectangle. Not drawn with reduced motion or in
    /// 16 colours.
    #[must_use]
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// Makes the window one the pointer moves, resizes and closes, and shows its marks; `message`
    /// turns each [`WindowEvent`] into the application's message.
    #[must_use]
    pub fn on_event(mut self, message: impl Fn(WindowEvent) -> Msg + 'static) -> Self {
        self.on_event = Some(Box::new(message));
        self
    }
}

fn body<Msg: 'static>(children: Vec<Node<Msg>>) -> Node<Msg> {
    let mut column = Node::new(Flex::new(Axis::Column, children), 0);
    column.layout.width = Length::Fill(1);
    column.layout.height = Length::Fill(1);
    column
}

impl<Msg: 'static> Container<Msg> for Window<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        self.body[0] = body(children);
    }
}

/// A mark in the title row, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Minimize,
    Maximize,
    Close,
}

impl Mark {
    const ALL: [Self; 3] = [Self::Minimize, Self::Maximize, Self::Close];

    fn event(self) -> WindowEvent {
        match self {
            Self::Minimize => WindowEvent::Minimize,
            Self::Maximize => WindowEvent::ToggleMaximize,
            Self::Close => WindowEvent::Close,
        }
    }
}

/// The part of a window a cell belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Title,
    Mark(Mark),
    Handle(WindowEdge),
    Body,
}

/// What a held button is doing to the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grab {
    /// Moving it; the pointer was last at `last`.
    Move { button: MouseButton, last: (i32, i32) },
    /// Resizing it by `edge`.
    Resize { button: MouseButton, edge: WindowEdge, last: (i32, i32) },
    /// Held on a mark, which acts when released over it.
    Mark(Mark),
}

#[derive(Debug, Default)]
struct WindowMemory {
    grab: Option<Grab>,
    /// Whether the held grab has moved the window or an edge, so the release is a drop.
    moved: bool,
    /// When the title was last pressed without being dragged, to tell a double click.
    title_press: Option<std::time::Duration>,
}

impl<Msg: 'static> Window<Msg> {
    fn interactive(&self) -> bool {
        self.on_event.is_some()
    }

    /// Where the body's content goes: after the pillar column and a cell, above the bottom row
    /// and left of the right column.
    fn content(area: Rect) -> Rect {
        Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(3), area.height.saturating_sub(2))
    }

    /// The part of the window at `(x, y)`, or `None` outside it.
    fn part_at(&self, area: Rect, x: i32, y: i32) -> Option<Part> {
        if !area.contains(x, y) {
            return None;
        }
        let interactive = self.interactive();
        if y == area.y {
            let marks = area.right() - i32::from(MARKS);
            if interactive && x >= marks {
                let index = usize::try_from((x - marks) / i32::from(close_mark::WIDTH)).unwrap_or(0);
                return Some(Part::Mark(Mark::ALL[index.min(2)]));
            }
            return Some(Part::Title);
        }
        let (right, bottom) = (x == area.right() - 1, y == area.bottom() - 1);
        Some(match (interactive, right, bottom) {
            (true, true, true) => Part::Handle(WindowEdge::BottomRight),
            (true, true, false) => Part::Handle(WindowEdge::Right),
            (true, false, true) if x > area.x => Part::Handle(WindowEdge::Bottom),
            _ => Part::Body,
        })
    }

    /// The edge or corner nearest to `(x, y)`: the corners and edges each take a third of the
    /// window, and in the middle third the closest side wins, counting a row as two columns
    /// since a cell is about twice as tall as it is wide.
    fn nearest_edge(area: Rect, x: i32, y: i32) -> WindowEdge {
        let third = |offset: i32, length: u16| (offset * 3 / i32::from(length.max(1))).clamp(0, 2);
        match (third(x - area.x, area.width), third(y - area.y, area.height)) {
            (0, 0) => WindowEdge::TopLeft,
            (1, 0) => WindowEdge::Top,
            (2, 0) => WindowEdge::TopRight,
            (0, 1) => WindowEdge::Left,
            (2, 1) => WindowEdge::Right,
            (0, 2) => WindowEdge::BottomLeft,
            (1, 2) => WindowEdge::Bottom,
            (2, 2) => WindowEdge::BottomRight,
            _ => {
                let sides = [
                    (x - area.x, WindowEdge::Left),
                    (area.right() - 1 - x, WindowEdge::Right),
                    (2 * (y - area.y), WindowEdge::Top),
                    (2 * (area.bottom() - 1 - y), WindowEdge::Bottom),
                ];
                sides.into_iter().min_by_key(|(distance, _)| *distance).map_or(WindowEdge::Right, |(_, edge)| edge)
            }
        }
    }

    fn send(&self, cx: &mut EventCx<'_, Msg>, event: WindowEvent) {
        if let Some(message) = &self.on_event {
            cx.emit(message(event));
        }
    }

    fn press(&self, cx: &mut EventCx<'_, Msg>, mouse: MouseEvent, button: MouseButton) -> bool {
        let area = cx.area();
        let Some(part) = self.part_at(area, mouse.x, mouse.y) else {
            return false;
        };
        if !self.focused {
            self.send(cx, WindowEvent::Focus);
        }
        let at = (mouse.x, mouse.y);
        let now = cx.now();
        let memory = cx.memory::<WindowMemory>();
        let grab = match (mouse.mods.alt, button, part) {
            (true, MouseButton::Left, _) => Grab::Move { button, last: at },
            (true, MouseButton::Right, _) => {
                Grab::Resize { button, edge: Self::nearest_edge(area, mouse.x, mouse.y), last: at }
            }
            // Other buttons on the window's own parts do nothing yet, but they are the window's.
            (_, MouseButton::Left, Part::Body) | (_, MouseButton::Right | MouseButton::Middle, _) => {
                return part != Part::Body;
            }
            (_, MouseButton::Left, Part::Mark(mark)) => Grab::Mark(mark),
            (_, MouseButton::Left, Part::Handle(edge)) => Grab::Resize { button, edge, last: at },
            (_, MouseButton::Left, Part::Title) => {
                if memory.title_press.is_some_and(|last| click::is_double(last, now)) {
                    memory.title_press = None;
                    memory.grab = None;
                    cx.capture_pointer();
                    self.send(cx, WindowEvent::ToggleMaximize);
                    return true;
                }
                memory.title_press = Some(now);
                Grab::Move { button, last: at }
            }
        };
        if !matches!(grab, Grab::Move { .. }) || part != Part::Title {
            memory.title_press = None;
        }
        memory.grab = Some(grab);
        memory.moved = false;
        cx.capture_pointer();
        true
    }

    fn drag(&self, cx: &mut EventCx<'_, Msg>, mouse: MouseEvent, button: MouseButton) -> bool {
        let at = (mouse.x, mouse.y);
        let memory = cx.memory::<WindowMemory>();
        let event = match memory.grab {
            Some(Grab::Move { button: held, last }) if held == button => {
                memory.grab = Some(Grab::Move { button, last: at });
                let (dx, dy) = (at.0 - last.0, at.1 - last.1);
                if (dx, dy) != (0, 0) {
                    memory.title_press = None;
                }
                ((dx, dy) != (0, 0)).then_some(WindowEvent::Move { dx, dy })
            }
            Some(Grab::Resize { button: held, edge, last }) if held == button => {
                memory.grab = Some(Grab::Resize { button, edge, last: at });
                let dx = if edge.left() || edge.right() { at.0 - last.0 } else { 0 };
                let dy = if edge.top() || edge.bottom() { at.1 - last.1 } else { 0 };
                ((dx, dy) != (0, 0)).then_some(WindowEvent::Resize { edge, dx, dy })
            }
            Some(Grab::Mark(_)) => None,
            _ => return false,
        };
        if let Some(event) = event {
            cx.memory::<WindowMemory>().moved = true;
            self.send(cx, event);
        }
        true
    }

    fn release(&self, cx: &mut EventCx<'_, Msg>, mouse: MouseEvent) -> bool {
        let area = cx.area();
        let memory = cx.memory::<WindowMemory>();
        let (Some(grab), moved) = (memory.grab.take(), memory.moved) else {
            return false;
        };
        memory.moved = false;
        match grab {
            Grab::Mark(mark) if self.part_at(area, mouse.x, mouse.y) == Some(Part::Mark(mark)) => {
                self.send(cx, mark.event());
            }
            Grab::Move { .. } | Grab::Resize { .. } if moved => self.send(cx, WindowEvent::Dropped),
            _ => {}
        }
        true
    }

    /// Darkens the column right of `area` and the row below it.
    fn paint_shadow(cx: &mut PaintCx<'_>, area: Rect) {
        let depth = cx.env().depth();
        if depth == ColorDepth::Ansi16 || cx.reduced_motion() {
            return;
        }
        let style = cx.style("window-shadow", None, &[]);
        let scrim = style.color("scrim").unwrap_or_else(|| cx.color("canvas"));
        let strength = f32::from(style.cells("strength").unwrap_or(DEFAULT_SHADOW).min(100)) / 100.0;
        let rects = [
            Rect::new(area.right(), area.y + 1, 1, area.height.saturating_sub(1)),
            Rect::new(area.x + 1, area.bottom(), area.width, 1),
        ];
        for rect in rects {
            if depth == ColorDepth::TrueColor {
                cx.tint(rect, scrim, strength);
            } else {
                // Palette cells cannot be blended; the shadow darkens the canvas instead.
                let ground = cx.color("canvas").mix(scrim, strength);
                cx.fill(rect, ground);
            }
        }
    }

    /// The icon, name and subtitle that fit in `room` cells: the subtitle shortens first, then it
    /// goes, then the name shortens.
    fn fit_title(&self, icon: Option<&str>, room: u16) -> (Option<String>, String, Option<String>) {
        let lead = icon.map_or(0, |glyph| text::width(glyph).saturating_add(1));
        let name = text::width(&self.title);
        let icon = icon.filter(|glyph| text::width(glyph) <= room).map(str::to_owned);
        let before = lead.saturating_add(name).saturating_add(TITLE_GAP);
        if let Some(subtitle) = self.subtitle.as_deref().filter(|subtitle| !subtitle.is_empty()) {
            let left = room.saturating_sub(before);
            if before <= room && left >= MIN_SUBTITLE.min(text::width(subtitle)) {
                return (icon, self.title.clone(), Some(text::truncate(subtitle, left).into_owned()));
            }
        }
        (icon, text::truncate(&self.title, room.saturating_sub(lead)).into_owned(), None)
    }

    /// The title strip's colour and the styles of the name and the subtitle. In 16 colours the
    /// surface tones share two greys, too few to tell the focused window's strip from the others,
    /// so the strip takes the accent on the focused window and a grey on the others, with dark
    /// text on both.
    fn title_look(&self, cx: &mut PaintCx<'_>, states: &[State], ground: Rgb) -> (Rgb, CellStyle, CellStyle) {
        let name = cx.style("window-title", None, states).text();
        let subtitle = cx.style("window-subtitle", None, states).text();
        if cx.env().depth() != ColorDepth::Ansi16 {
            return (name.bg.unwrap_or(ground), CellStyle { bg: None, ..name }, CellStyle { bg: None, ..subtitle });
        }
        let strip = cx.color(if self.focused { "accent" } else { "muted" });
        let ink = Some(cx.color("ink"));
        (strip, CellStyle { bg: None, fg: ink, ..name }, CellStyle { bg: None, fg: ink, ..subtitle })
    }

    fn paint_title(&self, cx: &mut PaintCx<'_>, area: Rect, style: CellStyle, subtitle_style: CellStyle) {
        let marks = if self.interactive() { MARKS } else { 0 };
        let start = area.x + 1;
        let room = clamp_u16(i32::from(area.width) - 2 - i32::from(marks));
        let icon = self.icon.as_ref().map(|icon| icon.resolve(cx.env().icons()).into_owned());
        let (icon, name, subtitle) = self.fit_title(icon.as_deref(), room);
        let text_style = style;
        let mut x = start;
        if let Some(icon) = icon {
            let width = cx.text(x, area.y, &icon, text_style, room);
            x += i32::from(width) + 1;
        }
        let limit = clamp_u16(i32::from(room) - (x - start));
        let width = cx.text(x, area.y, &name, text_style, limit);
        x += i32::from(width) + i32::from(TITLE_GAP);
        if let Some(subtitle) = subtitle {
            let limit = clamp_u16(i32::from(room) - (x - start));
            cx.text(x, area.y, &subtitle, subtitle_style, limit);
        }
        if self.interactive() {
            let restore = if self.maximized { "window-restore" } else { "window-maximize" };
            let marks_x = area.right() - i32::from(MARKS);
            for (index, key) in ["window-minimize", restore, "close"].into_iter().enumerate() {
                let offset = i32::try_from(index).unwrap_or(0) * i32::from(close_mark::WIDTH);
                close_mark::paint_glyph(cx, marks_x + offset, area.y, self.focused, key);
            }
        }
    }

    /// Lights the right column and the bottom row under the pointer or while dragged.
    fn paint_handles(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.height < 2 || area.width < 2 {
            return;
        }
        let hovered = cx.pointer().and_then(|(x, y)| match self.part_at(area, x, y) {
            Some(Part::Handle(edge)) => Some(edge),
            _ => None,
        });
        let dragged = match cx.memory::<WindowMemory>().grab {
            Some(Grab::Resize { edge, .. }) => Some(edge),
            _ => None,
        };
        let handles = [
            (Rect::new(area.right() - 1, area.y + 1, 1, area.height - 1), WindowEdge::right as fn(WindowEdge) -> bool),
            (Rect::new(area.x + 1, area.bottom() - 1, area.width - 1, 1), WindowEdge::bottom),
        ];
        for (rect, moves) in handles {
            let state = if dragged.is_some_and(moves) {
                State::Active
            } else if hovered.is_some_and(moves) {
                State::Hover
            } else {
                continue;
            };
            let style = cx.style("split-handle", None, &[state]).text();
            if let Some(bg) = style.bg {
                cx.fill(rect, bg);
            }
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Window<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let states: &[State] = if self.focused { &[State::Focus] } else { &[] };
        if self.shadow {
            Self::paint_shadow(cx, area);
        }
        cx.register_hit(area);
        if self.interactive() {
            cx.preview_presses();
        }
        let surface = cx.style("window", None, states);
        let ground = surface.text().bg.unwrap_or_else(|| cx.color("surface"));
        let (strip, name, subtitle) = self.title_look(cx, states, ground);
        cx.clear(area, ground);
        cx.clear(area.row(0), strip);
        if let Some(pillar) = surface.color("pillar").filter(|_| self.focused) {
            for y in area.y..area.bottom() {
                cx.pillar(area.x, y, pillar);
            }
        }
        self.paint_title(cx, area, name, subtitle);
        cx.paint_child(&self.body[0], Self::content(area));
        if self.interactive() {
            self.paint_handles(cx, area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.interactive() {
            return false;
        }
        let Event::Mouse(mouse) = event else {
            return false;
        };
        match mouse.kind {
            // Presses are read before the body sees them; a press the body left bubbles here
            // afterwards and stays the body's.
            MouseKind::Down(button) if cx.is_preview() => self.press(cx, *mouse, button),
            MouseKind::Drag(button) => self.drag(cx, *mouse, button),
            MouseKind::Up(_) => self.release(cx, *mouse),
            _ => false,
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.body
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.body
    }
}
