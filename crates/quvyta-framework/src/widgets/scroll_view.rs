//! Vertical scrolling of any content.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::widget::{Axis, Container, EventCx, Flex, Length, MeasureCx, Node, PaintCx, Widget, WidgetId};

use super::rows::WHEEL_ROWS;
use super::scrollbar::{self, ScrollMetrics, ScrollbarStyle};

/// Shows content taller than its area and scrolls it with the wheel, the scrollbar or the
/// keyboard (↑/↓, PgUp/PgDn, Home/End while focused). When focus moves to a widget inside that
/// is out of view, the view scrolls to it.
///
/// Style keys: `scrollbar` (`style`, `track`, `thumb`) with `hover`, and `scrollbar.<style>`.
pub struct ScrollView<Msg> {
    content: Vec<Node<Msg>>,
    scrollbar: Option<ScrollbarStyle>,
}

#[derive(Debug, Default)]
struct ScrollMemory {
    offset: u16,
    content_height: u16,
    revealed: Option<WidgetId>,
    dragging: bool,
}

impl<Msg: 'static> ScrollView<Msg> {
    /// An empty scroll view; add content with [`View::add_with`](crate::widget::View::add_with).
    #[must_use]
    pub fn new() -> Self {
        Self { content: vec![Node::new(Flex::new(Axis::Column, Vec::new()), 0)], scrollbar: None }
    }

    /// Draws the scrollbar in `style` whatever the theme chooses.
    #[must_use]
    pub fn scrollbar(mut self, style: ScrollbarStyle) -> Self {
        self.scrollbar = Some(style);
        self
    }

    fn metrics(memory: &ScrollMemory, area: Rect) -> ScrollMetrics {
        ScrollMetrics {
            total: usize::from(memory.content_height),
            visible: usize::from(area.height),
            offset: usize::from(memory.offset),
        }
    }

    fn scroll_to(cx: &mut EventCx<'_, Msg>, offset: i32) {
        let area = cx.area();
        let memory = cx.memory::<ScrollMemory>();
        let max = memory.content_height.saturating_sub(area.height);
        memory.offset = clamp_u16(offset).min(max);
    }
}

impl<Msg: 'static> Default for ScrollView<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: 'static> Container<Msg> for ScrollView<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.width = Length::Fill(1);
        self.content = vec![column];
    }
}

