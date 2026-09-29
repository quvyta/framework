//! Empty states: what an area says when it has nothing to show.

use super::Button;
use super::cells;
use super::toast::ToastKind;
use crate::color::Rgb;
use crate::event::Event;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::widget::{EventCx, MeasureCx, Node, PaintCx, Widget};

/// Widest a message line gets, so the explanation reads as a short paragraph on wide screens.
const MAX_WIDTH: u16 = 52;

/// Cells between two buttons standing side by side, the gap a dialog keeps its action row at.
const ACTION_GAP: u16 = 2;

/// A centred block explaining why an area is empty and what to do about it.
///
/// Reads, from top to bottom: an optional muted icon, a title, an optional explanation that wraps
/// to at most 52 cells, and the action buttons. When the area is short, the icon goes first, then
/// the space above the buttons, then explanation lines; the title and the buttons stay.
///
/// [`action`](Self::action) may be called more than once: every call adds a button, and the first
/// one added is the primary choice in reading and focus order. They stand side by side, centred,
/// `ACTION_GAP` cells apart as in a dialog; where they do not fit the width they stand one under
/// another, centred, with a blank row between so their surfaces never merge. Tab and the arrow keys
/// visit them in the order they were added.
///
/// [`tone`](Self::tone) marks the block as a status: the icon and the title take the `success`,
/// `warning`, `danger` or `info` colour a [`Toast`](super::Toast) marks its kind with, and the
/// icon is the sign that goes with the colour, so an error is never a bare colour.
///
/// Style keys: `empty-state-icon` (`fg`), `empty-state-title` (`fg`, `bold`),
/// `empty-state-message` (`fg`).
pub struct EmptyState<Msg> {
    title: String,
    icon: Option<String>,
    message: Option<String>,
    tone: Option<ToastKind>,
    actions: Vec<Node<Msg>>,
}

impl<Msg: Clone + 'static> EmptyState<Msg> {
    /// An empty state reading `title`, e.g. "No containers yet".
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), icon: None, message: None, tone: None, actions: Vec::new() }
    }

    /// Icon key drawn above the title in a muted colour when no status tone supplies its sign.
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

    /// The status the block reports: the icon and the title take the `success`, `warning`,
    /// `danger` or `info` colour, and the icon is the sign that goes with it. Without a tone both
    /// keep the colours the theme gives them. A tone supplies its own sign in place of a custom
    /// [`icon`](Self::icon) key.
    #[must_use]
    pub fn tone(mut self, tone: ToastKind) -> Self {
        self.tone = Some(tone);
        self
    }

    /// The way out, e.g. `Button::new("Create container").variant("primary").on_press(..)`.
    ///
    /// Call it once for each of several equal choices, such as "Install here" beside "Show the
    /// command": they keep the order they are added in, so the first one is the primary, the one
    /// to mark `primary`, and the one Tab and the arrow keys reach first.
    #[must_use]
    pub fn action(mut self, button: Button<Msg>) -> Self {
        let index = self.actions.len();
        self.actions.push(Node::new(button, index));
        self
    }

    /// The sign above the title: a tone brings its own sign, while a state without one keeps the
    /// key given to [`icon`](Self::icon).
    fn icon_key(&self) -> Option<&str> {
        self.tone.map(ToastKind::icon).or(self.icon.as_deref())
    }
}

/// Which parts fit, and the message lines shown.
struct Plan {
    icon: bool,
    lines: Vec<String>,
    action_gap: bool,
    /// Rows the buttons take, zero when there is nowhere to put them.
    action_rows: u16,
}

impl Plan {
    fn height(&self) -> u16 {
        let lines = clamp_u16(i32::try_from(self.lines.len()).unwrap_or(i32::MAX));
        cells::sum([u16::from(self.icon) * 2, 1, lines, u16::from(self.action_gap), self.action_rows])
    }
}

/// Where the action buttons go under the message, in the order they were added.
///
/// They share one centred row while they fit `width`; where they do not they stand one under
/// another, each centred on its own width, a blank row apart so their surfaces never merge.
struct ActionRow {
    /// Rows the buttons take, the blank ones included.
    rows: u16,
    /// One rect per button: `x` counts cells from the left edge of the area, `y` rows down from
    /// the first button row.
    rects: Vec<Rect>,
}

