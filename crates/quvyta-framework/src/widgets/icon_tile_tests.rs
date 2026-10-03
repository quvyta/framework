//! Tests of [`IconTile`](super::IconTile).

use ratatui_core::style::Color;

use super::IconTile;
use crate::color::{ColorDepth, Rgb};
use crate::icons::{Glyph, GlyphMode};
use crate::runtime::{App, Command, Harness};
use crate::text;
use crate::widget::{Length, View};
use crate::widgets::Panel;

/// A desktop's floor: three tiles in a row on an inset panel, the first of them chosen, and the
/// keyboard cursor on the last.
#[derive(Default)]
struct Floor {
    chosen: usize,
    on_tile: usize,
    backed: bool,
    faint: bool,
}

/// The screen of every test: an inset panel with three tiles in it, at columns 2, 13 and 24.
const WIDTH: u16 = 40;
const HEIGHT: u16 = 5;
/// The row the panel's padding puts the tiles on.
const TILE_ROW: u16 = 1;
/// Cells between two tiles of the row, so a tile's own columns can be told from its neighbour's.
const GAP: u16 = 1;
/// The inset panel's own padding.
const FIRST: u16 = 2;

impl App for Floor {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        // The inset panel is a ground of its own, so a tile that stands on the surface tone over a
        // picture can be told from one that stands on nothing.
        ui.add_with(Panel::new().variant("inset").gap(0), |ui| {
            ui.row(|ui| {
                for (index, (key, name)) in
                    [("file", "Notes"), ("file-image", "Photos"), ("file-archive", "Archive")].into_iter().enumerate()
                {
                    let tile = IconTile::new(key, name)
                        .selected(index == self.chosen)
                        .cursor(index == self.on_tile)
                        .backed(self.backed)
                        .faint(self.faint);
                    ui.add(tile);
                }
            })
            .gap(GAP)
            .fill_width();
        })
        .fill_width();
    }
}

/// The first column of the tile at `index`: the one kept for the pillar, which a chosen or a cursor
/// tile fills down its whole height.
fn tile(index: usize) -> (u16, u16) {
    (FIRST + u16::try_from(index).unwrap_or(0) * (IconTile::WIDTH + GAP), TILE_ROW)
}

/// The column the tile at `index` centres text of `width` cells in, beside its pillar.
fn centred(index: usize, width: u16) -> u16 {
    let (column, _) = tile(index);
    let span = IconTile::WIDTH - IconTile::PILLAR;
    column + IconTile::PILLAR + (span - width) / 2
}

fn floor() -> Harness<Floor> {
    screen_of(Floor::default())
}

/// A floor drawn in `state`, motion off and in Unicode glyphs.
fn screen_of(state: Floor) -> Harness<Floor> {
    let mut h = Harness::new(state, WIDTH, HEIGHT);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true);
    h
}

#[test]
fn a_tile_is_a_glyph_over_a_centred_name_in_one_column_span() {
    let h = floor();
    let (column, row) = tile(0);
    let glyph = h.env().icons().glyph("file").into_owned();
    assert_eq!(h.buffer()[(centred(0, 1), row)].symbol(), glyph, "the glyph stands beside the pillar's column");
    let (name, name_row) = h.find("Notes").expect("the name is drawn");
    assert_eq!(u16::try_from(name).unwrap_or(0), centred(0, 5), "the name is centred in the same span");
    assert_eq!(u16::try_from(name_row).unwrap_or(0), row + 1, "one row under the glyph");
    assert!(u16::try_from(name).unwrap_or(0) + 5 <= column + IconTile::WIDTH, "and never runs into the next pillar");
    // The free row under both is ground, so two tiles never touch.
    assert_eq!(h.buffer()[(centred(0, 1), row + 2)].symbol(), " ");
    assert_eq!(IconTile::SIZE, crate::geometry::Size::new(10, 3));
}

