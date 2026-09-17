//! Panels: raised surfaces that group content without drawing a frame.

use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::text;
use crate::theme::State;
use crate::widget::{Axis, Container, EventCx, Flex, MeasureCx, Node, PaintCx, Widget};

use super::cells;
use super::press::{self, Press};

/// A surface one step above its background, optionally titled, selectable and pressable.
///
/// A pressable panel answers the pointer like a button: hovering raises its surface one tone and
/// shows a soft pillar down its whole left edge, pressing flashes one tone brighter, and focus
/// reached with the keyboard raises it with a breathing pillar. A selected panel keeps the accent
/// pillar down the same edge, as on dialogs. Panels without `on_press`, and disabled ones, never react to the pointer.
///
/// Style keys: `panel` (`bg`, `padding`, `pillar`), `panel-title` (`fg`, `bold`), states
/// `hover`, `focus` and `pressed` (pressable panels only) and `selected`.
pub struct Panel<Msg> {
    title: Option<String>,
    variant: Option<String>,
    selected: bool,
    disabled: bool,
    gap: u16,
    on_press: Option<Msg>,
    body: Vec<Node<Msg>>,
}

impl<Msg: 'static> Panel<Msg> {
    /// An untitled panel. Add its content with [`View::add_with`](crate::widget::View::add_with).
    #[must_use]
    pub fn new() -> Self {
        Self {
            title: None,
            variant: None,
            selected: false,
            disabled: false,
            gap: 1,
            on_press: None,
            body: vec![Node::new(Flex::new(Axis::Column, Vec::new()), 0)],
        }
    }

    /// A small heading drawn in the panel's first row.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Theme variant.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Raises the panel to the selected surface with the accent pillar.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Rows between children; 1 by default.
    #[must_use]
    pub fn gap(mut self, rows: u16) -> Self {
        self.gap = rows;
        self
    }

    /// Makes the whole panel pressable, like a card that opens something. Enter or Space presses
    /// it while focused; a click presses it on release, and releasing away from the panel cancels.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Keeps a pressable panel from being hovered, focused or pressed; its message is not sent.
    /// It still shows whether it is selected.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_press.is_some()
    }

    fn title_rows(&self) -> u16 {
        if self.title.is_some() { 2 } else { 0 }
    }
}

