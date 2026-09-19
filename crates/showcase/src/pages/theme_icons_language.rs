//! Theme, icons and language: switching at runtime, colour tokens, the icon set and plurals.

use std::sync::LazyLock;

use qframe::env::{AssetDirs, Env};
use qframe::i18n::I18n;
use qframe::icons::{IconMode, PillarStyle};
use qframe::keymap::Scope;
use qframe::prelude::*;
use qframe::theme::REQUIRED_COLORS;
use qframe::widgets::{Segmented, Select};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "theme-icons-language";

/// Icons shown in the icon grid. The last row is what an application's main menu needs, so every
/// application in the family shows the same shapes for a project, who you are, the settings and
/// the way out.
const ICONS: [&str; 20] = [
    "check",
    "close",
    "dot",
    "dot-outline",
    "arrow-left",
    "arrow-right",
    "arrow-up",
    "arrow-down",
    "search",
    "folder",
    "file",
    "success",
    "warning",
    "error",
    "info",
    "prompt",
    "project",
    "profile",
    "settings",
    "power",
];

// region: sources
/// An application's own theme, icon set and keymap, given as text instead of as a path. Here they
/// are written out to be read; an application uses `include_str!` on its own files, so the
/// installed binary carries them and needs nothing beside it on disk.
const APP_THEME: &str = "[meta]\nname = \"Brand\"\nextends = \"monochrome\"\nicon-set = \"brand\"\n\n\
                         [colors]\naccent = \"#7C9EF5\"\n";
const APP_ICONS: &str = "[meta]\nname = \"Brand\"\n\n[icons]\n\
                         project = { nerd = \"\\uf1b2\", unicode = \"\u{25c8}\", ascii = \"#\" }\n\
                         \"category.internet\" = { nerd = \"\\uf0ac\", unicode = \"\u{25ce}\", ascii = \"@\" }\n";
const APP_KEYS: &str = "[app]\nsave = \"ctrl+s\"\n";

/// The environment those three texts make, loaded once and with no directory named at all. A
/// runtime is given the same three with `.theme_source(file, text)`, `.icon_source(file, text)`
/// and `.keymap_source(file, text)`, beside the `.locale_source(file, text)` of the language
/// files. A path named as well is then optional: when it cannot be read the text stands in for it
/// and the reason becomes a diagnostic instead of stopping the program.
static FROM_TEXT: LazyLock<Env> = LazyLock::new(|| {
    let dirs = AssetDirs {
        theme_sources: vec![("brand.toml".to_owned(), APP_THEME.to_owned())],
        icon_sources: vec![("brand.toml".to_owned(), APP_ICONS.to_owned())],
        keymap_source: Some(("keymap.toml".to_owned(), APP_KEYS.to_owned())),
        ..AssetDirs::default()
    };
    Env::load(&dirs).expect("text alone needs no files on disk")
});
// endregion

/// An application's English and Turkish text in which Turkish has not translated the file count
/// yet. Written out here to be read; an application loads its own locale files.
const APP_EN: &str = "[meta]\nname = \"English\"\ncode = \"en\"\n\n[app]\nsave = \"Save\"\n\
                      files = { one = \"{n} file\", other = \"{n} files\" }\n";
const APP_TR: &str = "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\nfallback = \"en\"\n\n[app]\nsave = \"Kaydet\"\n";

/// The translator those two texts make, with Turkish active, on its own so the showcase's complete
/// text is untouched. `has` names its language, so the active one does not change its answers.
static KEY_CHECK: LazyLock<I18n> = LazyLock::new(|| {
    let mut i18n = I18n::builtin();
    i18n.add_source("app-en.toml", APP_EN);
    i18n.add_source("app-tr.toml", APP_TR);
    i18n.set_active("tr");
    i18n
});

