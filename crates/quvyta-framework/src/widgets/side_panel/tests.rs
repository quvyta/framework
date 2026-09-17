//! Tests of the side panel.

use super::*;
use crate::event::{MouseButton, MouseKind};
use crate::runtime::{App, Command, Harness};
use crate::theme::State;
use crate::widgets::{Button, Text};

#[derive(Clone)]
enum Msg {
    Toggle(bool),
    Resize(u16),
    Strip(u16),
    Press,
}

struct Demo {
    side: Side,
    open: bool,
    width: u16,
    toggle: bool,
    resize: bool,
    strip: bool,
    closed: Closed,
    view: u16,
    max: Option<u16>,
    events: Vec<String>,
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Toggle(open) => self.open = open,
            Msg::Resize(width) => self.width = width,
            Msg::Strip(index) => {
                self.view = index;
                self.open = true;
                self.events.push(format!("strip {index}"));
            }
            Msg::Press => self.events.push("press".to_owned()),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut panel =
            SidePanel::new(self.width).side(self.side).open(self.open).closed(self.closed).limits(6, self.max);
        if self.toggle {
            panel = panel.on_toggle(Msg::Toggle);
        }
        if self.resize {
            panel = panel.on_resize(Msg::Resize);
        }
        if self.strip {
            panel = panel.strip(["folder", "search"], Msg::Strip).active_view(self.view);
        }
        panel
            .panel(|ui| {
                ui.add(Text::new("files"));
            })
            .body(|ui| {
                ui.add(Button::new("Run").on_press(Msg::Press)).id("run");
            })
            .show(ui)
            .id("side");
    }
}

fn demo() -> Demo {
    Demo {
        side: Side::Left,
        open: true,
        width: 10,
        toggle: false,
        resize: false,
        strip: false,
        closed: Closed::Collapse,
        view: 0,
        max: Some(14),
        events: Vec::new(),
    }
}

/// The colour of `key` in `widget`'s style for `states`, at the start of any pulse.
fn paint(h: &Harness<Demo>, widget: &str, states: &[State], key: &str) -> Option<crate::color::Rgb> {
    h.env().theme().style(widget, None, states).paint(key).map(|paint| paint.at(0.0))
}

fn brightness(color: Option<crate::color::Rgb>) -> u32 {
    color.map_or(0, |c| u32::from(c.r) + u32::from(c.g) + u32::from(c.b))
}

fn row(h: &Harness<Demo>, y: usize) -> String {
    h.screen().lines().nth(y).unwrap_or_default().to_owned()
}

#[test]
fn plain_panel_is_a_surface_beside_the_body() {
    let h = Harness::new(demo(), 30, 2);
    assert_eq!(h.screen(), "files       Run\n\n");
    let theme = h.env().theme();
    let panel_bg = theme.style("side-panel", None, &[]).paint("bg").map(|paint| paint.at(0.0));
    assert_eq!(h.bg(9, 1), panel_bg);
    assert_eq!(h.bg(10, 1), theme.color("canvas"));
    let right = Harness::new(Demo { side: Side::Right, ..demo() }, 30, 2);
    assert_eq!(right.screen(), "  Run               files\n\n");
}

#[test]
fn hovering_the_edge_brightens_it_and_raises_the_toggle() {
    let mut h = Harness::new(Demo { toggle: true, ..demo() }, 30, 5);
    h.set_reduced_motion(true);
    let surface = paint(&h, "side-panel", &[], "bg");
    assert_eq!(h.bg(9, 0), surface);
    assert!(!h.screen().contains('‹') && !h.screen().contains('▌'), "at rest the edge is only a tone");

    h.hover(9, 0);
    assert_eq!(h.bg(9, 0), paint(&h, "split-handle", &[State::Hover], "bg"), "the edge brightens");
    assert_eq!(row(&h, 2), "          ‹", "two raised cells, the arrow reaching into the body, no pillar yet");
    assert_eq!(row(&h, 0), "files       Run");
    let shown = paint(&h, "side-toggle", &[], "bg");
    assert_eq!((h.bg(9, 2), h.bg(10, 2)), (shown, shown));
    assert_eq!(h.fg(10, 2), paint(&h, "side-toggle", &[], "fg"));

    h.hover(10, 2);
    let hovered = paint(&h, "side-toggle", &[State::Hover], "bg");
    assert_eq!((h.bg(9, 2), h.bg(10, 2)), (hovered, hovered), "pointing at the toggle raises it:\n{}", h.screen());
    assert_eq!(row(&h, 2), "         ▌‹", "the pillar left of the arrow, reaching into the body");
    assert_eq!(h.fg(9, 2), paint(&h, "side-toggle", &[State::Hover], "pillar"));

    h.mouse(MouseKind::Down(MouseButton::Left), 10, 2);
    let pressed = paint(&h, "side-toggle", &[State::Hover, State::Pressed], "bg");
    assert_eq!(h.bg(10, 2), pressed, "held down it is one tone brighter");
    assert!(brightness(shown) < brightness(hovered) && brightness(hovered) < brightness(pressed));
    assert!(h.app().open, "nothing happens before the release");
    h.mouse(MouseKind::Up(MouseButton::Left), 10, 2);
    assert!(!h.app().open);
    assert_eq!(row(&h, 0), "   Run", "one gutter column keeps the edge");
    assert!(!h.screen().contains('▌'), "the pointer is not on the moved edge");

    h.hover(0, 2);
    assert_eq!(row(&h, 2), "▌›", "closed: the toggle reaches from the gutter into the body");
    h.click(1, 2);
    assert!(h.app().open);
}