#[test]
fn a_long_name_is_cut_with_an_ellipsis_and_a_short_one_is_not() {
    assert_eq!(IconTile::shown_name("htop"), "htop");
    assert_eq!(IconTile::shown_name("Midnight Commander"), format!("Midnight{}", text::ELLIPSIS));
    assert_eq!(text::width(&IconTile::shown_name("Midnight Commander")), IconTile::WIDTH - IconTile::PILLAR);
    assert!(text::width(&IconTile::shown_name("伺服器東京三號機")) <= IconTile::WIDTH - IconTile::PILLAR);
}

#[test]
fn a_cut_name_is_drawn_the_same_way_in_every_glyph_mode() {
    let mut h = Harness::new(Cut, WIDTH, HEIGHT);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true);
    assert!(h.screen().contains("Midnight…"), "{}", h.screen());
    // A terminal that cannot show an ellipsis is given the ASCII mark in the same cell.
    h.set_glyph_mode(GlyphMode::Ascii);
    let shown = h.screen();
    assert!(shown.is_ascii(), "{shown}");
    assert!(shown.contains('~'), "{shown}");
}

/// A floor whose one name is longer than a tile.
struct Cut;

impl App for Cut {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(IconTile::new("file", "Midnight Commander"));
    }
}

#[test]
fn a_selected_tile_takes_the_selected_surface_and_the_accent_pillar_down_its_whole_height() {
    let h = floor();
    let active = h.env().theme().color("active");
    let accent = h.env().theme().color("accent");
    let (column, row) = tile(0);
    for offset in 0..IconTile::HEIGHT {
        assert_eq!(h.buffer()[(column, row + offset)].symbol(), "▌", "the pillar is the whole height");
        assert_eq!(h.fg(column, row + offset), accent);
        assert_eq!(h.bg(column + 1, row + offset), active, "the tile is on the selected surface");
    }
    // The neighbour keeps the ground, so one chosen tile cannot be mistaken for the whole row.
    let (next, next_row) = tile(1);
    let ground = h.bg(next + 1, next_row);
    assert_eq!(h.buffer()[(next, next_row)].symbol(), " ", "no pillar on a tile that is not chosen");
    assert_ne!(ground, active);
    assert_eq!(h.bg(next + 1, next_row + 1), ground, "a bare tile keeps the ground it stands on");
}

#[test]
fn a_cursor_tile_takes_the_pillar_alone_and_the_selection_keeps_its_own() {
    let h = screen_of(Floor { chosen: 0, on_tile: 2, ..Floor::default() });
    let active = h.env().theme().color("active");
    let accent = h.env().theme().color("accent");
    let (cursor, row) = tile(2);
    let (chosen, chosen_row) = tile(0);
    assert_eq!(h.buffer()[(cursor, row)].symbol(), "▌", "the cursor's tile carries the pillar");
    assert_eq!(h.fg(cursor, row), accent);
    assert_ne!(h.bg(cursor + 1, row), active, "the cursor alone does not select");
    assert_eq!(h.buffer()[(chosen, chosen_row)].symbol(), "▌", "the selection keeps its own pillar");
    assert_eq!(h.bg(chosen + 1, chosen_row), active);
}

#[test]
fn the_pointer_lifts_the_ground_of_a_bare_tile_under_the_mouse() {
    let mut h = floor();
    let (column, row) = tile(1);
    let ground = h.bg(column + 1, row);
    h.hover(i32::from(column + 1), i32::from(row));
    let lit = h.bg(column + 1, row);
    assert_ne!(lit, ground, "the pointer lifts the ground");
    // The lift reaches the tile's own edges, pillar column aside.
    assert_eq!(h.bg(column + IconTile::WIDTH - 1, row), lit, "the whole cell of the tile lights together");
    assert_eq!(h.fg(centred(1, 5), row + 1), h.env().theme().color("text"), "and the name brightens with it");
    h.hover(0, i32::from(HEIGHT - 1));
    assert_eq!(h.bg(column + 1, row), ground, "the lift leaves with the pointer");
}