/// Which language carries each key of the text above itself, and what Turkish shows for it.
fn key_check(ui: &mut View<'_, AppMsg>) {
    for key in ["app.save", "app.files"] {
        ui.row(|ui| {
            ui.add(Text::new(key).role("body").no_wrap()).width(Length::Cells(12));
            for code in ["en", "tr"] {
                // region: has
                let present = KEY_CHECK.has(code, key);
                // endregion
                let (marker, color) = if present { ("success", "success") } else { ("warning", "warning") };
                let glyph = ui.env().icons().glyph(marker).into_owned();
                ui.add(Text::new(format!("{glyph} {code}")).color(color).no_wrap()).width(Length::Cells(6));
            }
            let shown = KEY_CHECK.translate(key, &[("n", 2.into())]);
            ui.add(Text::new(shown).role("secondary").no_wrap());
        });
    }
    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(t!("theme-icons-language.has-hint")).role("faint"));
}

/// The display name of `id` in a list of `(id, name)`, empty when the list does not have it.
fn name_of(list: &[(String, String)], id: &str) -> String {
    list.iter().find(|(known, _)| known == id).map(|(_, name)| name.clone()).unwrap_or_default()
}

/// What the three texts above became: a theme, an icon set and a key binding an application has
/// without carrying a single file, and one of the application's own icons.
fn sources(ui: &mut View<'_, AppMsg>) {
    let env = &*FROM_TEXT;
    let chords: Vec<String> = env.keymap().chords_for(Scope::App, "save").iter().map(ToString::to_string).collect();
    // region: app-icon
    // A key the built-in set lacks is drawn in every theme, here the default one, in the glyph
    // mode this screen uses.
    let mut icons = env.icons().clone();
    icons.set_mode(ui.env().glyph_mode());
    let app_icon = format!("{} category.internet", icons.glyph("category.internet"));
    // endregion
    let rows = [
        (t!("theme-icons-language.sources-theme"), name_of(&env.themes(), "brand")),
        (t!("theme-icons-language.sources-icons"), name_of(&env.icon_sets(), "brand")),
        (t!("theme-icons-language.sources-app-icon"), app_icon),
        (t!("theme-icons-language.sources-keys"), chords.join("  ")),
    ];
    for (label, value) in rows {
        ui.row(|ui| {
            ui.add(Text::new(label).role("secondary").no_wrap()).width(Length::Cells(14));
            ui.add(Text::new(value).role("body").no_wrap());
        });
    }
    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(t!("theme-icons-language.sources-hint")).role("faint"));
}

/// The plural demo counter.
#[derive(Debug, Default)]
pub struct State {
    files: u32,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    More,
    Fewer,
    Sample,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ThemeIconsLanguage(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::More => state.files += 1,
        Msg::Fewer => state.files = state.files.saturating_sub(1),
        Msg::Sample => {
            log.push(PAGE, "Button#appearance-button", "pressed");
            return Command::none();
        }
    }
    log.push(PAGE, "Button", format!("n = {}", state.files));
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.row(|ui| {
            ui.column(switcher).width(Length::Fill(1));
            ui.column(appearance).width(Length::Fill(1));
        })
        .gap(4)
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("theme-icons-language.tokens")).gap(0), |ui| {
        let theme = ui.env().theme();
        let rows: Vec<Vec<(&str, String)>> = REQUIRED_COLORS
            .chunks(3)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|token| (*token, theme.color(token).map(|c| c.to_string()).unwrap_or_default()))
                    .collect()
            })
            .collect();
        for row in rows {
            ui.row(|ui| {
                for (token, hex) in row {
                    // region: swatch
                    ui.add(
                        Text::rich([
                            Span::new("    ").on(token),
                            Span::new(format!("  {token}")).role("body"),
                            Span::new(format!("  {hex}")).role("faint"),
                        ])
                        .no_wrap(),
                    )
                    .width(Length::Cells(30));
                    // endregion
                }
            });
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("theme-icons-language.icons")).gap(0), |ui| {
        for chunk in ICONS.chunks(4) {
            ui.row(|ui| {
                for key in chunk {
                    let glyph = ui.env().icons().glyph(key).into_owned();
                    ui.add(
                        Text::rich([Span::new(glyph).color("accent"), Span::new(format!("  {key}")).role("secondary")])
                            .no_wrap(),
                    )
                    .width(Length::Cells(22));
                }
            });
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("theme-icons-language.language")), |ui| {
        ui.row(|ui| {
            ui.add(Button::new("−").on_press(send(Msg::Fewer))).id("fewer");
            ui.add(Button::new("+").on_press(send(Msg::More))).id("more");
            // region: plural
            ui.add(Text::new(t!("theme-icons-language.files", n = state.files)).role("title").no_wrap());
            // endregion
        })
        .gap(2);
        ui.add(Text::new(t!("theme-icons-language.plural-hint")).role("faint"));
        ui.spacer().height(Length::Cells(1));
        key_check(ui);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("theme-icons-language.sources")), sources).fill_width();
}