/// The toggle's two cells on row 2 of a left panel 10 wide: `(bg, fg)` of the pillar cell,
/// then of the arrow cell.
fn toggle_cells(h: &Harness<Demo>) -> [(Option<crate::color::Rgb>, Option<crate::color::Rgb>); 2] {
    [(h.bg(9, 2), h.fg(9, 2)), (h.bg(10, 2), h.fg(10, 2))]
}

#[test]
fn the_pillar_shows_only_on_the_button_or_with_keyboard_focus() {
    let mut h = Harness::new(Demo { toggle: true, ..demo() }, 30, 5);
    h.set_reduced_motion(true);

    // Edge hovered, button not: the edge brightens and the button keeps both cells, the first
    // one in the button's own tone with nothing drawn on it.
    h.hover(9, 4);
    assert_eq!(h.bg(9, 4), paint(&h, "split-handle", &[State::Hover], "bg"), "the edge brightens");
    assert_eq!(row(&h, 2), "          ‹");
    let shown = paint(&h, "side-toggle", &[], "bg");
    let [first, arrow] = toggle_cells(&h);
    assert_eq!(first.0, shown, "the first cell stays the button's tone, with no pillar on it");
    assert_eq!(arrow, (shown, paint(&h, "side-toggle", &[], "fg")));

    // Button hovered: the pillar appears in its hover colour, both cells one step up.
    h.hover(9, 2);
    assert_eq!(row(&h, 2), "         ▌‹");
    let hovered = paint(&h, "side-toggle", &[State::Hover], "bg");
    let [first, arrow] = toggle_cells(&h);
    assert_eq!(first, (hovered, paint(&h, "side-toggle", &[State::Hover], "pillar")));
    assert_eq!(arrow, (hovered, paint(&h, "side-toggle", &[State::Hover], "fg")));
    h.hover(10, 2);
    assert_eq!(row(&h, 2), "         ▌‹", "the arrow cell is the button too");

    // Keyboard focus, pointer away: the pillar stays and breathes.
    h.hover(25, 4);
    assert!(!h.screen().contains('‹'), "pointer gone: the toggle hides");
    h.press("tab");
    assert!(h.is_focused("side"));
    assert_eq!(row(&h, 2), "         ▌‹", "{}", h.screen());
    let focused = paint(&h, "side-toggle", &[State::Focus], "bg");
    let [first, arrow] = toggle_cells(&h);
    assert_eq!(first, (focused, paint(&h, "side-toggle", &[State::Focus], "pillar")));
    assert_eq!(arrow, (focused, paint(&h, "side-toggle", &[State::Focus], "fg")));
}

#[test]
fn a_press_on_the_toggle_never_resizes_and_a_drag_elsewhere_never_toggles() {
    let mut h = Harness::new(Demo { toggle: true, resize: true, ..demo() }, 30, 5);
    h.set_reduced_motion(true);
    h.hover(9, 2);
    h.mouse(MouseKind::Down(MouseButton::Left), 9, 2);
    h.mouse(MouseKind::Drag(MouseButton::Left), 10, 2);
    assert_eq!(h.app().width, 10, "sliding across the toggle is not a drag");
    h.mouse(MouseKind::Up(MouseButton::Left), 10, 2);
    assert!(!h.app().open, "released on the toggle: it toggles");
    h.send(Msg::Toggle(true));
    h.hover(9, 0);
    h.mouse(MouseKind::Down(MouseButton::Left), 9, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 11, 0);
    h.mouse(MouseKind::Up(MouseButton::Left), 11, 0);
    assert_eq!((h.app().width, h.app().open), (12, true));
}

