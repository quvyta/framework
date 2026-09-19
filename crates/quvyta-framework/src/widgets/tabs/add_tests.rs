use std::time::Duration;

use super::*;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::{Length, View};
use crate::widgets::TabEdit;

/// An editor whose strip ends in a `+` that opens another file.
struct Files {
    files: Vec<String>,
    active: usize,
    width: u16,
    overflow: Overflow,
    added: usize,
}

#[derive(Debug, Clone, Copy)]
enum Msg {
    Open(usize),
    Edit(TabEdit),
    Add,
}

impl App for Files {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open(index) => self.active = index,
            Msg::Edit(edit) => edit.apply(&mut self.files, &mut self.active),
            Msg::Add => {
                self.added += 1;
                self.files.push(format!("new{}", self.added));
                self.active = self.files.len() - 1;
            }
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let tabs = Tabs::new(self.files.clone())
            .active(self.active)
            .on_select(Msg::Open)
            .closable(|i| Msg::Edit(TabEdit::Close(i)))
            .reorderable(|from, to| Msg::Edit(TabEdit::Move { from, to }))
            .overflow(self.overflow)
            .on_add(|| Msg::Add);
        ui.add(tabs).width(Length::Cells(self.width)).id("files");
    }
}

fn files(count: usize, width: u16) -> Files {
    let files = (1..=count).map(|n| format!("f{n}.rs")).collect();
    Files { files, active: 0, width, overflow: Overflow::Arrows, added: 0 }
}

/// The column of the `+` on the strip's line.
fn plus(h: &Harness<Files>) -> usize {
    let line = h.screen().lines().next().unwrap_or_default().to_owned();
    line.chars().position(|c| c == '+').unwrap_or_else(|| panic!("the strip carries a plus: {line:?}"))
}

/// The strip's line.
fn line(h: &Harness<Files>) -> String {
    h.screen().lines().next().unwrap_or_default().to_owned()
}

#[test]
fn the_plus_stands_one_gap_after_the_last_tabs_close_mark() {
    let mut app = files(3, 60);
    app.active = 2;
    let h = Harness::new(app, 60, 1);
    assert_eq!(line(&h), " f1.rs    ×    f2.rs    ×   ▌ f3.rs   ×    +");
    let raised = h.env().theme().color("raised");
    let at = u16::try_from(plus(&h)).expect("on screen");
    assert_eq!((h.bg(at - 1, 0), h.bg(at, 0), h.bg(at + 1, 0)), (raised, raised, raised), "three raised cells");
    assert_eq!(h.bg(at - 2, 0), h.env().theme().color("canvas"), "one gap of canvas after the tab");
}

#[test]
fn with_no_tabs_the_plus_starts_the_strip() {
    let mut h = Harness::new(files(0, 60), 60, 1);
    assert_eq!(line(&h), " +");
    h.press("tab").press("enter");
    assert_eq!(h.app().files, ["new1"], "the lone button takes focus and Enter presses it");
}

#[test]
fn with_twenty_tabs_the_plus_keeps_the_right_end_after_the_arrows() {
    let mut app = files(20, 60);
    app.active = 19;
    let h = Harness::new(app, 60, 1);
    let strip = line(&h);
    assert!(strip.ends_with("▶   +"), "{strip:?}");
    assert!(strip.contains("f20.rs"), "the open tab stays in view: {strip:?}");
    assert_eq!(plus(&h), 58);
    let mut app = files(20, 60);
    app.overflow = Overflow::Menu;
    let h = Harness::new(app, 60, 1);
    assert!(line(&h).ends_with("▾ 17   +"), "after the menu control: {:?}", line(&h));
}

#[test]
fn a_click_enter_and_space_send_the_message() {
    let mut h = Harness::new(files(3, 60), 60, 1);
    h.press("tab").press("tab").press("enter");
    assert_eq!(h.app().files.len(), 4, "Tab moves from the tabs to the plus; Enter presses it");
    h.press("space");
    assert_eq!(h.app().files.len(), 5, "and so does Space");
    h.press("shift+tab").press("enter");
    assert_eq!(h.app().files.len(), 5, "shift+Tab goes back to the tabs, where Enter adds nothing");
    h.press("left");
    assert_eq!(h.app().active, 3, "and the arrows open tabs again");

    let at = i32::try_from(plus(&h)).expect("on screen");
    h.click(at, 0);
    assert_eq!(h.app().files.len(), 6, "a click adds a tab");
    assert!(line(&h).contains("new3"), "{}", line(&h));
    h.mouse(MouseKind::Down(MouseButton::Right), at, 0);
    assert_eq!(h.app().files.len(), 6, "only a left press does");
}

