//! Settings storage: typed values saved as TOML in the config directory, the showcase's own
//! appearance choices, located diagnostics for a broken file, self-healing by a schema, and the
//! rest of what an application needs around its own files: the two platform folders and the
//! machine's name for files of its own in them, the family's shared settings folder and the
//! Documents folder, moving old settings into the family, an atomic write step by step, the lock
//! that keeps a second instance out, and the shared lock that wakes a waiter when the last
//! instance closes.

use std::path::{Path, PathBuf};

use qframe::diagnostics::Severity;
use qframe::env::Env;
use qframe::prelude::*;
use qframe::storage::{
    AppLock, Family, InstanceLock, Migration, Schema, SettingKind, Settings, WriteStep, atomic_write_reporting,
};
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

/// What the last attempt at the demo lock answered.
#[derive(Debug, Default)]
enum Attempt {
    /// Nobody has pressed the button yet.
    #[default]
    None,
    /// This instance took the lock and still holds it.
    Taken,
    /// Another holder has it; the process id is the one written in the file, when it holds one.
    Busy(Option<u32>),
    /// The lock could not be tried at all, e.g. on a platform without an advisory lock.
    Failed(String),
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
    /// The steps of the last safe write, or why it failed.
    write: Option<Result<Vec<WriteStep>, String>>,
    /// The demo lock, held for as long as this value lives.
    lock: Option<AppLock>,
    attempt: Attempt,
    /// The shared locks of the demo's pretend instances, one per open instance.
    instances: Vec<InstanceLock>,
    /// A background wait for the exclusive lock is running.
    waiting: bool,
    /// What the last wait for the exclusive lock came to.
    waited: Option<Result<(), String>>,
    /// What the last move of the demo's old folder into its family reported, or why the demo
    /// could not set the old folder up.
    adopt: Option<Result<Migration, String>>,
    /// The folder this page's demo files live in, one per instance of the page.
    demo: PathBuf,
}

impl State {
    /// State around `settings`, e.g. loaded from the config directory.
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            schema: schema(&Env::builtin()),
            last_save: None,
            show_broken: false,
            self_heal: false,
            write: None,
            lock: None,
            attempt: Attempt::None,
            instances: Vec::new(),
            waiting: false,
            waited: None,
            adopt: None,
            demo: demo_dir(),
        }
    }
}

impl Drop for State {
    /// Takes the demo folder away with the page, so a run leaves nothing in the temporary folder.
    /// The folder is this instance's own, named after the process; nothing else is removed.
    fn drop(&mut self) {
        self.lock = None;
        self.instances.clear();
        if self.demo.is_dir() {
            let _ = std::fs::remove_dir_all(&self.demo);
        }
    }
}

/// Tells the demo folders of two pages in one process apart, which the tests need.
static DEMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A folder for this page's own demo files: a folder of this run, never the user's own files.
fn demo_dir() -> PathBuf {
    let ticket = DEMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("quvyta-showcase-storage-{}-{ticket}", std::process::id()))
}

/// Writes the demo file safely and gives back every step it took.
fn write_safely(path: &Path) -> Result<Vec<WriteStep>, String> {
    let contents = "theme = \"nordic\"\nlanguage = \"en\"\n";
    let mut steps = Vec::new();
    // region: storage-atomic
    std::fs::create_dir_all(path.parent().expect("the demo file has a folder")).map_err(|e| e.to_string())?;
    atomic_write_reporting(path, contents.as_bytes(), |step| steps.push(step)).map_err(|e| e.to_string())?;
    // endregion
    Ok(steps)
}

/// An application's old settings folder, the way it looked before the application joined its
/// family: its settings, a profile, and a theme the family's folder already has a file for. Set
/// up once, so pressing the button again shows what a second start finds.
fn old_folder(root: &Path) -> std::io::Result<()> {
    if root.exists() {
        return Ok(());
    }
    let files = [
        ("quvyta-packages/settings.toml", "theme = \"nordic\"\n"),
        ("quvyta-packages/profiles/work.toml", "mirror = \"eu-west\"\n"),
        ("quvyta-packages/themes/dusk.toml", "# the old copy\n"),
        ("quvyta/packages/themes/dusk.toml", "# already here\n"),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap_or(root))?;
        std::fs::write(path, text)?;
    }
    Ok(())
}

