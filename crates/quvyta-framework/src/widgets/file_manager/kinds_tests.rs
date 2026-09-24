//! A file manager's icons by kind, on a real temporary folder, driven from where the person
//! looks and clicks.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::*;
use crate::color::{ColorDepth, Rgb};
use crate::icons::{GlyphMode, UserFolders};
use crate::runtime::{App, Command, Harness};
use crate::widget::View;

/// A folder of this test's own, removed when the test ends: a Rust file, a note, a program, a
/// file of no kind, and the folders `src` and `İndirilenler`.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = std::env::temp_dir().join(format!("qframe-file-kinds-{name}-{stamp}"));
        fs::create_dir_all(path.join("src")).expect("a folder");
        fs::create_dir_all(path.join("İndirilenler")).expect("a folder");
        fs::write(path.join("main.rs"), "fn main() {}\n").expect("a file");
        fs::write(path.join("notes.txt"), "hello\n").expect("a file");
        fs::write(path.join("mystery"), "?\n").expect("a file");
        fs::write(path.join("run"), "#!/bin/sh\n").expect("a file");
        fs::set_permissions(path.join("run"), fs::Permissions::from_mode(0o755)).expect("a program");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An application showing one file manager with the kind options it is given.
struct Demo {
    manager: FileManagerState,
    view: FileView,
    icons: bool,
    tones: bool,
    marked: Option<(String, RowMark)>,
    home: Option<UserFolders>,
}

#[derive(Clone)]
enum Msg {
    Files(FileManagerMsg),
    /// The application marks this entry this way.
    Mark(String, RowMark),
    /// The manager knows this home from now on.
    Home(UserFolders),
}

impl App for Demo {
    type Msg = Msg;

    fn init(&mut self) -> Command<Msg> {
        self.manager.load(Msg::Files)
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Files(message) => return self.manager.update(message, Msg::Files),
            Msg::Mark(key, mark) => self.marked = Some((key, mark)),
            Msg::Home(home) => self.home = Some(home),
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut manager =
            FileManager::new(&self.manager, Msg::Files).view(self.view).kind_icons(self.icons).kind_tones(self.tones);
        if let Some((key, mark)) = self.marked.clone() {
            manager = manager.row_mark(move |row| if row == key { mark.clone() } else { RowMark::new() });
        }
        if let Some(home) = &self.home {
            manager = manager.user_folders(home);
        }
        manager.show(ui).fill().id("files");
    }
}

/// The manager over `scratch`, drawn with Nerd Font glyphs.
fn harness(scratch: &Scratch, view: FileView, icons: bool, tones: bool) -> Harness<Demo> {
    let demo = Demo { manager: FileManagerState::new(scratch.0.clone()), view, icons, tones, marked: None, home: None };
    let mut h = Harness::new(demo, 72, 20);
    h.set_glyph_mode(GlyphMode::Nerd).set_reduced_motion(true).render();
    h
}

/// The glyph the icon `key` is drawn with in the harness' glyph mode.
fn glyph(h: &Harness<Demo>, key: &str) -> char {
    h.env().icons().glyph(key).chars().next().expect("a glyph")
}

/// The cell of the icon on the row of `name`: the glyph two cells before the name, and its colour.
fn icon_on(h: &Harness<Demo>, name: &str) -> (char, Option<Rgb>) {
    let (x, y) = h.find(name).unwrap_or_else(|| panic!("`{name}` is on screen:\n{}", h.screen()));
    let (x, y) = (u16::try_from(x - 2).expect("a column"), u16::try_from(y).expect("a row"));
    let line = h.screen().lines().nth(usize::from(y)).expect("the line").to_owned();
    let glyph = line.chars().nth(usize::from(x)).expect("the icon's cell");
    (glyph, h.fg(x, y))
}

/// The colour the terminal is sent for the icon on the row of `name`, a palette entry as well as a
/// colour of its own.
fn sent_colour(h: &Harness<Demo>, name: &str) -> ratatui_core::style::Color {
    let (x, y) = h.find(name).unwrap_or_else(|| panic!("`{name}` is on screen:\n{}", h.screen()));
    h.buffer()[(u16::try_from(x - 2).expect("a column"), u16::try_from(y).expect("a row"))].fg
}

/// The colour the name `name` itself is drawn in: the row's own colour.
fn name_colour(h: &Harness<Demo>, name: &str) -> Option<Rgb> {
    let (x, y) = h.find(name).unwrap_or_else(|| panic!("`{name}` is on screen:\n{}", h.screen()));
    h.fg(u16::try_from(x).expect("a column"), u16::try_from(y).expect("a row"))
}

/// The theme colour `token`.
fn token(h: &Harness<Demo>, token: &str) -> Option<Rgb> {
    h.env().theme().color(token)
}