/// Choices that change how every screen feels: the pillar, the selection slide and motion. They
/// apply at once everywhere and are remembered with the other settings.
fn appearance(ui: &mut View<'_, AppMsg>) {
    let env = ui.env();
    let pillar = env.pillar_style().unwrap_or(PillarStyle::Thick);
    let slide = env.slide();
    setting(ui, t!("theme-icons-language.pillar"), |ui| {
        // region: appearance
        let names = PillarStyle::ALL.map(|style| t!(&format!("theme-icons-language.pillar-{}", style.name())));
        let chosen = PillarStyle::ALL.iter().position(|style| *style == pillar).unwrap_or(0);
        ui.add(Segmented::new(names).selected(chosen).on_select(|i| AppMsg::Pillar(PillarStyle::ALL[i]))).id("pillar");
        // endregion
    });
    setting(ui, t!("theme-icons-language.slide"), |ui| {
        ui.add(toggle(slide, AppMsg::Slide)).id("slide");
    });
    super::motion::reduced_motion_setting(ui, t!("theme-icons-language.reduced"), AppMsg::ReducedMotion);
    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(t!("theme-icons-language.appearance-hint")).role("faint"));
    let sample = ["theme-icons-language.sample-1", "theme-icons-language.sample-2"];
    ui.row(|ui| {
        ui.add(List::new(sample.map(|key| ListItem::new(t!(key)))).selected(Some(1)))
            .width(Length::Cells(20))
            .height(Length::Cells(2))
            .id("appearance-sample");
        ui.add(Button::new(t!("theme-icons-language.sample-button")).on_press(send(Msg::Sample)))
            .id("appearance-button");
    })
    .gap(3);
}

/// A select of the installed themes that switches the whole application. The showcase header
/// uses it too.
pub fn theme_select(env: &Env) -> Select<AppMsg> {
    // region: switch
    let themes = env.themes();
    let current = themes.iter().position(|(id, _)| id == env.theme().id());
    let ids: Vec<String> = themes.iter().map(|(id, _)| id.clone()).collect();
    let names = themes.iter().map(|(_, name)| name.clone());
    Select::new(names).selected(current).on_select(move |i| AppMsg::Theme(ids[i].clone()))
    // endregion
}

/// A select of the installed languages that switches the whole application.
pub fn language_select(env: &Env) -> Select<AppMsg> {
    let locales = env.i18n().list();
    let current = locales.iter().position(|(code, _)| code == env.i18n().active());
    let codes: Vec<String> = locales.iter().map(|(code, _)| code.clone()).collect();
    Select::new(locales.iter().map(|(_, name)| name.clone()))
        .selected(current)
        .on_select(move |i| AppMsg::Locale(codes[i].clone()))
}

/// A select of the icon modes that switches the whole application.
pub fn icon_select(env: &Env) -> Select<AppMsg> {
    let current = IconMode::ALL.iter().position(|mode| *mode == env.icon_mode());
    Select::new(IconMode::ALL.iter().map(|mode| t!(&format!("icons.{}", mode.name()))))
        .selected(current)
        .on_select(|i| AppMsg::Icons(IconMode::ALL[i]))
}

