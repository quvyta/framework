//! Empty states: what an area says when it has nothing to show.

use super::Button;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, Node, PaintCx, Widget};

/// Widest a message line gets, so the explanation reads as a short paragraph on wide screens.
const MAX_WIDTH: u16 = 52;

/// A centred block explaining why an area is empty and what to do about it.
///
/// Reads, from top to bottom: an optional muted icon, a title, an optional explanation that wraps
/// to at most 52 cells, and an optional action button. When the area is short, the icon goes
/// first, then the space above the action, then explanation lines; the title and the action stay.
///
/// Style keys: `empty-state-icon` (`fg`), `empty-state-title` (`fg`, `bold`),
/// `empty-state-message` (`fg`).
pub struct EmptyState<Msg> {
    title: String,
    icon: Option<String>,
    message: Option<String>,
    action: Vec<Node<Msg>>,
}

impl<Msg: Clone + 'static> EmptyState<Msg> {
    /// An empty state reading `title`, e.g. "No containers yet".
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), icon: None, message: None, action: Vec::new() }
    }

    /// Icon key drawn above the title in a muted colour.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>) -> Self {
        self.icon = Some(key.into());
        self
    }

    /// One or two sentences under the title: why it is empty, or what will appear here.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// The way out, e.g. `Button::new("Create container").variant("primary").on_press(..)`.
    #[must_use]
    pub fn action(mut self, button: Button<Msg>) -> Self {
        self.action = vec![Node::new(button, 0)];
        self
    }
}

/// Which parts fit, and the message lines shown.
struct Plan {
    icon: bool,
    lines: Vec<String>,
    action_gap: bool,
    action: bool,
}

impl Plan {
    fn height(&self) -> u16 {
        let lines = clamp_u16(i32::try_from(self.lines.len()).unwrap_or(i32::MAX));
        u16::from(self.icon) * 2 + 1 + lines + u16::from(self.action_gap) + u16::from(self.action)
    }
}

impl<Msg: Clone + 'static> EmptyState<Msg> {
    fn plan(&self, width: u16, height: u16) -> Plan {
        let text_width = width.min(MAX_WIDTH);
        let lines = self.message.as_deref().map(|message| text::wrap(message, text_width)).unwrap_or_default();
        let action = !self.action.is_empty();
        let mut plan = Plan { icon: self.icon.is_some(), lines, action_gap: action, action };
        if plan.height() > height {
            plan.icon = false;
        }
        if plan.height() > height {
            plan.action_gap = false;
        }
        while plan.height() > height && !plan.lines.is_empty() {
            plan.lines.pop();
            if let Some(last) = plan.lines.last_mut() {
                // The explanation was cut: say so on its last visible line.
                let budget = text_width.saturating_sub(1);
                let cut = text::truncate(last, budget).into_owned();
                *last = if cut.ends_with(text::ELLIPSIS) { cut } else { format!("{cut}{}", text::ELLIPSIS) };
            }
        }
        if plan.height() > height {
            plan.action = false;
        }
        plan
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for EmptyState<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let plan = self.plan(available.width, u16::MAX);
        Size::new(available.width, plan.height()).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let plan = self.plan(area.width, area.height);
        let top = area.y + i32::from((area.height.saturating_sub(plan.height())) / 2);
        let mut y = top;
        let centred = |cx: &mut PaintCx<'_>, y: i32, line: &str, style: CellStyle| {
            let shown = text::truncate(line, area.width).into_owned();
            let x = area.x + i32::from((area.width - text::width(&shown)) / 2);
            cx.text(x, y, &shown, style, area.width);
        };

        if plan.icon
            && let Some(icon) = &self.icon
        {
            let glyph = cx.env().icons().glyph(icon).into_owned();
            let style = text_style(cx, "empty-state-icon", "muted");
            centred(cx, y, &glyph, style);
            y += 2;
        }
        let title_style = text_style(cx, "empty-state-title", "text");
        centred(cx, y, &self.title, title_style);
        y += 1;
        let message_style = text_style(cx, "empty-state-message", "dim");
        for line in &plan.lines {
            centred(cx, y, line, message_style);
            y += 1;
        }
        if plan.action_gap {
            y += 1;
        }
        if plan.action
            && let Some(button) = self.action.first()
        {
            let size = cx.measure_child(button, Size::new(area.width, 1));
            let x = area.x + i32::from((area.width - size.width) / 2);
            cx.paint_child(button, Rect::new(x, y, size.width, 1));
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.action
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.action
    }
}

/// The text style of `widget`, without a background, falling back to colour token `fallback`.
fn text_style(cx: &mut PaintCx<'_>, widget: &str, fallback: &str) -> CellStyle {
    let mut style = cx.style(widget, None, &[]).text();
    style.bg = None;
    style.fg = style.fg.or_else(|| Some(cx.color(fallback)));
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    #[derive(Default)]
    struct Demo {
        created: u32,
    }

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            self.created += 1;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(
                EmptyState::new("No containers")
                    .icon("dot-outline")
                    .message("Containers you run appear here.")
                    .action(Button::new("Run").on_press(())),
            )
            .fill()
            .id("empty");
        }
    }

    #[test]
    fn centres_icon_title_message_and_action() {
        let h = Harness::new(Demo::default(), 36, 9);
        assert_eq!(
            h.screen(),
            "\n                 ○\n\n           No containers\n  Containers you run appear here.\n\n                Run\n\n\n"
        );
        let theme = h.env().theme();
        assert_eq!(h.fg(17, 1), theme.color("muted"));
        assert!(h.is_bold(11, 3));
        assert_eq!(h.fg(2, 4), theme.color("dim"));
    }

    #[test]
    fn action_is_focusable_and_clickable() {
        let mut h = Harness::new(Demo::default(), 36, 9);
        h.press("tab").press("enter");
        assert_eq!(h.app().created, 1);
        h.click_text("Run");
        assert_eq!(h.app().created, 2);
    }

    #[test]
    fn short_and_narrow_areas_keep_title_and_action() {
        let h = Harness::new(Demo::default(), 20, 4);
        assert_eq!(h.screen(), "   No containers\n Containers you run\n    appear here.\n        Run\n");
        let tiny = Harness::new(Demo::default(), 20, 3);
        assert_eq!(tiny.screen(), "   No containers\nContainers you run…\n        Run\n");
    }
}
