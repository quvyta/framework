//! Accordions: a column of titled sections that open and close in place.

use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::widget::{Container, EventCx, MeasureCx, Node, PaintCx, Widget};

use super::sections::{Flow, Section, Sections};

/// A vertical list of titled sections; each opens with a click or Enter to show its body.
///
/// Add one child per section with [`View::add_with`](crate::widget::View::add_with): the first
/// child is the body of the first section, and so on. The application owns which sections are
/// open and hears about changes through [`Accordion::on_toggle`]. Sections are separated by
/// surface tones, never frames: the title row is one tone above the body. An open body takes
/// its natural height, so an accordion usually lives inside a [`ScrollView`](super::ScrollView)
/// or a panel; opening unfolds the body row by row over twice `motion.enter`.
///
/// A hovered title (or the keyboard's title while focused) rises with the pillar, and its icon and
/// title slide one cell right. The chevron and the detail never move.
///
/// Keys while focused: ↑/↓, Home/End move between titles, Enter or Space opens or closes. A
/// click anywhere on a title row opens or closes it.
///
/// Style keys: `section` (`gap`), `section-title` (`bg`, `fg`, `bold`, `pillar`) with `hover`,
/// `focus`, `checked` (open), `section-chevron` (`fg`), `section-detail` (`fg`),
/// `section-body` (`bg`, `padding`). Icons: `section-open`, `section-closed`.
pub struct Accordion<Msg> {
    model: Sections<Msg>,
}

impl<Msg: 'static> Accordion<Msg> {
    /// An accordion of `sections`, all closed.
    #[must_use]
    pub fn new(sections: impl IntoIterator<Item = impl Into<Section>>) -> Self {
        Self { model: Sections::new(sections.into_iter().map(Into::into).collect()) }
    }

    /// Which sections are open; `open[i]` for section `i`, missing entries are closed.
    #[must_use]
    pub fn open(mut self, open: impl IntoIterator<Item = bool>) -> Self {
        self.model.open = open.into_iter().collect();
        self
    }

    /// Message for opening (`true`) or closing (`false`) section `index`.
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(usize, bool) -> Msg + 'static) -> Self {
        self.model.on_toggle = Some(Box::new(message));
        self
    }

    /// Keeps at most one section open: opening one also sends close messages for the others.
    #[must_use]
    pub fn single(mut self, single: bool) -> Self {
        self.model.single = single;
        self
    }
}

impl<Msg: 'static> Container<Msg> for Accordion<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        self.model.bodies = children;
    }
}

