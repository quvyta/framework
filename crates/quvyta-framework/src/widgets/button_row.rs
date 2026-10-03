//! A row of buttons that gives up its end to a menu when the row is too narrow.

use crate::env::Env;
use crate::event::{Event, KeyEvent, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, KeyChord};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, Node, PaintCx, Widget};

use super::button::Button;
use super::cells;
use super::popup_menu::{PopupAction, PopupMenu};
use super::press::{self, Press};

/// Cells between two buttons, the gap a dialog's action row keeps.
const GAP: u16 = 2;

/// The icon before the control's label, and the locale key of the label itself.
const MORE_ICON: &str = "chevron-down";
const MORE_LABEL: &str = "quvyta.button-row.more";

/// A row of buttons that becomes a menu when it is too narrow.
///
/// It is a row of [`Button`]s and nothing else: the buttons are the ones given, with their own
/// label, icon, shortcut, variant and disabled state, two cells apart as a dialog keeps its
/// action row. While they all fit there is no other control in the row, so the widget is a plain
/// row of buttons. When they do not fit, the last ones move, from the end, into a control that
/// stands where the first of them would have been and opens a menu of them; choosing an entry
/// sends that button's own message, as pressing the button itself would. A toolbar therefore
/// keeps every action at any width instead of dropping or cutting the ones at its end.
///
/// Keys: Tab reaches the buttons one after another and then the control that opens the menu;
/// Enter or Space opens its menu, ↑ ↓ Home End move in it, typing a letter jumps to an entry,
/// Enter chooses and Esc closes. A press on a button beside an open menu closes the menu and
/// still presses the button. A press on the control flashes it, as a button's does.
///
/// Style keys: the keys of [`Button`] for the buttons and for the menu's control, which takes
/// `hover`, `focus` and `pressed` like a button; `popup-menu`, `popup-item` and `popup-check`
/// for the menu. Icons: `chevron-down` before the control's label, whose words are the
/// `quvyta.button-row.more` locale key.
pub struct ButtonRow<Msg> {
    labels: Vec<String>,
    buttons: Vec<Node<Msg>>,
}

#[derive(Debug, Default)]
struct RowMemory {
    /// Where each button stands, `None` for the ones the menu holds.
    places: Vec<Option<Rect>>,
    /// The control that opens the menu, when some buttons do not fit.
    more: Option<Rect>,
}

/// Where the buttons of a row sit, and the control that opens the menu for the ones that do not
/// fit. Every button has a place, though only the ones with a rectangle are drawn.
struct Placed {
    places: Vec<Option<Rect>>,
    more: Option<Rect>,
}