impl<Msg: 'static> Default for Panel<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> Container<Msg> for Panel<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.gap = self.gap;
        column.layout.width = crate::widget::Length::Fill(1);
        self.body = vec![column];
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Panel<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let style = cx.env().theme().style("panel", self.variant.as_deref(), &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((1, 3));
        let inner = Size::new(
            available.width.saturating_sub(horizontal.saturating_mul(2)),
            available.height.saturating_sub(vertical.saturating_mul(2).saturating_add(self.title_rows())),
        );
        let content = self.body.first().map_or(Size::default(), |body| cx.measure_child(body, inner));
        let title_width = self.title.as_deref().map_or(0, text::width);
        Size::new(
            content.width.max(title_width).saturating_add(horizontal.saturating_mul(2)),
            cells::sum([content.height, vertical.saturating_mul(2), self.title_rows()]),
        )
        .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let mut states = if self.active() { cx.pressable_states() } else { Vec::new() };
        if self.selected {
            states.push(State::Selected);
        }
        let variant = self.variant.as_deref();
        let style = cx.style("panel", variant, &states);
        let background = style.text().bg.unwrap_or_else(|| cx.color("surface"));
        cx.clear(area, background);
        let padding = style.padding();
        let inner = area.inset(padding);
        if let Some(pillar) = style.color("pillar")
            && (self.selected || padding.left >= 1)
        {
            // A panel is a whole surface, so every mark on it (hover, focus, pressed or selected)
            // runs down its full left edge, like a dialog's pillar.
            for row in 0..area.height {
                cx.pillar(area.x, area.y + i32::from(row), pillar);
            }
        }
        if self.active() {
            cx.register_hit(area);
        }
        let mut body = inner;
        if let Some(title) = &self.title {
            let title_style = cx.style("panel-title", variant, &states).text();
            cx.text(inner.x, inner.y, title, title_style, inner.width);
            body = Rect::new(inner.x, inner.y + 2, inner.width, inner.height.saturating_sub(2));
        }
        if let Some(content) = self.body.first() {
            cx.paint_child(content, body);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let Some(message) = self.on_press.as_ref().filter(|_| !self.disabled) else {
            return false;
        };
        match press::read(cx, event) {
            Press::Ignored => false,
            Press::Used => true,
            Press::Key | Press::Click(..) => {
                cx.flash();
                cx.emit(message.clone());
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.active()
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
    use crate::color::Rgb;
    use crate::event::{MouseButton, MouseKind};
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::Text;
    use std::time::Duration;

    #[derive(Default)]
    struct Demo {
        opened: bool,
        untitled: bool,
        inset: bool,
        disabled: bool,
        plain: bool,
        /// Pressing does not select, to see the pressed tone on its own.
        unselected: bool,
    }

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            self.opened = true;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let mut panel = Panel::new().selected(self.opened && !self.unselected).disabled(self.disabled);
            if !self.untitled {
                panel = panel.title("LIVE");
            }
            if self.inset {
                panel = panel.variant("inset");
            }
            if !self.plain {
                panel = panel.on_press(());
            }
            ui.add_with(panel, |ui| {
                ui.add(Text::new("Content"));
            })
            .fill_width();
        }
    }

    const THEMES: [&str; 4] = ["monochrome", "iris", "nordic", "amber"];

    fn harness(demo: Demo) -> Harness<Demo> {
        Harness::new(demo, 20, 6)
    }

    fn color(h: &Harness<Demo>, token: &str) -> Rgb {
        h.env().theme().color(token).expect("token")
    }

    /// `mix($text, $<token>, percent%)` as the theme writes it.
    fn lifted(h: &Harness<Demo>, token: &str, percent: f32) -> Option<Rgb> {
        Some(color(h, token).mix(color(h, "text"), percent / 100.0))
    }

    /// The rows whose first cell shows the pillar.
    fn pillar_rows(h: &Harness<Demo>) -> Vec<usize> {
        h.screen().lines().enumerate().filter(|(_, line)| line.starts_with('▌')).map(|(row, _)| row).collect()
    }

    #[test]
    fn draws_title_and_content_on_surface() {
        let h = harness(Demo::default());
        assert_eq!(h.screen(), "\n   LIVE\n\n   Content\n\n\n");
        assert_eq!(h.bg(0, 0), h.env().theme().color("surface"));
        assert_eq!(h.fg(3, 1), h.env().theme().color("muted"));
    }

    #[test]
    fn hover_raises_the_surface_a_clear_step_with_a_pillar_down_the_whole_left_edge() {
        for inset in [false, true] {
            let mut h = harness(Demo { inset, ..Demo::default() });
            let rest_token = if inset { "raised" } else { "surface" };
            for id in THEMES {
                h.set_theme(id);
                h.hover(19, 5);
                h.hover(40, 40);
                assert_eq!(h.bg(10, 0), Some(color(&h, rest_token)), "{id}: rest");
                assert!(pillar_rows(&h).is_empty(), "{id}: no pillar at rest");
                h.hover(10, 3);
                let hover = h.bg(10, 0).expect("hover tone");
                assert_eq!(Some(hover), lifted(&h, rest_token, 8.0), "{id}");
                let ratio = hover.contrast_ratio(color(&h, rest_token));
                assert!(ratio >= 1.15, "{id} inset={inset}: rest to hover is {ratio:.3}:1");
                assert_eq!(h.fg(if inset { 2 } else { 3 }, 1), Some(color(&h, "dim")), "{id}: the title wakes");
                assert_eq!(pillar_rows(&h), vec![0, 1, 2, 3, 4], "{id}: the pillar runs down the whole left edge");
                assert_eq!(h.fg(0, 1), Some(color(&h, "active").mix(color(&h, "accent"), 0.45)), "{id}");
            }
        }
    }

    #[test]
    fn an_untitled_panel_shows_the_pillar_down_its_left_edge() {
        let mut h = harness(Demo { untitled: true, ..Demo::default() });
        h.hover(10, 1);
        assert_eq!(h.screen().lines().nth(1), Some("▌  Content"));
        assert_eq!(pillar_rows(&h), vec![0, 1, 2]);
    }

    #[test]
    fn keyboard_focus_raises_like_hover_with_a_breathing_pillar() {
        let mut h = harness(Demo::default());
        h.press("tab");
        assert_eq!(h.bg(10, 0), lifted(&h, "surface", 8.0));
        assert_eq!(h.fg(3, 1), Some(color(&h, "dim")));
        assert_eq!(pillar_rows(&h), vec![0, 1, 2, 3, 4]);
        let start = h.fg(0, 1);
        h.advance(h.env().theme().motion().pulse_period / 2);
        assert_ne!(h.fg(0, 1), start, "the pillar breathes");
    }

    #[test]
    fn pointer_focus_stays_calm_once_the_pointer_leaves() {
        let mut h = harness(Demo::default());
        h.mouse(MouseKind::Down(MouseButton::Left), 10, 3);
        h.hover(40, 40).advance(Duration::from_millis(200));
        assert_eq!(h.bg(10, 0), Some(color(&h, "surface")), "a click does not leave a raised panel behind");
        assert!(pillar_rows(&h).is_empty());
    }

    #[test]
    fn pressing_flashes_one_tone_brighter() {
        let mut h = harness(Demo::default());
        h.press("tab");
        let focus = h.bg(10, 0).expect("focus tone");
        h.press("enter");
        let pressed = h.bg(10, 0).expect("pressed tone");
        assert_eq!(Some(pressed), lifted(&h, "active", 16.0), "the press lands on the now selected panel");
        assert!(pressed.contrast_ratio(focus) >= 1.15, "the flash is a visible step up from focus");
        assert_eq!(h.fg(3, 1), Some(color(&h, "text")));

        let mut h = harness(Demo::default());
        h.hover(10, 3);
        let hover = h.bg(10, 0).expect("hover tone");
        h.mouse(MouseKind::Down(MouseButton::Left), 10, 3);
        h.mouse(MouseKind::Up(MouseButton::Left), 10, 3);
        let pressed = h.bg(10, 0).expect("pressed tone");
        assert!(pressed.contrast_ratio(hover) >= 1.15, "a click flashes brighter than hover");
        h.advance(Duration::from_millis(200));
        assert_eq!(h.bg(10, 0), lifted(&h, "active", 8.0), "selected and still hovered");
    }

    #[test]
    fn the_pressed_flash_is_brighter_than_hover_on_an_unselected_panel() {
        for inset in [false, true] {
            let mut h = harness(Demo { inset, unselected: true, ..Demo::default() });
            let rest_token = if inset { "raised" } else { "surface" };
            for id in THEMES {
                h.set_theme(id);
                h.hover(10, 3);
                let hover = h.bg(10, 0).expect("hover tone");
                h.click(10, 3);
                assert_eq!(h.bg(10, 0), lifted(&h, rest_token, 16.0), "{id}");
                let pressed = h.bg(10, 0).expect("pressed tone");
                assert!(pressed.contrast_ratio(hover) >= 1.15, "{id} inset={inset}: hover to pressed");
                assert_eq!(h.fg(0, 1), Some(color(&h, "accent")), "{id}: the pillar takes the accent");
                assert_eq!(h.fg(if inset { 2 } else { 3 }, 1), Some(color(&h, "text")), "{id}");
                h.advance(Duration::from_millis(200));
                assert_eq!(h.bg(10, 0), Some(hover), "{id}: back to hover after the flash");
            }
        }
    }

    #[test]
    fn pressing_selects_with_a_full_height_pillar() {
        let mut h = harness(Demo::default());
        h.click(5, 3);
        assert!(h.app().opened);
        h.hover(40, 40).advance(Duration::from_millis(200));
        assert_eq!(pillar_rows(&h), vec![0, 1, 2, 3, 4]);
        assert_eq!(h.bg(10, 1), h.env().theme().color("active"));
    }

    #[test]
    fn a_disabled_panel_shows_no_hover_and_cannot_be_pressed() {
        let mut h = harness(Demo { disabled: true, ..Demo::default() });
        let rest = h.screen();
        h.hover(10, 3);
        assert_eq!(h.bg(10, 0), Some(color(&h, "surface")));
        assert_eq!(h.fg(3, 1), Some(color(&h, "muted")));
        assert_eq!(h.screen(), rest);
        h.press("tab").press("enter").click(10, 3);
        assert!(!h.app().opened);
        assert!(pillar_rows(&h).is_empty());
    }

    #[test]
    fn a_panel_without_on_press_stays_as_it_was() {
        let mut h = harness(Demo { plain: true, ..Demo::default() });
        let rest = h.screen();
        h.hover(10, 3);
        assert_eq!(h.screen(), rest);
        assert_eq!(h.bg(10, 0), Some(color(&h, "surface")));
        assert_eq!(h.fg(3, 1), Some(color(&h, "muted")));
        h.press("tab").click(10, 3);
        assert!(!h.app().opened);
        assert!(pillar_rows(&h).is_empty());
    }

    #[test]
    fn presses_on_release_and_moving_away_cancels() {
        let mut h = harness(Demo::default());
        h.mouse(MouseKind::Down(MouseButton::Left), 5, 3);
        assert!(!h.app().opened, "the mouse going down is not a press yet");
        h.mouse(MouseKind::Up(MouseButton::Left), 5, 3);
        assert!(h.app().opened, "releasing over the panel presses it");

        let mut h = harness(Demo::default());
        h.mouse(MouseKind::Down(MouseButton::Left), 5, 3);
        h.mouse(MouseKind::Drag(MouseButton::Left), 5, 5);
        h.mouse(MouseKind::Up(MouseButton::Left), 5, 5);
        assert!(!h.app().opened, "releasing away from the panel cancels");
    }
}
