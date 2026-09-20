//! The rules of mouse text selection, checked through the whole runtime: where a selection may
//! start, what releasing does, the copy key and the selection's Copy and Raw copy menu.

use std::time::Duration;

use crate::event::{MouseButton, MouseKind};
use crate::runtime::{App, ClipboardEvent, Command, Harness};
use crate::widget::{Length, View};
use crate::widgets::{Button, Markdown, ScrollView, Text};

#[derive(Default)]
struct Deploys {
    heard: Vec<String>,
    restarts: u32,
}

#[derive(Clone)]
enum Msg {
    Heard(String),
    Restart,
}

impl App for Deploys {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Heard(text) => self.heard.push(text),
            Msg::Restart => self.restarts += 1,
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("Deploy log").no_wrap());
        ui.add(Button::new("Restart").on_press(Msg::Restart)).id("restart");
        ui.add_with(ScrollView::new(), |ui| {
            for line in 0..12 {
                ui.add(Text::new(format!("12:00:{line:02} deploy-api pod/web-{line} ready")).no_wrap());
            }
            ui.add(Text::new("token qv_live_8f3a").no_wrap()).selectable(false);
        })
        .height(Length::Cells(4))
        .selectable(true)
        .id("logs");
        ui.add(Markdown::new("## Notes\n\nRollout done")).fill_width();
    }
    fn clipboard(&self, event: &ClipboardEvent) -> Option<Msg> {
        match event {
            ClipboardEvent::Copied(text) => Some(Msg::Heard(text.clone())),
            ClipboardEvent::Pasted(_) => None,
        }
    }
}

/// Rows: 0 title, 1 button, 2 to 5 the log, 6 and 8 the notes, 9 and below empty canvas.
fn deploys() -> Harness<Deploys> {
    let mut h = Harness::new(Deploys::default(), 44, 12);
    h.set_reduced_motion(true);
    h
}

fn right_press(h: &mut Harness<Deploys>, x: i32, y: i32) {
    h.mouse(MouseKind::Down(MouseButton::Right), x, y);
    h.mouse(MouseKind::Up(MouseButton::Right), x, y);
}

#[test]
fn nothing_is_selectable_unless_asked() {
    let mut h = deploys();
    let plain = h.bg(3, 0);
    h.drag((0, 0), (9, 0));
    assert_eq!(h.bg(3, 0), plain, "the title is not selectable");
    h.press("ctrl+c");
    assert!(h.copied().is_empty());
    h.drag((2, 10), (30, 11));
    h.press("ctrl+c");
    assert!(h.copied().is_empty(), "neither is the empty canvas");
    h.drag((2, 1), (30, 1));
    h.press("ctrl+c");
    assert_eq!((h.app().restarts, h.copied().len()), (0, 0), "the button kept the mouse");
}

#[test]
fn a_drag_selects_inside_its_region_and_releasing_copies_nothing() {
    let mut h = deploys();
    assert!(h.screen().lines().nth(2).is_some_and(|line| line.starts_with("12:00:00 deploy-api pod/web-0 ready")));
    let plain = h.bg(9, 2);
    h.drag((9, 2), (18, 3));
    assert_ne!(h.bg(9, 2), plain, "selected cells take the selection colour");
    assert_eq!(h.bg(20, 3), plain, "cells after the end stay plain");
    assert!(h.copied().is_empty() && h.clipboard().is_none(), "releasing copies nothing");
    assert!(h.app().heard.is_empty());
    h.press("ctrl+c");
    assert_eq!(h.clipboard(), Some("deploy-api pod/web-0 ready\n12:00:01 deploy-api"));
    assert_eq!(h.app().heard, ["deploy-api pod/web-0 ready\n12:00:01 deploy-api"]);
    // Dragging far outside stays within the log's text, without its scrollbar.
    h.drag((30, 5), (43, 10)).press("ctrl+c");
    assert_eq!(h.clipboard(), Some("ready"));
    assert_eq!(h.bg(40, 6), h.bg(40, 7), "nothing below the log was selected");
}

#[test]
fn faint_text_stays_readable_under_the_selection_in_every_theme() {
    // Asked of the registry rather than a list written here: a fifth built-in theme is then
    // measured the day it is added, instead of the day someone remembers this test.
    for (theme, _) in crate::theme::ThemeRegistry::builtin().list() {
        let mut h = Harness::new(Faint, 30, 1);
        h.set_theme(&theme);
        let muted = h.fg(0, 0);
        h.drag((0, 0), (10, 0));
        let (fg, bg) = (h.fg(0, 0), h.bg(0, 0));
        let ratio = fg.zip(bg).map_or(0.0, |(fg, bg)| fg.contrast_ratio(bg));
        assert!(ratio >= 3.0, "{theme}: selected faint text reads at {ratio:.2}:1 (was {muted:?})");
    }
}

