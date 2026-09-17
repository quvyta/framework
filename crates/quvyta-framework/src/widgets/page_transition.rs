//! Animated changes between pages: a cross-fade, optionally with a short slide that follows the
//! navigation direction.

use std::time::Duration;

use ratatui_core::buffer::Cell;
use ratatui_core::style::Color;

use crate::color::Rgb;
use crate::geometry::{Rect, Size};
use crate::motion::{Easing, steps};
use crate::router::Navigation;
use crate::text;
use crate::widget::{Axis, Container, Flex, Length, MeasureCx, Node, PaintCx, Widget};

/// The most cells a sliding page travels. Motion whispers: the slide hints at direction, it does
/// not throw the page across the screen.
const MAX_SLIDE: u16 = 6;

/// Wraps the current page and animates whenever its key changes.
///
/// The outgoing screen is remembered cell by cell. When the key changes, each cell of the old
/// page blends into the matching cell of the new one over the theme's `motion.page`: the old text
/// fades into the background during the first half and the new text rises out of it during the
/// second, while backgrounds blend continuously. With [`PageTransition::slide`] the new page also
/// travels a few cells into place, stepping one cell at a time with its colours blended in
/// proportion, from the right when going forward and from the left when going back.
/// With reduced motion, or when the area changed size, the new page appears at once. The new
/// page is live from the first frame: clicks and keys already reach it.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::PageTransition;
///
/// # #[derive(Clone)] enum Msg {}
/// # fn body(router: &Router<String>, ui: &mut View<'_, Msg>) {
/// let page = router.current().clone();
/// ui.add_with(PageTransition::new(page.clone()).slide(true).direction(router.direction()), |ui| {
///     ui.page(page, |ui| {
///         ui.add(Text::new("Deploys"));
///     });
/// })
/// .fill();
/// # }
/// ```
pub struct PageTransition<Msg> {
    key: String,
    slide: bool,
    direction: Navigation,
    content: Vec<Node<Msg>>,
}

#[derive(Default)]
struct TransitionMemory {
    key: Option<String>,
    area: Rect,
    /// The cells shown in the last frame.
    shown: Grid,
    /// The cells of the outgoing page while a transition runs.
    from: Grid,
    started: Option<Duration>,
    direction: Navigation,
}

/// Cells copied from the part of the screen a transition can change. The two grids of a
/// transition trade places instead of being rebuilt, so after the first frames copying allocates
/// nothing.
#[derive(Default)]
struct Grid {
    /// Where the cells came from: the visible part of the transition's area.
    rect: Rect,
    /// The cells of `rect`, row by row.
    cells: Vec<Cell>,
}

impl Grid {
    /// Copies the cells of `rect`, which lies on the buffer, reusing the grid's storage.
    fn copy(&mut self, cx: &PaintCx<'_>, rect: Rect) {
        self.rect = rect;
        let len = usize::from(rect.width) * usize::from(rect.height);
        self.cells.truncate(len);
        let mut index = 0;
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                let Some(at) = buffer_cell(cx, x, y) else { continue };
                match self.cells.get_mut(index) {
                    Some(cell) => cell.clone_from(&cx.buf[at]),
                    None => self.cells.push(cx.buf[at].clone()),
                }
                index += 1;
            }
        }
    }

    /// The copied cell at `x`, `y`, when it was copied.
    fn get(&self, x: i32, y: i32) -> Option<&Cell> {
        if !self.rect.contains(x, y) {
            return None;
        }
        let index = (y - self.rect.y) * i32::from(self.rect.width) + (x - self.rect.x);
        self.cells.get(usize::try_from(index).ok()?)
    }

    fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    fn clear(&mut self) {
        self.cells.clear();
        self.rect = Rect::default();
    }
}

impl<Msg: 'static> PageTransition<Msg> {
    /// A transition keyed by `key`, usually the router's current page. Cross-fades by default.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            slide: false,
            direction: Navigation::Forward,
            content: vec![Node::new(Flex::new(Axis::Column, Vec::new()), 0)],
        }
    }

    /// Slides the incoming page a few cells into place as it fades in.
    #[must_use]
    pub fn slide(mut self, slide: bool) -> Self {
        self.slide = slide;
        self
    }

    /// The way the navigation went, e.g. [`Router::direction`](crate::router::Router::direction);
    /// decides which side a sliding page comes from.
    #[must_use]
    pub fn direction(mut self, direction: Navigation) -> Self {
        self.direction = direction;
        self
    }
}

