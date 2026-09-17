//! Settings storage: typed values saved as TOML in the config directory, the showcase's own
//! appearance choices, located diagnostics for a broken file, and self-healing by a schema.

use qframe::diagnostics::Severity;
use qframe::env::Env;
use qframe::prelude::*;
use qframe::storage::{Schema, SettingKind, Settings};
use qframe::widgets::{CodeView, Language, Segmented, Switch, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "storage";

/// Regions a deploy can target.
const REGIONS: [&str; 3] = ["eu-west", "us-east", "ap-south"];

/// A settings file a user broke by hand: an unknown key, invalid values and a broken table, then
/// a valid and an invalid optional value and a plugin's table the showcase does not know.
const BROKEN: &str = "language = \"sjds\"\ncolor = \"red\"\ntheme = \"nordic\"\nicons = \"sparkly\"\nreduced-motion = \"yes\"\nslide = true\n[deploy\nregion = \"eu-west\"\n\n[deploy]\nnote = \"Freeze until the 3.2 release\"\nretries = \"twice\"\n\n[plugins]\nspellcheck = true\n";

// region: storage-schema
/// Every key the showcase stores, with what it may hold and its default. Themes and languages
/// are the ones `env` has installed, and a broken language falls back to the one `env` chose.
pub fn schema(env: &Env) -> Schema {
    let themes: Vec<String> = env.themes().into_iter().map(|(id, _)| id).collect();
    let languages: Vec<String> = env.i18n().list().into_iter().map(|(code, _)| code).collect();
    Schema::builtin()
        .choice(Settings::THEME, themes, "monochrome")
        .choice(Settings::LANGUAGE, languages, env.i18n().active())
        .flag("deploy.confirm", true)
        .choice("deploy.region", REGIONS, REGIONS[0])
        .check("deploy.branch", String::new(), |branch| !branch.contains(char::is_whitespace))
        // No default: kept while valid, removed when invalid, never added.
        .optional("deploy.note", SettingKind::text())
        .optional("deploy.retries", SettingKind::check(|retries: &u8| (1..=5).contains(retries)))
        // Plugins store their own keys here; they are kept as they are.
        .open("plugins")
        // The animation studio keeps its working animations here.
        .open("studio")
}
// endregion

/// The broken file loaded with the demo schema, healed or not.
fn load_broken(schema: &Schema, self_heal: bool) -> Settings {
    // region: storage-heal
    Settings::parse_str("settings.toml", BROKEN).schema(schema.clone()).self_heal(self_heal)
    // endregion
}

/// The showcase's settings, shared with the shell, and the playground.
#[derive(Debug)]
pub struct State {
    pub settings: Settings,
    /// The schema of the demo file, from the built-in themes and languages.
    schema: Schema,
    last_save: Option<Result<(), String>>,
    show_broken: bool,
    self_heal: bool,
}

impl State {
    /// State around `settings`, e.g. loaded from the config directory.
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self { settings, schema: schema(&Env::builtin()), last_save: None, show_broken: false, self_heal: false }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new(Settings::in_memory())
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Confirm(bool),
    Region(usize),
    Branch(String),
    Saved(Result<(), String>),
    Reset,
    ShowBroken(bool),
    SelfHeal(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Storage(message))
}

// region: storage-save
/// Stores one value and saves the file in the background when it changed.
pub fn remember<T: qframe::storage::Setting>(settings: &mut Settings, key: &str, value: T) -> Command<AppMsg> {
    if settings.set(key, value) { settings.save_command(|result| send(Msg::Saved(result))) } else { Command::none() }
}
// endregion

/// Applies an application-wide choice at once with `apply` and remembers it under `key`.
pub fn apply_and_remember<T: qframe::storage::Setting>(
    settings: &mut Settings,
    key: &str,
    value: T,
    apply: Command<AppMsg>,
) -> Command<AppMsg> {
    let save = remember(settings, key, value);
    Command::batch([apply, save])
}

/// Writes what loading the broken file reported into the log: the repairs when healing, the
/// warnings otherwise.
fn log_broken(state: &State, log: &mut EventLog) {
    let checked = if state.self_heal { "Settings::self_heal" } else { "Settings::schema" };
    for diagnostic in load_broken(&state.schema, state.self_heal).diagnostics() {
        // Syntax errors come from reading the file, not from the check or the repair.
        let source = if diagnostic.severity == Severity::Error { "Settings::parse_str" } else { checked };
        log.push(PAGE, source, diagnostic.message.clone());
    }
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Confirm(on) => {
            log.push(PAGE, "Switch#confirm", format!("deploy.confirm = {on}"));
            remember(&mut state.settings, "deploy.confirm", on)
        }
        Msg::Region(index) => {
            let Some(region) = REGIONS.get(index) else {
                return Command::none();
            };
            log.push(PAGE, "Segmented#region", format!("deploy.region = {region:?}"));
            remember(&mut state.settings, "deploy.region", (*region).to_owned())
        }
        Msg::Branch(branch) => {
            log.push(PAGE, "TextInput#branch", format!("deploy.branch = {branch:?}"));
            remember(&mut state.settings, "deploy.branch", branch)
        }
        Msg::Saved(result) => {
            let text = match &result {
                Ok(()) => "saved".to_owned(),
                Err(error) => format!("not saved: {error}"),
            };
            log.push(PAGE, "Settings::save_command", text);
            state.last_save = Some(result);
            Command::none()
        }
        Msg::Reset => {
            log.push(PAGE, "Button#reset", "removed the deploy keys");
            let removed = ["deploy.confirm", "deploy.region", "deploy.branch"].map(|key| state.settings.remove(key));
            if removed.contains(&true) {
                state.settings.save_command(|result| send(Msg::Saved(result)))
            } else {
                Command::none()
            }
        }
        Msg::ShowBroken(on) => {
            log.push(PAGE, "Playground", format!("broken file = {on}"));
            state.show_broken = on;
            if on {
                log_broken(state, log);
            }
            Command::none()
        }
        Msg::SelfHeal(on) => {
            log.push(PAGE, "Playground", format!("self-heal = {on}"));
            state.self_heal = on;
            if state.show_broken {
                log_broken(state, log);
            }
            Command::none()
        }
    }
}