/// Moves the demo's old folder into the demo family folder under `root`.
fn adopt_old_folder(root: &Path) -> Result<Migration, String> {
    old_folder(root).map_err(|error| error.to_string())?;
    let (family, old) = (root.join("quvyta"), root.join("quvyta-packages"));
    // region: storage-adopt
    // An application calls Family::QUVYTA.adopt("packages", &old) once at start, before
    // Settings::load_member. The demo moves into a folder of this run, not into your own.
    let migration = Family::QUVYTA.adopt_in(&family, "packages", &old);
    // endregion
    Ok(migration)
}

/// `path` as the demo shows it: from the demo's own folder on, which is all that differs.
fn shown(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).display().to_string()
}

/// `message` with the demo's own folder taken out of the paths in it.
fn shown_message(root: &Path, message: &str) -> String {
    message.replace(&root.join("").display().to_string(), "")
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
    Write,
    Take,
    Release,
    OpenInstance,
    CloseInstance,
    WaitForLast,
    LastClosed(Result<(), String>),
    Adopt,
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

/// Tries the lock at `path` and says what came of it. The lock has to be kept: dropping the
/// value releases it at once.
fn take_the_lock(path: &Path) -> (Option<AppLock>, Attempt) {
    if let Some(parent) = path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return (None, Attempt::Failed(error.to_string()));
    }
    // region: storage-lock
    match AppLock::acquire(path) {
        Ok(Some(lock)) => (Some(lock), Attempt::Taken),
        // Nobody else's business but the message: the process id is diagnostics, not a decision.
        Ok(None) => (None, Attempt::Busy(qframe::storage::holder_pid(path))),
        Err(error) => (None, Attempt::Failed(error.to_string())),
    }
    // endregion
}

/// Opens one pretend instance: a shared lock, held for as long as the instance runs.
fn open_instance(path: &Path) -> Result<InstanceLock, String> {
    std::fs::create_dir_all(path.parent().expect("the lock file has a folder")).map_err(|e| e.to_string())?;
    // region: storage-instance
    let instance = InstanceLock::shared(path).map_err(|e| e.to_string())?;
    // endregion
    Ok(instance)
}

