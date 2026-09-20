//! Whole-showcase tests (the catalog, the aesthetics rules, the shell and sweeps over every page)
//! and the harness helpers that page tests share.

use qframe::env::Env;
use qframe::event::{MouseButton, MouseKind};
use qframe::icons::GlyphMode;
use qframe::runtime::Harness;

use crate::app::{Msg, Showcase};

/// Terminal size used by showcase tests.
pub const SIZE: (u16, u16) = (140, 44);

/// The showcase environment as the installed program builds it: built-in files plus the
/// showcase's compiled-in locales and keymap, English, Unicode glyphs.
pub fn env() -> Env {
    Env::load(&crate::assets::dirs()).expect("compiled-in files need no disk")
}

/// The folder the disk demos start in under test: this crate's own folder, whose contents the
/// tests know, where the installed program uses the home folder.
pub fn home_folder() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A showcase harness as it starts, before any input.
pub fn fresh() -> Harness<Showcase> {
    Harness::with_env(Showcase::new(), env(), SIZE.0, SIZE.1)
}

/// A showcase harness showing `page`.
pub fn showcase_on(page: &str) -> Harness<Showcase> {
    showcase_tall(Showcase::new(), page, SIZE.1)
}

/// A harness showing `page` of `showcase` in a terminal `height` rows tall, for pages whose lower
/// panels sit below the rows of `SIZE`.
pub fn showcase_tall(showcase: Showcase, page: &str, height: u16) -> Harness<Showcase> {
    let mut harness = Harness::with_env(showcase, env(), SIZE.0, height);
    harness.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    harness.send(Msg::Open(page.to_owned()));
    // Let the page transition finish so tests see the page itself.
    harness.advance(std::time::Duration::from_secs(1));
    harness
}

/// Clicks the first occurrence of `text` at or below `row`. Menus open under the text they act
/// on, while hints above may name the same entries.
pub fn click_text_below(harness: &mut Harness<Showcase>, text: &str, row: i32) {
    let screen = harness.screen();
    let skip = usize::try_from(row).unwrap_or(0);
    let found = screen.lines().enumerate().skip(skip).find_map(|(y, line)| {
        let column = line[..line.find(text)?].chars().count();
        Some((i32::try_from(column).ok()?, i32::try_from(y).ok()?))
    });
    let (x, y) = found.unwrap_or_else(|| panic!("`{text}` is not on screen below row {row}:\n{screen}"));
    harness.click(x, y);
}

/// Presses and releases the right mouse button at `x`, `y`.
pub fn right_click(harness: &mut Harness<Showcase>, x: i32, y: i32) {
    harness.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
}

/// The close mark's column and row for a dialog whose first content row shows `title`: the mark
/// sits in the surface's top right corner, on the row above. Without a mark, the column is `None`
/// and the row is the last row showing `title`. Dialog titles can repeat a button label shown
/// beneath the dialog.
pub fn close_mark_beside(harness: &Harness<Showcase>, title: &str) -> (Option<i32>, i32) {
    let screen = harness.screen();
    let lines: Vec<&str> = screen.lines().collect();
    let rows: Vec<(usize, &str)> = lines.iter().copied().enumerate().filter(|(_, line)| line.contains(title)).collect();
    assert!(!rows.is_empty(), "`{title}` on screen:\n{screen}");
    let marked = rows.iter().find_map(|(y, _)| {
        let above = y.checked_sub(1)?;
        Some((lines[above].chars().position(|c| c == '×')?, above))
    });
    let to_i32 = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
    match marked {
        Some((x, y)) => (Some(to_i32(x)), to_i32(y)),
        None => (None, to_i32(rows.last().map_or(0, |(y, _)| *y))),
    }
}

mod fuzz;
mod reel;

mod sweeps {
    use qframe::icons::GlyphMode;
    use qframe::runtime::Harness;

    use super::{env, showcase_on};
    use crate::app::{Msg, Showcase};

    /// Every page and section draws in tiny and narrow terminals without panicking; the framework
    /// must shrink gracefully, never crash.
    #[test]
    fn every_page_survives_tiny_terminals() {
        for page in crate::pages::PAGES {
            for (width, height) in [(1, 1), (2, 3), (7, 4), (19, 9), (40, 12)] {
                let mut harness = Harness::with_env(Showcase::new(), env(), width, height);
                harness.set_glyph_mode(GlyphMode::Unicode);
                harness.send(Msg::Open(page.id.to_owned()));
                for section in 0..4 {
                    harness.send(Msg::Section(section));
                    harness.advance(std::time::Duration::from_millis(300));
                }
            }
        }
    }

