use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::*;
use crate::color::Rgb;
use crate::event::{MouseButton, MouseKind};
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widgets::{Button, ContextItem, Text};

/// A store page: `count` apps in a grid that fills the screen.
struct Store {
    count: usize,
    selected: Option<usize>,
    opened: Vec<usize>,
    checked: Option<Vec<bool>>,
    disabled: bool,
    /// Cards built since the counter was last reset.
    built: Rc<Cell<usize>>,
    /// Whether every card carries a menu of its own.
    menu: bool,
    /// The cards a menu entry was chosen on, in order.
    removed: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Select(usize),
    Open(usize),
    Toggle(usize),
    Browse,
    Remove(usize),
}

impl App for Store {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(index) => self.selected = Some(index),
            Msg::Open(index) => self.opened.push(index),
            Msg::Toggle(index) => {
                if let Some(checked) = &mut self.checked {
                    checked[index] = !checked[index];
                }
            }
            Msg::Browse => self.opened.push(usize::MAX),
            Msg::Remove(index) => self.removed.push(index),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let built = Rc::clone(&self.built);
        let mut grid = CardGrid::new(self.count)
            .selected(self.selected)
            .disabled(self.disabled)
            .on_select(Msg::Select)
            .on_activate(Msg::Open)
            .on_toggle(Msg::Toggle)
            .empty(EmptyState::new("No apps match").action(Button::new("Browse all").on_press(Msg::Browse)))
            .card(move |ui, index| {
                built.set(built.get() + 1);
                ui.add(Text::new(format!("app {index}")).role("title").no_wrap());
                ui.add(Text::new("a tool that does one thing well").role("secondary").no_wrap());
                ui.add(Text::new("Repo").role("faint").no_wrap());
            });
        if let Some(checked) = &self.checked {
            grid = grid.checked(checked.clone());
        }
        if self.menu {
            grid = grid.context_menu(|index| vec![ContextItem::new(format!("Remove app {index}"), Msg::Remove(index))]);
        }
        ui.add(grid).fill().id("grid");
    }
}

fn store(count: usize) -> Store {
    Store {
        count,
        selected: None,
        opened: Vec::new(),
        checked: None,
        disabled: false,
        built: Rc::new(Cell::new(0)),
        menu: false,
        removed: Vec::new(),
    }
}

/// A grid whose cards each carry a menu, with the motion off so a menu is there at once.
fn menu_store(count: usize) -> Harness<Store> {
    let mut store = store(count);
    store.menu = true;
    let mut h = Harness::new(store, 80, 16);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h
}

/// Right-clicks the cell at `(x, y)`.
fn right_click(h: &mut Harness<Store>, x: i32, y: i32) {
    h.mouse(MouseKind::Down(MouseButton::Right), x, y);
    h.mouse(MouseKind::Up(MouseButton::Right), x, y);
    h.render();
}

#[test]
fn a_cards_menu_acts_on_the_card_that_was_right_clicked_and_not_on_the_selected_one() {
    let mut h = menu_store(12);
    h.send(Msg::Select(0));
    let (x, y) = h.find("app 5").expect("the card is on screen");
    right_click(&mut h, x, y);
    assert!(h.screen().contains("Remove app 5"), "the menu is the card's own:\n{}", h.screen());
    assert_eq!(h.app().selected, Some(5), "the card the menu belongs to became the selection");
    h.click_text("Remove app 5").render();
    assert_eq!(h.app().removed, vec![5], "the entry acts on the card that was clicked");
}

#[test]
fn the_menu_key_opens_the_menu_of_the_card_the_keys_are_on() {
    let mut h = menu_store(12);
    h.press("tab").press("right").press("right");
    assert_eq!(h.app().selected, Some(1));
    h.press("menu").render();
    assert!(h.screen().contains("Remove app 1"), "{}", h.screen());
    h.press("enter").render();
    assert_eq!(h.app().removed, vec![1]);
}

#[test]
fn the_menu_key_scrolls_the_card_it_opens_on_into_view() {
    let mut h = menu_store(400);
    h.press("tab").press("end");
    assert_eq!(h.app().selected, Some(399));
    h.press("menu").render();
    assert!(h.screen().contains("Remove app 399"), "{}", h.screen());
}

#[test]
fn a_right_press_beside_the_cards_opens_nothing_and_one_on_another_card_moves_the_menu() {
    let mut h = menu_store(4);
    let (x, y) = h.find("app 1").expect("the card is on screen");
    right_click(&mut h, x, y);
    assert!(h.screen().contains("Remove app 1"), "{}", h.screen());
    let (other_x, other_y) = h.find("app 3").expect("another card");
    right_click(&mut h, other_x, other_y);
    assert!(h.screen().contains("Remove app 3"), "the other card's menu took its place:\n{}", h.screen());
    right_click(&mut h, 1, 15);
    assert!(!h.screen().contains("Remove app"), "below the cards there is none:\n{}", h.screen());
    assert!(h.app().removed.is_empty(), "nothing was chosen");
}

