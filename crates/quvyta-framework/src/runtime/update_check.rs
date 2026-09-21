//! Saying when a newer version of the application is out.
//!
//! A person who installed an application once keeps running that version until something tells
//! them otherwise, and an old copy on another machine is the kind of trouble nobody connects to
//! its cause. An [`UpdateCheck`] asks the package registry, at most once a day and never while the
//! application waits for it, whether a newer version has been published, and answers with a
//! message only when one has.
//!
//! What goes out is the package's name and nothing else: the request carries no identity, no
//! machine detail and no use of the application, and its `User-Agent` is the package's name and
//! version. The family's one switch, [`Family::update_notice`], turns the question off for every
//! application of the family, and then nothing is asked at all.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::storage::{Family, atomic_write};
use crate::widgets::Toast;

/// The registry asked unless another is given: crates.io's index, the one `cargo install` reads.
const CRATES_IO: &str = "https://index.crates.io";

/// How often the question is asked at most, whatever the answer was.
const ONCE_A_DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// How long an answer may take before the question is dropped until the next day.
const PATIENCE: Duration = Duration::from_secs(10);

/// The file in the application's state folder that remembers when the question was last asked.
const LAST_ASKED: &str = "update-check";

/// A question to the package registry: is there a version of this package newer than the one
/// running? Given to [`Command::check_for_update`](super::Command::check_for_update), usually from
/// [`App::init`](super::App::init).
///
/// The question is asked on a thread of its own, so the application starts without waiting for
/// it. It is asked at most once a day, remembered in the application's
/// [state folder](Family::state_dir), and not at all while the family's
/// [update notice](Family::update_notice) is off. No network, a registry that does not answer or
/// an answer that cannot be read is silence: nothing is shown and the next day asks again. Only a
/// newer version, not a pre-release and not a yanked one, becomes a message.
///
/// A [`Harness`](super::Harness) never reaches the network: it records the question, see
/// [`Harness::update_checks`](super::Harness::update_checks), and answers it with the version
/// [`Harness::set_latest_version`](super::Harness::set_latest_version) names, if any.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::runtime::{Update, UpdateCheck};
/// use qframe::storage::Family;
///
/// struct Code;
///
/// enum Msg {
///     NewVersion(Update),
/// }
///
/// impl App for Code {
///     type Msg = Msg;
///     fn init(&mut self) -> Command<Msg> {
///         let check = UpdateCheck::new(Family::QUVYTA, "code", "quvyta-code", env!("CARGO_PKG_VERSION"), Msg::NewVersion);
///         Command::check_for_update(check)
///     }
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::NewVersion(update) => Command::toast(update.toast()),
///         }
///     }
///     fn view(&self, _ui: &mut View<'_, Msg>) {}
/// }
///
/// let mut code = Harness::new(Code, 60, 8);
/// assert_eq!(code.update_checks()[0].package(), "quvyta-code", "asked, but not over the network");
/// ```
pub struct UpdateCheck<Msg> {
    family: Family,
    app: String,
    package: String,
    current: String,
    config_dir: Option<PathBuf>,
    state_dir: Option<PathBuf>,
    registry: String,
    on_newer: Box<dyn FnOnce(Update) -> Msg + Send>,
}