    /// Writes every page, section and theme to `target/showcase-review.html` when
    /// `QUVYTA_REVIEW` is set, for visual review in a browser.
    #[test]
    fn visual_review() {
        use qframe::runtime::html_page;

        if std::env::var_os("QUVYTA_REVIEW").is_none() {
            return;
        }
        let mut fragments = Vec::new();
        let pages: Vec<&str> = crate::pages::PAGES.iter().map(|page| page.id).collect();
        for theme in ["monochrome", "nordic", "amber", "iris"] {
            for page in &pages {
                let mut harness = showcase_on(page);
                harness.set_theme(theme);
                for (index, section) in ["Demo", "Code", "Guide", "Reference"].iter().enumerate() {
                    harness.send(Msg::Section(index));
                    fragments.push(harness.html(&format!("{theme} · {page} · {section}")));
                }
            }
        }
        let target = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/showcase-review.html");
        std::fs::write(target, html_page(&fragments)).expect("review file is writable");
    }
}

mod catalog {
    use std::collections::BTreeSet;

    use crate::catalog::{Catalog, GROUPS};
    use crate::pages::{FOUNDATIONS, PAGES, content};
    use crate::regions;

    /// The framework's list of public widget types, read from its source.
    const WIDGET_EXPORTS: &str = include_str!("../../quvyta-framework/src/widgets/mod.rs");

    fn exported_widget_types() -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for line in WIDGET_EXPORTS.lines().filter(|line| line.starts_with("pub use ")) {
            let path = line.trim_start_matches("pub use ").trim_end_matches(';');
            let list = match path.split_once("::{") {
                Some((_, rest)) => rest.trim_end_matches('}').to_owned(),
                None => path.rsplit("::").next().unwrap_or_default().to_owned(),
            };
            names.extend(list.split(',').map(|name| name.trim().to_owned()).filter(|name| !name.is_empty()));
        }
        names
    }

    #[test]
    fn entries_are_valid_and_unique() {
        let catalog = Catalog::load();
        let mut ids = BTreeSet::new();
        for item in &catalog.items {
            assert!(ids.insert(item.id.clone()), "duplicate id {}", item.id);
            assert!(GROUPS.contains(&item.group.as_str()), "{}: unknown group {}", item.id, item.group);
            assert!(["infrastructure", "widget", "example"].contains(&item.kind.as_str()), "{}: kind", item.id);
            assert!(["p0", "p1", "p2", "p3"].contains(&item.priority.as_str()), "{}: priority", item.id);
            if !item.done {
                assert!(
                    item.types.is_empty() && item.page.is_none(),
                    "{} is planned but claims types or a page",
                    item.id
                );
            }
        }
    }

    #[test]
    fn no_ghost_widgets() {
        let catalog = Catalog::load();
        let covered: BTreeSet<&str> =
            catalog.items.iter().flat_map(|item| item.types.iter().map(String::as_str)).collect();
        for name in exported_widget_types() {
            assert!(covered.contains(name.as_str()), "public widget type `{name}` is not covered by any catalog item");
        }
    }

    #[test]
    fn every_done_item_has_a_complete_page() {
        let catalog = Catalog::load();
        for item in catalog.items.iter().filter(|item| item.done) {
            let page_id = item.page.as_deref().unwrap_or(&item.id);
            let Some(page) = content(page_id) else {
                panic!("done item `{}` has no showcase page `{page_id}`", item.id);
            };
            assert!(!regions::extract(page.source).is_empty(), "page `{page_id}` shows no code regions");
            for (kind, texts) in [("guide", page.guide), ("reference", page.reference)] {
                for (language, text) in ["en", "tr"].iter().zip(texts) {
                    assert!(text.contains("## "), "page `{page_id}` {kind}.{language}.md has no sections");
                }
            }
        }
    }

    /// A region that is never closed, or opened inside another, silently disappears from the Code
    /// section, so every marker must yield a region.
    #[test]
    fn every_region_marker_yields_a_region() {
        for page in PAGES {
            let markers = page.source.lines().filter(|line| line.trim().starts_with("// region:")).count();
            let regions = regions::extract(page.source);
            assert_eq!(regions.len(), markers, "page `{}` loses a region: {regions:#?}", page.id);
        }
    }

    #[test]
    fn every_page_belongs_to_the_catalog() {
        let catalog = Catalog::load();
        for page in PAGES {
            let foundation = FOUNDATIONS.contains(&page.id);
            let referenced = catalog
                .items
                .iter()
                .any(|item| item.done && (item.id == page.id || item.page.as_deref() == Some(page.id)));
            assert!(referenced, "page `{}` is not the page of any done catalog item", page.id);
            if foundation {
                assert!(catalog.get(page.id).is_none(), "foundation page `{}` must not also be an item", page.id);
            }
        }
    }

    #[test]
    fn showcase_text_is_complete_in_every_language() {
        let env = super::env();
        let i18n = env.i18n();
        assert!(i18n.diagnostics().is_empty(), "{:?}", i18n.diagnostics());
        assert_eq!(i18n.missing_keys("tr", "en"), Vec::<String>::new());
        assert_eq!(i18n.missing_keys("en", "tr"), Vec::<String>::new());
        let catalog = Catalog::load();
        let ids = catalog.items.iter().map(|item| item.id.as_str()).chain(FOUNDATIONS);
        for id in ids {
            let key = format!("names.{id}");
            assert!(i18n.has("en", &key) && i18n.has("tr", &key), "no name for `{id}`");
        }
        assert!(env.keymap().conflicts().is_empty(), "{:?}", env.keymap().conflicts());
    }
}

