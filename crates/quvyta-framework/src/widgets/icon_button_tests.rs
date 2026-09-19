//! Tests of [`IconButton`](super::IconButton).

use std::time::Duration;

use super::IconButton;
use crate::color::Rgb;
use crate::event::{MouseButton, MouseKind};
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Panel, Text};

/// A title with a settings button at its right end, as in an application's header.
#[derive(Default)]
struct Header {
    presses: u32,
    disabled: bool,
    tooltip: bool,
}

impl App for Header {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        self.presses += 1;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        // The panel is the ground the button stands on: a surface with room around the row.
        ui.add_with(Panel::new(), |ui| {
            ui.row(|ui| {
                ui.add(Text::new("qpac")).fill_width();
                let mut button = IconButton::new("settings").on_press(()).disabled(self.disabled);
                if self.tooltip {
                    button = button.tooltip("Settings");
                }
                ui.add(button).id("settings");
            })
            .fill_width();
        })
        .fill_width();
    }
}

/// Screen size of every test: the panel's padding puts the row at line 1, from column 3 to 16.
const WIDTH: u16 = 20;
const HEIGHT: u16 = 5;
/// The glyph's cell, the cells either side of it and a cell of the panel's ground.
const GLYPH: (u16, u16) = (15, 1);
const LEFT: u16 = 14;
const RIGHT: u16 = 16;
const GROUND: (u16, u16) = (10, 1);
/// A cell off the button, for moving the pointer away.
const AWAY: (i32, i32) = (0, 4);

/// The sum of a colour's channels, to compare how bright two tones are.
fn brightness(color: Option<Rgb>) -> u32 {
    color.map_or(0, |c| u32::from(c.r) + u32::from(c.g) + u32::from(c.b))
}

#[test]
fn is_one_glyph_with_a_space_on_each_side_in_every_glyph_mode() {
    for (mode, glyph) in [(GlyphMode::Nerd, "\u{f013}"), (GlyphMode::Unicode, "▤"), (GlyphMode::Ascii, "*")] {
        let mut h = Harness::new(Header::default(), WIDTH, HEIGHT);
        h.set_glyph_mode(mode);
        let line = h.screen().lines().nth(1).unwrap_or_default().to_owned();
        assert_eq!(line, format!("   qpac        {glyph}"), "{mode:?}: the glyph ends the row but one cell");
        assert_eq!(h.find(glyph), Some((15, 1)), "{mode:?}: one space before the glyph and one after");
        // Lit, the button shows its three cells and nothing beside them.
        h.hover(i32::from(GLYPH.0), i32::from(GLYPH.1));
        let lit = h.bg(GLYPH.0, GLYPH.1);
        let lit_cells: Vec<u16> = (0..WIDTH).filter(|x| h.bg(*x, GLYPH.1) == lit).collect();
        assert_eq!(lit_cells, vec![LEFT, GLYPH.0, RIGHT], "{mode:?}: three cells wide");
    }
}

#[test]
fn rests_on_the_ground_it_stands_on_and_lightens_under_the_pointer() {
    let mut h = Harness::new(Header::default(), WIDTH, HEIGHT);
    h.set_glyph_mode(GlyphMode::Unicode);
    let ground = h.bg(GROUND.0, GROUND.1);
    assert_eq!(ground, h.env().theme().color("surface"));
    for x in LEFT..=RIGHT {
        assert_eq!(h.bg(x, GLYPH.1), ground, "no raised surface at rest");
    }
    let rest_fg = h.fg(GLYPH.0, GLYPH.1);
    h.hover(i32::from(GLYPH.0), i32::from(GLYPH.1));
    let hover = h.bg(GLYPH.0, GLYPH.1);
    assert!(brightness(hover) > brightness(ground), "hover lightens the button");
    assert_eq!(h.bg(LEFT, GLYPH.1), hover, "all three cells light together");
    assert_eq!(h.bg(RIGHT, GLYPH.1), hover);
    assert_eq!(hover, h.env().theme().color("active"), "the theme's hover tone");
    assert!(brightness(h.fg(GLYPH.0, GLYPH.1)) >= brightness(rest_fg), "the glyph brightens too");
    assert!(!h.screen().contains('▌'), "no pillar: {}", h.screen());
}