impl<Msg: Send + 'static> UpdateCheck<Msg> {
    /// Asks whether crates.io has a version of `package` newer than `current`, for application
    /// `app` of `family`; a newer one is sent as `on_newer`. `current` is the running version,
    /// usually `env!("CARGO_PKG_VERSION")`.
    #[must_use]
    pub fn new(
        family: Family,
        app: impl Into<String>,
        package: impl Into<String>,
        current: impl Into<String>,
        on_newer: impl FnOnce(Update) -> Msg + Send + 'static,
    ) -> Self {
        Self {
            family,
            app: app.into(),
            package: package.into(),
            current: current.into(),
            config_dir: None,
            state_dir: None,
            registry: CRATES_IO.to_owned(),
            on_newer: Box::new(on_newer),
        }
    }

    /// Reads the family's switch from `config_dir` and remembers the last question in `state_dir`
    /// instead of this platform's folders, for a test or a demo that must leave the user's own
    /// files alone.
    #[must_use]
    pub fn in_folders(mut self, config_dir: impl Into<PathBuf>, state_dir: impl Into<PathBuf>) -> Self {
        self.config_dir = Some(config_dir.into());
        self.state_dir = Some(state_dir.into());
        self
    }

    /// Asks the sparse index at `address` instead of crates.io's: a mirror, or a server of a
    /// test's own. The index is read the way cargo reads it, `<address>/<prefix>/<package>`.
    #[must_use]
    pub fn registry(mut self, address: impl Into<String>) -> Self {
        self.registry = address.into().trim_end_matches('/').to_owned();
        self
    }

    /// The same question, answering with `map(message)`.
    pub(crate) fn map<B: Send + 'static>(self, map: impl FnOnce(Msg) -> B + Send + 'static) -> UpdateCheck<B> {
        let on_newer = self.on_newer;
        UpdateCheck {
            family: self.family,
            app: self.app,
            package: self.package,
            current: self.current,
            config_dir: self.config_dir,
            state_dir: self.state_dir,
            registry: self.registry,
            on_newer: Box::new(move |update| map(on_newer(update))),
        }
    }

    /// What a harness records of the question.
    pub(crate) fn request(&self) -> UpdateCheckRequest {
        UpdateCheckRequest { package: self.package.clone(), current: self.current.clone() }
    }

    /// The answer of a harness that was told `latest` is the newest version: the message, when it
    /// is newer than the running one. Nothing is read, written or asked.
    pub(crate) fn answer(self, latest: &str) -> Option<Msg> {
        self.newer(latest)
    }

    /// Asks the question at `now`, when it is due, and returns the message a newer version makes.
    /// Runs on a thread of its own; every failure on the way is silence.
    pub(crate) fn ask(self, now: SystemTime) -> Option<Msg> {
        self.ask_with(now, fetch)
    }

    /// [`ask`](Self::ask) with the registry reached through `fetch`, which returns the index text
    /// of an address or `None`.
    fn ask_with(self, now: SystemTime, fetch: impl FnOnce(&str, &str) -> Option<String>) -> Option<Msg> {
        let config_dir = self.config_dir.clone().or_else(|| self.family.config_dir())?;
        if !self.family.update_notice_in(&config_dir) {
            return None;
        }
        let state_dir = self.state_dir.clone().or_else(|| self.family.state_dir(&self.app))?;
        if !due(&state_dir, now) {
            return None;
        }
        // Remembered before asking, so a question that fails waits for the next day as one that
        // succeeds does. A question that cannot be remembered is not asked: it would be asked
        // again at every start.
        remember(&state_dir, now)?;
        let agent = format!("{}/{}", self.package, self.current);
        let index = fetch(&index_address(&self.registry, &self.package), &agent)?;
        let latest = newest(&index)?;
        self.newer(&latest)
    }

    /// The message for `latest`, when it is newer than the running version.
    fn newer(self, latest: &str) -> Option<Msg> {
        let (running, found) = (version(&self.current)?, version(latest)?);
        (found > running).then(|| {
            (self.on_newer)(Update {
                family: self.family,
                package: self.package,
                current: self.current,
                latest: latest.to_owned(),
            })
        })
    }
}

/// A question to the registry that a [`Harness`](super::Harness) recorded instead of asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckRequest {
    package: String,
    current: String,
}

impl UpdateCheckRequest {
    /// The package asked about.
    #[must_use]
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The version the application said it runs.
    #[must_use]
    pub fn current(&self) -> &str {
        &self.current
    }
}

/// A newer version of the application, found by an [`UpdateCheck`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    family: Family,
    package: String,
    current: String,
    latest: String,
}

impl Update {
    /// The news that `latest` of `package` is out while `current` runs, for application `family`.
    /// An [`UpdateCheck`] makes one when the registry says so; made by hand it shows what the
    /// notice looks like, on a settings page or in a guide.
    #[must_use]
    pub fn new(
        family: Family,
        package: impl Into<String>,
        current: impl Into<String>,
        latest: impl Into<String>,
    ) -> Self {
        Self { family, package: package.into(), current: current.into(), latest: latest.into() }
    }

    /// The package that has a newer version.
    #[must_use]
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The version running now.
    #[must_use]
    pub fn current(&self) -> &str {
        &self.current
    }