#[test]
fn a_menu_card_keeps_its_surface_raised_while_its_menu_is_open() {
    let mut h = menu_store(12);
    let (x, y) = h.find("app 5").expect("the card is on screen");
    let (cell_x, cell_y) = (u16::try_from(x).expect("on screen"), u16::try_from(y).expect("on screen"));
    let resting = h.bg(cell_x, cell_y);
    right_click(&mut h, x, y);
    assert_ne!(h.bg(cell_x, cell_y), resting, "the card the menu acts on stays lit:\n{}", h.screen());
}

#[test]
fn a_disabled_grid_opens_no_menu() {
    let mut store = store(12);
    store.menu = true;
    store.disabled = true;
    let mut h = Harness::new(store, 80, 16);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    right_click(&mut h, 4, 1);
    assert!(!h.screen().contains("Remove app"), "{}", h.screen());
}

fn color(h: &Harness<Store>, token: &str) -> Rgb {
    h.env().theme().color(token).expect("token")
}

/// The card names on line `row` of the screen, left to right.
fn names_on(h: &Harness<Store>, row: usize) -> Vec<String> {
    let screen = h.screen();
    let line = screen.lines().nth(row).unwrap_or_default().to_owned();
    line.split("app ").skip(1).map(|rest| format!("app {}", rest.split_whitespace().next().unwrap_or(""))).collect()
}

#[test]
fn the_column_count_follows_the_width() {
    let h = Harness::new(store(12), 96, 20);
    assert_eq!(names_on(&h, 0), ["app 0", "app 1", "app 2"], "{}", h.screen());
    assert!(h.screen().lines().next().is_some_and(|line| line.starts_with("  app 0")), "{}", h.screen());
    let h = Harness::new(store(12), 140, 20);
    assert_eq!(names_on(&h, 0), ["app 0", "app 1", "app 2", "app 3", "app 4"], "{}", h.screen());
    // Cards are 26 cells wide and two apart at 140 columns: the second starts at 28.
    assert_eq!(h.screen().lines().next().and_then(|line| line.find("app 1")), Some(30));
    assert_eq!(h.bg(27, 0), Some(color(&h, "canvas")), "the gap between cards is the page");
    assert_eq!(h.bg(28, 0), Some(color(&h, "raised")), "a card rests on the raised tone");
    assert_eq!(h.bg(0, 3), Some(color(&h, "canvas")), "the gap between rows");
}

#[test]
fn arrows_move_in_two_dimensions_and_stop_at_the_edges() {
    let mut h = Harness::new(store(8), 96, 20);
    h.press("tab").press("right");
    assert_eq!(h.app().selected, Some(0), "the first key picks the first card");
    h.press("right").press("right").press("right");
    assert_eq!(h.app().selected, Some(2), "the end of a row does not wrap");
    h.press("down").press("down");
    assert_eq!(h.app().selected, Some(7), "a short last row gives its last card");
    h.press("right");
    assert_eq!(h.app().selected, Some(7), "the last card stops");
    h.press("left").press("left").press("left");
    assert_eq!(h.app().selected, Some(6), "the start of a row does not wrap back");
    h.press("up").press("up").press("up");
    assert_eq!(h.app().selected, Some(0));
    h.press("end");
    assert_eq!(h.app().selected, Some(7));
    h.press("home");
    assert_eq!(h.app().selected, Some(0));
}

#[test]
fn the_selected_card_stays_in_view_while_moving_and_the_wheel_scrolls() {
    let mut h = Harness::new(store(1000), 96, 12);
    h.press("tab").press("end");
    assert_eq!(h.app().selected, Some(999));
    assert!(h.screen().contains("app 999"), "{}", h.screen());
    assert!(!crate::widgets::scrollbar::column(&h, 95).contains(' '), "the scrollbar is drawn");
    h.press("pgup");
    assert_eq!(h.app().selected, Some(990), "a page is the three rows that fit");
    assert!(h.screen().contains("app 990"), "{}", h.screen());
    h.press("home");
    assert!(h.screen().starts_with("▌ app 0"), "{}", h.screen());
    h.mouse(MouseKind::ScrollDown, 10, 5);
    assert!(!h.screen().contains("app 0 "), "{}", h.screen());
    assert!(h.screen().contains("app 3"), "{}", h.screen());
    h.press("right");
    assert!(h.screen().contains("app 1"), "moving brings the selection back into view:\n{}", h.screen());
}