#[test]
fn a_backed_tile_stands_on_a_tile_of_the_surface_tone_under_its_glyph_and_name_only() {
    let h = screen_of(Floor { backed: true, ..Floor::default() });
    let (column, row) = tile(1);
    let backing = h.bg(column + 1, row);
    assert_eq!(backing, h.env().theme().color("surface"), "the backing is the theme's surface tone");
    assert_ne!(backing, h.bg(column + 1, row + 2), "and the ground under the tile is not");
    assert_eq!(h.bg(column + 1, row + 1), backing, "the name stands on the same tile");
    assert_eq!(backing, h.bg(column + 2, row), "tiles side by side keep a column of the ground between them");
}

#[test]
fn a_faint_tile_quietens_its_glyph_and_its_name() {
    let plain = floor();
    let h = screen_of(Floor { chosen: 1, faint: true, ..Floor::default() });
    let muted = h.env().theme().color("muted");
    assert_eq!(h.fg(centred(0, 1), TILE_ROW), muted, "the glyph steps back");
    assert_eq!(h.fg(centred(0, 5), TILE_ROW + 1), muted, "and so does the name");
    assert_eq!(plain.fg(centred(0, 1), TILE_ROW), h.env().theme().color("text"), "a whole tile does not step back");
}

#[test]
fn a_selected_faint_tile_keeps_the_selection_s_colour_as_any_other_selected_row_does() {
    let h = screen_of(Floor { faint: true, ..Floor::default() });
    let (column, row) = tile(0);
    assert_eq!(h.buffer()[(column, row)].symbol(), "▌", "it is still the chosen one");
    assert_eq!(h.bg(column + 1, row), h.env().theme().color("active"), "on the selected surface");
    assert_eq!(h.fg(centred(0, 5), row + 1), h.env().theme().color("text"), "and its name takes the text colour");
}

#[test]
fn the_glyph_follows_the_glyph_mode_and_a_colored_tile_keeps_its_name_quiet() {
    for (mode, glyph) in [(GlyphMode::Nerd, "\u{f013}"), (GlyphMode::Unicode, "▤"), (GlyphMode::Ascii, "*")] {
        let mut h = screen_of_one(Colored);
        h.set_glyph_mode(mode);
        assert_eq!(h.buffer()[(5, 0)].symbol(), glyph, "{mode:?}: the key is read from the icon set");
    }
    let h = screen_of_one(Colored);
    let accent = h.env().theme().color("accent");
    assert_eq!(h.fg(5, 0), accent, "the glyph takes the colour the tile is given");
    assert_eq!(h.fg(4, 1), h.env().theme().color("dim"), "the name keeps its own tone");
}

/// A single tile, drawn as a person gives a colour to a kind of file.
struct Colored;

impl App for Colored {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(IconTile::new("settings", "Notes").color("accent"));
    }
}

/// A harness showing one tile at the left of the screen, motion off and in Unicode glyphs.
fn screen_of_one<A: App>(app: A) -> Harness<A> {
    let mut h = Harness::new(app, WIDTH, HEIGHT);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true);
    h
}

#[test]
fn a_literal_glyph_is_drawn_as_it_is_in_every_mode() {
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        let mut h = screen_of_one(Tinted);
        h.set_glyph_mode(mode);
        assert_eq!(h.buffer()[(5, 0)].symbol(), "★", "{mode:?}: a character of the application's own");
    }
}

/// A tile whose glyph is a character of the application's own, in the colour of a family.
struct Tinted;

impl App for Tinted {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(IconTile::new(Glyph::literal('★'), "Stars").color("accent"));
    }
}

#[test]
fn every_glyph_of_a_tile_reads_in_every_built_in_theme() {
    let mut h = floor();
    let themes: Vec<String> = h.env().themes().into_iter().map(|(id, _)| id).collect();
    assert!(themes.len() >= 4, "{themes:?}");
    for theme in themes {
        h.set_theme(&theme);
        h.hover(0, i32::from(HEIGHT - 1));
        let mut checks = Vec::new();
        for index in 0..3 {
            let (_, row) = tile(index);
            checks.push((format!("{theme} icon {index}"), h.fg(centred(index, 1), row), h.bg(centred(index, 1), row)));
            checks.push((
                format!("{theme} name {index}"),
                h.fg(centred(index, 5), row + 1),
                h.bg(centred(index, 5), row + 1),
            ));
        }
        let (column, row) = tile(1);
        h.hover(i32::from(column + 1), i32::from(row));
        checks.push((format!("{theme} hovered name"), h.fg(centred(1, 5), row + 1), h.bg(centred(1, 5), row + 1)));
        for (what, fg, bg) in checks {
            let (Some(fg), Some(bg)) = (fg, bg) else { panic!("{what}: no colour") };
            let ratio = fg.contrast_ratio(bg);
            assert!(ratio >= 4.5, "{what}: contrast {ratio:.2}\n{}", h.screen());
        }
    }
}