/// The showcase as an installed program runs it: from any folder, with no repository beside it.
mod installed {
    use std::path::{Path, PathBuf};

    use qframe::env::Env;
    use qframe::runtime::Harness;

    use super::SIZE;
    use crate::app::{Msg, Showcase};
    use crate::pages::PAGES;

    /// The environment is built from compiled-in text alone, and every page and section of the
    /// whole application draws from it in every language.
    #[test]
    fn every_page_draws_from_compiled_in_files_only() {
        let dirs = crate::assets::dirs();
        let paths = [&dirs.themes, &dirs.icons, &dirs.locales, &dirs.keymap];
        assert!(paths.iter().all(|path| path.is_none()), "a path is read at start: {dirs:?}");
        let env = Env::load(&dirs).expect("compiled-in files need no disk");
        assert!(env.diagnostics().is_empty(), "{:?}", env.diagnostics());
        for language in ["en", "tr"] {
            for page in PAGES {
                let mut harness = Harness::with_env(Showcase::new(), env.clone(), SIZE.0, SIZE.1);
                harness.set_locale(language);
                harness.send(Msg::Open(page.id.to_owned()));
                for section in 0..4 {
                    harness.send(Msg::Section(section));
                    harness.advance(std::time::Duration::from_millis(300));
                    let screen = harness.screen();
                    assert!(!screen.trim().is_empty(), "{language} {} section {section} is blank", page.id);
                }
            }
        }
    }

    /// Every `.rs` file under `dir`, recursively.
    fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("the source folder is readable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
    }

    /// A path built from the build folder exists on the machine that compiled the showcase and
    /// nowhere else, and a file compiled in from outside this crate is not in its package. The
    /// program's code, tests apart, uses neither.
    #[test]
    fn the_program_reads_nothing_from_the_build_folder() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let crate_dir = root.canonicalize().expect("the crate folder exists");
        let mut files = Vec::new();
        sources(&root.join("src"), &mut files);
        let test_code =
            |path: &Path| path.ends_with("tests.rs") || path.components().any(|part| part.as_os_str() == "tests");
        for file in files.iter().filter(|file| !test_code(file)) {
            let text = std::fs::read_to_string(file).expect("the source is readable");
            let program = text.split("#[cfg(test)]\nmod tests").next().unwrap_or_default();
            assert!(!program.contains("CARGO_MANIFEST_DIR"), "{} builds a path from the build folder", file.display());
            for include in program.split("include_str!(\"").skip(1) {
                let relative = include.split('"').next().unwrap_or_default();
                let target = file.parent().expect("a file has a folder").join(relative);
                let target = target.canonicalize().unwrap_or(target);
                assert!(
                    target.starts_with(&crate_dir),
                    "{} compiles in {relative} from outside the crate",
                    file.display()
                );
            }
        }
    }
}