#[test]
fn dragging_the_scrollbar_scrolls_the_rows() {
    let mut h = Harness::new(store(300), 96, 12);
    h.mouse(MouseKind::Down(MouseButton::Left), 95, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 95, 11);
    h.mouse(MouseKind::Up(MouseButton::Left), 95, 11);
    assert!(h.screen().contains("app 299"), "{}", h.screen());
    assert_eq!(h.app().selected, None, "scrolling selects nothing");
}

#[test]
fn ten_thousand_cards_build_and_draw_only_the_screen() {
    let mut h = Harness::new(store(10_000), 160, 48);
    h.press("tab").press("end");
    let built = Rc::clone(&h.app().built);
    built.set(0);
    let frames = 20;
    let started = Instant::now();
    for _ in 0..frames {
        h.render();
    }
    let per_frame = started.elapsed() / frames;
    // 160 columns hold six cards a row, 48 rows hold twelve rows of cards: 72 on screen.
    let per_frame_built = built.get() / usize::try_from(frames).unwrap_or(1);
    assert!(per_frame_built <= 72, "only the cards on screen are built: {per_frame_built}");
    assert!(per_frame_built >= 60, "and all of them are: {per_frame_built}");
    assert!(h.screen().contains("app 9999"));
    // Debug builds on a busy machine included; the release benchmark measures the real cost.
    assert!(per_frame < Duration::from_millis(200), "a frame took {per_frame:?}");
}

#[test]
fn a_click_selects_and_opens_and_enter_opens() {
    let mut h = Harness::new(store(8), 96, 20);
    h.click_text("app 4");
    assert_eq!(h.app().selected, Some(4));
    assert_eq!(h.app().opened, [4]);
    h.click(31, 3);
    assert_eq!(h.app().opened, [4], "a click in a gap does nothing");
    h.press("left").press("enter");
    assert_eq!(h.app().selected, Some(3));
    assert_eq!(h.app().opened, [4, 3]);
    h.press("space");
    assert_eq!(h.app().opened, [4, 3, 3], "without checks Space opens too");
}

#[test]
fn space_checks_and_a_checked_card_carries_the_mark_in_its_corner() {
    let mut h = Harness::new(Store { checked: Some(vec![false; 6]), ..store(6) }, 96, 12);
    h.press("tab").press("right").press("right").press("space");
    assert_eq!(h.app().checked.as_deref(), Some(&[false, true, false, false, false, false][..]));
    assert!(h.app().opened.is_empty(), "Space only checks");
    let line = h.screen().lines().next().unwrap_or_default().to_owned();
    // The second card spans cells 32 to 61; its mark sits two cells in from the right edge.
    assert_eq!(line.chars().nth(60), Some('✓'), "{line}");
    assert_eq!(h.fg(60, 0), Some(color(&h, "accent")));
    h.hover(60, 2).hover(10, 1);
    assert_eq!(h.screen().lines().next().and_then(|l| l.chars().nth(28)), Some('✓'), "a lit card offers a faint mark");
    assert_eq!(h.fg(28, 0), Some(color(&h, "muted")));
    h.click(28, 0);
    assert_eq!(h.app().checked.as_deref(), Some(&[true, true, false, false, false, false][..]));
    assert!(h.app().opened.is_empty(), "a click on the mark only checks");
    h.click(10, 1);
    assert_eq!(h.app().opened, [0], "a click elsewhere on the card opens it");
}

#[test]
fn a_narrow_area_is_one_column_as_wide_as_the_area_with_content_cut() {
    let h = Harness::new(store(3), 16, 12);
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert_eq!(lines[0], "  app 0", "{screen}");
    assert_eq!(lines[1], "  a tool that…", "{screen}");
    assert_eq!(lines[4], "  app 1", "{screen}");
    assert_eq!(h.bg(15, 0), Some(color(&h, "raised")), "the card takes the whole width");
}

#[test]
fn ascii_mode_keeps_marks_as_letters_and_the_pillar_as_a_cell() {
    let mut h = Harness::new(Store { checked: Some(vec![true, false]), ..store(2) }, 96, 8);
    h.set_glyph_mode(GlyphMode::Ascii);
    h.press("tab").press("right");
    let screen = h.screen();
    let first = screen.lines().next().unwrap_or_default();
    assert_eq!(first.chars().nth(28), Some('v'), "{screen}");
    for bracket in ['[', ']', '(', ')', '{', '}', '|'] {
        assert!(!screen.contains(bracket), "{bracket} in:\n{screen}");
    }
    assert_eq!(h.bg(10, 1), Some(color(&h, "active")), "the selection shows by surface alone");
    assert_eq!(first.chars().next(), Some(' '), "{screen}");
    assert_ne!(h.bg(0, 0), h.bg(1, 0), "the pillar is a cell of colour");
}

