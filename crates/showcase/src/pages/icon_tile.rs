//! Icon tile: the desktop's own icon, a glyph over a centred name, and a grid of them as a file
//! manager draws its entries.

use qframe::prelude::*;
use qframe::widgets::{CardGrid, Click, IconTile, Select};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "icon-tile";

/// The demo folder: what icon each entry is drawn with, and the key its name is written in.
const ENTRIES: [(&str, &str); 6] = [
    ("folder", "icon-tile.entry-photos"),
    ("file-image", "icon-tile.entry-wallpaper"),
    ("file-archive", "icon-tile.entry-backups"),
    ("file", "icon-tile.entry-notes"),
    ("file-shell", "icon-tile.entry-build"),
    ("file", "icon-tile.entry-midnight"),
];

/// Icons the playground offers a tile, by their key in the icon set.
const ICONS: [&str; 5] = ["folder", "file-image", "file-shell", "file-archive", "settings"];

/// The series tones a kind may be told by, so a grid of coloured icons still reads as one.
const SERIES: [&str; 5] = ["series-1", "series-2", "series-3", "series-4", "series-5"];

/// What the demo folder shows and what its tiles are told.
#[derive(Debug)]
pub struct State {
    selected: Option<usize>,
    /// The tile the keyboard is on, which carries the pillar of its own.
    on_tile: usize,
    icon: usize,
    backed: bool,
    faint: bool,
    /// Whether the glyph takes the colour of the series that matches its kind.
    coloured: bool,
    /// Whether the names carry a year after them, so a tile is too narrow for them.
    long: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { selected: Some(0), on_tile: 1, icon: 0, backed: true, faint: false, coloured: true, long: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Select(usize),
    Open(usize),
    Icon(usize),
    Backed(bool),
    Faint(bool),
    Coloured(bool),
    Long(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::IconTile(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Select(index) => {
            state.selected = Some(index);
            state.on_tile = index;
            log.push(PAGE, "IconTile#entries", format!("selected {index}"));
        }
        Msg::Open(index) => {
            state.on_tile = index;
            log.push(PAGE, "IconTile#entries", format!("opened {index}"));
        }
        Msg::Icon(index) => {
            state.icon = index;
            log.push(PAGE, "Playground", format!("icon = {}", ICONS[index]));
        }
        Msg::Backed(on) => {
            state.backed = on;
            log.push(PAGE, "Playground", format!("backed = {on}"));
        }
        Msg::Faint(on) => {
            state.faint = on;
            log.push(PAGE, "Playground", format!("faint = {on}"));
        }
        Msg::Coloured(on) => {
            state.coloured = on;
            log.push(PAGE, "Playground", format!("coloured = {on}"));
        }
        Msg::Long(on) => {
            state.long = on;
            log.push(PAGE, "Playground", format!("long names = {on}"));
        }
    }
    Command::none()
}

/// The name the tile of `index` shows: the entry's own, or one with a year after it.
fn entry_name(state: &State, index: usize) -> String {
    let name = t!(ENTRIES[index % ENTRIES.len()].1);
    if state.long { format!("{name} 2026") } else { name }
}

// region: icon-grid
/// The folder as a grid of tiles: each stands in a cell of ten by three, so nothing moves when one
/// of them is chosen.
fn grid(ui: &mut View<'_, AppMsg>, state: &State) {
    let chosen = state.selected;
    let on_tile = state.on_tile;
    let (faint, coloured) = (state.faint, state.coloured);
    // What every tile shows is worked out here, because the grid keeps the closure for as long as
    // it is on screen.
    let tiles: Vec<(&str, String, Option<&'static str>)> = ENTRIES
        .iter()
        .enumerate()
        .map(|(index, (key, _))| (*key, entry_name(state, index), tone_of(index, coloured)))
        .collect();
    let grid = CardGrid::new(ENTRIES.len())
        .card_width(IconTile::WIDTH, IconTile::WIDTH)
        .card_height(IconTile::HEIGHT)
        .gap(1, 1)
        .bare_cards(true)
        .activate_on(Click::Double)
        .selected(chosen)
        .on_select(|index| send(Msg::Select(index)))
        .on_activate(|index| send(Msg::Open(index)))
        .card(move |ui, index| {
            let Some((key, name, tone)) = tiles.get(index) else { return };
            // A tile carries the cursor's pillar of its own only while another tile is chosen, so
            // the two are never told by the same column.
            let cursor = on_tile == index && chosen != Some(index);
            let mut tile =
                IconTile::new(*key, name.clone()).selected(chosen == Some(index)).cursor(cursor).faint(faint);
            if let Some(tone) = *tone {
                tile = tile.color(tone);
            }
            ui.add(tile);
        });
    ui.add(grid).fill_width().id("entries");
}
// endregion

/// The series tone the kind of the entry at `index` is told by, or none to keep every glyph the
/// same colour. The name under it keeps its own tone either way.
fn tone_of(index: usize, coloured: bool) -> Option<&'static str> {
    coloured.then(|| SERIES[index % SERIES.len()])
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("icon-tile.hint")).role("secondary"));
        grid(ui, state);
        ui.spacer().height(Length::Cells(1));
        // A floor over a picture: the inset panel is a ground of its own, so a backed tile can be
        // told from a bare one; the cursor stands on the tile the keys are on.
        ui.add_with(Panel::new().variant("inset").gap(0), |ui| {
            // region: icon-floor
            ui.row(|ui| {
                for (index, (key, _)) in ENTRIES.iter().enumerate().take(3) {
                    ui.add(
                        IconTile::new(*key, entry_name(state, index))
                            .selected(state.selected == Some(index))
                            .cursor(state.on_tile == index && state.selected != Some(index))
                            .backed(state.backed)
                            .faint(state.faint),
                    );
                }
            })
            .gap(1)
            .fill_width();
            // endregion
        })
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        let icons = ICONS.map(|icon| t!(&format!("icon-tile.icon.{icon}")));
        setting(ui, t!("icon-tile.icon-label"), |ui| {
            ui.add(Select::new(icons).selected(Some(state.icon)).on_select(|i| send(Msg::Icon(i))))
                .width(Length::Cells(18))
                .id("icon");
        });
        setting(ui, t!("icon-tile.backed"), |ui| {
            ui.add(toggle(state.backed, |on| send(Msg::Backed(on)))).id("backed");
        });
        setting(ui, t!("icon-tile.faint"), |ui| {
            ui.add(toggle(state.faint, |on| send(Msg::Faint(on)))).id("faint");
        });
        setting(ui, t!("icon-tile.coloured"), |ui| {
            ui.add(toggle(state.coloured, |on| send(Msg::Coloured(on)))).id("coloured");
        });
        setting(ui, t!("icon-tile.long"), |ui| {
            ui.add(toggle(state.long, |on| send(Msg::Long(on)))).id("long");
        });
        // region: icon-configured
        setting(ui, t!("icon-tile.configured"), |ui| {
            ui.add(IconTile::new(ICONS[state.icon], t!("icon-tile.configured-name")).backed(state.backed))
                .id("configured");
        });
        // endregion
        ui.add(Text::new(t!("icon-tile.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::{click_setting, showcase_on};
    use qframe::runtime::Harness;

    /// The cells a text spans.
    fn cells(text: &str) -> i32 {
        i32::from(qframe::text::width(text))
    }

    /// The column the tile whose name starts at `name` begins at: the name is centred in the
    /// columns beside the pillar's column.
    fn tile_of(name: i32, width: i32) -> i32 {
        name - i32::from(IconTile::PILLAR) - (i32::from(IconTile::WIDTH - IconTile::PILLAR) - width) / 2
    }

    /// The column and row of the last `text` on screen, for a word the demo shows more than once:
    /// the grid of tiles first, the floor of three tiles under it.
    fn floor_of(h: &Harness<Showcase>, text: &str) -> (i32, i32) {
        let screen = h.screen();
        let (y, line) = screen
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(text))
            .last()
            .unwrap_or_else(|| panic!("`{text}` is on screen:\n{screen}"));
        let x = line.rfind(text).map_or(0, |byte| line[..byte].chars().count());
        (i32::try_from(x).unwrap_or(0), i32::try_from(y).unwrap_or(0))
    }

    #[test]
    fn a_tile_s_glyph_and_name_share_one_centred_column_span_one_under_the_other() {
        let mut h = showcase_on(PAGE);
        h.set_glyph_mode(qframe::icons::GlyphMode::Unicode);
        let glyph = h.env().icons().glyph("folder").into_owned();
        let (gx, gy) = h.find(&glyph).expect("the first tile's glyph");
        let (nx, ny) = h.find("Photos").expect("the first tile's name");
        assert_eq!(ny, gy + 1, "the name is one row under the glyph: {}", h.screen());
        let column = tile_of(nx, cells("Photos"));
        let span = i32::from(IconTile::WIDTH - IconTile::PILLAR);
        assert_eq!(
            gx,
            column + i32::from(IconTile::PILLAR) + (span - cells(&glyph)) / 2,
            "the glyph is centred beside the pillar's column"
        );
        assert_eq!(
            nx - column,
            i32::from(IconTile::PILLAR) + (span - cells("Photos")) / 2,
            "and the name is centred in the same span"
        );
        assert!(nx + cells("Photos") <= column + i32::from(IconTile::WIDTH), "and never runs into the next pillar");
    }

    #[test]
    fn the_chosen_tile_takes_the_active_ground_and_the_pillar_down_its_whole_height() {
        let h = showcase_on(PAGE);
        let (name, row) = h.find("Photos").expect("the first tile's name");
        let (column, top) = (tile_of(name, cells("Photos")), row - 1);
        let (column, name) = (u16::try_from(column).unwrap_or(0), u16::try_from(name).unwrap_or(0));
        let active = h.env().theme().color("active");
        let accent = h.env().theme().color("accent");
        for offset in 0..i32::from(IconTile::HEIGHT) {
            let y = u16::try_from(top + offset).unwrap_or(0);
            assert_eq!(h.buffer()[(column, y)].symbol(), "▌", "the pillar is the tile's first column");
            assert_eq!(h.fg(column, y), accent);
            assert_eq!(h.bg(name, y), active, "the chosen tile is on the selected ground");
        }
        // The tile the keys are on carries the pillar alone, and the tile after it carries none, so
        // one chosen tile cannot be mistaken for the whole row.
        let (beside, y) = (column + IconTile::WIDTH + 1, u16::try_from(row).unwrap_or(0));
        assert_eq!(h.buffer()[(beside, y)].symbol(), "▌", "the cursor's tile takes the pillar too");
        assert_ne!(h.bg(beside + 1, y), active, "but the pillar alone does not choose it");
        let (next, _) = (beside + IconTile::WIDTH + 1, y);
        assert_eq!(h.buffer()[(next, y)].symbol(), " ", "a tile that is neither chosen nor on keeps no pillar");
        assert_ne!(h.bg(next + 1, y), active);
    }

    #[test]
    fn the_keys_and_the_mouse_move_the_selection_as_a_grid_of_tiles_does() {
        let mut h = showcase_on(PAGE);
        // A click selects and a double click opens, as a desktop's icons do; the log is below the
        // page's own panels, so the screen is given the rows it needs to be read in.
        h.resize(140, 58);
        h.click_text("Notes");
        assert_eq!(h.app().pages.icon_tile.selected, Some(3), "{}", h.screen());
        assert!(!h.screen().contains("opened"), "a single click only chooses: {}", h.screen());
        h.press("right");
        assert_eq!(h.app().pages.icon_tile.selected, Some(4));
        h.press("right");
        assert_eq!(h.app().pages.icon_tile.selected, Some(5));
        h.press("right");
        assert_eq!(h.app().pages.icon_tile.selected, Some(5), "the end of a row of six stops there");
        h.press("left");
        assert_eq!(h.app().pages.icon_tile.selected, Some(4));
        h.press("down");
        assert_eq!(h.app().pages.icon_tile.selected, Some(4), "and one row of tiles has no row under it");
        h.press("enter");
        assert!(h.screen().contains("opened 4"), "{}", h.screen());
        let (x, y) = h.find("Backups").expect("the third tile of the row");
        h.click(x, y).click(x, y);
        assert!(h.screen().contains("opened 2"), "a double click opens the tile under it: {}", h.screen());
    }

    #[test]
    fn a_backed_tile_stands_on_the_surface_tone_and_a_long_name_is_cut() {
        let mut h = showcase_on(PAGE);
        let surface = h.env().theme().color("surface");
        // The floor of three tiles under the grid, where every tile stands over a picture.
        let (name, row) = floor_of(&h, "Wallpaper");
        let beside = u16::try_from(tile_of(name, cells("Wallpaper")) + i32::from(IconTile::PILLAR)).unwrap_or(0);
        let row = u16::try_from(row).unwrap_or(0);
        assert_eq!(h.bg(beside, row), surface, "a backed tile stands on the surface tone: {}", h.screen());
        click_setting(&mut h, "Over a picture");
        assert_ne!(h.bg(beside, row), surface, "and a bare tile keeps the ground it is given");
        click_setting(&mut h, "Over a picture");
        click_setting(&mut h, "Long names");
        assert!(h.screen().contains("Photos 2…"), "a name too wide for a tile is cut: {}", h.screen());
        h.set_locale("tr");
        assert!(h.screen().contains("Notlar"), "and the tiles follow the language: {}", h.screen());
    }
}