#[test]
fn a_chosen_tile_is_told_by_its_pillar_when_the_surfaces_collapse_in_sixteen_colours() {
    // Reduced to the nearest of the sixteen, a theme's dark surfaces land on black, so a chosen
    // tile's ground can vanish into the ground it stands on. The pillar is what is left, which is
    // why the first column is kept for it in every state.
    for depth in [ColorDepth::Ansi16, ColorDepth::Ansi256] {
        let mut h = floor();
        h.set_depth(depth);
        let (column, row) = tile(0);
        let pillar = h.buffer()[(column, row)].clone();
        let beside = h.buffer()[(column + 1, row)].clone();
        assert_eq!(pillar.symbol(), "▌", "{depth:?}: the pillar is drawn");
        assert_ne!(pillar.fg, beside.fg, "{depth:?}: and it is a colour of its own beside the ground");
        for index in 1..3 {
            let (x, y) = tile(index);
            assert_eq!(h.buffer()[(x, y)].symbol(), " ", "{depth:?}: a tile that is not chosen has no pillar");
        }
        // The glyph and the name still read, whatever the palette left of their colours.
        for index in 0..3 {
            let (_, y) = tile(index);
            for (what, cell) in [
                ("icon", h.buffer()[(centred(index, 1), y)].clone()),
                ("name", h.buffer()[(centred(index, 5), y + 1)].clone()),
            ] {
                let ratio = palette(cell.fg).contrast_ratio(palette(cell.bg));
                assert!(ratio > 1.6, "{depth:?}: the {what} of tile {index} keeps only {ratio:.2}:1\n{}", h.screen());
            }
        }
    }
}

/// The colour a palette entry is, as xterm shows it.
fn palette(color: Color) -> Rgb {
    match color {
        Color::Indexed(index) if index < 16 => Rgb::from_ansi16(index),
        Color::Indexed(index) => Rgb::from_ansi256(index),
        Color::Rgb(r, g, b) => Rgb::new(r, g, b),
        other => panic!("{other:?} is not a colour a frame sends"),
    }
}

/// A floor that counts every message its tiles' surface sends.
#[derive(Default)]
struct Quiet {
    heard: usize,
}

impl App for Quiet {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        self.heard += 1;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.row(|ui| {
            ui.add(IconTile::new("file", "Notes").selected(true));
            ui.add(IconTile::new("folder", "Photos"));
        })
        .fill_width();
    }
}

#[test]
fn a_tile_is_drawn_and_not_driven_so_the_surface_around_it_keeps_every_key_and_press() {
    let mut h = screen_of_one(Quiet::default());
    h.press("tab").press("enter").press("right").press("space");
    h.click(4, 0).click(4, 0);
    assert_eq!(h.app().heard, 0, "a tile answers nothing: {}", h.screen());
}

#[test]
fn a_tile_shrinks_to_the_room_it_is_given_and_draws_what_fits() {
    let mut h = Harness::new(Narrow, WIDTH, HEIGHT);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true);
    // Six columns leave five beside the pillar: the glyph is centred in what is left and the name
    // is cut to the room there is.
    let glyph = h.env().icons().glyph("file").into_owned();
    assert_eq!(h.find(&glyph), Some((3, 0)), "the glyph is centred in the room the tile has");
    assert!(h.screen().contains("Repo…"), "the name is cut there: {}", h.screen());
}

/// A tile in a column too narrow for a whole one.
struct Narrow;

impl App for Narrow {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(IconTile::new("file", "Reports")).width(Length::Cells(6));
    }
}
