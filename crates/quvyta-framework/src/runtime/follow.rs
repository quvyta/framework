//! Following the ecosystem's shared preferences while the application runs.
//!
//! An application started as a member of an ecosystem ([`Runtime::member`](super::Runtime::member))
//! reads its own settings and the shared preferences once, before the first frame, and then
//! watches the ecosystem's folder. When the shared file or the application's own file changes,
//! the preferences are resolved again, without writing anything, and only what differs from what
//! was heard last is applied. The folder is watched with the system's own events
//! ([`FolderWatch`]), on a thread that sleeps in the kernel until something changes.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::env::Env;
use crate::i18n::I18n;
use crate::icons::PillarStyle;
use crate::storage::{Ecosystem, FolderChange, FolderWatch, Preferences, Settings};

/// Which application of which ecosystem the runtime runs, and where the ecosystem's files are.
#[derive(Debug, Clone)]
pub(crate) struct Member {
    ecosystem: Ecosystem,
    app: String,
    /// The ecosystem's folder when the application chose one; this platform's otherwise.
    folder: Option<PathBuf>,
}

impl Member {
    pub(crate) fn new(ecosystem: Ecosystem, app: &str, folder: Option<PathBuf>) -> Self {
        Self { ecosystem, app: app.to_owned(), folder }
    }

    /// The ecosystem's folder, `None` when there is no home folder.
    fn folder(&self) -> Option<PathBuf> {
        self.folder.clone().or_else(|| self.ecosystem.config_dir())
    }

    /// The application's own settings, as [`Settings::load_member`] reads them.
    pub(crate) fn settings(&self) -> Settings {
        match &self.folder {
            Some(folder) => Settings::open(own_file(folder, &self.app)).member_of(&self.ecosystem),
            None => Settings::load_member(&self.ecosystem, &self.app),
        }
    }

    /// The shared preferences as the application starts with them, as
    /// [`Ecosystem::preferences`] resolves them: a missing shared file is written for the next
    /// application.
    pub(crate) fn preferences(&self, i18n: &I18n) -> Preferences {
        match &self.folder {
            Some(folder) => self.ecosystem.preferences_in(folder, &self.app, i18n),
            None => self.ecosystem.preferences(&self.app, i18n),
        }
    }

    /// What follows the files from here on, starting from what the application started with.
    /// `watch` starts the thread that hears the folder; a test drives the check itself instead.
    pub(crate) fn follow(self, settings: &Settings, preferences: Preferences, watch: bool) -> Follow {
        let folder = self.folder();
        let watch = match (&folder, watch) {
            (Some(folder), true) => Watch::start(folder, [shared_name(&self.ecosystem), file_name(&self.app)]),
            _ => None,
        };
        Follow {
            member: self,
            folder,
            heard: Heard { preferences, pillar: settings.pillar_style() },
            told: false,
            watch,
        }
    }
}

/// What an application starts with besides the built-in files: a theme, saved settings and the
/// ecosystem's preferences, each winning over the one before.
pub(crate) struct Start<'a> {
    pub(crate) theme: Option<&'a str>,
    pub(crate) settings: Option<Settings>,
    pub(crate) preferences: Option<Preferences>,
}

impl Start<'_> {
    /// Sets `env` up with what the application starts with. For a `member`, what was not given is
    /// read from its files, once, and what follows those files from now on is returned, with the
    /// folder watched when `watch` says so.
    pub(crate) fn apply(self, env: &mut Env, member: Option<Member>, watch: bool) -> Option<Follow> {
        let (settings, preferences) = match &member {
            Some(member) => (
                Some(self.settings.unwrap_or_else(|| member.settings())),
                Some(self.preferences.unwrap_or_else(|| member.preferences(env.i18n()))),
            ),
            None => (self.settings, self.preferences),
        };
        if let Some(theme) = self.theme {
            env.set_theme(theme);
        }
        if let Some(settings) = &settings {
            env.apply_settings(settings);
        }
        if let Some(preferences) = &preferences {
            env.apply_preferences(preferences);
        }
        let (member, settings, preferences) = (member?, settings?, preferences?);
        Some(member.follow(&settings, preferences, watch))
    }
}

/// The name of an application's or the ecosystem's file in the ecosystem's folder.
fn file_name(id: &str) -> OsString {
    OsString::from(crate::storage::ecosystem_file_name(id))
}

fn shared_name(ecosystem: &Ecosystem) -> OsString {
    file_name(ecosystem.id())
}

fn own_file(folder: &Path, app: &str) -> PathBuf {
    folder.join(file_name(app))
}

/// What the files said the last time they were read.
#[derive(Debug, Clone)]
pub(crate) struct Heard {
    pub(crate) preferences: Preferences,
    pub(crate) pillar: Option<PillarStyle>,
}

/// A member's preferences, followed while the application runs.
#[derive(Debug)]
pub(crate) struct Follow {
    member: Member,
    /// The ecosystem's folder; `None` when there is no home folder, and then nothing is followed.
    folder: Option<PathBuf>,
    /// What the files said last.
    pub(crate) heard: Heard,
    /// Whether [`App::preferences`](super::App::preferences) heard the start.
    pub(crate) told: bool,
    watch: Option<Watch>,
}