impl<Msg: Clone + 'static> ButtonRow<Msg> {
    /// A row with no buttons in it yet.
    #[must_use]
    pub fn new() -> Self {
        Self { labels: Vec::new(), buttons: Vec::new() }
    }

    /// Adds a button at the end of the row, e.g. `Button::new("Save").on_press(Msg::Save)`.
    ///
    /// Call it once for each button. They keep the order they are added in, which is the order
    /// they are read in and the order Tab visits them; the first one is the primary choice, the
    /// one to mark `primary`, and the one the row and the menu reach first. Where they do not all
    /// fit, the last ones move into the menu, so the row stays the same row at every width.
    #[must_use]
    pub fn button(mut self, button: Button<Msg>) -> Self {
        let index = self.buttons.len();
        self.labels.push(button.label().to_owned());
        self.buttons.push(Node::new(button, index));
        self
    }

    /// The cells the buttons of `sizes` take in a row: the buttons and the gaps between them.
    fn row_width(sizes: &[Size]) -> u16 {
        let count = u16::try_from(sizes.len()).unwrap_or(u16::MAX);
        cells::sum(sizes.iter().map(|size| size.width)).saturating_add(count.saturating_sub(1) * GAP)
    }

    /// Where the buttons of `sizes` sit in `area`, with the control that opens the menu for the
    /// ones that do not fit after the last of them. It is `None` while every button fits, and
    /// the row then has no other control in it.
    fn placed(sizes: &[Size], more: u16, area: Rect) -> Placed {
        let height = sizes.iter().map(|size| size.height).max().unwrap_or(1);
        if Self::row_width(sizes) <= area.width {
            let (mut x, mut places) = (area.x, Vec::with_capacity(sizes.len()));
            for size in sizes {
                places.push(Some(Rect::new(x, area.y, size.width, height)));
                x += i32::from(size.width) + i32::from(GAP);
            }
            return Placed { places, more: None };
        }
        // The end of the row belongs to the control that opens the menu, so a button shows only
        // where the control still fits after it, a gap apart as two of them stand. The row's end
        // moves from the last button, so the first button that does not fit takes the rest of the
        // row with it; a row too narrow even for that shows the control alone, cut to the width
        // it has.
        let mut x = area.x;
        let mut places = vec![None; sizes.len()];
        for (index, size) in sizes.iter().enumerate() {
            if x + i32::from(cells::sum([size.width, GAP, more])) > area.right() {
                break;
            }
            places[index] = Some(Rect::new(x, area.y, size.width, height));
            x += i32::from(size.width) + i32::from(GAP);
        }
        let width = clamp_u16(area.right() - x);
        Placed { places, more: Some(Rect::new(x, area.y, width.min(more), height)) }
    }

    /// The cells the control that opens the menu takes: the padding of a button on both sides,
    /// the icon with a space after it and the label, the shape of a button with an icon.
    fn more_width(env: &Env) -> u16 {
        let (_, padding) = env.theme().style("button", None, &[]).pair("padding").unwrap_or((0, 2));
        cells::sum([
            padding.saturating_mul(2),
            text::width(&env.icons().glyph(MORE_ICON)) + 1,
            text::width(&Self::label(env)),
        ])
    }

    /// The words on the control that opens the menu.
    fn label(env: &Env) -> String {
        env.i18n().translate(MORE_LABEL, &[])
    }

    /// The press the keyboard gives a button, for one that is not on screen to be pressed itself.
    fn enter() -> Event {
        Event::Key(KeyEvent::from_chord(KeyChord::plain(Key::Enter)))
    }

    /// The buttons the menu of a row painted as `places` holds, and the words it lists them with,
    /// in the order they were added.
    fn menu_lists(&self, places: &[Option<Rect>]) -> (Vec<usize>, Vec<String>) {
        let index: Vec<usize> =
            places.iter().enumerate().filter_map(|(index, place)| place.is_none().then_some(index)).collect();
        let labels = index.iter().map(|index| self.labels[*index].clone()).collect();
        (index, labels)
    }

    /// The size each button asks for in a row `available` cells wide. Capped to the width offered,
    /// so a label too long for the row is cut instead of pushing the row past its edge.
    fn button_sizes<M: ButtonSizer<Msg>>(&self, cx: &mut M, available: Size) -> Vec<Size> {
        self.buttons.iter().map(|button| cx.size_of(button, available)).collect()
    }

    /// Paints the control that opens the menu of the buttons that did not fit. It is a button in
    /// every way a theme can see: the surface, the padding and the label of `button`, with the
    /// pillar the style raises on hover, on focus and under a press.
    fn paint_more(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let glyph = cx.env().icons().glyph(MORE_ICON).into_owned();
        let label = Self::label(cx.env());
        let open = PopupMenu::is_open_paint(cx);
        let mut states = Vec::new();
        if cx.pointer_within().is_some_and(|(x, y)| rect.contains(x, y)) {
            states.push(State::Hover);
        }
        // An open menu holds the keyboard, so the control it belongs to is the focused one
        // whether it was reached with the keyboard or with the pointer.
        if cx.is_focused() && (cx.is_focus_visible() || open) {
            states.push(State::Focus);
        }
        if cx.is_pressed() {
            states.push(State::Pressed);
        }
        let style = cx.style("button", None, &states);
        let padding = style.padding();
        let text_style = style.text();
        cx.clear(rect, text_style.bg.unwrap_or_else(|| cx.color("raised")));
        cx.register_hit(rect);
        if let Some(color) = style.color("pillar").filter(|_| padding.left >= 1) {
            cx.pillar(rect.x, rect.y + i32::from(padding.top), color);
        }
        let y = rect.y + i32::from(padding.top);
        let budget = rect.width.saturating_sub(padding.horizontal());
        let icon = text::width(&glyph) + 1;
        cx.text(
            rect.x + i32::from(padding.left),
            y,
            &glyph,
            CellStyle { bg: None, ..text_style },
            icon.saturating_sub(1),
        );
        let shown = text::truncate(&label, budget.saturating_sub(icon)).into_owned();
        cx.text(
            rect.x + i32::from(padding.left) + i32::from(icon),
            y,
            &shown,
            CellStyle { bg: None, ..text_style },
            budget,
        );
        if open {
            cx.request_overlay(rect);
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ButtonRow<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.buttons.is_empty() {
            return Size::new(0, 0);
        }
        let sizes = self.button_sizes(cx, available);
        let height = sizes.iter().map(|size| size.height).max().unwrap_or(1);
        Size::new(Self::row_width(&sizes).min(available.width), height).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        // A row with no room at all has no control either, so what the memory holds is emptied
        // with it: a menu opened before the row shrank closes at the next event.
        let mut placed = Placed { places: Vec::new(), more: None };
        if !self.buttons.is_empty() && !area.is_empty() {
            let sizes = self.button_sizes(cx, Size::new(area.width, area.height));
            placed = Self::placed(&sizes, Self::more_width(cx.env()), area);
            for (button, place) in self.buttons.iter().zip(&placed.places) {
                if let Some(rect) = place {
                    cx.paint_child(button, *rect);
                }
            }
            if let Some(rect) = placed.more {
                self.paint_more(cx, rect);
                // The row takes the focus for its own control only, and it takes it after its
                // buttons, so Tab walks the row from the left and reaches the menu at its end.
                cx.register_focusable();
            }
        }
        let memory = cx.memory::<RowMemory>();
        memory.places = placed.places;
        memory.more = placed.more;
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        if !PopupMenu::is_open_paint(cx) {
            return;
        }
        let places = cx.memory::<RowMemory>().places.clone();
        let (_, labels) = self.menu_lists(&places);
        PopupMenu::paint(cx, anchor, &labels, None);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let (places, more) = {
            let memory = cx.memory::<RowMemory>();
            (memory.places.clone(), memory.more)
        };
        if more.is_none() && PopupMenu::is_open(cx) {
            // The row grew while the menu was open and takes the control back with it.
            PopupMenu::close(cx);
        }
        if PopupMenu::is_open(cx) {
            let (index, labels) = self.menu_lists(&places);
            let area = cx.area();
            match PopupMenu::event(cx, event, &labels) {
                PopupAction::Chosen(row) => {
                    // A button in the menu is not on screen to be pressed, so it is given the
                    // press the keyboard gives it, and sends the message it would send itself.
                    if let Some(button) = index.get(row).map(|index| &self.buttons[*index]) {
                        cx.forward(button, area, &Self::enter());
                    }
                    return true;
                }
                PopupAction::Used | PopupAction::Closed => return true,
                PopupAction::Ignored => {}
            }
        }
        let Some(more) = more else {
            return false;
        };
        match event {
            Event::Key(key) if cx.is_focused() && (key.is_plain(Key::Enter) || key.is_plain(Key::Space)) => {}
            // A press that only begins or ends the one the control is showing is the control's.
            Event::Mouse(mouse)
                if matches!(mouse.kind, MouseKind::Down(_) | MouseKind::Up(_)) && more.contains(mouse.x, mouse.y) =>
            {
                match press::read(cx, event) {
                    Press::Ignored | Press::Used => return true,
                    Press::Key | Press::Click(..) => {}
                }
            }
            _ => return false,
        }
        PopupMenu::open(cx, 0);
        // The control is a button, and a press on a button is confirmed by flashing it.
        cx.flash();
        true
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.buttons
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.buttons
    }
}