mod aesthetics {
    use qframe::icons::GlyphMode;

    use super::showcase_on;
    use crate::app::Msg;
    use crate::pages::PAGES;

    /// Pages whose demos show code or documents, where brackets are content.
    const CONTENT_PAGES: [&str; 5] = ["code-view", "markdown", "storage", "cell-animation", "document"];

    /// Labels that are content: the layout demo names the lengths it uses.
    const CONTENT_LABELS: [&str; 3] = ["Fill(1)", "Fill(2)", "Cells(14)"];

    /// Frame and divider characters. The heavy horizontal `━` is allowed as the rail of rail
    /// switches and sliders, the one place a line is the control itself rather than an edge.
    fn is_box_drawing(c: char) -> bool {
        ('\u{2500}'..='\u{257f}').contains(&c) && !['▌', '━'].contains(&c)
    }

    #[test]
    fn no_frames_anywhere_and_no_brackets_on_demo_surfaces() {
        for mode in [GlyphMode::Unicode, GlyphMode::Ascii, GlyphMode::Nerd] {
            for page in PAGES {
                let mut harness = showcase_on(page.id);
                harness.set_glyph_mode(mode);
                for section in 0..4 {
                    harness.send(Msg::Section(section));
                    let screen = harness.screen();
                    if let Some(bad) = screen.chars().find(|c| is_box_drawing(*c)) {
                        panic!("`{bad}` drawn on {} section {section} in {mode:?}:\n{screen}", page.id);
                    }
                    if section != 0 || CONTENT_PAGES.contains(&page.id) {
                        continue;
                    }
                    let mut cleaned = screen.clone();
                    for label in CONTENT_LABELS {
                        cleaned = cleaned.replace(label, "");
                    }
                    if let Some(bad) = cleaned.chars().find(|c| "[](){}".contains(*c)) {
                        panic!("`{bad}` drawn on the {} demo in {mode:?}:\n{screen}", page.id);
                    }
                }
            }
        }
    }
}

mod shell {
    use super::{SIZE, showcase_on, showcase_tall};
    use crate::app::{Msg, Showcase};

    #[test]
    fn planned_pages_show_their_design_notes_in_the_active_language() {
        // Once every item is built no planned item is left, so the test brings its own catalog.
        let catalog = crate::catalog::Catalog::parse(
            r#"
    [[item]]
    id       = "widget-dock"
    name     = "Widget dock"
    group    = "structure"
    kind     = "widget"
    priority = "p2"
    status   = "planned"
    notes    = "Collapsible widgets of the right panel."
    notes-tr = "Sağ panelin açılıp kapanan bileşenleri."
    "#,
        )
        .expect("test catalog parses");
        let mut harness = showcase_tall(Showcase::with_catalog(catalog), "widget-dock", SIZE.1);
        assert!(harness.screen().contains("DESIGN NOTES"), "{}", harness.screen());
        assert!(harness.screen().contains("right panel"));
        harness.set_locale("tr");
        assert!(harness.screen().contains("TASARIM NOTLARI"));
        assert!(harness.screen().contains("Sağ panel"));
    }

    #[test]
    fn code_guide_and_reference_sections_are_selectable() {
        for (section, word) in [(1, "pub"), (2, "When"), (3, "Methods")] {
            let mut harness = showcase_on("text-selection");
            harness.send(Msg::Section(section));
            let (x, y) =
                harness.find(word).unwrap_or_else(|| panic!("`{word}` on section {section}:\n{}", harness.screen()));
            let end = x + i32::try_from(word.chars().count()).unwrap_or(1) - 1;
            harness.drag((x, y), (end, y)).press("ctrl+c");
            assert_eq!(harness.clipboard(), Some(word), "section {section}");
        }
    }

    #[test]
    fn menu_and_titles_carry_stable_outline_numbers() {
        let mut harness = showcase_on("motion");
        let screen = harness.screen();
        assert!(screen.contains("1  FOUNDATIONS"), "{screen}");
        assert!(screen.contains("1.1   Getting started"), "numbers are padded to 1.10: {screen}");
        assert!(screen.contains("1.5   Motion") && screen.contains("1.5  Motion"), "menu and title:\n{screen}");
        // Searching by number finds the page without renumbering it.
        harness.send(Msg::Search("1.5".to_owned()));
        let screen = harness.screen();
        assert!(screen.contains("1.5   Motion") && !screen.contains("1.1   Getting started"), "{screen}");
    }