impl<Msg: 'static> Container<Msg> for PageTransition<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.width = Length::Fill(1);
        column.layout.height = Length::Fill(1);
        self.content = vec![column];
    }
}

impl<Msg: 'static> Widget<Msg> for PageTransition<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.content.first().map_or(Size::default(), |content| cx.measure_child(content, available))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let now = cx.now();
        let mut memory = std::mem::take(cx.memory::<TransitionMemory>());
        if memory.key.as_deref() != Some(self.key.as_str()) {
            let can_blend = memory.key.is_some() && memory.area == area && !memory.shown.is_empty();
            if can_blend && !cx.reduced_motion() {
                std::mem::swap(&mut memory.from, &mut memory.shown);
                memory.started = Some(now);
                memory.direction = self.direction;
            }
            memory.key = Some(self.key.clone());
        }
        if memory.area != area {
            memory.started = None;
            memory.area = area;
        }

        if let Some(content) = self.content.first() {
            cx.paint_child(content, area);
        }

        if let Some(started) = memory.started {
            let duration = cx.env().theme().motion().page;
            let progress = cx.progress_since(started, duration, Easing::EaseOut);
            if progress >= 1.0 {
                memory.started = None;
                memory.from.clear();
            } else {
                let shift = if self.slide {
                    let distance = (area.width / 8).min(MAX_SLIDE);
                    let cells = i32::from(steps(1.0 - progress, distance));
                    match memory.direction {
                        Navigation::Forward => cells,
                        Navigation::Back => -cells,
                    }
                } else {
                    0
                };
                compose(cx, area, &memory.from, progress, shift);
            }
        }
        // Only the visible cells can take part in the next transition.
        memory.shown.copy(cx, visible(cx, area));
        *cx.memory::<TransitionMemory>() = memory;
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.content
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.content
    }
}

/// The buffer position of the cell at `x`, `y`, when it lies on the buffer.
fn buffer_cell(cx: &PaintCx<'_>, x: i32, y: i32) -> Option<(u16, u16)> {
    let (x, y) = (u16::try_from(x).ok()?, u16::try_from(y).ok()?);
    let buffer = cx.buf.area;
    (x >= buffer.x && y >= buffer.y && x < buffer.x + buffer.width && y < buffer.y + buffer.height).then_some((x, y))
}

/// The part of `area` this frame can draw on: inside the clip and on the buffer.
fn visible(cx: &PaintCx<'_>, area: Rect) -> Rect {
    let buffer = cx.buf.area;
    let buffer = Rect::new(i32::from(buffer.x), i32::from(buffer.y), buffer.width, buffer.height);
    cx.clip().intersect(area).intersect(buffer)
}

fn rgb(color: Color) -> Option<Rgb> {
    match color {
        Color::Rgb(r, g, b) => Some(Rgb::new(r, g, b)),
        _ => None,
    }
}

/// Blends two terminal colours; colours that are not 24-bit switch halfway.
fn blend(from: Color, to: Color, amount: f32) -> Color {
    match (rgb(from), rgb(to)) {
        (Some(a), Some(b)) => {
            let mixed = a.mix(b, amount);
            Color::Rgb(mixed.r, mixed.g, mixed.b)
        }
        _ if amount < 0.5 => from,
        _ => to,
    }
}