impl<Msg: Clone + 'static> Default for ButtonRow<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

/// The one thing [`ButtonRow::button_sizes`] needs from a measuring or painting context, so both
/// read the buttons the same way.
trait ButtonSizer<Msg> {
    fn size_of(&mut self, button: &Node<Msg>, available: Size) -> Size;
}

impl<Msg: Clone + 'static> ButtonSizer<Msg> for MeasureCx<'_> {
    fn size_of(&mut self, button: &Node<Msg>, available: Size) -> Size {
        self.measure_child(button, available)
    }
}

impl<Msg: Clone + 'static> ButtonSizer<Msg> for PaintCx<'_> {
    fn size_of(&mut self, button: &Node<Msg>, available: Size) -> Size {
        self.measure_child(button, available)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    /// What a toolbar offers, one button each, in a row of the width under test.
    const ACTIONS: [&str; 6] = ["open", "save", "rename", "compare", "publish", "settings"];

    /// A toolbar whose buttons each send their own name.
    struct Toolbar {
        width: u16,
        heard: Vec<&'static str>,
    }

    impl App for Toolbar {
        type Msg = &'static str;

        fn update(&mut self, msg: &'static str) -> Command<&'static str> {
            self.heard.push(msg);
            Command::none()
        }

        fn view(&self, ui: &mut View<'_, &'static str>) {
            let mut row = ButtonRow::new();
            for action in ACTIONS {
                row = row.button(Button::new(action).on_press(action));
            }
            ui.add(row).width(Length::Cells(self.width)).id("bar");
        }
    }

    /// A toolbar as wide as its row, and tall enough for the menu of the buttons that do not fit.
    fn toolbar(width: u16) -> Harness<Toolbar> {
        Harness::new(Toolbar { width, heard: Vec::new() }, width, 6)
    }

    /// The time the menu takes to unfold, so a test looks at it where it settles.
    const OPEN: Duration = Duration::from_millis(300);

    /// The column the pillar stands in on the row, which is what the focus and the pointer show.
    fn pillar(h: &Harness<Toolbar>) -> Option<usize> {
        let screen = h.screen();
        let line = screen.lines().next().unwrap_or_default();
        line.find('▌').map(|byte| line[..byte].chars().count())
    }

    /// The column `text` is drawn in, as `find` gives it.
    fn column(h: &Harness<Toolbar>, text: &str) -> i32 {
        h.find(text).unwrap_or_else(|| panic!("`{text}` is not on screen:\n{}", h.screen())).0
    }

    #[test]
    fn a_row_wide_enough_for_its_buttons_is_a_plain_row_of_them() {
        let h = toolbar(72);
        let screen = h.screen();
        for action in ACTIONS {
            assert!(screen.contains(action), "{action} is on the row:\n{screen}");
        }
        assert!(!screen.contains("More"), "no other control while every button fits:\n{screen}");
        assert_eq!(column(&h, "open"), 2, "a button keeps its padding before the label");
        assert_eq!(column(&h, "save"), 12, "and the row keeps two cells between two of them");
    }

    #[test]
    fn a_row_too_narrow_gives_its_end_to_the_menu_of_the_rest() {
        let h = toolbar(30);
        let screen = h.screen();
        assert!(screen.contains("More"), "the control stands at the end of the row:\n{screen}");
        for waiting in ["rename", "compare", "publish", "settings"] {
            assert!(!screen.contains(waiting), "{waiting} waits in the menu:\n{screen}");
        }
        assert_eq!(column(&h, "open"), 2, "the buttons that fit keep their places");
        assert_eq!(column(&h, "save"), 12, "and the row's gap is the same in a narrow row");
    }

    #[test]
    fn choosing_a_button_in_the_menu_sends_the_message_it_sends_itself() {
        let mut h = toolbar(30);
        h.click_text("More").advance(OPEN);
        assert!(h.screen().contains("publish"), "the menu lists what did not fit:\n{}", h.screen());
        h.click_text("publish");
        assert_eq!(h.app().heard, ["publish"]);
        assert!(!h.screen().contains("publish"), "and the menu closed with the choice:\n{}", h.screen());
    }

    #[test]
    fn tab_walks_the_buttons_and_reaches_the_control_that_opens_the_menu() {
        let mut h = toolbar(30);
        h.press("tab");
        assert_eq!(pillar(&h), Some(0), "the first button has the focus:\n{}", h.screen());
        h.press("tab");
        assert_eq!(pillar(&h), Some(10), "then the second:\n{}", h.screen());
        h.press("tab");
        assert!(h.is_focused("bar"), "and the row itself, which is the menu's control:\n{}", h.screen());
        assert_eq!(pillar(&h), Some(20), "the pillar stands in the control's first cell:\n{}", h.screen());
        h.press("enter").advance(OPEN);
        assert!(h.screen().contains("rename"), "Enter opens the menu:\n{}", h.screen());
        h.press("enter");
        assert_eq!(h.app().heard, ["rename"], "and Enter takes the entry it stands on");
    }

    #[test]
    fn a_press_beside_an_open_menu_presses_the_button_it_landed_on() {
        let mut h = toolbar(30);
        h.click_text("More").advance(OPEN);
        h.click_text("open");
        assert_eq!(h.app().heard, ["open"]);
        assert!(!h.screen().contains("rename"), "the menu closed with the press:\n{}", h.screen());
    }

    /// A toolbar of two actions in a row `width` cells wide, for the widths a fixed list of
    /// actions does not reach.
    struct Pair {
        labels: [&'static str; 2],
        width: u16,
    }

    impl App for Pair {
        type Msg = &'static str;

        fn update(&mut self, _msg: &'static str) -> Command<&'static str> {
            Command::none()
        }

        fn view(&self, ui: &mut View<'_, &'static str>) {
            let mut row = ButtonRow::new();
            for action in self.labels {
                row = row.button(Button::new(action).on_press(action));
            }
            ui.add(row).width(Length::Cells(self.width)).id("bar");
        }
    }

    /// A toolbar of `labels` in a row `width` cells wide.
    fn pair(labels: [&'static str; 2], width: u16) -> Harness<Pair> {
        Harness::new(Pair { labels, width }, width, 6)
    }

    #[test]
    fn the_row_moves_its_end_from_the_end_and_never_leaves_a_hole() {
        // Neither button fits beside the control, so what is left of the row is the control
        // alone. A row that dropped the buttons one by one would keep the narrow "ok" and leave
        // a hole where the wide one stood.
        let h = pair(["open a file", "ok"], 20);
        let screen = h.screen();
        assert!(screen.contains("More"), "the control stands alone:\n{screen}");
        assert!(!screen.contains("ok"), "and no button from the middle of the row:\n{screen}");
    }

    #[test]
    fn a_button_gives_up_its_place_before_the_control_loses_its_label() {
        // Nineteen cells are one short of the first button, the gap and the control standing
        // together, so "Save" moves into the menu rather than the control being cut short of the
        // words on it.
        let h = pair(["Save", "Settings"], 19);
        let screen = h.screen();
        assert!(screen.contains("More"), "the control stands whole at the left of the row:\n{screen}");
        assert!(!screen.contains("Save"), "and the button that did not fit waits in the menu:\n{screen}");
    }

    #[test]
    fn the_control_is_a_button_like_any_other() {
        let mut h = toolbar(30);
        let (x, y) = h.find("More").expect("the control");
        // Two cells of padding, the icon and a space stand between the control's first cell and
        // its label, so the surface starts four cells to the left of the word.
        let cell = (u16::try_from(x - 4).expect("on screen"), u16::try_from(y).expect("on screen"));
        let (save_x, save_y) = h.find("save").expect("a button to compare with");
        let button = (u16::try_from(save_x).expect("on screen"), u16::try_from(save_y).expect("on screen"));
        assert_eq!(h.bg(cell.0, cell.1), h.bg(button.0, button.1), "at rest it is a surface, like a button");
        h.hover(x, y);
        let lit = h.bg(cell.0, cell.1);
        h.hover(save_x, save_y);
        assert_eq!(lit, h.bg(button.0, button.1), "which brightens as a button does:\n{}", h.screen());
        // The pointer off the row, so the pillar that is left is the focused control's own.
        h.hover(0, 5);
        h.press("tab").press("tab").press("tab");
        assert_eq!(pillar(&h), Some(20), "and the keyboard finds it at the end of the row:\n{}", h.screen());
    }

    /// The sum of a colour's channels, to compare how bright two tones are.
    fn brightness(color: Option<crate::color::Rgb>) -> u32 {
        color.map_or(0, |c| u32::from(c.r) + u32::from(c.g) + u32::from(c.b))
    }

    #[test]
    fn a_press_on_the_control_confirms_it_as_a_button_does() {
        let mut h = toolbar(30);
        let (x, y) = h.find("More").expect("the control");
        let cell = (u16::try_from(x - 4).expect("on screen"), u16::try_from(y).expect("on screen"));
        h.hover(x, y);
        let rested = brightness(h.bg(cell.0, cell.1));
        h.click(x, y);
        assert!(brightness(h.bg(cell.0, cell.1)) > rested, "the press flashes it one tone brighter:\n{}", h.screen());
        h.advance(Duration::from_millis(200));
        assert_eq!(brightness(h.bg(cell.0, cell.1)), rested, "and it settles back to the tone it had");
    }
}