struct Faint;

impl App for Faint {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Text::new("pulled image 2026.9.1").role("faint").no_wrap()).selectable(true);
    }
}

#[test]
fn double_and_triple_presses_select_words_and_lines() {
    let mut h = deploys();
    h.click(22, 2).click(22, 2).press("ctrl+c");
    assert_eq!(h.clipboard(), Some("pod/web-0"));
    h.click(22, 3).click(22, 3).click(22, 3).press("ctrl+c");
    assert_eq!(h.clipboard(), Some("12:00:01 deploy-api pod/web-1 ready"));
}

#[test]
fn selection_follows_scrolled_content_and_other_keys_clear_it() {
    let mut h = deploys();
    h.drag((0, 5), (7, 5));
    let (plain, selected) = (h.bg(0, 4), h.bg(0, 5));
    assert_ne!(plain, selected);
    h.mouse(MouseKind::ScrollDown, 5, 3);
    assert_eq!(h.find("12:00:03").map(|(_, y)| y), Some(2), "{}", h.screen());
    assert_eq!((h.bg(0, 2), h.bg(0, 5)), (selected, plain), "the highlight moved with its text");
    h.press("ctrl+c");
    assert_eq!(h.copied(), ["12:00:03"]);
    h.press("x").press("ctrl+c");
    assert_eq!(h.copied().len(), 1, "any other key cleared the selection");
    assert_eq!(h.bg(0, 2), plain);
}

#[test]
fn text_kept_out_of_a_region_is_not_selectable() {
    let mut h = deploys();
    for _ in 0..4 {
        h.mouse(MouseKind::ScrollDown, 5, 3);
    }
    let (x, y) = h.find("qv_live").expect("the token is on screen");
    h.drag((x, y), (x + 6, y)).press("ctrl+c");
    assert!(h.copied().is_empty(), "{}", h.screen());
}

#[test]
fn a_right_press_on_the_selection_offers_copy_and_raw_copy() {
    let mut h = deploys();
    h.click(3, 6).click(3, 6).click(3, 6);
    let selected = h.bg(3, 6);
    right_press(&mut h, 3, 6);
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert_eq!(lines[7], "     Copy        ctrl c", "{screen}");
    assert_eq!(lines[8], "Rol  Raw copy");
    assert_eq!(h.bg(3, 6), selected, "the selection stays while its menu is open");
    let theme = h.env().theme();
    assert_eq!(h.bg(5, 7), theme.color("overlay"), "the menu covers the selection highlight");
    h.click_text("Raw copy");
    assert_eq!(h.clipboard(), Some(format!("▌ Notes{}", " ".repeat(37)).as_str()), "every cell, as shown");
    assert!(!h.screen().contains("Raw copy"), "choosing closed the menu");
    assert_ne!(h.bg(3, 6), selected, "the copied selection flashes");
    h.advance(Duration::from_millis(500));
    assert_eq!(h.bg(3, 6), selected, "and stays for another copy");
    right_press(&mut h, 3, 6);
    h.click_text("Copy");
    assert_eq!(h.clipboard(), Some("Notes"), "no pillar, no gap after it, no padding");
    assert_eq!(h.app().heard, [format!("▌ Notes{}", " ".repeat(37)), "Notes".to_owned()]);
}

#[test]
fn the_selection_menu_takes_keys_and_esc_closes_it_first() {
    let mut h = deploys();
    h.drag((0, 2), (7, 2));
    right_press(&mut h, 3, 2);
    assert!(h.screen().contains("Raw copy"));
    h.press("down").press("down").press("enter");
    assert_eq!(h.clipboard(), Some("12:00:00"), "the keys chose Raw copy instead of clearing");
    right_press(&mut h, 3, 2);
    h.press("esc");
    assert!(!h.screen().contains("Raw copy"));
    h.press("ctrl+c");
    assert_eq!(h.copied().len(), 2, "Esc closed only the menu");
    h.advance(Duration::from_millis(500)).press("esc").press("ctrl+c");
    assert_eq!(h.copied().len(), 2, "the next key cleared the selection");
}

#[test]
fn a_right_press_elsewhere_clears_the_selection_and_opens_nothing() {
    let mut h = deploys();
    let plain = h.bg(0, 2);
    h.drag((0, 2), (7, 2));
    right_press(&mut h, 20, 4);
    assert!(!h.screen().contains("Copy"), "{}", h.screen());
    assert_eq!(h.bg(0, 2), plain);
    right_press(&mut h, 3, 8);
    assert!(!h.screen().contains("Copy"), "no selection, no menu");
}