    /// The newest version published.
    #[must_use]
    pub fn latest(&self) -> &str {
        &self.latest
    }

    /// The notice every application of the family shows the same way: an info toast naming the
    /// new version and the running one, and how to update — from the family's launcher, or with
    /// `cargo install`. Texts come from the framework's language files.
    #[must_use]
    pub fn toast<Msg>(&self) -> Toast<Msg> {
        let title = crate::t!("quvyta.update.title", package = self.package.as_str(), latest = self.latest.as_str());
        let body = crate::t!(
            "quvyta.update.body",
            current = self.current.as_str(),
            launcher = self.family.id(),
            package = self.package.as_str()
        );
        Toast::info(title).body(body).key("quvyta-update").duration(Duration::from_secs(12))
    }
}

/// Whether the question is due at `now`: never asked, asked a day or more ago, or remembered at a
/// time after `now`, which only a clock set back makes and which must not silence it for good.
fn due(state_dir: &Path, now: SystemTime) -> bool {
    let Some(last) = std::fs::read_to_string(state_dir.join(LAST_ASKED))
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds))
    else {
        return true;
    };
    now.duration_since(last).map_or(true, |since| since >= ONCE_A_DAY)
}

/// Writes `now` as the time the question was last asked.
fn remember(state_dir: &Path, now: SystemTime) -> Option<()> {
    let seconds = now.duration_since(UNIX_EPOCH).ok()?.as_secs();
    std::fs::create_dir_all(state_dir).ok()?;
    atomic_write(&state_dir.join(LAST_ASKED), format!("{seconds}\n").as_bytes()).ok()
}

/// The address of `package` in the sparse index at `registry`, laid out as cargo lays it out:
/// `1/a`, `2/ab`, `3/a/abc`, and `ab/cd/abcd…` for longer names.
fn index_address(registry: &str, package: &str) -> String {
    let name = package.to_lowercase();
    let prefix = match name.len() {
        0 => String::new(),
        1 => "1".to_owned(),
        2 => "2".to_owned(),
        3 => format!("3/{}", &name[..1]),
        _ => format!("{}/{}", &name[..2], &name[2..4]),
    };
    format!("{registry}/{prefix}/{name}")
}

/// Reads the index text at `address`, sending `agent` as the `User-Agent` and nothing else about
/// the person or the machine. `None` for any failure: no network, no answer in time, an error
/// status or a body that is not text.
fn fetch(address: &str, agent: &str) -> Option<String> {
    let config = ureq::Agent::config_builder().timeout_global(Some(PATIENCE)).build();
    let agent_of_requests: ureq::Agent = config.into();
    let mut response = agent_of_requests.get(address).header("User-Agent", agent).call().ok()?;
    response.body_mut().read_to_string().ok()
}

/// The newest published version in a sparse index text: one JSON object per line, each with its
/// `vers` and `yanked`. Yanked versions and pre-releases are passed over; a line that cannot be
/// read is skipped.
fn newest(index: &str) -> Option<String> {
    index
        .lines()
        .filter(|line| field(line, "yanked") != Some("true"))
        .filter_map(|line| field(line, "vers"))
        .filter_map(|text| version(text).map(|parsed| (parsed, text)))
        .max_by_key(|(parsed, _)| *parsed)
        .map(|(_, text)| text.to_owned())
}

/// The value of `name` in one compact JSON object line: the text between the quotes of a string,
/// or the bare word of a literal. Enough for the index's flat lines, whose values never hold a
/// quote.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("\"{name}\"");
    let after = line[line.find(&key)? + key.len()..].trim_start().strip_prefix(':')?.trim_start();
    match after.strip_prefix('"') {
        Some(text) => Some(&text[..text.find('"')?]),
        None => Some(after[..after.find([',', '}']).unwrap_or(after.len())].trim()),
    }
}

/// `major.minor.patch` as numbers; `None` for a pre-release or anything else.
fn version(text: &str) -> Option<(u64, u64, u64)> {
    let text = text.split('+').next()?;
    let mut parts = text.split('.').map(|part| part.parse::<u64>().ok());
    let parsed = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(parsed)
}

#[cfg(test)]
#[path = "update_check_tests.rs"]
mod tests;
