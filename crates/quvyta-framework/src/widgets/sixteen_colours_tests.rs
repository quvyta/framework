//! A screen keeps its shape in a terminal with a palette. Reduced to the nearest of the sixteen,
//! every dark surface tone of a theme lands on black: a dialog's dimmed page vanished into the
//! ground behind it, and tab strips and raised surfaces melted into the canvas. In 256 colours
//! the dimmed page vanished too, its text and ground both taking the dialog's scrim. These tests
//! draw real screens in sixteen and in 256 colours, reached the way a person reaches them, and
//! read the palette entries the terminal is sent.

use std::time::Duration;

use ratatui_core::style::Color;

use crate::color::{ColorDepth, Rgb};
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, Modal, Panel, Tabs, Text};

const THEMES: [&str; 4] = ["monochrome", "nordic", "amber", "iris"];

/// Contrast under which a glyph starts to disappear into its background, the bar the showcase's
/// sweep holds sixteen-colour text to.
const READABLE: f64 = 1.6;

/// A page with no accent colours of its own: tabs, a line of text, a raised panel and a button
/// that opens a dialog.
#[derive(Default)]
struct Today {
    asking: bool,
}

#[derive(Clone)]
enum Msg {
    Tab,
    Ask,
    Close,
}

impl App for Today {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Tab => {}
            Msg::Ask => self.asking = true,
            Msg::Close => self.asking = false,
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            ui.add(Tabs::new(["Today", "Week", "Later"]).on_select(|_| Msg::Tab));
            ui.add(Text::new("Nothing planned for today"));
            ui.add_with(Panel::new().variant("inset").title("Inbox"), |ui| {
                ui.add(Text::new("Three notes waiting"));
            });
            ui.add(Button::new("Empty trash").on_press(Msg::Ask));
            if self.asking {
                ui.add_with(Modal::new().title("Empty trash?").width(30).on_close(Msg::Close), |ui| {
                    ui.add(Text::new("Gone for good."));
                });
            }
        });
    }
}

/// The palette entry of a cell colour, which in a palette frame every cell carries.
fn entry(color: Color) -> u8 {
    match color {
        Color::Indexed(index) => index,
        other => panic!("a palette frame sent {other:?}"),
    }
}

/// Contrast between the text and background of the cell at `x`, `y`, as xterm shows the entries.
fn contrast(h: &Harness<Today>, x: u16, y: u16) -> f64 {
    let cell = &h.buffer()[(x, y)];
    Rgb::from_ansi256(entry(cell.fg)).contrast_ratio(Rgb::from_ansi256(entry(cell.bg)))
}

fn today(theme: &str) -> Harness<Today> {
    today_in(theme, ColorDepth::Ansi16)
}

fn today_in(theme: &str, depth: ColorDepth) -> Harness<Today> {
    let mut h = Harness::new(Today::default(), 60, 18);
    h.set_theme(theme).set_depth(depth);
    h
}

#[test]
fn a_dialog_leaves_the_page_behind_faint_but_visible() {
    a_dialog_leaves_the_page_behind_faint_but_visible_in(ColorDepth::Ansi16);
}

#[test]
fn in_256_colours_a_dialog_leaves_the_page_behind_faint_but_visible() {
    a_dialog_leaves_the_page_behind_faint_but_visible_in(ColorDepth::Ansi256);
}

/// Opens the dialog by its button over a page of text in every theme, and reads the page behind.
fn a_dialog_leaves_the_page_behind_faint_but_visible_in(depth: ColorDepth) {
    for theme in THEMES {
        let mut h = today_in(theme, depth);
        let (x, y) = h.find("Nothing").expect("the page text");
        let (x, y) = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
        let open = contrast(&h, x, y);
        h.click_text("Empty trash").advance(Duration::from_secs(1));
        assert!(h.screen().contains("Gone for good."), "{theme}: the dialog is open:\n{}", h.screen());
        let cell = &h.buffer()[(x, y)];
        assert_ne!(cell.fg, cell.bg, "{theme}: the dimmed page text keeps a colour of its own");
        let dimmed = contrast(&h, x, y);
        assert!(dimmed < open, "{theme}: and is fainter than before the dialog, {dimmed:.2} against {open:.2}");
        // Every glyph, the faint tab names and panel title behind the dialog included.
        every_glyph_reads(&h, theme);
    }
}