impl ActionRow {
    fn place(sizes: &[Size], width: u16) -> Self {
        let count = u16::try_from(sizes.len()).unwrap_or(u16::MAX);
        if count == 0 {
            return Self { rows: 0, rects: Vec::new() };
        }
        let one_row =
            cells::sum(sizes.iter().map(|size| size.width)).saturating_add(ACTION_GAP.saturating_mul(count - 1));
        if one_row <= width || count == 1 {
            let row_width = one_row.min(width);
            let mut from_left = (width - row_width) / 2;
            let rects = sizes
                .iter()
                .map(|size| {
                    let button_width = size.width.min(width);
                    let rect = Rect::new(i32::from(from_left), 0, button_width, 1);
                    from_left = from_left.saturating_add(button_width).saturating_add(ACTION_GAP);
                    rect
                })
                .collect();
            return Self { rows: 1, rects };
        }
        let rects = sizes
            .iter()
            .enumerate()
            .map(|(index, size)| {
                let button_width = size.width.min(width);
                let left = (width - button_width) / 2;
                let down = i32::try_from(index).unwrap_or(i32::MAX).saturating_mul(2);
                Rect::new(i32::from(left), down, button_width, 1)
            })
            .collect();
        Self { rows: count.saturating_mul(2).saturating_sub(1), rects }
    }
}

impl<Msg: Clone + 'static> EmptyState<Msg> {
    /// Which parts of the block fit in `width` by `height`, given where the buttons were placed.
    fn plan(&self, width: u16, height: u16, actions: &ActionRow) -> Plan {
        let text_width = width.min(MAX_WIDTH);
        let lines = self.message.as_deref().map(|message| text::wrap(message, text_width)).unwrap_or_default();
        let mut plan =
            Plan { icon: self.icon_key().is_some(), lines, action_gap: actions.rows > 0, action_rows: actions.rows };
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
            plan.action_rows = 0;
        }
        plan
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for EmptyState<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let sizes = self.button_sizes(cx, available.width);
        let plan = self.plan(available.width, u16::MAX, &ActionRow::place(&sizes, available.width));
        Size::new(available.width, plan.height()).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let sizes = self.button_sizes(cx, area.width);
        let row = ActionRow::place(&sizes, area.width);
        let plan = self.plan(area.width, area.height, &row);
        let top = area.y + i32::from((area.height.saturating_sub(plan.height())) / 2);
        let mut y = top;
        let status = self.tone.map(|tone| cx.color(tone.name()));
        let centred = |cx: &mut PaintCx<'_>, y: i32, line: &str, style: CellStyle| {
            let shown = text::truncate(line, area.width).into_owned();
            let x = area.x + i32::from((area.width - text::width(&shown)) / 2);
            cx.text(x, y, &shown, style, area.width);
        };

        if plan.icon
            && let Some(icon) = self.icon_key()
        {
            let glyph = cx.env().icons().glyph(icon).into_owned();
            let style = text_style(cx, "empty-state-icon", "muted", status);
            centred(cx, y, &glyph, style);
            y += 2;
        }
        let title_style = text_style(cx, "empty-state-title", "text", status);
        centred(cx, y, &self.title, title_style);
        y += 1;
        let message_style = text_style(cx, "empty-state-message", "dim", None);
        for line in &plan.lines {
            centred(cx, y, line, message_style);
            y += 1;
        }
        if plan.action_gap {
            y += 1;
        }
        if plan.action_rows == 0 {
            return;
        }
        // Painted in the order they were added, so Tab reads them the way the eye does whether
        // they share a row or stand one under another.
        for (button, rect) in self.actions.iter().zip(&row.rects) {
            cx.paint_child(button, Rect::new(area.x + rect.x, y + rect.y, rect.width, rect.height));
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let Event::Key(key) = event else { return false };
        let step = if key.is_plain(Key::Left) || key.is_plain(Key::Up) {
            -1
        } else if key.is_plain(Key::Right) || key.is_plain(Key::Down) {
            1
        } else {
            return false;
        };
        if self.actions.len() < 2 || cx.focused_area().is_none() {
            return false;
        }
        cx.focus_step(step);
        true
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.actions
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.actions
    }
}