#[test]
fn a_rust_file_shows_the_rust_glyph_quiet_until_its_row_is_clicked() {
    let scratch = Scratch::new("list");
    let mut h = harness(&scratch, FileView::List, true, false);
    let rust = glyph(&h, "file-rust");
    let (icon, colour) = icon_on(&h, "main.rs");
    assert_eq!(icon, rust, "the Rust glyph on the Rust file:\n{}", h.screen());
    assert_eq!(colour, token(&h, "muted"), "a kind has no colour of its own");

    h.click_text("main.rs").render();
    assert_eq!(state(&h).selected(), Some("main.rs"), "the click selected the row");
    let (icon, colour) = icon_on(&h, "main.rs");
    assert_eq!(icon, rust);
    assert_eq!(colour, name_colour(&h, "main.rs"), "the selected row's icon takes the row's colour");
    assert_ne!(colour, token(&h, "muted"));
}

fn state(h: &Harness<Demo>) -> &FileManagerState {
    &h.app().manager
}

#[test]
fn without_the_option_every_file_is_a_plain_file() {
    let scratch = Scratch::new("off");
    let h = harness(&scratch, FileView::List, false, false);
    assert_eq!(icon_on(&h, "main.rs").0, glyph(&h, "file"), "{}", h.screen());
    assert_eq!(icon_on(&h, "src").0, glyph(&h, "folder"));
}

#[test]
fn every_row_takes_its_kind_in_the_tree_the_list_and_the_icons() {
    let scratch = Scratch::new("views");
    for view in [FileView::Tree, FileView::List, FileView::Icons] {
        let h = harness(&scratch, view, true, false);
        let expected = [
            ("main.rs", "file-rust"),
            ("notes.txt", "file-text"),
            ("mystery", "file"),
            ("run", "file-executable"),
            ("src", "folder-source"),
        ];
        for (name, key) in expected {
            assert_eq!(icon_on(&h, name).0, glyph(&h, key), "{name} in {view:?}:\n{}", h.screen());
        }
        // No colour of its own in any view: the icon is drawn as the plain one would be.
        let plain = harness(&scratch, view, false, false);
        assert_eq!(icon_on(&h, "main.rs").1, icon_on(&plain, "main.rs").1, "{view:?}");
    }
}

#[test]
fn a_sign_of_the_application_wins_over_the_kind() {
    let scratch = Scratch::new("mark");
    let mut h = harness(&scratch, FileView::List, true, false);
    h.send(Msg::Mark("main.rs".to_owned(), RowMark::new().sign("warning", "warning"))).render();
    let (icon, colour) = icon_on(&h, "main.rs");
    assert_eq!(icon, glyph(&h, "warning"), "{}", h.screen());
    assert_eq!(colour, token(&h, "warning"));
}

#[test]
fn tones_colour_the_families_only_where_they_can_be_told_apart() {
    let scratch = Scratch::new("tones");
    let mut h = harness(&scratch, FileView::List, true, true);
    assert_eq!(icon_on(&h, "main.rs").1, token(&h, "series-2"), "code takes its family's tone");
    assert_eq!(icon_on(&h, "src").1, token(&h, "accent"), "folders take the accent");
    assert_eq!(icon_on(&h, "mystery").1, token(&h, "muted"), "a file of no kind keeps the row's colour");

    // Tones need kinds: alone they do nothing.
    let alone = harness(&scratch, FileView::List, false, true);
    assert_eq!(icon_on(&alone, "main.rs").1, token(&alone, "muted"));

    // In sixteen colours and in ASCII the icon is drawn as it is without tones.
    let mut plain = harness(&scratch, FileView::List, true, false);
    for (depth, mode) in [(ColorDepth::Ansi16, GlyphMode::Nerd), (ColorDepth::TrueColor, GlyphMode::Ascii)] {
        h.set_depth(depth).set_glyph_mode(mode).render();
        plain.set_depth(depth).set_glyph_mode(mode).render();
        for name in ["main.rs", "src"] {
            assert_eq!(sent_colour(&h, name), sent_colour(&plain, name), "{name} in {depth:?} {mode:?}");
        }
    }
}

#[test]
fn the_home_and_its_folders_are_known_by_the_persons_own_names() {
    let scratch = Scratch::new("home");
    let mut h = harness(&scratch, FileView::Tree, true, false);
    assert_eq!(icon_on(&h, "İndirilenler").0, glyph(&h, "folder"), "no home is known yet");
    let text = "XDG_DOWNLOAD_DIR=\"$HOME/İndirilenler\"\n";
    h.send(Msg::Home(UserFolders::parse(scratch.0.clone(), text))).render();
    assert_eq!(icon_on(&h, "İndirilenler").0, glyph(&h, "folder-downloads"), "{}", h.screen());
    let root = scratch.0.file_name().and_then(|name| name.to_str()).expect("the root's name").to_owned();
    assert_eq!(icon_on(&h, &root).0, glyph(&h, "folder-home"), "the root is the home");
    assert_eq!(icon_on(&h, "src").0, glyph(&h, "folder-source"), "other folders keep their kind");
}