#[test]
fn keyboard_focus_lights_the_plus_and_moves_on_with_tab() {
    let mut h = Harness::new(files(3, 60), 60, 1);
    let at = u16::try_from(plus(&h)).expect("on screen");
    let (rest, calm_pillar) = (h.bg(at, 0), h.fg(0, 0));
    h.press("tab");
    assert_eq!(h.bg(at, 0), rest, "focus starts on the tabs");
    assert_ne!(h.fg(0, 0), calm_pillar, "where the open tab's pillar breathes");
    h.press("tab");
    assert_eq!(h.bg(at, 0), h.env().theme().color("active"), "then lights the plus");
    assert_ne!(h.bg(at - 1, 0), h.fg(at - 1, 0), "with the pillar in its first cell");
    assert_eq!(h.fg(0, 0), calm_pillar, "while the open tab's pillar calms down");
    h.press("tab");
    assert_eq!(h.bg(at, 0), rest, "the next Tab leaves the strip");
}

#[test]
fn resting_on_the_plus_or_reaching_it_shows_its_hint() {
    let mut h = Harness::new(files(3, 60), 60, 4);
    let at = i32::try_from(plus(&h)).expect("on screen");
    h.hover(at, 0);
    assert!(!h.screen().contains("New tab"), "not at once under the pointer");
    h.advance(Duration::from_secs(1));
    assert!(h.screen().contains("New tab"), "after the hover delay:\n{}", h.screen());
    h.hover(2, 3).advance(Duration::from_millis(50));
    assert!(!h.screen().contains("New tab"), "gone with the pointer");
    h.press("tab").press("tab");
    assert!(h.screen().contains("New tab"), "at once from the keyboard:\n{}", h.screen());
    h.set_locale("tr");
    assert!(h.screen().contains("Yeni sekme"), "in the language of the interface:\n{}", h.screen());
}

#[test]
fn a_tab_dropped_on_the_plus_moves_to_the_end() {
    let mut h = Harness::new(files(3, 60), 60, 1);
    let at = i32::try_from(plus(&h)).expect("on screen");
    h.mouse(MouseKind::Down(MouseButton::Left), 3, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 20, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), at, 0);
    h.mouse(MouseKind::Up(MouseButton::Left), at, 0);
    assert_eq!(h.app().files, ["f2.rs", "f3.rs", "f1.rs"]);
    assert_eq!(h.app().added, 0, "the drop adds no tab");

    let mut h = Harness::new(files(20, 60), 60, 1);
    h.mouse(MouseKind::Down(MouseButton::Left), 6, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 20, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 58, 0);
    h.mouse(MouseKind::Up(MouseButton::Left), 58, 0);
    assert_eq!(h.app().files.last().map(String::as_str), Some("f1.rs"), "even past tabs scrolled out of view");
}

#[test]
fn ascii_draws_only_ascii() {
    for count in [0, 3, 20] {
        let mut h = Harness::new(files(count, 60), 60, 1);
        h.set_glyph_mode(GlyphMode::Ascii);
        let screen = h.screen();
        assert!(screen.is_ascii(), "{screen:?}");
        assert!(screen.contains('+'), "{screen:?}");
    }
}

#[test]
fn reduced_motion_keeps_the_same_place() {
    for count in [0, 3, 20] {
        let still = {
            let mut h = Harness::new(files(count, 60), 60, 1);
            h.set_reduced_motion(true);
            plus(&h)
        };
        assert_eq!(still, plus(&Harness::new(files(count, 60), 60, 1)), "{count} tabs");
    }
}

#[test]
fn fill_width_tabs_leave_room_for_the_plus() {
    let tabs = Tabs::<()>::new(["a", "b"]).tab_width(TabWidth::Fill).on_add(|| ());
    let widths = tabs.widths(40);
    assert_eq!(Tabs::<()>::total_width(&widths) + tabs.add_room(), 40, "the tabs and the plus share the strip");
}