/// One diagnostic as a quiet line with a warning marker; a long one wraps under its own text,
/// so the markers stay a column of their own.
fn diagnostic_line(ui: &mut View<'_, AppMsg>, text: String) {
    let marker = ui.env().icons().glyph("warning").into_owned();
    ui.row(|ui| {
        ui.add(Text::new(marker).color("warning").no_wrap());
        ui.add(Text::new(text).role("secondary")).fill_width();
    })
    .gap(1);
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let settings = &state.settings;
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        let place = settings.path().map_or_else(|| t!("storage.in-memory"), |path| path.display().to_string());
        ui.add(
            Text::rich([Span::new(t!("storage.file")).role("faint"), Span::new(format!("  {place}")).role("body")])
                .no_wrap(),
        );
        ui.add(Text::new(t!("storage.hint")).role("secondary"));
        if !settings.diagnostics().is_empty() {
            ui.spacer().height(Length::Cells(1));
            ui.add(Text::new(t!("storage.at-start")).role("faint"));
            for diagnostic in settings.diagnostics() {
                diagnostic_line(ui, diagnostic.to_string());
            }
        }
        ui.spacer().height(Length::Cells(1));
        // region: storage-read
        let confirm = settings.get_or("deploy.confirm", true);
        let region = settings.get::<String>("deploy.region").and_then(|r| REGIONS.iter().position(|x| *x == r));
        let branch = settings.get_or("deploy.branch", String::new());
        // endregion
        setting(ui, t!("storage.confirm"), |ui| {
            ui.add(Switch::new(confirm).on_toggle(|on| send(Msg::Confirm(on)))).id("confirm");
        });
        setting(ui, t!("storage.region"), |ui| {
            ui.add(Segmented::new(REGIONS).selected(region.unwrap_or(0)).on_select(|i| send(Msg::Region(i))))
                .id("region");
        });
        setting(ui, t!("storage.branch"), |ui| {
            ui.add(TextInput::new(branch).placeholder("main").on_change(|text| send(Msg::Branch(text))))
                .width(Length::Cells(28))
                .id("branch");
        });
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("storage.reset")).on_press(send(Msg::Reset))).id("reset");
            let (text, color) = match &state.last_save {
                None => (t!("storage.unsaved"), "muted"),
                Some(Ok(())) => (format!("{} {}", ui.env().icons().glyph("success"), t!("storage.saved")), "success"),
                Some(Err(error)) => (format!("{} {error}", ui.env().icons().glyph("error")), "danger"),
            };
            ui.add(Text::new(text).color(color).no_wrap());
        })
        .gap(2);
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("storage.contents")).role("faint"));
        let contents = settings.to_toml();
        let contents = if contents.is_empty() { t!("storage.empty") } else { contents };
        ui.add(CodeView::new(contents, Language::Toml).line_numbers(false)).fill_width().id("contents");
    })
    .fill_width();

    if state.show_broken {
        ui.add_with(Panel::new().title(t!("storage.broken")).gap(0), |ui| {
            ui.add(CodeView::new(BROKEN, Language::Toml)).fill_width().id("broken");
            ui.spacer().height(Length::Cells(1));
            // region: storage-diagnostics
            let loaded = load_broken(&state.schema, state.self_heal);
            for diagnostic in loaded.diagnostics() {
                diagnostic_line(ui, diagnostic.to_string());
            }
            // endregion
            ui.spacer().height(Length::Cells(1));
            if state.self_heal {
                ui.add(Text::new(t!("storage.repaired")).role("faint"));
                ui.add(CodeView::new(loaded.to_toml(), Language::Toml)).fill_width().id("repaired");
            } else {
                let kept = loaded.theme().unwrap_or_default();
                ui.add(Text::new(t!("storage.kept", theme = kept)).role("faint"));
            }
        })
        .fill_width();
    }

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("storage.show-broken"), |ui| {
            ui.add(toggle(state.show_broken, |on| send(Msg::ShowBroken(on)))).id("show-broken");
        });
        setting(ui, t!("storage.self-heal"), |ui| {
            ui.add(toggle(state.self_heal, |on| send(Msg::SelfHeal(on)))).id("self-heal");
        });
        ui.add(Text::new(t!("storage.shell-hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use qframe::runtime::Harness;

    use crate::app::Showcase;
    use crate::tests::showcase_on;

    #[test]
    fn values_are_stored_and_shown_as_toml() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("in memory"), "{}", h.screen());
        h.click_text("us-east");
        let settings = &h.app().pages.storage.settings;
        assert_eq!(settings.get::<String>("deploy.region").as_deref(), Some("us-east"));
        assert!(h.screen().contains("region = \"us-east\""), "{}", h.screen());
        assert!(h.screen().contains("saved"));
        h.click_text("Forget deploy settings");
        assert!(h.app().pages.storage.settings.value("deploy.region").is_none());
    }

    #[test]
    fn an_unknown_region_changes_nothing() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Region(REGIONS.len())));
        assert!(h.app().pages.storage.settings.value("deploy.region").is_none());
    }

    #[test]
    fn the_broken_file_is_repaired_or_only_reported_and_the_log_says_which() {
        let mut h = tall(Showcase::new());
        h.send(send(Msg::ShowBroken(true)));
        let screen = h.screen();
        assert!(
            screen.contains("settings.toml:2:1: warning: `color` is not a known setting; it is ignored"),
            "{screen}"
        );
        assert!(!screen.contains("REPAIRED FILE") && screen.contains("good values are still used: theme = nordic"));
        let log: Vec<String> = h.app().log.recent(PAGE, 30).iter().map(|entry| entry.message.clone()).collect();
        assert!(log.last().is_some_and(|line| line.ends_with("it is ignored")), "{log:?}");

        // The self-heal switch is on the row under the broken-file switch, after the 24-cell labels.
        let (x, y) = h.find("Show a broken file").expect("playground row");
        h.click(x + 25, y + 1);
        assert!(h.app().pages.storage.self_heal, "{}", h.screen());
        let screen = h.screen();
        assert!(screen.contains("settings.toml:2:1: warning: `color` is not a known setting; removed"), "{screen}");
        assert!(screen.contains("`language` must be one of en, tr, found \"sjds\"; replaced with \"en\""), "{screen}");
        assert!(screen.contains("REPAIRED FILE"), "{screen}");
        let log: Vec<String> = h.app().log.recent(PAGE, 20).iter().map(|entry| entry.message.clone()).collect();
        assert!(log.iter().any(|line| line == "`color` is not a known setting; removed"), "{log:?}");
        assert!(
            log.iter()
                .any(|line| line.starts_with("`icons` must be one of") && line.ends_with("replaced with \"auto\""))
        );
        assert!(log.iter().any(|line| line == "self-heal = true"), "{log:?}");
        assert!(screen.contains("`deploy.retries` must be a value this application accepts, found"), "{screen}");
        let removed = "`deploy.retries` must be a value this application accepts, found \"twice\"; removed";
        assert!(log.iter().any(|line| line == removed), "{log:?}");
        let healed = load_broken(&h.app().pages.storage.schema, true);
        assert_eq!(healed.get::<String>("deploy.note").as_deref(), Some("Freeze until the 3.2 release"));
        assert_eq!(healed.get::<bool>("plugins.spellcheck"), Some(true), "the open table is kept");
        assert_eq!(
            healed.to_toml(),
            "language = \"en\"\ntheme = \"nordic\"\nicons = \"auto\"\nreduced-motion = false\nslide = true\n\n[deploy]\nnote = \"Freeze until the 3.2 release\"\n\n[plugins]\nspellcheck = true\n",
            "the invalid optional value is gone and nothing missing was added"
        );
        let parse = h.app().log.recent(PAGE, 30).iter().filter(|entry| entry.source == "Settings::parse_str").count();
        assert_eq!(parse, 2, "the syntax error is logged as read, once per load");
    }

    /// The storage page in a terminal tall enough for the broken file and the playground.
    fn tall(showcase: Showcase) -> Harness<Showcase> {
        crate::tests::showcase_tall(showcase, PAGE, 100)
    }

    #[test]
    fn repairs_made_at_start_are_shown_with_the_file() {
        let text = "language = \"tr\"\ncolor = \"red\"\n";
        let settings = Settings::parse_str("settings.toml", text).schema(schema(&Env::builtin())).self_heal(true);
        let h = tall(Showcase::with_settings(settings));
        let screen = h.screen();
        assert!(screen.contains("FOUND WHILE LOADING"), "{screen}");
        assert!(screen.contains("settings.toml:2:1: warning: `color` is not a known setting; removed"), "{screen}");
    }

    #[test]
    fn the_schema_accepts_everything_the_showcase_writes() {
        let mut h = showcase_on(PAGE);
        h.send(AppMsg::Theme("amber".into())).send(AppMsg::Locale("tr".into()));
        h.send(AppMsg::Icons(qframe::icons::IconMode::Ascii));
        h.send(AppMsg::Pillar(qframe::icons::PillarStyle::Thin)).send(AppMsg::Slide(false));
        h.send(AppMsg::ReducedMotion(true));
        h.click_text("ap-south");
        h.send(send(Msg::Confirm(false))).send(send(Msg::Branch("release-2026".into())));
        let written = h.app().pages.storage.settings.to_toml();
        let reread = Settings::parse_str("settings.toml", &written).schema(schema(h.env())).self_heal(true);
        assert_eq!(reread.diagnostics(), &[], "{written}");
        assert_eq!(reread.to_toml(), written, "healing keeps every value the showcase saves");
    }

    #[test]
    fn the_shell_remembers_theme_language_and_icons() {
        let mut h = showcase_on(PAGE);
        h.send(AppMsg::Theme("amber".into())).send(AppMsg::Locale("tr".into()));
        let settings = &h.app().pages.storage.settings;
        assert_eq!(settings.theme().as_deref(), Some("amber"));
        assert_eq!(settings.language().as_deref(), Some("tr"));
    }
}