#[test]
fn tab_shows_the_toggle_breathing_and_enter_flashes_it() {
    let mut h = Harness::new(Demo { toggle: true, side: Side::Right, ..demo() }, 30, 5);
    h.set_reduced_motion(true);
    assert!(!h.screen().contains('›'));
    h.press("tab");
    assert!(h.is_focused("side"));
    assert_eq!(row(&h, 2), format!("{}▌›", " ".repeat(19)), "the same toggle, pillar in the body's last cell");
    let focused = paint(&h, "side-toggle", &[State::Focus], "bg");
    assert_eq!((h.bg(19, 2), h.bg(20, 2)), (focused, focused));
    assert_eq!(h.fg(19, 2), paint(&h, "side-toggle", &[State::Focus], "pillar"));
    h.press("enter");
    assert!(!h.app().open);
    let pressed = paint(&h, "side-toggle", &[State::Focus, State::Pressed], "bg");
    assert_eq!(row(&h, 2), format!("{}▌‹", " ".repeat(28)), "{}", h.screen());
    assert_eq!(h.bg(29, 2), pressed, "Enter flashes the toggle one tone brighter");
    h.advance(h.env().theme().motion().flash);
    assert_eq!(h.bg(29, 2), focused);
}

#[test]
fn a_clicked_edge_stays_calm_and_ascii_draws_the_pillar_as_a_cell() {
    let mut h = Harness::new(Demo { toggle: true, resize: true, ..demo() }, 30, 5);
    h.set_reduced_motion(true).set_glyph_mode(crate::icons::GlyphMode::Ascii);
    h.hover(9, 0);
    assert_eq!(row(&h, 2), format!("{}<", " ".repeat(10)), "{}", h.screen());
    assert_eq!(h.bg(9, 2), paint(&h, "side-toggle", &[], "bg"), "the lit edge leaves the pillar cell plain");
    h.hover(9, 2);
    assert_eq!(h.bg(9, 2), paint(&h, "side-toggle", &[State::Hover], "pillar"), "ASCII fills the pillar cell");
    h.hover(9, 0);
    h.click(9, 0);
    assert!(h.is_focused("side") && h.app().open, "a click on the edge focuses without toggling");
    h.hover(20, 4);
    assert!(!h.screen().contains('<'), "focus by pointer does not keep the toggle up:\n{}", h.screen());
    assert_eq!(h.bg(9, 0), paint(&h, "side-panel", &[], "bg"), "nor leaves the edge tinted like a line");
    h.press("tab").press("shift+tab");
    assert_eq!(h.bg(9, 0), paint(&h, "split-handle", &[State::Focus], "bg"), "keyboard focus tints it");
}

#[test]
fn limits_can_leave_the_widest_open() {
    let mut h = Harness::new(Demo { toggle: true, resize: true, max: None, ..demo() }, 40, 5);
    h.set_reduced_motion(true);
    h.mouse(MouseKind::Down(MouseButton::Left), 9, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 35, 0);
    assert_eq!(h.app().width, 30, "no upper limit: only the body's quarter stops it");
    h.mouse(MouseKind::Drag(MouseButton::Left), 1, 0);
    assert_eq!(h.app().width, 6, "the lower limit still holds");
    h.mouse(MouseKind::Up(MouseButton::Left), 1, 0);
    h.press("end");
    assert_eq!(h.app().width, 30);
}

#[test]
fn closing_slides_the_panel_away() {
    let mut h = Harness::new(Demo { toggle: true, ..demo() }, 30, 3);
    h.send(Msg::Toggle(false));
    h.advance(Duration::from_millis(40));
    let x = h.find("Run").map(|(x, _)| x).unwrap_or_default();
    assert!(x > 3 && x < 12, "mid-slide the body is between its positions: {x}\n{}", h.screen());
    h.advance(Duration::from_millis(400));
    assert_eq!(h.find("Run"), Some((3, 0)));
}