impl<Msg: 'static> Widget<Msg> for Accordion<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.model.measure(cx, available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        self.model.paint(cx, area, Flow::Natural);
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
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    struct Demo {
        open: Vec<bool>,
        single: bool,
        pressed: bool,
        icons: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Toggle(usize, bool),
        Press,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Toggle(index, open) => self.open[index] = open,
                Msg::Press => self.pressed = true,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let mut sections = [Section::new("Ports").detail("3"), Section::new("Volumes"), Section::new("Logs")];
            if self.icons {
                sections = sections.map(|section| section.icon("folder"));
            }
            let accordion = Accordion::new(sections).open(self.open.clone()).single(self.single).on_toggle(Msg::Toggle);
            ui.add_with(accordion, |ui| {
                ui.add(Text::new("8080 → 80"));
                ui.column(|ui| {
                    ui.add(Text::new("data"));
                    ui.add(Button::new("Prune").on_press(Msg::Press)).id("prune");
                });
                ui.add(Text::new("ready"));
            })
            .fill_width()
            .id("accordion");
        }
    }

    fn demo(open: [bool; 3], single: bool) -> Demo {
        Demo { open: open.to_vec(), single, pressed: false, icons: false }
    }

    #[test]
    fn closed_sections_are_title_rows_separated_by_a_gap() {
        let h = Harness::new(demo([false; 3], false), 24, 6);
        assert_eq!(h.screen(), "  ▸ Ports            3\n\n  ▸ Volumes\n\n  ▸ Logs\n\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(10, 0), theme.color("raised"));
        assert_eq!(h.bg(10, 1), theme.color("canvas"));
    }

    #[test]
    fn click_opens_with_a_row_by_row_reveal() {
        let mut h = Harness::new(demo([false; 3], false), 24, 12);
        h.click_text("Volumes");
        assert_eq!(h.app().open, vec![false, true, false]);
        // The rows are reserved at once; the body unfolds into them.
        assert!(!h.screen().contains("Prune"), "{}", h.screen());
        h.advance(Duration::from_millis(400));
        assert_eq!(h.screen(), "  ▸ Ports            3\n\n▌ ▾  Volumes\n\n  data\n    Prune\n\n\n  ▸ Logs\n\n\n\n");
        assert_eq!(
            h.bg(10, 4),
            h.env().theme().color("surface").map(|s| s.mix(h.env().theme().color("raised").unwrap_or(s), 0.5))
        );
        h.click_text("Prune");
        assert!(h.app().pressed);
    }

    #[test]
    fn keyboard_moves_between_titles_and_toggles() {
        let mut h = Harness::new(demo([false; 3], false), 24, 12);
        h.set_reduced_motion(true);
        h.press("tab").press("down").press("down").press("enter");
        assert_eq!(h.app().open, vec![false, false, true]);
        assert!(h.screen().contains("ready"));
        h.press("home").press("space");
        assert_eq!(h.app().open, vec![true, false, true]);
        h.press("space");
        assert_eq!(h.app().open, vec![false, false, true]);
    }

    #[test]
    fn single_closes_the_others() {
        let mut h = Harness::new(demo([true, false, false], true), 24, 12);
        h.set_reduced_motion(true);
        h.click_text("Logs");
        assert_eq!(h.app().open, vec![false, false, true]);
    }

    #[test]
    fn the_chevron_stays_put_while_icon_and_title_slide() {
        let app = Demo { icons: true, ..demo([false; 3], false) };
        let mut h = Harness::new(app, 24, 6);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        assert_eq!(
            h.screen(),
            "  ▸ ■ Ports          3

  ▸ ■ Volumes

  ▸ ■ Logs

"
        );
        h.hover(10, 2);
        assert_eq!(h.screen().lines().nth(2), Some("▌ ▸  ■ Volumes"), "{}", h.screen());
        assert_eq!(h.screen().lines().next(), Some("  ▸ ■ Ports          3"), "the detail never moves");
        assert_eq!(h.fg(2, 2), h.env().theme().color("muted"), "the chevron keeps its own colour");

        let mut env = crate::env::Env::builtin();
        env.set_slide(false);
        let app = Demo { icons: true, ..demo([false; 3], false) };
        let mut h = Harness::with_env(app, env, 24, 6);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        h.hover(10, 2);
        assert_eq!(h.screen().lines().nth(2), Some("▌ ▸ ■ Volumes"), "{}", h.screen());
    }

    #[test]
    fn a_click_on_the_chevron_of_a_hovered_title_toggles() {
        let mut h = Harness::new(demo([false; 3], false), 24, 12);
        h.set_reduced_motion(true);
        h.hover(10, 2).click(2, 2);
        assert_eq!(h.app().open, vec![false, true, false]);
        assert_eq!(h.screen().lines().nth(2), Some("▌ ▾  Volumes"), "{}", h.screen());
    }

    #[test]
    fn narrow_titles_truncate_and_drop_the_detail() {
        let h = Harness::new(demo([false; 3], false), 12, 6);
        assert_eq!(h.screen().lines().next(), Some("  ▸ Ports"));
        let h = Harness::new(demo([false; 3], false), 10, 6);
        assert_eq!(h.screen().lines().nth(2), Some("  ▸ Vo…"));
    }

    #[test]
    fn ascii_mode_has_no_brackets() {
        let mut h = Harness::new(demo([true, false, false], false), 24, 10);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        let screen = h.screen();
        assert!(screen.starts_with("  v Ports"), "{screen}");
        assert!(!screen.contains(['[', ']', '(', ')']));
    }
}
