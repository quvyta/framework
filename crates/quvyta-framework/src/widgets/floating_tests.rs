//! Floating surfaces keep apart from the ground they open over, in every built-in theme: a menu
//! over the screen ground keeps the theme's `overlay` tone, one opened inside a panel of nearly
//! the same tone steps away from it (see `PaintCx::floating`).

use std::time::Duration;

use crate::color::{APART, Rgb};
use crate::runtime::{App, Command, Harness};
use crate::widget::{Length, View};
use crate::widgets::{Panel, Popover, Select, Side, SidePanel, Text};

const THEMES: [&str; 4] = ["monochrome", "nordic", "amber", "iris"];

/// A select inside a side panel on the right, as in an editor's tool panel, or in the body.
struct Menus {
    in_panel: bool,
}

impl App for Menus {
    type Msg = usize;
    fn update(&mut self, _: usize) -> Command<usize> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, usize>) {
        let select = |ui: &mut View<'_, usize>| {
            ui.add(Select::new(["Alpha", "Beta", "Gamma"]).placeholder("Pick").on_select(|i| i)).id("pick");
        };
        let panel = SidePanel::new(30).side(Side::Right).open(true);
        if self.in_panel {
            panel.panel(select).body(|ui| {
                ui.add(Text::new("body"));
            })
        } else {
            panel
                .panel(|ui| {
                    ui.add(Text::new("tools"));
                })
                .body(select)
        }
        .show(ui);
    }
}

fn open_menu(in_panel: bool, theme: &str) -> Harness<Menus> {
    let mut h = Harness::new(Menus { in_panel }, 70, 14);
    h.set_theme(theme);
    h.click_text("Pick");
    h.advance(Duration::from_millis(300));
    h
}

/// The background of the menu row showing `label`, a few cells right of the label.
fn menu_bg(h: &Harness<Menus>, label: &str) -> Option<Rgb> {
    let (x, y) = h.find(label).unwrap_or_else(|| panic!("{label} is on screen: {}", h.screen()));
    h.bg(u16::try_from(x + 8).unwrap_or(0), u16::try_from(y).unwrap_or(0))
}

#[test]
fn a_menu_over_a_side_panel_is_lifted_apart_from_it_in_every_theme() {
    for theme in THEMES {
        let h = open_menu(true, theme);
        let overlay = h.env().theme().color("overlay");
        // The bottom right cell is side panel, far from the menu.
        let panel = h.bg(69, 13).expect("a true-colour panel");
        let menu = menu_bg(&h, "Gamma").expect("a true-colour menu");
        assert_ne!(Some(menu), overlay, "{theme}: over a panel the menu is not left on the plain overlay tone");
        assert!(
            menu.perceptual_distance(panel) >= APART,
            "{theme}: menu {menu} sits {:.3} from panel {panel}",
            menu.perceptual_distance(panel)
        );
        // The highlighted row moves with the menu and stays a step apart from it.
        let highlighted = menu_bg(&h, "Alpha").expect("a true-colour row");
        assert!(highlighted.perceptual_distance(menu) >= 0.02, "{theme}: {highlighted} against {menu}");
    }
}

#[test]
fn a_menu_over_the_screen_ground_keeps_the_overlay_tone() {
    for theme in THEMES {
        let h = open_menu(false, theme);
        let overlay = h.env().theme().color("overlay");
        assert_eq!(menu_bg(&h, "Gamma"), overlay, "{theme}");
    }
}

#[test]
fn a_popover_inside_a_panel_is_lifted_and_one_beside_it_is_not() {
    struct Popovers;
    impl App for Popovers {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.row(|ui| {
                ui.add_with(Panel::new().title("Panel"), |ui| {
                    Popover::new(true)
                        .anchor(|ui| {
                            ui.add(Text::new("inner"));
                        })
                        .content(|ui| {
                            ui.add(Text::new("lifted"));
                        })
                        .show(ui);
                    for _ in 0..6 {
                        ui.add(Text::new(" "));
                    }
                })
                .width(Length::Cells(30));
                ui.column(|ui| {
                    Popover::new(true)
                        .anchor(|ui| {
                            ui.add(Text::new("outer"));
                        })
                        .content(|ui| {
                            ui.add(Text::new("plain"));
                        })
                        .show(ui);
                })
                .fill();
            })
            .gap(4);
        }
    }
    for theme in THEMES {
        let mut h = Harness::new(Popovers, 70, 14);
        h.set_theme(theme);
        h.advance(Duration::from_millis(300));
        let overlay = h.env().theme().color("overlay");
        let (px, py) = h.find("Panel").expect("the panel title");
        let panel = h.bg(u16::try_from(px + 20).unwrap_or(0), u16::try_from(py + 8).unwrap_or(0)).expect("panel");
        let (x, y) = h.find("lifted").expect("the inner popover");
        let lifted = h.bg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0)).expect("inner popover");
        assert!(lifted.perceptual_distance(panel) >= APART, "{theme}: {lifted} over {panel}");
        let (x, y) = h.find("plain").expect("the outer popover");
        assert_eq!(h.bg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0)), overlay, "{theme}");
    }
}

#[test]
fn reduced_colour_depth_leaves_the_palette_cells_alone() {
    let mut h = Harness::new(Menus { in_panel: true }, 70, 14);
    h.set_depth(crate::color::ColorDepth::Ansi256);
    h.click_text("Pick");
    h.advance(Duration::from_millis(300));
    let (x, y) = h.find("Gamma").expect("the menu");
    let cell = &h.buffer()[(u16::try_from(x + 8).unwrap_or(0), u16::try_from(y).unwrap_or(0))];
    let overlay = h.env().theme().color("overlay").expect("overlay");
    assert_eq!(cell.bg, ratatui_core::style::Color::Indexed(overlay.to_ansi256()));
}