/// Asserts every letter and digit on screen keeps more than [`READABLE`] against its background.
fn every_glyph_reads(h: &Harness<Today>, theme: &str) {
    for y in 0..h.buffer().area.height {
        for x in 0..h.buffer().area.width {
            if h.buffer()[(x, y)].symbol().chars().any(char::is_alphanumeric) {
                let ratio = contrast(h, x, y);
                assert!(ratio > READABLE, "{theme}: the glyph at {x},{y} keeps only {ratio:.2}:1\n{}", h.screen());
            }
        }
    }
}

#[test]
fn raised_surfaces_stand_off_the_ground() {
    raised_surfaces_stand_off_the_ground_in(ColorDepth::Ansi16);
}

#[test]
fn in_256_colours_raised_surfaces_stand_off_the_ground() {
    raised_surfaces_stand_off_the_ground_in(ColorDepth::Ansi256);
}

fn raised_surfaces_stand_off_the_ground_in(depth: ColorDepth) {
    for theme in THEMES {
        let h = today_in(theme, depth);
        let (width, height) = (h.buffer().area.width, h.buffer().area.height);
        let canvas = entry(h.buffer()[(width - 1, height - 1)].bg);
        let (x, y) = h.find("Three notes").expect("the panel text");
        let panel = entry(h.buffer()[(u16::try_from(x - 1).expect("x"), u16::try_from(y).expect("y"))].bg);
        assert_ne!(panel, canvas, "{theme}: the raised panel stands off the ground");
        let (x, y) = h.find("Today").expect("the open tab");
        let tab = entry(h.buffer()[(u16::try_from(x).expect("x"), u16::try_from(y).expect("y"))].bg);
        assert_ne!(tab, canvas, "{theme}: the open tab stands off the ground");
    }
}

#[test]
fn text_on_a_raised_surface_still_reads() {
    for theme in THEMES {
        every_glyph_reads(&today(theme), theme);
    }
}

#[test]
fn in_256_colours_text_on_a_raised_surface_still_reads() {
    for theme in THEMES {
        every_glyph_reads(&today_in(theme, ColorDepth::Ansi256), theme);
    }
}

/// A page with a legend under it and a button that opens a dialog, as a day's timeline shows its
/// categories.
#[derive(Default)]
struct Legended {
    asking: bool,
}

impl App for Legended {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        self.asking = matches!(msg, Msg::Ask);
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            ui.add(crate::widgets::Legend::new(["Work", "Rest"]));
            ui.add(Button::new("Quit").on_press(Msg::Ask));
            if self.asking {
                ui.add_with(Modal::new().title("Quit?").width(30).on_close(Msg::Close), |ui| {
                    ui.add(Text::new("The timer keeps running."));
                });
            }
        });
    }
}

/// Contrast of the cell at `x`, `y` in whatever depth it was drawn in.
fn any_contrast(h: &Harness<Legended>, x: u16, y: u16) -> f64 {
    let colour = |color: Color| match color {
        Color::Rgb(r, g, b) => Rgb::new(r, g, b),
        Color::Indexed(index) if index < 16 => Rgb::from_ansi16(index),
        Color::Indexed(index) => Rgb::from_ansi256(index),
        other => panic!("{other:?} is not a colour a frame sends"),
    };
    let cell = &h.buffer()[(x, y)];
    colour(cell.fg).contrast_ratio(colour(cell.bg))
}

#[test]
fn a_legend_s_names_keep_a_colour_of_their_own_behind_a_dialog() {
    for depth in [ColorDepth::TrueColor, ColorDepth::Ansi256, ColorDepth::Ansi16] {
        for theme in THEMES {
            let mut h = Harness::new(Legended::default(), 60, 12);
            h.set_theme(theme).set_depth(depth);
            let (x, y) = h.find("Work").expect("the legend's name");
            let (x, y) = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
            assert!(any_contrast(&h, x, y) > READABLE, "{theme} {depth:?}: the name reads before the dialog");
            h.click_text("Quit").advance(Duration::from_secs(1));
            assert!(h.screen().contains("The timer keeps running."), "{theme} {depth:?}: the dialog is open");
            let ratio = any_contrast(&h, x, y);
            assert!(ratio > READABLE, "{theme} {depth:?}: behind the dialog the name keeps {ratio:.2}:1");
        }
    }
}