impl Follow {
    /// Whether the watch heard a change in one of the two files since the last call.
    pub(crate) fn take_change(&self) -> bool {
        self.watch.as_ref().is_some_and(Watch::take_change)
    }

    /// Reads both files again, as they are now, without writing anything: a missing shared file
    /// stays missing. `None` when there is no folder to read.
    pub(crate) fn read(&self, i18n: &I18n) -> Option<Heard> {
        let folder = self.folder.as_ref()?;
        let ecosystem = &self.member.ecosystem;
        let preferences = ecosystem.preferences_without_saving_in(folder, &self.member.app, i18n);
        let own = Settings::open(own_file(folder, &self.member.app)).member_of(ecosystem);
        Some(Heard { preferences, pillar: own.pillar_style() })
    }
}

/// The thread that hears the ecosystem's folder, and the flag it raises.
///
/// The thread sleeps in the kernel until the folder changes, raises the flag when one of the two
/// files it cares about is among the changes, and wakes the terminal loop, which takes the flag
/// on its next pass. Dropping the watch closes it, which wakes the thread one last time with an
/// empty answer, and the thread ends.
#[derive(Debug)]
struct Watch {
    /// Kept only so that dropping the follow closes the watch and ends the thread.
    _watch: FolderWatch,
    changed: Arc<AtomicBool>,
}

impl Watch {
    /// Watches `folder` for changes to `names`. `None` when the folder cannot be watched: it is
    /// missing, the platform has no watch, the system's limit is reached, or no thread can
    /// start. The application then runs with what it started with, as before.
    fn start(folder: &Path, names: [OsString; 2]) -> Option<Self> {
        let mut watch = FolderWatch::new().ok()?;
        watch.watch(folder).ok()?;
        let changes = watch.changes();
        let changed = Arc::new(AtomicBool::new(false));
        let raised = Arc::clone(&changed);
        std::thread::Builder::new()
            .name("quvyta-preferences".to_owned())
            .spawn(move || {
                loop {
                    // Empty only once the watch is dropped: the application is ending.
                    let batch = changes.next();
                    if batch.is_empty() {
                        return;
                    }
                    if batch.iter().any(|change| concerns(change, &names)) {
                        raised.store(true, Ordering::SeqCst);
                        super::signals::wake();
                    }
                }
            })
            .ok()?;
        Some(Self { _watch: watch, changed })
    }

    fn take_change(&self) -> bool {
        self.changed.swap(false, Ordering::SeqCst)
    }
}

/// Whether `change` may have changed one of the files named `names`. A file saved in place is
/// modified; one written the safe way is renamed over the old one, which arrives here as a rename
/// to its name. Events the system dropped may have been anything.
fn concerns(change: &FolderChange, names: &[OsString; 2]) -> bool {
    use crate::storage::FolderChangeKind;
    match change.kind {
        FolderChangeKind::Overflow => true,
        _ => change.name.as_ref().is_some_and(|name| names.contains(name)),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::storage::{FolderChangeKind, Scope, Shared};

    /// A folder of its own for each test, empty at the start.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-follow-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("test folder");
        dir
    }

    /// Waits for the watch's flag, at most `bound`.
    fn heard_within(watch: &Watch, bound: Duration) -> bool {
        let until = Instant::now() + bound;
        while Instant::now() < until {
            if watch.take_change() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    fn names() -> [OsString; 2] {
        [OsString::from("quvyta.conf"), OsString::from("code.conf")]
    }

    #[test]
    fn only_the_two_files_and_dropped_events_concern_the_follow() {
        let change = |name: Option<&str>, kind| FolderChange {
            folder: PathBuf::from("/c"),
            name: name.map(OsString::from),
            kind,
        };
        let names = names();
        assert!(concerns(&change(Some("quvyta.conf"), FolderChangeKind::Modified), &names));
        assert!(concerns(
            &change(Some("code.conf"), FolderChangeKind::Renamed { from: ".code.conf.tmp".into() }),
            &names
        ));
        assert!(!concerns(&change(Some("desktop.conf"), FolderChangeKind::Modified), &names));
        assert!(!concerns(&change(None, FolderChangeKind::Gone), &names));
        assert!(concerns(&change(None, FolderChangeKind::Overflow), &names));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_change_to_the_shared_file_raises_the_flag_and_one_elsewhere_does_not() {
        let dir = folder("flag");
        let watch = Watch::start(&dir, names()).expect("a watch on a folder that is there");
        fs::write(dir.join("notes.txt"), "not a settings file").expect("write");
        // Longer than the tenth of a second a batch gathers, so the batch was read and passed over.
        assert!(!heard_within(&watch, Duration::from_millis(600)), "another file is not a change");
        Ecosystem::QUVYTA.set_in(&dir, "code", Shared::Theme, "amber", Scope::Ecosystem).expect("set");
        assert!(heard_within(&watch, Duration::from_secs(10)), "the shared file changed");
        drop(watch);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_folder_is_not_watched_and_nothing_fails() {
        let dir = std::env::temp_dir().join(format!("quvyta-follow-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(Watch::start(&dir, names()).is_none());
    }
}