/// Waits in the background until no instance holds its shared lock, as a service would.
fn wait_for_last(path: PathBuf) -> Command<AppMsg> {
    // region: storage-wait-last
    Command::perform(move || {
        // Sleeps in the kernel until the last shared lock is released, however its process ended.
        let result = InstanceLock::wait_exclusive(&path).map_err(|e| e.to_string());
        // A service would clean up here, then drop the lock so a new instance can start.
        send(Msg::LastClosed(result.map(drop)))
    })
    // endregion
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
        Msg::Write => {
            let result = write_safely(&state.demo.join("tree.toml"));
            match &result {
                Ok(steps) => {
                    for step in steps {
                        log.push(PAGE, "atomic_write_reporting", format!("{step:?}"));
                    }
                }
                Err(error) => log.push(PAGE, "atomic_write_reporting", error.clone()),
            }
            state.write = Some(result);
            Command::none()
        }
        Msg::Take => {
            let (lock, attempt) = take_the_lock(&state.demo.join("lock"));
            log.push(PAGE, "AppLock::acquire", format!("{attempt:?}"));
            // A lock already held stays held: dropping it first would free the very lock the
            // second attempt is supposed to meet.
            if let Some(lock) = lock {
                state.lock = Some(lock);
            }
            state.attempt = attempt;
            Command::none()
        }
        Msg::Release => {
            log.push(PAGE, "AppLock", "dropped the lock");
            state.lock = None;
            state.attempt = Attempt::None;
            Command::none()
        }
        Msg::OpenInstance => {
            match open_instance(&state.demo.join("instances")) {
                Ok(instance) => {
                    state.instances.push(instance);
                    log.push(PAGE, "InstanceLock::shared", format!("{} instances open", state.instances.len()));
                }
                Err(error) => {
                    log.push(PAGE, "InstanceLock::shared", error.clone());
                    state.waited = Some(Err(error));
                }
            }
            Command::none()
        }
        Msg::CloseInstance => {
            state.instances.pop();
            log.push(PAGE, "InstanceLock", format!("{} instances open", state.instances.len()));
            Command::none()
        }
        Msg::WaitForLast => {
            if state.waiting {
                return Command::none();
            }
            if let Err(error) = std::fs::create_dir_all(&state.demo) {
                state.waited = Some(Err(error.to_string()));
                return Command::none();
            }
            log.push(PAGE, "InstanceLock::wait_exclusive", "waiting for the last instance");
            state.waiting = true;
            state.waited = None;
            wait_for_last(state.demo.join("instances"))
        }
        Msg::LastClosed(result) => {
            let text = match &result {
                Ok(()) => "the last instance closed".to_owned(),
                Err(error) => error.clone(),
            };
            log.push(PAGE, "InstanceLock::wait_exclusive", text);
            state.waiting = false;
            state.waited = Some(result);
            Command::none()
        }
        Msg::Adopt => {
            let root = state.demo.join("adopt");
            let result = adopt_old_folder(&root);
            match &result {
                Ok(migration) => {
                    for (from, to) in migration.moved() {
                        log.push(
                            PAGE,
                            "Family::adopt",
                            format!("moved {} to {}", shown(&root, from), shown(&root, to)),
                        );
                    }
                    for diagnostic in migration.diagnostics() {
                        log.push(PAGE, "Family::adopt", shown_message(&root, &diagnostic.message));
                    }
                    if migration.moved().is_empty() && migration.is_clean() {
                        log.push(PAGE, "Family::adopt", "nothing to move");
                    }
                }
                Err(error) => log.push(PAGE, "Family::adopt", error.clone()),
            }
            state.adopt = Some(result);
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

/// The two folders a platform gives an application, one for settings and one for its own data,
/// and the name that keeps one machine's file apart from another's in a folder they share.
fn folders(ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("storage.folders")).gap(0), |ui| {
        ui.add(Text::new(t!("storage.folders-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: storage-folders
        let config = qframe::storage::config_dir("qfocus");
        let data = qframe::storage::data_dir("qfocus");
        // endregion
        for (label, dir) in [(t!("storage.config-dir"), config), (t!("storage.data-dir"), data)] {
            setting(ui, label, |ui| {
                let (text, role) = match dir {
                    Some(dir) => (dir.display().to_string(), "body"),
                    None => (t!("storage.no-folder"), "faint"),
                };
                ui.add(Text::new(text).role(role)).fill_width();
            });
        }
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("storage.machine-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: storage-machine
        let machine = qframe::storage::machine_name();
        let running = machine.as_ref().map(|machine| format!("running-{machine}.toml"));
        // endregion
        setting(ui, t!("storage.machine"), |ui| {
            let (text, role) = match machine {
                Some(machine) => (machine, "body"),
                None => (t!("storage.no-machine"), "faint"),
            };
            ui.add(Text::new(text).role(role)).fill_width();
        });
        if let Some(running) = running {
            setting(ui, t!("storage.machine-file"), |ui| {
                ui.add(Text::new(running).role("body")).fill_width();
            });
        }
    })
    .fill_width();
}

/// The family's shared settings folder and the Documents folder, as they are on this machine.
fn family(ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("storage.family")).gap(0), |ui| {
        ui.add(Text::new(t!("storage.family-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: storage-family
        let family = Family::QUVYTA;
        let rows = [
            (t!("storage.documents-dir"), qframe::storage::documents_dir()),
            (t!("storage.workspace-dir"), family.workspace_dir("Code")),
            (t!("storage.family-dir"), family.config_dir()),
            (t!("storage.shared-file"), family.shared_file()),
            (t!("storage.app-file"), family.app_file("code")),
            (t!("storage.app-dir"), family.app_dir("code")),
        ];
        // endregion
        for (label, path) in rows {
            setting(ui, label, |ui| {
                let (text, role) = match path {
                    Some(path) => (path.display().to_string(), "body"),
                    None => (t!("storage.no-folder"), "faint"),
                };
                ui.add(Text::new(text).role(role)).fill_width();
            });
        }
    })
    .fill_width();
}

/// An old settings folder moved into the family, and what the move reported.
fn adoption(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("storage.adopt")).gap(0), |ui| {
        ui.add(Text::new(t!("storage.adopt-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("storage.adopt-button")).on_press(send(Msg::Adopt))).id("adopt");
        ui.spacer().height(Length::Cells(1));
        let root = state.demo.join("adopt");
        let icons = ui.env().icons();
        let (success, warning, error) = (
            icons.glyph("success").into_owned(),
            icons.glyph("warning").into_owned(),
            icons.glyph("error").into_owned(),
        );
        let line = |ui: &mut View<'_, AppMsg>, marker: &str, color: &'static str, text: String| {
            ui.row(|ui| {
                ui.add(Text::new(marker.to_owned()).color(color).no_wrap());
                ui.add(Text::new(text).role("body")).fill_width();
            })
            .gap(1);
        };
        match &state.adopt {
            None => {
                ui.add(Text::new(t!("storage.not-adopted")).role("faint"));
            }
            Some(Err(problem)) => line(ui, &error, "danger", problem.clone()),
            Some(Ok(migration)) => {
                if migration.moved().is_empty() {
                    ui.add(Text::new(t!("storage.nothing-moved")).role("faint"));
                }
                // region: storage-adopt-report
                for (from, to) in migration.moved() {
                    let text = t!("storage.moved", from = shown(&root, from), to = shown(&root, to));
                    line(ui, &success, "success", text);
                }
                for left in migration.diagnostics() {
                    let (marker, color) = match left.severity {
                        Severity::Error => (&error, "danger"),
                        Severity::Warning => (&warning, "warning"),
                    };
                    line(ui, marker, color, shown_message(&root, &left.message));
                }
                // endregion
            }
        }
    })
    .fill_width();
}

/// One safe write, step by step, so the order that makes it safe is visible.
fn safe_write(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("storage.safe-write")).gap(0), |ui| {
        ui.add(Text::new(t!("storage.safe-write-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("storage.write")).on_press(send(Msg::Write))).id("write");
        match &state.write {
            None => {
                ui.spacer().height(Length::Cells(1));
                ui.add(Text::new(t!("storage.not-written")).role("faint"));
            }
            Some(Ok(steps)) => {
                ui.spacer().height(Length::Cells(1));
                let marker = ui.env().icons().glyph("success").into_owned();
                for step in steps {
                    let text = match step {
                        WriteStep::Wrote(temporary) => {
                            let name = temporary.file_name().unwrap_or_default().display().to_string();
                            t!("storage.step-wrote", name = name)
                        }
                        WriteStep::SyncedFile => t!("storage.step-synced-file"),
                        WriteStep::Renamed => t!("storage.step-renamed"),
                        WriteStep::SyncedDirectory => t!("storage.step-synced-directory"),
                    };
                    ui.row(|ui| {
                        ui.add(Text::new(marker.clone()).color("success").no_wrap());
                        ui.add(Text::new(text).role("body")).fill_width();
                    })
                    .gap(1);
                }
                if !steps.contains(&WriteStep::SyncedDirectory) {
                    ui.add(Text::new(t!("storage.no-directory-sync")).role("faint"));
                }
            }
            Some(Err(error)) => {
                ui.spacer().height(Length::Cells(1));
                let marker = ui.env().icons().glyph("error").into_owned();
                ui.row(|ui| {
                    ui.add(Text::new(marker).color("danger").no_wrap());
                    ui.add(Text::new(error.clone()).color("danger")).fill_width();
                })
                .gap(1);
            }
        }
    })
    .fill_width();
}

/// Taking the lock twice: the first attempt has it, the second is told who does.
fn one_instance(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("storage.one-instance")).gap(0), |ui| {
        ui.add(Text::new(t!("storage.one-instance-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("storage.take")).on_press(send(Msg::Take))).id("take");
            if state.lock.is_some() {
                ui.add(Button::new(t!("storage.release")).on_press(send(Msg::Release))).id("release");
            }
        })
        .gap(2);
        ui.spacer().height(Length::Cells(1));
        let (marker, color, text) = {
            let icons = ui.env().icons();
            match &state.attempt {
                Attempt::None => (icons.glyph("info").into_owned(), "muted", t!("storage.lock-free")),
                Attempt::Taken => (icons.glyph("success").into_owned(), "success", t!("storage.lock-taken")),
                Attempt::Busy(Some(pid)) => {
                    (icons.glyph("warning").into_owned(), "warning", t!("storage.lock-busy-pid", pid = pid.to_string()))
                }
                Attempt::Busy(None) => (icons.glyph("warning").into_owned(), "warning", t!("storage.lock-busy")),
                Attempt::Failed(error) => (icons.glyph("error").into_owned(), "danger", error.clone()),
            }
        };
        ui.row(|ui| {
            ui.add(Text::new(marker).color(color).no_wrap());
            ui.add(Text::new(text).color(color)).fill_width();
        })
        .gap(1);
        instances(state, ui);
    })
    .fill_width();
}

/// How many pretend instances hold the shared lock, in words.
fn instances_open(open: usize) -> String {
    if open == 0 { t!("storage.instances-none") } else { t!("storage.instances-open", n = open) }
}

/// Instances that share a lock, and a waiter that wakes when the last of them closes.
fn instances(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(t!("storage.instances-hint")).role("secondary"));
    ui.spacer().height(Length::Cells(1));
    let open = state.instances.len();
    ui.row(|ui| {
        ui.add(Button::new(t!("storage.open-instance")).on_press(send(Msg::OpenInstance))).id("open-instance");
        ui.add(Button::new(t!("storage.close-instance")).disabled(open == 0).on_press(send(Msg::CloseInstance)))
            .id("close-instance");
        ui.add(Button::new(t!("storage.wait-last")).disabled(state.waiting).on_press(send(Msg::WaitForLast)))
            .id("wait-last");
    })
    .gap(2);
    ui.spacer().height(Length::Cells(1));
    let (marker, color, text) = {
        let icons = ui.env().icons();
        match (&state.waited, state.waiting) {
            (_, true) => (icons.glyph("info").into_owned(), "accent", t!("storage.waiting-last", n = open)),
            (Some(Ok(())), false) => (icons.glyph("success").into_owned(), "success", t!("storage.last-closed")),
            (Some(Err(error)), false) => (icons.glyph("error").into_owned(), "danger", error.clone()),
            (None, false) => (icons.glyph("info").into_owned(), "muted", instances_open(open)),
        }
    };
    ui.row(|ui| {
        ui.add(Text::new(marker).color(color).no_wrap());
        ui.add(Text::new(text).color(color)).fill_width();
    })
    .gap(1);
    if state.waited.is_some() && !state.waiting {
        ui.add(Text::new(instances_open(open)).role("faint"));
    }
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

    folders(ui);
    family(ui);
    adoption(state, ui);
    safe_write(state, ui);
    one_instance(state, ui);

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
        assert!(screen.contains("`language` must be one of de, en, es, fr, ja, pt-BR, ru, tr,"), "{screen}");
        assert!(screen.contains("REPAIRED FILE"), "{screen}");
        let log: Vec<String> = h.app().log.recent(PAGE, 20).iter().map(|entry| entry.message.clone()).collect();
        assert!(log.iter().any(|line| line == "`color` is not a known setting; removed"), "{log:?}");
        let language = "`language` must be one of de, en, es, fr, ja, pt-BR, ru, tr, zh-Hans, found \"sjds\"; replaced with \"en\"";
        assert!(log.iter().any(|line| line == language), "{log:?}");
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
        crate::tests::showcase_tall(showcase, PAGE, 140)
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
    fn both_platform_folders_are_shown() {
        let h = tall(Showcase::new());
        let screen = h.screen();
        assert!(screen.contains("THE TWO FOLDERS"), "{screen}");
        for dir in [qframe::storage::config_dir("qfocus"), qframe::storage::data_dir("qfocus")] {
            // The panel is narrower than a long path, so the last segment is what is checked.
            let shown = match dir {
                Some(dir) => dir.file_name().expect("the folder ends in the app name").display().to_string(),
                None => "this platform gives no folder here".to_owned(),
            };
            assert!(screen.contains(&shown), "{shown} is missing from {screen}");
        }
    }

    #[test]
    fn the_machine_and_its_own_file_are_shown_next_to_the_folders() {
        let h = tall(Showcase::new());
        let screen = h.screen();
        let row = |label: &str| screen.lines().find(|line| line.contains(label)).map(str::to_owned);
        match qframe::storage::machine_name() {
            Some(machine) => {
                assert!(row("This machine").is_some_and(|line| line.contains(&machine)), "{screen}");
                let file = format!("running-{machine}.toml");
                assert!(row("Its file").is_some_and(|line| line.contains(&file)), "{screen}");
            }
            None => {
                assert!(row("This machine").is_some_and(|line| line.contains("gives no name")), "{screen}");
                assert!(row("Its file").is_none(), "no file name is made up:\n{screen}");
            }
        }
    }

    #[test]
    fn the_family_folders_are_shown_as_they_are_on_this_machine() {
        let h = tall(Showcase::new());
        let screen = h.screen();
        assert!(screen.contains("THE FAMILY'S FOLDERS"), "{screen}");
        let family = Family::QUVYTA;
        let places = [
            qframe::storage::documents_dir(),
            family.workspace_dir("Code"),
            family.config_dir(),
            family.shared_file(),
            family.app_file("code"),
            family.app_dir("code"),
        ];
        for place in places {
            // The panel is narrower than a long path, so the last segment is what is checked.
            let shown = match place {
                Some(place) => place.file_name().expect("a named place").display().to_string(),
                None => "this platform gives no folder here".to_owned(),
            };
            assert!(screen.contains(&shown), "{shown} is missing from {screen}");
        }
    }

    #[test]
    fn moving_the_old_folder_in_twice_moves_once_and_keeps_the_taken_file() {
        let mut h = tall(Showcase::new());
        assert!(h.screen().contains("Nothing moved yet"), "{}", h.screen());
        h.click_text("Move the old folder in");
        let root = h.app().pages.storage.demo.join("adopt");
        let first = match h.app().pages.storage.adopt.as_ref().expect("the move ran") {
            Ok(migration) => migration.clone(),
            Err(error) => panic!("the demo could not set up its old folder: {error}"),
        };
        assert_eq!(first.moved().len(), 2, "{first:?}");
        assert_eq!(first.diagnostics().len(), 1, "{first:?}");
        let read = |name: &str| std::fs::read_to_string(root.join(name)).expect("a demo file");
        assert_eq!(read("quvyta/packages.conf"), "theme = \"nordic\"\n");
        assert_eq!(read("quvyta/packages/profiles/work.toml"), "mirror = \"eu-west\"\n");
        assert_eq!(read("quvyta/packages/themes/dusk.toml"), "# already here\n", "never overwritten");
        assert_eq!(read("quvyta-packages/themes/dusk.toml"), "# the old copy\n", "never lost");
        let screen = h.screen();
        assert!(screen.contains("quvyta-packages/settings.toml moved to quvyta/packages.conf"), "{screen}");
        assert!(screen.contains("already exists"), "{screen}");
        assert!(!screen.contains(&root.display().to_string()), "the demo folder is taken out of the paths");

        h.click_text("Move the old folder in");
        let second = match h.app().pages.storage.adopt.as_ref().expect("the move ran") {
            Ok(migration) => migration.clone(),
            Err(error) => panic!("{error}"),
        };
        assert!(second.moved().is_empty(), "{second:?}");
        assert_eq!(second.diagnostics(), first.diagnostics(), "the file left behind is reported again");
        assert!(h.screen().contains("Nothing left to move"), "{}", h.screen());
        let log: Vec<String> = h.app().log.recent(PAGE, 10).iter().map(|entry| entry.message.clone()).collect();
        assert!(
            log.iter().any(|line| line == "moved quvyta-packages/settings.toml to quvyta/packages.conf"),
            "{log:?}"
        );
        std::fs::remove_dir_all(&h.app().pages.storage.demo).expect("clean up the demo folder");
    }

    #[test]
    fn the_safe_write_shows_every_step_it_took() {
        let mut h = tall(Showcase::new());
        assert!(h.screen().contains("Nothing written yet"), "{}", h.screen());
        h.click_text("Write a file safely");
        let steps = match h.app().pages.storage.write.as_ref().expect("the write ran") {
            Ok(steps) => steps.clone(),
            Err(error) => panic!("the demo write failed: {error}"),
        };
        assert!(matches!(steps.first(), Some(WriteStep::Wrote(_))), "{steps:?}");
        assert!(steps.contains(&WriteStep::Renamed), "{steps:?}");
        let screen = h.screen();
        assert!(screen.contains("in the same folder"), "{screen}");
        assert!(screen.contains("renamed it over the real name"), "{screen}");
        if cfg!(unix) {
            assert!(steps.contains(&WriteStep::SyncedDirectory), "{steps:?}");
            assert!(screen.contains("survives a power cut"), "{screen}");
        } else {
            assert!(screen.contains("cannot flush a folder"), "{screen}");
        }
        let log: Vec<String> = h.app().log.recent(PAGE, 10).iter().map(|entry| entry.message.clone()).collect();
        assert_eq!(log.len(), steps.len(), "every step is logged: {log:?}");
        std::fs::remove_dir_all(&h.app().pages.storage.demo).expect("clean up the demo folder");
    }

    #[test]
    fn the_second_attempt_at_the_lock_says_another_process_holds_it() {
        if !cfg!(unix) {
            return;
        }
        let mut h = tall(Showcase::new());
        assert!(h.screen().contains("Nobody has tried the lock yet"), "{}", h.screen());

        h.click_text("Take the lock");
        assert!(matches!(h.app().pages.storage.attempt, Attempt::Taken), "{:?}", h.app().pages.storage.attempt);
        assert!(h.screen().contains("This instance holds the lock"), "{}", h.screen());

        // The same call a second process would make; the first lock is still held.
        h.click_text("Take the lock");
        let pid = std::process::id();
        match &h.app().pages.storage.attempt {
            Attempt::Busy(Some(shown)) => assert_eq!(*shown, pid, "the file names the holder"),
            other => panic!("the second attempt must meet the first, not {other:?}"),
        }
        assert!(h.screen().contains("Another process holds it"), "{}", h.screen());

        h.click_text("Release the lock");
        assert!(h.app().pages.storage.lock.is_none());
        h.click_text("Take the lock");
        assert!(matches!(h.app().pages.storage.attempt, Attempt::Taken), "the released lock is free again");
        h.send(send(Msg::Release));
        std::fs::remove_dir_all(&h.app().pages.storage.demo).expect("clean up the demo folder");
    }

    #[test]
    fn instances_share_the_lock_and_the_waiter_takes_it_once_none_is_left() {
        if !cfg!(unix) {
            return;
        }
        let mut h = tall(Showcase::new());
        assert!(h.screen().contains("No instance is open"), "{}", h.screen());
        h.click_text("Open an instance");
        h.click_text("Open an instance");
        assert_eq!(h.app().pages.storage.instances.len(), 2, "shared locks do not keep each other out");
        assert!(h.screen().contains("2 instances hold the shared lock"), "{}", h.screen());
        let path = h.app().pages.storage.demo.join("instances");
        assert!(InstanceLock::try_exclusive(&path).expect("attempt").is_none(), "the instances keep it out");

        h.click_text("Close an instance");
        h.click_text("Close an instance");
        // The harness runs background work inline, so the wait is asked for once nobody is left;
        // in the terminal it runs on a thread and the buttons above wake it.
        h.click_text("Wait for the last to close");
        assert!(matches!(h.app().pages.storage.waited, Some(Ok(()))), "{:?}", h.app().pages.storage.waited);
        assert!(h.screen().contains("The last instance closed"), "{}", h.screen());
        std::fs::remove_dir_all(&h.app().pages.storage.demo).expect("clean up the demo folder");
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