impl<Msg: Clone + 'static> EmptyState<Msg> {
    /// The size each button asks for in `width` cells: capped to the width it is given, so a
    /// label too long for the area is cut instead of pushing the row past the edge.
    fn button_sizes<M>(&self, cx: &mut M, width: u16) -> Vec<Size>
    where
        M: ButtonSizer<Msg>,
    {
        self.actions.iter().map(|action| cx.size_of(action, Size::new(width, 1))).collect()
    }
}

/// The one thing [`EmptyState::button_sizes`] needs from a painting or measuring context, so both
/// read the buttons the same way.
trait ButtonSizer<Msg> {
    fn size_of(&mut self, button: &Node<Msg>, available: Size) -> Size;
}

impl<Msg: 'static> ButtonSizer<Msg> for MeasureCx<'_> {
    fn size_of(&mut self, button: &Node<Msg>, available: Size) -> Size {
        self.measure_child(button, available)
    }
}

impl<Msg: 'static> ButtonSizer<Msg> for PaintCx<'_> {
    fn size_of(&mut self, button: &Node<Msg>, available: Size) -> Size {
        self.measure_child(button, available)
    }
}

/// The text style of `widget`, without a background, falling back to colour token `fallback`. A
/// tone takes the cells over, so the icon and the title read as one sign.
fn text_style(cx: &mut PaintCx<'_>, widget: &str, fallback: &str, tone: Option<Rgb>) -> CellStyle {
    let mut style = cx.style(widget, None, &[]).text();
    style.bg = None;
    style.fg = tone.or(style.fg).or_else(|| Some(cx.color(fallback)));
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

    /// Two equal ways out of a missing program, and how many of them were taken.
    #[derive(Default)]
    struct Missing {
        installed: u32,
        shown: u32,
    }

    #[derive(Clone, Copy)]
    enum MissingMsg {
        Install,
        Show,
    }

    impl App for Missing {
        type Msg = MissingMsg;
        fn update(&mut self, message: MissingMsg) -> Command<MissingMsg> {
            match message {
                MissingMsg::Install => self.installed += 1,
                MissingMsg::Show => self.shown += 1,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, MissingMsg>) {
            ui.add(
                EmptyState::new("ripgrep was not found")
                    .message("It comes with the packages of this system.")
                    .action(Button::new("Install here").variant("primary").on_press(MissingMsg::Install))
                    .action(Button::new("Show the command").on_press(MissingMsg::Show)),
            )
            .fill();
        }
    }

    /// The same failed index, with and without its status tone.
    #[derive(Default)]
    struct Failed {
        danger: bool,
    }

    impl App for Failed {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let mut empty = EmptyState::new("The index could not be read");
            if self.danger {
                empty = empty.tone(ToastKind::Danger);
            }
            ui.add(empty.message("Run quvyta index --repair to build it again.")).fill();
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

    /// The cell a label is drawn in, so a click lands on the button that carries it.
    fn cell(h: &Harness<impl App>, label: &str) -> (i32, i32) {
        let (x, y) = h.find(label).unwrap_or_else(|| panic!("`{label}` on screen:\n{}", h.screen()));
        (x, y)
    }

    #[test]
    fn two_buttons_share_a_row_and_each_answers_for_itself() {
        let h = Harness::new(Missing::default(), 46, 9);
        let (install, shown) = (cell(&h, "Install here"), cell(&h, "Show the command"));
        assert_eq!(install.1, shown.1, "side by side on one row:\n{}", h.screen());
        // Two cells of gap between the two surfaces, as a dialog keeps its action row.
        assert_eq!(shown.0 - install.0, i32::try_from("Install here".len()).unwrap_or(i32::MAX) + 6);
        let mut h = h;
        let (x, y) = install;
        h.click(x, y);
        assert_eq!((h.app().installed, h.app().shown), (1, 0));
        let (x, y) = cell(&h, "Show the command");
        h.click(x, y);
        assert_eq!((h.app().installed, h.app().shown), (1, 1));
    }

    #[test]
    fn tab_reaches_the_buttons_in_the_order_they_were_added() {
        let mut h = Harness::new(Missing::default(), 46, 9);
        h.press("tab").press("enter");
        assert_eq!((h.app().installed, h.app().shown), (1, 0), "the first button is first");
        h.press("tab").press("enter");
        assert_eq!((h.app().installed, h.app().shown), (1, 1));
        h.press("shift+tab").press("enter");
        assert_eq!((h.app().installed, h.app().shown), (2, 1));
    }

    #[test]
    fn arrows_move_between_the_buttons() {
        let mut row = Harness::new(Missing::default(), 46, 9);
        row.press("tab").press("right").press("enter");
        assert_eq!((row.app().installed, row.app().shown), (0, 1), "right reaches the second action");

        let mut column = Harness::new(Missing::default(), 24, 11);
        column.press("tab").press("down").press("enter");
        assert_eq!((column.app().installed, column.app().shown), (0, 1), "down reaches the stacked action");
    }

    #[test]
    fn a_narrow_area_stacks_the_buttons_and_both_still_answer() {
        let mut h = Harness::new(Missing::default(), 24, 11);
        let (install, shown) = (cell(&h, "Install here"), cell(&h, "Show the command"));
        assert!(shown.1 > install.1, "one under another:\n{}", h.screen());
        assert_eq!(shown.1 - install.1, 2, "a blank row between them:\n{}", h.screen());
        h.click(install.0, install.1);
        assert_eq!((h.app().installed, h.app().shown), (1, 0));
        h.click(shown.0, shown.1);
        assert_eq!((h.app().installed, h.app().shown), (1, 1));
    }

    #[test]
    fn a_short_area_keeps_the_title_and_all_the_actions() {
        let mut h = Harness::new(Missing::default(), 24, 5);
        let install = cell(&h, "Install here");
        let shown = cell(&h, "Show the command");
        assert!(install.1 < shown.1, "the actions stay in the short block:\n{}", h.screen());
        h.click(install.0, install.1);
        h.click(shown.0, shown.1);
        assert_eq!((h.app().installed, h.app().shown), (1, 1));
    }

    #[test]
    fn a_tone_paints_the_icon_and_the_title_in_the_status_colour() {
        let danger = Harness::new(Failed { danger: true }, 40, 6);
        let normal = Harness::new(Failed::default(), 40, 6);
        let title = "The index could not be read";
        let (title_x, title_y) = cell(&danger, title);
        let (x, y) = (u16::try_from(title_x).unwrap_or(0), u16::try_from(title_y).unwrap_or(0));
        let danger_colour = danger.env().theme().color("danger");
        assert_eq!(danger.fg(x, y), danger_colour, "the title in the danger colour");
        let (normal_x, normal_y) = cell(&normal, title);
        assert_eq!(
            normal.fg(u16::try_from(normal_x).unwrap_or(0), u16::try_from(normal_y).unwrap_or(0)),
            normal.env().theme().color("text"),
            "without a tone the title keeps its normal colour"
        );
        let icon_glyph = danger.env().icons().glyph("error").into_owned();
        let (icon_x, icon_y) = cell(&danger, &icon_glyph);
        assert_eq!(
            danger.fg(u16::try_from(icon_x).unwrap_or(0), u16::try_from(icon_y).unwrap_or(0)),
            danger_colour,
            "the icon is the sign that goes with it"
        );
        assert!(danger.is_bold(x, y), "the title keeps its weight");
    }

    #[test]
    fn a_tone_keeps_its_sign_in_ascii_and_sixteen_colours() {
        let mut h = Harness::new(Failed { danger: true }, 40, 6);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        let glyph = h.env().icons().glyph("error").into_owned();
        let (x, y) = cell(&h, &glyph);
        let (x, y) = (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0));
        assert_ne!(h.buffer()[(x, y)].symbol(), " ", "the ASCII sign is not blank: {}", h.screen());
        h.set_depth(crate::color::ColorDepth::Ansi16);
        assert_ne!(h.buffer()[(x, y)].symbol(), " ", "the sign remains in sixteen colours: {}", h.screen());
    }
}
