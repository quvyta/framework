//! The application frame: header, sidebar, body and footer.

use crate::geometry::{Rect, Size};
use crate::widget::{Axis, Flex, Length, MeasureCx, Node, NodeMut, PaintCx, View, Widget};

type Part<'a, Msg> = Box<dyn FnOnce(&mut View<'_, Msg>) + 'a>;

/// Builds the common application frame: a header on top, a sidebar on the left, the body
/// beside it and a footer at the bottom.
///
/// The parts are separated by surface colour only. Below `collapse_below` columns the sidebar
/// is hidden to give the body room; while `sidebar_open` is true it is drawn over the body as a
/// layer instead. Style keys: `shell-header`, `shell-sidebar`, `shell-body`, `shell-footer`
/// (`bg`).
pub struct AppShell<'a, Msg> {
    header: Option<Part<'a, Msg>>,
    sidebar: Option<Part<'a, Msg>>,
    body: Option<Part<'a, Msg>>,
    footer: Option<Part<'a, Msg>>,
    sidebar_width: u16,
    collapse_below: u16,
    sidebar_open: bool,
}

impl<'a, Msg: 'static> AppShell<'a, Msg> {
    /// A shell with a 28-column sidebar that collapses below 90 columns.
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: None,
            sidebar: None,
            body: None,
            footer: None,
            sidebar_width: 28,
            collapse_below: 90,
            sidebar_open: false,
        }
    }

    /// The top bar.
    #[must_use]
    pub fn header(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.header = Some(Box::new(build));
        self
    }

    /// The navigation column.
    #[must_use]
    pub fn sidebar(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.sidebar = Some(Box::new(build));
        self
    }

    /// The main content.
    #[must_use]
    pub fn body(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.body = Some(Box::new(build));
        self
    }

    /// The bottom bar, usually [`KeyHints`](crate::widgets::KeyHints).
    #[must_use]
    pub fn footer(mut self, build: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.footer = Some(Box::new(build));
        self
    }

    /// Sidebar width in columns.
    #[must_use]
    pub fn sidebar_width(mut self, columns: u16) -> Self {
        self.sidebar_width = columns;
        self
    }

    /// Screen width under which the sidebar collapses.
    #[must_use]
    pub fn collapse_below(mut self, columns: u16) -> Self {
        self.collapse_below = columns;
        self
    }

    /// Shows the collapsed sidebar as a layer over the body.
    #[must_use]
    pub fn sidebar_open(mut self, open: bool) -> Self {
        self.sidebar_open = open;
        self
    }

    /// Adds the shell to `ui`, filling the space it gets.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let build = |part: Option<Part<'a, Msg>>| {
            let mut children = Vec::new();
            if let Some(part) = part {
                part(&mut ui.nested(&mut children));
            }
            let mut node = Node::new(Flex::new(Axis::Column, children), 0);
            node.layout.width = Length::Fill(1);
            node.layout.height = Length::Fill(1);
            node
        };
        let shell = Shell {
            parts: vec![build(self.header), build(self.sidebar), build(self.body), build(self.footer)],
            sidebar_width: self.sidebar_width,
            collapse_below: self.collapse_below,
            sidebar_open: self.sidebar_open,
        };
        ui.add(shell).fill()
    }
}

impl<Msg: 'static> Default for AppShell<'_, Msg> {
    fn default() -> Self {
        Self::new()
    }
}

const HEADER: usize = 0;
const SIDEBAR: usize = 1;
const BODY: usize = 2;
const FOOTER: usize = 3;

struct Shell<Msg> {
    parts: Vec<Node<Msg>>,
    sidebar_width: u16,
    collapse_below: u16,
    sidebar_open: bool,
}