#[test]
fn dragging_and_keys_resize_within_limits() {
    let mut h = Harness::new(Demo { toggle: true, resize: true, side: Side::Right, ..demo() }, 40, 3);
    h.set_reduced_motion(true);
    let edge = 40 - 10;
    h.mouse(MouseKind::Down(MouseButton::Left), edge, 1);
    h.mouse(MouseKind::Drag(MouseButton::Left), edge - 2, 1);
    assert_eq!(h.app().width, 12);
    h.mouse(MouseKind::Drag(MouseButton::Left), 2, 1);
    assert_eq!(h.app().width, 14);
    h.mouse(MouseKind::Up(MouseButton::Left), 2, 1);
    assert!(h.app().open, "a drag does not toggle");
    assert!(h.is_focused("side"), "pressing the edge focuses it:\n{}", h.screen());
    h.press("right").press("shift+right");
    assert_eq!(h.app().width, 8);
    h.press("enter");
    assert!(!h.app().open);
}

#[test]
fn toggle_shortcut_works_from_inside_and_strip_stays() {
    let mut h = Harness::new(Demo { toggle: true, strip: true, ..demo() }, 30, 5);
    h.set_reduced_motion(true);
    h.press("tab").press("tab").press("tab");
    assert!(h.is_focused("run"), "the edge, the strip, then the body");
    h.press("alt+b");
    assert!(!h.app().open);
    let screen = h.screen();
    assert!(screen.lines().nth(1).is_some_and(|l| l.starts_with(" ■")), "{screen}");
    assert!(screen.lines().nth(3).is_some_and(|l| l.starts_with(" ⌕")), "{screen}");
    h.hover(1, 3);
    assert_eq!(row(&h, 3), "▌⌕", "a pointed-at icon is raised with the pillar");
    assert_eq!(h.fg(0, 3), paint(&h, "side-strip-item", &[State::Hover], "pillar"));
    assert_eq!(h.bg(0, 3), paint(&h, "side-strip-item", &[State::Hover], "bg"));
    assert!(row(&h, 1).starts_with(" ■"), "the other icon stays calm");
    h.click(1, 3);
    assert_eq!(h.app().events, vec!["strip 1".to_owned()]);
    assert!(h.app().open);
}

/// A panel with toggle and strip, reduced motion, `width` × 6.
fn with_strip(demo: Demo, width: u16) -> Harness<Demo> {
    let mut h = Harness::new(Demo { toggle: true, strip: true, ..demo }, width, 6);
    h.set_reduced_motion(true);
    h
}

#[test]
fn the_strip_stays_beside_the_open_panel_and_marks_the_shown_view() {
    let h = with_strip(demo(), 30);
    assert_eq!(h.screen(), "    files       Run\n▌■\n\n ⌕\n\n\n", "strip, panel, body");
    let selected = [State::Selected];
    assert_eq!(row(&h, 1), "▌■", "the shown view's icon carries the pillar");
    assert_eq!(h.fg(0, 1), paint(&h, "side-strip-item", &selected, "pillar"));
    assert_eq!((h.bg(0, 1), h.bg(1, 1), h.bg(2, 1)), {
        let bg = paint(&h, "side-strip-item", &selected, "bg");
        (bg, bg, bg)
    });
    assert_eq!(h.fg(1, 1), paint(&h, "side-strip-item", &selected, "fg"));
    let strip = paint(&h, "side-strip", &[], "bg");
    assert_eq!((h.bg(0, 3), h.bg(3, 1)), (strip, strip), "the other icon and the spare column are the strip");
    assert!(
        brightness(strip) < brightness(paint(&h, "side-strip-item", &[State::Hover], "bg"))
            && brightness(paint(&h, "side-strip-item", &[State::Hover], "bg"))
                < brightness(paint(&h, "side-strip-item", &selected, "bg")),
        "strip < hovered icon < shown view"
    );
    assert_eq!(h.bg(4, 1), paint(&h, "side-panel", &[], "bg"));
    assert_eq!(h.bg(14, 1), h.env().theme().color("canvas"), "the body starts after strip and panel");

    let right = with_strip(Demo { side: Side::Right, ..demo() }, 30);
    assert_eq!(right.screen(), format!("  Run            files\n{0}▌■\n\n{0} ⌕\n\n\n", " ".repeat(27)));
    assert_eq!(right.fg(27, 1), paint(&right, "side-strip-item", &selected, "pillar"));
    assert_eq!(right.bg(26, 1), paint(&right, "side-strip", &[], "bg"), "the spare column faces the panel");

    let closed = with_strip(Demo { open: false, ..demo() }, 30);
    assert_eq!(row(&closed, 1), " ■", "a closed panel shows no view");
    assert_eq!(closed.fg(0, 1), None);
}