    #[test]
    fn a_press_with_the_language_list_open_still_changes_the_page_or_opens_the_theme_list() {
        let unfold = std::time::Duration::from_millis(300);
        let mut harness = showcase_on("spinner");
        harness.click_text("English").advance(unfold);
        assert!(harness.screen().contains("Türkçe"), "the language list is open:\n{}", harness.screen());
        harness.click_text("Button").advance(std::time::Duration::from_secs(1));
        assert_eq!(harness.app().current(), "button", "one press on the menu changed the page");
        assert!(!harness.screen().contains("Türkçe"), "{}", harness.screen());

        harness.click_text("English").advance(unfold);
        harness.click_text("Monochrome").advance(unfold);
        let screen = harness.screen();
        assert!(!screen.contains("Türkçe"), "the language list closed:\n{screen}");
        assert!(screen.contains("Nordic") && screen.contains("Amber"), "the theme list opened:\n{screen}");
    }
}

/// The pictures in the repository's README, drawn from fixed scenes so they come out the same on
/// every machine. Regenerate with
/// `cargo test -p quvyta-framework-showcase readme_shots -- --ignored`; the normal run skips it,
/// so the gate never writes a file.
mod readme_shots {
    use std::time::Duration;

    use qframe::icons::GlyphMode;
    use qframe::runtime::Harness;

    use super::showcase_tall;
    use crate::app::Showcase;

    /// `page` of the showcase in `theme`, `width` × `height`, with Nerd Font icons and every
    /// transition finished.
    fn scene(page: &str, theme: &str, width: u16, height: u16) -> Harness<Showcase> {
        let mut harness = showcase_tall(Showcase::new(), page, height);
        harness.resize(width, height).set_theme(theme).set_glyph_mode(GlyphMode::Nerd);
        harness.advance(Duration::from_secs(2));
        harness
    }

    fn save(harness: &Harness<Showcase>, name: &str) {
        save_shot(&qshots::Shot::of(harness).title("qframe"), name);
    }

    fn save_shot(shot: &qshots::Shot, name: &str) {
        assert!(shot.missing().is_empty(), "{name}: the font lacks {:?}", shot.missing());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/screenshots/");
        shot.save(format!("{path}{name}")).expect("the screenshots folder is writable");
    }

    #[test]
    #[ignore = "writes docs/screenshots; run on purpose to refresh the README pictures"]
    fn readme_shots() {
        save(&scene("example-dashboard", "nordic", 120, 36), "dashboard");
        // One per theme for the website's theme switcher: the same scene, so only colour changes.
        // Square corners filled with the ground: the site frames them itself, and a transparent
        // corner would show the page behind it in a different colour for every theme.
        for theme in ["iris", "nordic", "amber", "monochrome"] {
            let shot = qshots::Shot::of(&scene("example-dashboard", theme, 120, 36)).title("qframe").square();
            save_shot(&shot, &format!("dashboard-{theme}"));
        }
        save(&scene("heatmap", "iris", 120, 44), "heatmap");
        let mut palette = scene("getting-started", "amber", 120, 36);
        palette.press("ctrl+p").type_text("tab");
        palette.advance(Duration::from_secs(1));
        save(&palette, "command-palette");
        save(&scene("example-setup-wizard", "iris", 120, 36), "setup-wizard");
        save(&scene("markdown", "monochrome", 120, 40), "markdown");
        social();
    }

    /// The card a repository shows when its link is shared. A compact scene: on a card the
    /// screenshot is a third of the width, so a wide one turns to texture.
    fn social() {
        let card = qshots::Card::new(qshots::Shot::of(&scene("example-dashboard", "iris", 84, 36)))
            .name("qframe")
            .promise("Terminal applications in Rust, where shape comes from colour, not from box characters.");
        assert!(card.missing().is_empty(), "social: the font lacks {:?}", card.missing());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/screenshots/social");
        card.save(path).expect("the screenshots folder is writable");
    }
}