#[test]
fn the_pointer_and_the_keys_never_light_two_cards() {
    let mut h = Harness::new(store(8), 96, 20);
    let active = color(&h, "active");
    let hover = color(&h, "raised").mix(color(&h, "text"), 0.08);
    h.press("tab").press("right");
    assert_eq!(h.bg(10, 1), Some(active), "the keys light the first card");
    h.hover(40, 1);
    assert_eq!(h.bg(40, 1), Some(hover), "the pointer lights the card under it");
    assert_eq!(h.bg(10, 1), Some(color(&h, "raised")), "and the selected card rests");
    h.press("down");
    assert_eq!(h.app().selected, Some(4), "the key goes on from the card under the pointer");
    assert_eq!(h.bg(40, 5), Some(active));
    assert_eq!(h.bg(40, 1), Some(color(&h, "raised")), "a resting pointer does not keep its card lit");
    h.hover(40, 9);
    assert_eq!(h.bg(40, 5), Some(color(&h, "raised")), "a pointer moving over the grid keeps the highlight");
    h.hover(500, 500);
    assert_eq!(h.bg(40, 5), Some(active), "the pointer leaving the grid hands it back to the selection");
}

#[test]
fn keyboard_focus_breathes_the_pillar_and_reduced_motion_holds_it() {
    let mut h = Harness::new(store(4), 96, 8);
    h.press("tab").press("right");
    assert_eq!(h.screen().lines().map(|l| l.starts_with('▌')).collect::<Vec<_>>()[..3], [true, true, true]);
    let start = h.fg(0, 0);
    h.advance(h.env().theme().motion().pulse_period / 2);
    assert_ne!(h.fg(0, 0), start, "the pillar breathes");

    let mut h = Harness::new(store(4), 96, 8);
    h.set_reduced_motion(true);
    h.press("tab").press("right");
    let start = h.fg(0, 0);
    h.advance(h.env().theme().motion().pulse_period / 2);
    assert_eq!(h.fg(0, 0), start, "reduced motion holds the pillar still");
}

#[test]
fn a_hovered_card_rises_a_clear_step_with_a_soft_pillar() {
    for theme in ["monochrome", "iris", "nordic", "amber"] {
        let mut h = Harness::new(store(4), 96, 8);
        h.set_theme(theme);
        h.hover(10, 1);
        let rest = color(&h, "raised");
        let hover = h.bg(10, 1).expect("hover tone");
        assert!(hover.contrast_ratio(rest) >= 1.15, "{theme}: rest to hover is {:.3}:1", hover.contrast_ratio(rest));
        assert_eq!(h.fg(0, 2), Some(color(&h, "active").mix(color(&h, "accent"), 0.45)), "{theme}");
        assert!(h.screen().starts_with("▌ app 0"), "{theme}: nothing slides:\n{}", h.screen());
    }
}

#[test]
fn a_disabled_grid_neither_lights_nor_answers() {
    let mut h = Harness::new(Store { disabled: true, selected: Some(1), ..store(4) }, 96, 8);
    let rest = h.screen();
    h.hover(10, 1);
    assert_eq!(h.screen(), rest);
    assert_eq!(h.bg(10, 1), Some(color(&h, "raised")));
    h.press("tab").press("right").click(10, 1);
    assert_eq!((h.app().selected, h.app().opened.len()), (Some(1), 0));
    assert_ne!(h.fg(34, 0), Some(color(&h, "text")), "the content fades");
}

#[test]
fn an_empty_grid_shows_the_applications_empty_state() {
    let mut h = Harness::new(store(0), 60, 9);
    assert!(h.screen().contains("No apps match"), "{}", h.screen());
    h.click_text("Browse all");
    assert_eq!(h.app().opened, [usize::MAX], "the empty state's action works");
}

#[test]
fn a_press_flashes_the_card_one_tone_brighter() {
    let mut h = Harness::new(store(4), 96, 8);
    h.hover(10, 1);
    let hover = h.bg(10, 1).expect("hover tone");
    h.click(10, 1);
    let pressed = h.bg(10, 1).expect("pressed tone");
    assert!(pressed.contrast_ratio(hover) >= 1.15, "a press flashes brighter than hover");
    h.advance(Duration::from_millis(200));
    assert_eq!(h.bg(10, 1), Some(color(&h, "active").mix(color(&h, "text"), 0.08)), "selected and hovered");
}