/// Theme, language and icon selects that switch the whole application.
fn switcher(ui: &mut View<'_, AppMsg>) {
    let env = ui.env();
    let (theme, language, icons) = (theme_select(env), language_select(env), icon_select(env));
    let problems = env.diagnostics().len();
    setting(ui, t!("theme-icons-language.theme-label"), |ui| {
        ui.add(theme).width(Length::Cells(18)).id("page-theme");
    });
    setting(ui, t!("theme-icons-language.language-label"), |ui| {
        ui.add(language).width(Length::Cells(18)).id("page-language");
    });
    setting(ui, t!("theme-icons-language.icons-label"), |ui| {
        ui.add(icons).width(Length::Cells(18)).id("page-icons");
    });
    // The marker comes from the icon set, so it has an ASCII form too.
    let (marker, text, color) = if problems == 0 {
        ("success", t!("theme-icons-language.no-problems"), "success")
    } else {
        ("warning", t!("theme-icons-language.problems", n = problems), "warning")
    };
    let marker = ui.env().icons().glyph(marker).into_owned();
    // Wrapped, not cut: half a playground is narrower than the sentence.
    ui.add(Text::new(format!("{marker} {text}")).color(color));
}

#[cfg(test)]
mod tests {
    use qframe::icons::GlyphMode;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::{showcase_on, showcase_tall};

    #[test]
    fn a_theme_an_icon_set_and_a_keymap_given_as_text_need_no_directory() {
        let h = showcase_tall(Showcase::new(), PAGE, 70);
        let screen = h.screen();
        let panel = screen.split("FILES GIVEN AS TEXT").nth(1).unwrap_or_default();
        let row = |label: &str| panel.lines().find(|line| line.contains(label)).unwrap_or_default().to_owned();
        assert!(row("theme").contains("Brand"), "the theme came from text:\n{screen}");
        assert!(row("icon set").contains("Brand"), "the icon set came from text:\n{screen}");
        assert!(row("save key").contains("ctrl+s"), "the keymap came from text:\n{screen}");
    }