impl<Msg: 'static> Widget<Msg> for ScrollView<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let content = self.content.first().map_or(Size::default(), |c| cx.measure_child(c, available));
        Size::new(content.width.saturating_add(1), content.height).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let Some(content) = self.content.first() else {
            return;
        };
        cx.register_hit(area);
        let full = cx.measure_child(content, Size::new(area.width, u16::MAX)).height;
        let overflows = full > area.height;
        let width = if overflows { area.width.saturating_sub(2) } else { area.width };
        let height = if overflows { cx.measure_child(content, Size::new(width, u16::MAX)).height } else { full };
        let offset = {
            let memory = cx.memory::<ScrollMemory>();
            memory.content_height = height;
            memory.offset = memory.offset.min(height.saturating_sub(area.height));
            memory.offset
        };
        let content_rect = Rect::new(area.x, area.y - i32::from(offset), width, height);
        cx.with_clip(area, |cx| cx.paint_child(content, content_rect));

        if let Some(focused) = cx.interaction.focused
            && focused != cx.id()
            && cx.frame.is_within(focused, cx.id())
            && let Some(rect) = cx.frame.rects.get(&focused).copied()
            && cx.memory::<ScrollMemory>().revealed != Some(focused)
        {
            let memory = cx.memory::<ScrollMemory>();
            memory.revealed = Some(focused);
            let top = rect.y - content_rect.y;
            let bottom = top + i32::from(rect.height);
            let new_offset = if top < i32::from(offset) {
                clamp_u16(top)
            } else if bottom > i32::from(offset) + i32::from(area.height) {
                clamp_u16(bottom - i32::from(area.height))
            } else {
                offset
            };
            if new_offset != offset {
                memory.offset = new_offset;
                cx.request_frame_in(std::time::Duration::ZERO);
            }
        }

        if overflows {
            let metrics = Self::metrics(cx.memory::<ScrollMemory>(), area);
            let active =
                cx.memory::<ScrollMemory>().dragging || cx.pointer().is_some_and(|(x, _)| x >= area.right() - 1);
            scrollbar::paint(cx, Rect::new(area.right() - 1, area.y, 1, area.height), metrics, active, self.scrollbar);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let area = cx.area();
        let (offset, metrics) = {
            let memory = cx.memory::<ScrollMemory>();
            (i32::from(memory.offset), Self::metrics(memory, area))
        };
        let page = i32::from(area.height.saturating_sub(1).max(1));
        match event {
            Event::Key(key) => {
                let target = if key.is_plain(Key::Up) {
                    offset - 1
                } else if key.is_plain(Key::Down) {
                    offset + 1
                } else if key.is_plain(Key::PageUp) {
                    offset - page
                } else if key.is_plain(Key::PageDown) {
                    offset + page
                } else if key.is_plain(Key::Home) {
                    0
                } else if key.is_plain(Key::End) {
                    i32::MAX
                } else {
                    return false;
                };
                if !metrics.overflows() {
                    return false;
                }
                Self::scroll_to(cx, target);
                true
            }
            Event::Mouse(mouse) => {
                let on_bar = metrics.overflows() && mouse.x == area.right() - 1;
                match mouse.kind {
                    MouseKind::ScrollUp if metrics.overflows() => {
                        Self::scroll_to(cx, offset - i32::from(WHEEL_ROWS));
                        true
                    }
                    MouseKind::ScrollDown if metrics.overflows() => {
                        Self::scroll_to(cx, offset + i32::from(WHEEL_ROWS));
                        true
                    }
                    MouseKind::Down(MouseButton::Left) if on_bar => {
                        cx.capture_pointer();
                        cx.memory::<ScrollMemory>().dragging = true;
                        let target = metrics.offset_at(clamp_u16(mouse.y - area.y), area.height);
                        Self::scroll_to(cx, i32::try_from(target).unwrap_or(i32::MAX));
                        true
                    }
                    MouseKind::Drag(MouseButton::Left) if cx.memory::<ScrollMemory>().dragging => {
                        let target = metrics.offset_at(clamp_u16(mouse.y - area.y), area.height);
                        Self::scroll_to(cx, i32::try_from(target).unwrap_or(i32::MAX));
                        true
                    }
                    MouseKind::Up(MouseButton::Left) if cx.memory::<ScrollMemory>().dragging => {
                        cx.memory::<ScrollMemory>().dragging = false;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.content
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    struct Demo;

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add_with(ScrollView::new(), |ui| {
                for i in 0..20 {
                    ui.add(Text::new(format!("line {i}")));
                }
                ui.add(Button::new("Bottom").on_press(())).id("bottom");
            })
            .fill();
        }
    }

    #[test]
    fn scrolls_with_keys_and_wheel() {
        let mut h = Harness::new(Demo, 20, 5);
        assert!(h.screen().starts_with("line 0"));
        h.press("tab").press("pgdn");
        assert!(h.screen().starts_with("line 4"), "{}", h.screen());
        h.mouse(MouseKind::ScrollDown, 2, 2);
        assert!(h.screen().starts_with("line 7"));
        h.press("end");
        assert!(h.screen().contains("Bottom"));
        h.press("home");
        assert!(h.screen().starts_with("line 0"));
    }

    #[test]
    fn reveals_focused_widget() {
        let mut h = Harness::new(Demo, 20, 5);
        h.press("tab").press("tab");
        assert!(h.is_focused("bottom"));
        assert!(h.screen().contains("Bottom"), "{}", h.screen());
    }

    #[test]
    fn draws_scrollbar_only_when_needed() {
        // The default block style is colour only, so the scrollbar column is read by its colours.
        let h = Harness::new(Demo, 20, 30);
        let muted = h.env().theme().color("muted");
        assert_ne!(h.bg(19, 0), muted);
        let short = Harness::new(Demo, 20, 5);
        assert_eq!(short.bg(19, 0), muted, "the thumb sits at the top");
        assert_eq!(short.bg(19, 4), short.env().theme().color("raised"), "the track runs below it");
    }

    struct Pinned(Option<ScrollbarStyle>);

    impl App for Pinned {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let view = self.0.map_or_else(ScrollView::new, |style| ScrollView::new().scrollbar(style));
            ui.add_with(view, |ui| {
                for i in 0..20 {
                    ui.add(Text::new(format!("line {i}")));
                }
            })
            .fill();
        }
    }

    /// The scrollbar column, top to bottom, with blank cells as spaces.
    fn bar(h: &Harness<Pinned>) -> String {
        h.screen().lines().map(|line| format!("{line:<10}").chars().nth(9).unwrap_or(' ')).collect()
    }

    #[test]
    fn every_style_draws_its_own_column() {
        let expected = [
            (ScrollbarStyle::Block, "    "),
            (ScrollbarStyle::Half, "▐▕▕▕"),
            (ScrollbarStyle::Thin, "▕   "),
            (ScrollbarStyle::Dots, "•···"),
        ];
        for (style, column) in expected {
            let mut h = Harness::new(Pinned(Some(style)), 10, 4);
            assert_eq!(bar(&h), column, "{style:?}");
            let theme = h.env().theme();
            let (raised, muted, canvas) = (theme.color("raised"), theme.color("muted"), theme.color("canvas"));
            let thumb = if style == ScrollbarStyle::Dots { theme.color("dim") } else { muted };
            match style {
                ScrollbarStyle::Block => assert_eq!((h.bg(9, 0), h.bg(9, 3)), (muted, raised)),
                ScrollbarStyle::Thin => assert_eq!(h.bg(9, 3), canvas, "thin draws no track"),
                ScrollbarStyle::Half => assert_eq!(h.fg(9, 0), muted),
                ScrollbarStyle::Dots => assert_eq!((h.fg(9, 0), h.fg(9, 3)), (theme.color("dim"), muted)),
            }
            h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
            assert!(h.screen().is_ascii(), "{style:?}");
            assert_eq!(h.bg(9, 0), thumb, "ASCII thumb is a coloured cell in {style:?}");
        }
    }

    #[test]
    fn theme_word_chooses_the_style_and_pinning_wins() {
        let dir = std::env::temp_dir().join(format!("quvyta-scrollbar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let theme = "[meta]\nname = \"Dotted\"\nextends = \"monochrome\"\n[style.scrollbar]\nstyle = \"dots\"\n";
        std::fs::write(dir.join("dotted.toml"), theme).expect("theme file");
        let dirs = crate::env::AssetDirs { themes: Some(dir.clone()), ..Default::default() };
        let env = crate::env::Env::load(&dirs).expect("loads");
        let mut h = Harness::with_env(Pinned(None), env.clone(), 10, 4);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode).set_theme("dotted");
        assert_eq!(bar(&h), "•···");
        let mut pinned = Harness::with_env(Pinned(Some(ScrollbarStyle::Thin)), env, 10, 4);
        pinned.set_glyph_mode(crate::icons::GlyphMode::Unicode).set_theme("dotted");
        assert_eq!(bar(&pinned), "▕   ");
        std::fs::remove_dir_all(dir).ok();
    }
}