#[test]
fn clicking_icons_switches_views_and_the_shown_icon_closes() {
    let mut h = with_strip(demo(), 30);
    h.click(1, 3);
    assert_eq!((h.app().view, h.app().open), (1, true), "another icon switches the open panel's view");
    assert_eq!(row(&h, 1), " ■");
    assert_eq!(row(&h, 3), "▌⌕", "{}", h.screen());
    assert_eq!(h.bg(0, 3), paint(&h, "side-strip-item", &[State::Hover, State::Selected], "bg"));
    h.click(1, 3);
    assert!(!h.app().open, "the shown view's icon closes the panel");
    assert_eq!(h.app().events, vec!["strip 1".to_owned()], "closing goes through on_toggle");
    assert_eq!(row(&h, 0), "      Run", "collapsed to the strip");
    assert_eq!(row(&h, 3), "▌⌕", "still hovered, no longer selected");
    assert_eq!(h.bg(0, 3), paint(&h, "side-strip-item", &[State::Hover], "bg"));
    h.click(1, 1);
    assert_eq!((h.app().view, h.app().open), (0, true), "any icon opens a closed panel on its view");
    assert_eq!(h.screen(), "    files       Run\n▌■\n\n ⌕\n\n\n");
    assert_eq!(h.bg(0, 1), paint(&h, "side-strip-item", &[State::Hover, State::Selected], "bg"));
    assert_eq!(h.fg(0, 1), paint(&h, "side-strip-item", &[State::Selected], "pillar"), "a click never breathes it");
}

#[test]
fn the_keyboard_moves_along_the_strip() {
    let mut h = with_strip(demo(), 30);
    h.press("tab").press("tab");
    assert!(!h.is_focused("side") && !h.is_focused("run"), "the strip comes after the edge");
    let focused = [State::Selected, State::Focus];
    assert_eq!(row(&h, 1), "▌■", "focus lands on the shown view");
    assert_eq!(h.fg(0, 1), paint(&h, "side-strip-item", &focused, "pillar"), "and its pillar breathes");
    h.press("down");
    assert_eq!(row(&h, 3), "▌⌕");
    assert_eq!(h.fg(0, 3), paint(&h, "side-strip-item", &[State::Focus], "pillar"));
    assert_eq!(h.bg(0, 3), paint(&h, "side-strip-item", &[State::Focus], "bg"));
    assert_eq!(h.fg(0, 1), paint(&h, "side-strip-item", &[State::Selected], "pillar"), "the shown view stays calm");
    h.press("enter");
    assert_eq!((h.app().view, h.app().open), (1, true));
    assert_eq!(row(&h, 1), " ■");
    assert_eq!(h.bg(0, 3), paint(&h, "side-strip-item", &[State::Selected, State::Focus, State::Pressed], "bg"));
    h.advance(h.env().theme().motion().flash);
    h.press("space");
    assert!(!h.app().open, "Space on the shown view closes");
    h.press("up").press("enter");
    assert_eq!((h.app().view, h.app().open), (0, true));
    h.press("alt+b");
    assert!(!h.app().open, "the shortcut works from the strip");
}

#[test]
fn hide_leaves_an_invisible_edge_that_opens_the_panel() {
    for side in [Side::Left, Side::Right] {
        let mut h = with_strip(Demo { side, closed: Closed::Hide, view: 1, ..demo() }, 30);
        h.send(Msg::Toggle(false));
        assert!(!h.app().open, "{side:?} {}", h.screen());
        let (edge, pillar_cell): (u16, u16) = if side == Side::Left { (0, 0) } else { (29, 28) };
        let body = if side == Side::Left { "   Run" } else { "  Run" };
        assert_eq!(h.screen(), format!("{body}\n\n\n\n\n\n"), "{side:?}: no strip, no panel");
        let canvas = h.env().theme().color("canvas");
        assert_eq!((h.bg(edge, 0), h.bg(edge, 3)), (canvas, canvas), "at rest the edge is the body's tone");

        h.hover(i32::from(edge), 0);
        assert_eq!(h.bg(edge, 0), paint(&h, "split-handle", &[State::Hover], "bg"), "the edge lights up");
        let arrow = if side == Side::Left { "›" } else { "‹" };
        let toggle_row: String = row(&h, 3).chars().skip(usize::from(pillar_cell)).collect();
        assert_eq!(toggle_row, format!(" {arrow}"), "{side:?}: the toggle without its pillar");
        assert_eq!(h.bg(pillar_cell, 3), paint(&h, "side-toggle", &[], "bg"));

        let arrow_x = i32::from(pillar_cell) + 1;
        h.hover(arrow_x, 3);
        let toggle_row: String = row(&h, 3).chars().skip(usize::from(pillar_cell)).collect();
        assert_eq!(toggle_row, format!("▌{arrow}"), "{side:?}: on the button the pillar shows");
        assert_eq!(h.fg(pillar_cell, 3), paint(&h, "side-toggle", &[State::Hover], "pillar"));

        h.click(arrow_x, 3);
        assert!(h.app().open, "{side:?}");
        let icon: u16 = if side == Side::Left { 1 } else { 28 };
        assert_eq!(h.find("⌕").map(|(x, _)| x), Some(i32::from(icon)), "{side:?}: the strip is back:\n{}", h.screen());
        assert_eq!(h.fg(icon - 1, 3), paint(&h, "side-strip-item", &[State::Selected], "pillar"), "on the last view");
    }
}