impl<Msg: 'static> Shell<Msg> {
    fn part_height(&self, cx: &mut PaintCx<'_>, index: usize, area: Rect) -> u16 {
        if self.parts[index].widget.children().is_empty() {
            return 0;
        }
        cx.measure_child(&self.parts[index], area.size()).height
    }

    fn collapsed(&self, area: Rect) -> bool {
        area.width < self.collapse_below
    }

    /// The heights of the header and the footer in `area`.
    fn bars(&self, cx: &mut PaintCx<'_>, area: Rect) -> (u16, u16) {
        (self.part_height(cx, HEADER, area), self.part_height(cx, FOOTER, area))
    }

    /// The row band between a header `header` rows tall and a footer `footer` rows tall.
    fn middle(area: Rect, (header, footer): (u16, u16)) -> Rect {
        Rect::new(
            area.x,
            area.y + i32::from(header),
            area.width,
            area.height.saturating_sub(header.saturating_add(footer)),
        )
    }

    fn has_sidebar(&self) -> bool {
        !self.parts[SIDEBAR].widget.children().is_empty()
    }
}

impl<Msg: 'static> Widget<Msg> for Shell<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let (header, footer) = self.bars(cx, area);
        let middle = Self::middle(area, (header, footer));
        if header > 0 {
            let rect = Rect::new(area.x, area.y, area.width, header);
            paint_part(cx, &self.parts[HEADER], rect, "shell-header");
        }
        let sidebar = self.has_sidebar() && !self.collapsed(area);
        let sidebar_width = if sidebar { self.sidebar_width.min(area.width) } else { 0 };
        if sidebar {
            let rect = Rect::new(middle.x, middle.y, sidebar_width, middle.height);
            paint_part(cx, &self.parts[SIDEBAR], rect, "shell-sidebar");
        }
        let body = Rect::new(
            middle.x + i32::from(sidebar_width),
            middle.y,
            middle.width.saturating_sub(sidebar_width),
            middle.height,
        );
        paint_part(cx, &self.parts[BODY], body, "shell-body");
        if footer > 0 {
            let rect = Rect::new(area.x, area.bottom() - i32::from(footer), area.width, footer);
            paint_part(cx, &self.parts[FOOTER], rect, "shell-footer");
        }
        if self.has_sidebar() && self.collapsed(area) && self.sidebar_open {
            cx.request_overlay(area);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let middle = Self::middle(anchor, self.bars(cx, anchor));
        let width = self.sidebar_width.min(middle.width);
        let rect = Rect::new(middle.x, middle.y, width, middle.height);
        cx.register_hit(rect);
        // Drawn over the body, the sidebar floats: it keeps apart from the body and the bars.
        cx.floating(rect, |cx| paint_part(cx, &self.parts[SIDEBAR], rect, "shell-sidebar"));
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.parts
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.parts
    }
}

fn paint_part<Msg: 'static>(cx: &mut PaintCx<'_>, node: &Node<Msg>, rect: Rect, style: &str) {
    let background = cx.style(style, None, &[]).text().bg;
    if let Some(bg) = background {
        cx.clear(rect, bg);
    }
    cx.paint_child(node, rect);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widgets::Text;

    struct Demo {
        open: bool,
    }

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            AppShell::new()
                .sidebar_width(8)
                .collapse_below(30)
                .sidebar_open(self.open)
                .header(|ui| {
                    ui.add(Text::new("head"));
                })
                .sidebar(|ui| {
                    ui.add(Text::new("menu"));
                })
                .body(|ui| {
                    ui.add(Text::new("body"));
                })
                .footer(|ui| {
                    ui.add(Text::new("foot"));
                })
                .show(ui);
        }
    }

    #[test]
    fn lays_out_parts_on_their_surfaces() {
        let h = Harness::new(Demo { open: false }, 30, 4);
        assert_eq!(h.screen(), "head\nmenu    body\n\nfoot\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(0, 1), theme.color("surface"));
        assert_eq!(h.bg(10, 1), theme.color("canvas"));
    }

    #[test]
    fn collapses_sidebar_and_opens_it_as_a_layer() {
        let closed = Harness::new(Demo { open: false }, 20, 4);
        assert_eq!(closed.screen(), "head\nbody\n\nfoot\n");
        let open = Harness::new(Demo { open: true }, 20, 4);
        assert_eq!(open.screen(), "head\nmenu\n\nfoot\n");
    }
}