    #[test]
    fn an_application_icon_from_text_is_drawn_in_the_glyph_mode_in_use() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 70);
        for (mode, glyph) in [(GlyphMode::Nerd, "\u{f0ac}"), (GlyphMode::Unicode, "◎"), (GlyphMode::Ascii, "@")] {
            h.set_glyph_mode(mode);
            let screen = h.screen();
            let row = screen.lines().find(|line| line.contains("app icon")).unwrap_or_default();
            assert!(row.contains(&format!("{glyph} category.internet")), "{mode:?}: {row:?}");
        }
    }

    #[test]
    fn the_key_check_reports_a_key_turkish_only_borrows() {
        let h = showcase_tall(Showcase::new(), PAGE, 90);
        let screen = h.screen();
        let row = |key: &str| screen.lines().find(|line| line.contains(key)).unwrap_or_default().to_owned();
        let save = row("app.save");
        assert!(save.contains("✓ en") && save.contains("✓ tr") && save.contains("Kaydet"), "{screen}");
        let files = row("app.files");
        assert!(files.contains("✓ en"), "{files:?}");
        assert!(!files.contains("✓ tr") && files.contains(" tr"), "Turkish lacks it: {files:?}");
        assert!(files.contains("2 files"), "the screen falls back to English: {files:?}");
    }

    #[test]
    fn the_application_menu_icons_are_in_the_gallery_in_every_mode() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 70);
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            h.set_glyph_mode(mode);
            let screen = h.screen();
            for name in ["project", "profile", "settings", "power"] {
                let row = screen.lines().find(|line| line.contains(name)).unwrap_or_default();
                let glyph = row
                    .split(name)
                    .next()
                    .and_then(|before| before.trim_end().chars().last())
                    .unwrap_or_else(|| panic!("{name} is missing in {mode:?}:\n{screen}"));
                assert!(!glyph.is_whitespace(), "{name} has no glyph in {mode:?}: {row:?}");
                if mode == GlyphMode::Ascii {
                    assert!(glyph.is_ascii_graphic(), "{name} is not ASCII in {mode:?}: {glyph:?}");
                }
            }
        }
    }

    #[test]
    fn switches_theme_and_language_and_pluralises() {
        // The colour token table lists every required colour, so the plural demo below it needs
        // more rows than the default terminal of the showcase tests.
        let mut h = showcase_tall(Showcase::new(), PAGE, 70);
        assert!(h.screen().contains("0 files"));
        h.click_text("+");
        assert!(h.screen().contains("1 file"));
        h.send(AppMsg::Locale("tr".into()));
        assert!(h.screen().contains("1 dosya"));
        h.send(AppMsg::Theme("amber".into()));
        assert_eq!(h.env().theme().id(), "amber");
    }

    #[test]
    fn appearance_choices_apply_everywhere_and_are_remembered() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("Slide on selection"), "{}", h.screen());
        h.click_text("Thin");
        assert_eq!(h.env().pillar_style(), Some(PillarStyle::Thin));
        assert!(h.screen().contains('▎'), "the menu's pillar is thin now:\n{}", h.screen());
        h.send(AppMsg::Slide(false));
        assert!(!h.env().slide());
        let settings = &h.app().pages.storage.settings;
        assert_eq!(settings.pillar_style(), Some(PillarStyle::Thin));
        assert_eq!(settings.slide(), Some(false));
    }

    #[test]
    fn playground_labels_share_one_case_and_the_status_marker_has_an_ascii_form() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        let line = |text: &str| screen.lines().find(|line| line.contains(text)).unwrap_or_default().to_owned();
        assert!(line("Pillar").contains("Theme"), "{screen}");
        assert!(line("Slide on selection").contains("Language") && line("Reduce motion").contains("Icons"));
        assert!(screen.contains("✓ theme, icon, language and keymap"), "{screen}");
        assert!(screen.contains("without problems"), "wrapped, not cut: {screen}");
        h.set_glyph_mode(qframe::icons::GlyphMode::Ascii);
        let ascii = h.screen();
        let status = ascii.lines().find(|line| line.contains("theme, icon, language and keymap")).unwrap_or_default();
        let marker = status.split("theme, icon").next().and_then(|before| before.trim_end().chars().last());
        assert!(marker.is_some_and(|marker| marker.is_ascii()), "the status marker has an ASCII form: {status:?}");
    }

    #[test]
    fn an_off_switch_track_is_readable_in_amber() {
        let mut h = showcase_on(PAGE);
        h.send(AppMsg::Theme("amber".into()));
        let (x, y) = h.find("Reduce motion").expect("appearance row");
        // A four-cell switch after the 24-cell label: two cells of knob, then two of track.
        let (knob, track) = (u16::try_from(x + 24).unwrap_or(0), u16::try_from(x + 26).unwrap_or(0));
        let y = u16::try_from(y).unwrap_or(0);
        let theme = h.env().theme().clone();
        let resting = theme.style("switch", None, &[]).paint("knob").map(|paint| paint.at(0.0));
        assert_eq!(h.bg(knob, y), resting, "the quiet knob of an off switch");
        assert_ne!(h.bg(knob, y), h.bg(track, y), "the knob shows on its track");
        assert_eq!(h.bg(track, y), theme.color("active"), "the track one step up from raised");
        assert_ne!(h.bg(track, y), h.bg(knob - 2, y), "a tone apart from the panel under the label");
        h.hover(i32::from(track), i32::from(y));
        let hovered =
            theme.style("switch", None, &[qframe::theme::State::Hover]).paint("track").map(|paint| paint.at(0.0));
        assert_eq!(h.bg(track, y), hovered);
        assert_ne!(hovered, theme.color("active"), "hover still lifts it");
        h.send(AppMsg::Theme("monochrome".into()));
        let mono = h.env().theme().clone();
        h.hover(0, 0);
        assert_eq!(h.bg(track, y), mono.color("raised"), "other themes keep their track");
    }
}