#[test]
fn keyboard_focus_changes_the_tone_without_a_pillar() {
    let mut h = Harness::new(Header::default(), WIDTH, HEIGHT);
    h.set_glyph_mode(GlyphMode::Unicode);
    let rest = h.bg(GLYPH.0, GLYPH.1);
    h.hover(i32::from(GLYPH.0), i32::from(GLYPH.1));
    let hover = h.bg(GLYPH.0, GLYPH.1);
    h.hover(AWAY.0, AWAY.1).press("tab");
    assert!(h.is_focused("settings"));
    let focus = h.bg(GLYPH.0, GLYPH.1);
    assert!(brightness(rest) < brightness(hover) && brightness(hover) < brightness(focus), "rest < hover < focus");
    assert_eq!(h.screen().lines().nth(1), Some("   qpac        ▤"), "the mark is the tone alone");
}

#[test]
fn presses_with_enter_space_and_a_click_and_flashes() {
    let mut h = Harness::new(Header::default(), WIDTH, HEIGHT);
    h.press("tab");
    let focus = h.bg(GLYPH.0, GLYPH.1);
    h.press("enter");
    assert_eq!(h.app().presses, 1);
    assert!(brightness(h.bg(GLYPH.0, GLYPH.1)) > brightness(focus), "a press flashes one tone brighter");
    h.advance(Duration::from_millis(200));
    assert_eq!(h.bg(GLYPH.0, GLYPH.1), focus, "the flash settles back");
    h.press("space");
    assert_eq!(h.app().presses, 2);
    h.mouse(MouseKind::Down(MouseButton::Left), 14, 1).mouse(MouseKind::Up(MouseButton::Left), 14, 1);
    assert_eq!(h.app().presses, 3, "the space beside the glyph is part of the target");
    h.mouse(MouseKind::Down(MouseButton::Left), 16, 1).mouse(MouseKind::Up(MouseButton::Left), 5, 1);
    assert_eq!(h.app().presses, 3, "releasing elsewhere cancels");
}

#[test]
fn a_disabled_button_is_faint_and_ignores_presses() {
    let mut h = Harness::new(Header { disabled: true, ..Header::default() }, WIDTH, HEIGHT);
    h.press("tab").press("enter");
    h.click(15, 1);
    assert_eq!(h.app().presses, 0);
    assert!(!h.is_focused("settings"));
    assert_eq!(h.fg(GLYPH.0, GLYPH.1), h.env().theme().color("muted"));
}

#[test]
fn the_tooltip_names_it_after_the_hover_delay_and_at_once_from_the_keyboard() {
    let mut h = Harness::new(Header { tooltip: true, ..Header::default() }, WIDTH, HEIGHT + 2);
    h.hover(i32::from(GLYPH.0), i32::from(GLYPH.1));
    assert!(!h.screen().contains("Settings"));
    h.advance(Duration::from_secs(1));
    assert!(h.screen().contains("Settings"), "{}", h.screen());
    h.hover(AWAY.0, AWAY.1).advance(Duration::from_millis(10));
    assert!(!h.screen().contains("Settings"), "it leaves with the pointer");
    h.set_reduced_motion(true).press("tab");
    assert!(h.screen().contains("Settings"), "keyboard focus names it at once: {}", h.screen());
}

#[test]
fn every_state_keeps_the_glyph_readable_in_every_built_in_theme() {
    let mut h = Harness::new(Header::default(), WIDTH, HEIGHT);
    let themes: Vec<String> = h.env().themes().into_iter().map(|(id, _)| id).collect();
    assert!(themes.len() >= 4, "{themes:?}");
    for theme in themes {
        h.set_theme(&theme);
        h.hover(AWAY.0, AWAY.1);
        let mut checks = vec![("rest", h.fg(GLYPH.0, GLYPH.1), h.bg(GROUND.0, GROUND.1))];
        h.hover(i32::from(GLYPH.0), i32::from(GLYPH.1));
        checks.push(("hover", h.fg(GLYPH.0, GLYPH.1), h.bg(GLYPH.0, GLYPH.1)));
        h.hover(AWAY.0, AWAY.1).press("tab");
        checks.push(("focus", h.fg(GLYPH.0, GLYPH.1), h.bg(GLYPH.0, GLYPH.1)));
        h.press("enter");
        checks.push(("pressed", h.fg(GLYPH.0, GLYPH.1), h.bg(GLYPH.0, GLYPH.1)));
        h.advance(Duration::from_millis(300)).press("shift+tab");
        for (state, fg, bg) in checks {
            let (Some(fg), Some(bg)) = (fg, bg) else { panic!("{theme} {state}: no colour") };
            let ratio = fg.contrast_ratio(bg);
            assert!(ratio >= 4.5, "{theme} {state}: contrast {ratio:.2}");
        }
    }
}