/// Draws the frame of a transition at `progress`: `from` blends into what was just painted,
/// which is shifted `shift` cells to the right (negative: to the left).
fn compose(cx: &mut PaintCx<'_>, area: Rect, from: &Grid, progress: f32, shift: i32) {
    let width = i32::from(area.width);
    let visible = visible(cx, area);
    let blank = Cell::default();
    for y in visible.y..visible.bottom() {
        for step in 0..i32::from(visible.width) {
            // The new page is read from the buffer while the frame is written into it. Walking each
            // row against the shift reads every source cell before it is overwritten, so the page
            // needs no copy.
            let x = if shift > 0 { visible.right() - 1 - step } else { visible.x + step };
            let column = x - area.x;
            let (Some(old), Some(at)) = (from.get(x, y), buffer_cell(cx, x, y)) else { continue };
            // The incoming page, displaced; the edge it uncovers repeats the nearest background.
            let source = column - shift;
            let new = buffer_cell(cx, area.x + source.clamp(0, width - 1), y).map_or(&blank, |at| &cx.buf[at]);
            let background = blend(old.bg, new.bg, progress);
            let mut cell = if progress < 0.5 {
                let mut cell = old.clone();
                cell.fg = blend(old.fg, background, progress * 2.0);
                cell
            } else {
                let mut cell = new.clone();
                if !(0..width).contains(&source) {
                    cell.set_symbol(" ");
                }
                cell.fg = blend(background, new.fg, (progress - 0.5) * 2.0);
                cell
            };
            cell.bg = background;
            // Half of a wide glyph cut by the edge of the area becomes a space.
            let symbol_width = text::width(cell.symbol());
            if (symbol_width > 1 && column + 1 >= width) || (cell.symbol().is_empty() && column == 0) {
                cell.set_symbol(" ");
            }
            cx.buf[at] = cell;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::Router;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::Text;

    struct Pages {
        router: Router<String>,
        slide: bool,
    }

    enum Msg {
        Open(&'static str),
        Back,
    }

    impl App for Pages {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Open(page) => self.router.push(page.to_owned()),
                Msg::Back => {
                    self.router.back();
                }
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let page = self.router.current().clone();
            let transition = PageTransition::new(page.clone()).slide(self.slide).direction(self.router.direction());
            ui.add_with(transition, |ui| {
                ui.page(page.clone(), |ui| {
                    ui.add(Text::new(page.clone()));
                });
            })
            .fill();
        }
    }

    fn pages(slide: bool) -> Harness<Pages> {
        Harness::new(Pages { router: Router::new("deploys".into()), slide }, 48, 2)
    }

    fn page_duration<A: App>(h: &Harness<A>) -> Duration {
        h.env().theme().motion().page
    }

    #[test]
    fn fades_through_the_background_and_settles() {
        let mut h = pages(false);
        assert_eq!(h.screen(), "deploys\n\n");
        let text = h.fg(0, 0);
        let canvas = h.bg(0, 0);
        h.send(Msg::Open("releases"));
        assert_eq!(h.screen(), "deploys\n\n", "the first frame still shows the old page");
        assert_eq!(h.fg(0, 0), text);
        // Just before the middle the old text has almost dissolved into the background.
        let middle = page_duration(&h) / 6;
        h.advance(middle);
        assert_eq!(h.screen(), "deploys\n\n");
        assert_ne!(h.fg(0, 0), text);
        h.advance(page_duration(&h));
        assert_eq!(h.screen(), "releases\n\n");
        assert_eq!((h.fg(0, 0), h.bg(0, 0)), (text, canvas));
    }

    #[test]
    fn slides_from_the_side_the_navigation_went() {
        let mut h = pages(true);
        // A quarter of the way in time the eased progress is past the middle: the new text shows,
        // three of its six cells still to travel.
        let quarter = page_duration(&h) / 4;
        h.send(Msg::Open("releases")).advance(quarter);
        assert_eq!(h.screen(), "   releases\n\n");
        h.advance(page_duration(&h));
        assert_eq!(h.screen(), "releases\n\n");
        h.send(Msg::Back).advance(quarter);
        assert_eq!(h.screen(), "loys\n\n");
        h.advance(page_duration(&h));
        assert_eq!(h.screen(), "deploys\n\n");
    }

    #[test]
    fn a_clipped_transition_blends_only_what_shows() {
        struct Clipped(Pages);
        impl App for Clipped {
            type Msg = Msg;
            fn update(&mut self, msg: Msg) -> Command<Msg> {
                self.0.update(msg)
            }
            fn view(&self, ui: &mut View<'_, Msg>) {
                // The transition is taller than the scroll view showing it.
                ui.add(Text::new("header"));
                ui.add_with(crate::widgets::ScrollView::new(), |ui| {
                    ui.column(|ui| self.0.view(ui)).height(crate::widget::Length::Cells(6)).fill_width();
                })
                .height(crate::widget::Length::Cells(2))
                .fill_width();
                ui.add(Text::new("footer"));
            }
        }
        let mut h = Harness::new(Clipped(Pages { router: Router::new("deploys".into()), slide: true }), 20, 4);
        assert_eq!(h.screen(), "header\ndeploys\n\nfooter\n");
        let quarter = page_duration(&h) / 4;
        h.send(Msg::Open("releases")).advance(quarter);
        // Nineteen cells wide, the page travels two cells; one is still to go.
        assert_eq!(h.screen(), "header\n releases\n\nfooter\n");
        h.advance(page_duration(&h));
        assert_eq!(h.screen(), "header\nreleases\n\nfooter\n");
    }

    #[test]
    fn reduced_motion_changes_pages_at_once() {
        let mut h = pages(true);
        h.set_reduced_motion(true);
        h.send(Msg::Open("releases"));
        assert_eq!(h.screen(), "releases\n\n");
    }
}