#[test]
fn hiding_the_focused_strip_hands_focus_to_the_edge() {
    let mut h = with_strip(Demo { closed: Closed::Hide, ..demo() }, 30);
    h.press("tab").press("tab").press("enter");
    assert!(!h.app().open);
    assert!(h.is_focused("side"), "the edge takes focus:\n{}", h.screen());
    assert_eq!(row(&h, 3), "▌›", "and shows its toggle breathing");
    h.press("enter");
    assert!(h.app().open);
}

#[test]
fn dragging_a_closed_edge_opens_the_panel() {
    for closed in [Closed::Collapse, Closed::Hide] {
        let mut h = with_strip(Demo { closed, open: false, resize: true, max: None, ..demo() }, 40);
        let edge = if closed == Closed::Hide { 0 } else { 3 };
        h.mouse(MouseKind::Down(MouseButton::Left), edge, 0);
        h.mouse(MouseKind::Drag(MouseButton::Left), edge + 2, 0);
        assert!(!h.app().open, "{closed:?}: a short drag only holds");
        h.mouse(MouseKind::Drag(MouseButton::Left), 20, 0);
        assert_eq!((h.app().open, h.app().width), (true, 17), "{closed:?}: past half the minimum it opens there");
        h.mouse(MouseKind::Drag(MouseButton::Left), 24, 0);
        assert_eq!(h.app().width, 21, "{closed:?}: and keeps resizing");
        h.mouse(MouseKind::Up(MouseButton::Left), 24, 0);
        assert_eq!(row(&h, 0).find("files"), Some(4), "{closed:?}:\n{}", h.screen());
    }
    let mut h = with_strip(Demo { side: Side::Right, closed: Closed::Hide, open: false, resize: true, ..demo() }, 40);
    h.mouse(MouseKind::Down(MouseButton::Left), 39, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 27, 0);
    assert_eq!((h.app().open, h.app().width), (true, 9), "right: the strip's columns are not the panel's");
}

#[test]
fn a_narrow_area_keeps_the_strip_and_a_quarter_for_the_body() {
    let h = with_strip(demo(), 16);
    // 16 columns: the body keeps 4, the strip takes 4, the panel gets the other 8.
    assert_eq!(row(&h, 1), "▌■");
    assert_eq!(h.bg(11, 1), paint(&h, "side-panel", &[], "bg"));
    assert_eq!(h.bg(12, 1), h.env().theme().color("canvas"), "{}", h.screen());
    let tiny = with_strip(Demo { open: false, ..demo() }, 3);
    assert_eq!(tiny.screen().lines().nth(1), Some(" ■"), "a strip wider than the area is cut, not overflowing");
}

#[test]
fn hide_slides_the_strip_away_with_the_panel_unless_motion_is_reduced() {
    let mut h = Harness::new(Demo { toggle: true, strip: true, closed: Closed::Hide, ..demo() }, 30, 6);
    h.send(Msg::Toggle(false));
    h.advance(Duration::from_millis(30));
    let x = h.find("Run").map_or(0, |(x, _)| x);
    assert!(x > 3 && x < 16, "mid-slide the body is between its positions: {x}\n{}", h.screen());
    assert!(!h.screen().contains('■'), "the strip leaves first, from the outer edge:\n{}", h.screen());
    h.advance(Duration::from_millis(400));
    assert_eq!(h.find("Run"), Some((3, 0)));

    h.set_reduced_motion(true);
    h.send(Msg::Toggle(true));
    assert_eq!(row(&h, 0), "    files       Run", "reduced motion: at once");
}
