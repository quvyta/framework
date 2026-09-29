//! Installing a missing program's package here, in front of the person.
//!
//! When an application needs a program the machine does not have, it does not tell the person to
//! copy a command into a terminal: it offers to do it. [`Install`] finds this machine's package
//! manager, builds the exact command, asks with a [`Confirm`] that shows that command, and hands
//! the terminal to the package manager with a [`Handoff`] once the person agreed. The package
//! manager then asks for the password and its own questions itself, on the terminal, and its last
//! lines stay on screen until a key is pressed.
//!
//! ```
//! use qframe::install::{Install, Manager};
//! use qframe::prelude::*;
//! use qframe::runtime::HandoffOutcome;
//!
//! enum Msg {
//!     AskToInstall,
//!     Install,
//!     Installed(HandoffOutcome),
//! }
//!
//! struct Viewer {
//!     /// `None` when this machine has no package manager the framework knows.
//!     install: Option<Install>,
//! }
//!
//! impl Viewer {
//!     fn new() -> Self {
//!         // Debian and its relatives ship `bsdtar` in a package of another name.
//!         Self { install: Install::package("libarchive").map(|i| i.name_for(Manager::Apt, "libarchive-tools")) }
//!     }
//!
//!     fn update(&mut self, msg: Msg) -> Command<Msg> {
//!         match (msg, &self.install) {
//!             // First the question: nothing runs until the person answers it.
//!             (Msg::AskToInstall, Some(install)) => Command::confirm(install.confirm(Msg::Install)),
//!             (Msg::Install, Some(install)) => Command::handoff(install.handoff(Msg::Installed)),
//!             _ => Command::none(),
//!         }
//!     }
//! }
//! ```

use std::ffi::OsString;
use std::path::PathBuf;

use crate::runtime::{Confirm, Handoff, HandoffOutcome};

/// A package manager [`Install`] knows how to ask for a package.
///
/// AUR helpers such as `paru` and `yay` are not among them on purpose: they build packages from
/// source as the user who runs them, which is a different act from "install this package", and
/// the package a program needs is in the official repositories.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Manager {
    /// Arch Linux and its relatives: `pacman -S --needed`.
    Pacman,
    /// Debian, Ubuntu and their relatives: `apt-get install`.
    Apt,
    /// Fedora and its relatives: `dnf install`.
    Dnf,
    /// openSUSE: `zypper install`.
    Zypper,
    /// Alpine Linux: `apk add`.
    Apk,
    /// Homebrew, on macOS and Linux: `brew install`, never as root.
    Brew,
}

impl Manager {
    /// Every manager, in the order [`Install::package`] looks for them.
    #[must_use]
    pub fn all() -> &'static [Manager] {
        &[Manager::Pacman, Manager::Apt, Manager::Dnf, Manager::Zypper, Manager::Apk, Manager::Brew]
    }

    /// The manager's program, as it is found on `PATH` and run: `pacman`, `apt-get`, `dnf`,
    /// `zypper`, `apk` or `brew`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Manager::Pacman => "pacman",
            Manager::Apt => "apt-get",
            Manager::Dnf => "dnf",
            Manager::Zypper => "zypper",
            Manager::Apk => "apk",
            Manager::Brew => "brew",
        }
    }

    /// The words after the program that install one package. No "yes" flag is among them: the
    /// person answers the manager's own question.
    fn install_words(self) -> &'static [&'static str] {
        match self {
            Manager::Pacman => &["-S", "--needed"],
            Manager::Apt | Manager::Dnf | Manager::Zypper | Manager::Brew => &["install"],
            Manager::Apk => &["add"],
        }
    }

    /// Whether installing needs root. Homebrew installs into a folder the user owns and refuses
    /// to run as root.
    fn needs_root(self) -> bool {
        self != Manager::Brew
    }
}

/// One package to install with this machine's package manager: the command, the question that
/// shows it and the handoff that runs it.
///
/// Made by [`Install::package`], which looks for `pacman`, `apt-get`, `dnf`, `zypper`, `apk` and
/// `brew` on `PATH` in that order and takes the first. The command is the manager's own install
/// command with `sudo` in front when the application does not run as root, such as
/// `sudo pacman -S --needed libarchive`; no "yes" flag is added, so the manager shows its own
/// question and the person answers it.
///
/// ```
/// use std::path::PathBuf;
///
/// use qframe::install::{Install, Manager};
///
/// // A machine with `apt-get` and nothing else, the application not running as root.
/// let lookup = |program: &str| (program == "apt-get").then(|| PathBuf::from("/usr/bin/apt-get"));
/// let install = Install::package_with("libarchive", lookup, false)
///     .expect("a package manager")
///     .name_for(Manager::Apt, "libarchive-tools");
/// assert_eq!(install.manager(), Manager::Apt);
/// assert_eq!(install.command_line(), "sudo apt-get install libarchive-tools");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    manager: Manager,
    package: String,
    root: bool,
}

impl Install {
    /// The package `name` with this machine's package manager, found on `PATH`; `None` when there
    /// is none, or when `name` cannot be a package name (empty, starting with `-`, or holding a
    /// space or a control character).
    ///
    /// Homebrew is passed over when the application runs as root, since it refuses to run so.
    #[must_use]
    pub fn package(name: &str) -> Option<Install> {
        let path = std::env::var_os("PATH");
        let lookup = |program: &str| {
            crate::desktop::program::find_program(program, path.as_deref(), crate::desktop::program::is_executable)
        };
        Self::package_with(name, lookup, running_as_root())
    }

    /// [`Install::package`] with the search given: `lookup` says where a program is, or `None`
    /// when it is not installed, and `root` whether the application runs as root. For tests, and
    /// for an application that decides itself where programs are.
    #[must_use]
    pub fn package_with(name: &str, lookup: impl Fn(&str) -> Option<PathBuf>, root: bool) -> Option<Install> {
        if !is_package_name(name) {
            return None;
        }
        let manager = Manager::all()
            .iter()
            .copied()
            .filter(|manager| !(root && *manager == Manager::Brew))
            .find(|manager| lookup(manager.name()).is_some())?;
        Some(Install { manager, package: name.to_owned(), root })
    }

    /// Uses `name` instead when the package manager is `manager`, for a package that is called
    /// differently there: `libarchive` on Arch is `libarchive-tools` on Debian. Nothing changes on
    /// other managers, nor when `name` cannot be a package name.
    #[must_use]
    pub fn name_for(mut self, manager: Manager, name: &str) -> Self {
        if manager == self.manager && is_package_name(name) {
            name.clone_into(&mut self.package);
        }
        self
    }

    /// The package manager that installs the package.
    #[must_use]
    pub fn manager(&self) -> Manager {
        self.manager
    }

    /// The package's name for this manager.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.package
    }

    /// The command's words, the program first: `["sudo", "pacman", "-S", "--needed", "libarchive"]`.
    #[must_use]
    pub fn program(&self) -> Vec<OsString> {
        self.words().into_iter().map(OsString::from).collect()
    }

    /// The command as the person reads it: `sudo pacman -S --needed libarchive`. The words need
    /// no quoting, since a package name holds no space.
    #[must_use]
    pub fn command_line(&self) -> String {
        self.words().join(" ")
    }

    /// The question to ask before installing: which package, the exact command line, and that
    /// the package manager may ask for a password. The confirm button reads Install and sends
    /// `on_yes`; Cancel, Esc and the close mark install nothing. Add
    /// [`on_cancel`](Confirm::on_cancel) for a message when the person declines.
    ///
    /// The text comes from `quvyta.install.title`, `quvyta.install.runs`,
    /// `quvyta.install.asks` and `quvyta.install.confirm` in the active language.
    #[must_use]
    pub fn confirm<Msg>(&self, on_yes: Msg) -> Confirm<Msg> {
        let manager = self.manager.name();
        let message = format!(
            "{}\n\n{}\n\n{}",
            crate::t!("quvyta.install.runs"),
            self.command_line(),
            crate::t!("quvyta.install.asks", manager = manager),
        );
        Confirm::new(crate::t!("quvyta.install.title", package = self.package.as_str()), on_yes)
            .message(message)
            .confirm_label(crate::t!("quvyta.install.confirm"))
    }

    /// The handoff that runs the command: the terminal is the package manager's until it ends,
    /// with a line saying what is being installed (`quvyta.install.notice`) above its output,
    /// and a key press awaited afterwards so its last lines can be read. `on_finish` receives how
    /// it ended; an exit code other than zero means the package was not installed.
    ///
    /// Return it only after the person agreed to [`confirm`](Self::confirm).
    #[must_use]
    pub fn handoff<Msg: Send + 'static>(
        &self,
        on_finish: impl FnOnce(HandoffOutcome) -> Msg + Send + 'static,
    ) -> Handoff<Msg> {
        let words = self.words();
        let notice = crate::t!("quvyta.install.notice", package = self.package.as_str(), manager = self.manager.name());
        Handoff::new(words[0], on_finish).args(words[1..].iter().copied()).notice(notice).pause(true)
    }

    /// The command's words, the program first.
    fn words(&self) -> Vec<&str> {
        let mut words = Vec::with_capacity(5);
        if !self.root && self.manager.needs_root() {
            words.push("sudo");
        }
        words.push(self.manager.name());
        words.extend_from_slice(self.manager.install_words());
        words.push(&self.package);
        words
    }
}

/// Whether `name` can stand as one package name on a command line: not empty, not taken for an
/// option, one word.
fn is_package_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && !name.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Whether this process runs as root, which needs no `sudo`.
fn running_as_root() -> bool {
    #[cfg(unix)]
    {
        rustix::process::geteuid().is_root()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{Install, Manager};
    use crate::runtime::{App, Command, HandoffOutcome, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    /// A machine that has exactly `programs`, in `/usr/bin`.
    fn machine(programs: &'static [&'static str]) -> impl Fn(&str) -> Option<PathBuf> {
        move |program| programs.contains(&program).then(|| PathBuf::from("/usr/bin").join(program))
    }

    fn words(line: &str) -> Vec<OsString> {
        line.split(' ').map(OsString::from).collect()
    }

    #[test]
    fn without_a_package_manager_there_is_nothing_to_offer() {
        assert_eq!(Install::package_with("libarchive", machine(&["ls", "paru", "yay"]), false), None);
    }

    #[test]
    fn pacman_installs_with_sudo_and_only_what_is_missing() {
        let install = Install::package_with("libarchive", machine(&["pacman"]), false).expect("pacman");
        assert_eq!(install.manager(), Manager::Pacman);
        assert_eq!(install.command_line(), "sudo pacman -S --needed libarchive");
        assert_eq!(install.program(), words("sudo pacman -S --needed libarchive"));
    }

    #[test]
    fn as_root_the_command_has_no_sudo() {
        let install = Install::package_with("libarchive", machine(&["pacman"]), true).expect("pacman");
        assert_eq!(install.command_line(), "pacman -S --needed libarchive");
        assert_eq!(install.program()[0], OsString::from("pacman"));
    }

    #[test]
    fn apt_get_installs_and_asks_its_own_question() {
        let install = Install::package_with("bsdtar", machine(&["apt-get"]), false).expect("apt-get");
        assert_eq!(install.command_line(), "sudo apt-get install bsdtar");
    }

    #[test]
    fn every_manager_has_its_own_command() {
        let cases: [(&'static [&'static str], &str); 4] = [
            (&["dnf"], "sudo dnf install bsdtar"),
            (&["zypper"], "sudo zypper install bsdtar"),
            (&["apk"], "sudo apk add bsdtar"),
            (&["brew"], "brew install bsdtar"),
        ];
        for (programs, line) in cases {
            let install = Install::package_with("bsdtar", machine(programs), false).expect("a manager");
            assert_eq!(install.command_line(), line);
        }
    }

    #[test]
    fn the_first_manager_in_the_order_wins() {
        let install = Install::package_with("bsdtar", machine(&["brew", "dnf", "apt-get"]), false).expect("a manager");
        assert_eq!(install.manager(), Manager::Apt, "apt-get comes before dnf and brew");
    }

    #[test]
    fn homebrew_is_passed_over_as_root() {
        assert_eq!(Install::package_with("bsdtar", machine(&["brew"]), true), None);
        let install = Install::package_with("bsdtar", machine(&["brew"]), false).expect("brew");
        assert_eq!(install.manager(), Manager::Brew);
    }

    #[test]
    fn a_distribution_specific_name_is_used_on_its_manager_only() {
        let named = |programs| {
            Install::package_with("libarchive", machine(programs), false)
                .expect("a manager")
                .name_for(Manager::Apt, "libarchive-tools")
                .name_for(Manager::Dnf, "bsdtar")
        };
        assert_eq!(named(&["apt-get"]).command_line(), "sudo apt-get install libarchive-tools");
        assert_eq!(named(&["dnf"]).name(), "bsdtar");
        assert_eq!(named(&["pacman"]).command_line(), "sudo pacman -S --needed libarchive");
    }

    #[test]
    fn a_name_that_is_not_one_package_is_refused() {
        for name in ["", "-Syu", "libarchive; rm", "a\tb", "a\nb"] {
            assert_eq!(Install::package_with(name, machine(&["pacman"]), false), None, "{name:?}");
        }
        let kept = Install::package_with("libarchive", machine(&["apt-get"]), false)
            .expect("apt-get")
            .name_for(Manager::Apt, "--force");
        assert_eq!(kept.name(), "libarchive", "an option is never taken for a name");
    }

    /// A program that misses `bsdtar` and offers to install it.
    struct Missing {
        install: Install,
        outcomes: Vec<HandoffOutcome>,
    }

    #[derive(Clone)]
    enum Msg {
        Ask,
        Install,
        Installed(HandoffOutcome),
    }

    impl App for Missing {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Ask => Command::confirm(self.install.confirm(Msg::Install)),
                Msg::Install => Command::handoff(self.install.handoff(Msg::Installed)),
                Msg::Installed(outcome) => {
                    self.outcomes.push(outcome);
                    Command::none()
                }
            }
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.add(Text::new("bsdtar is missing"));
                ui.add(Button::new("Install").on_press(Msg::Ask)).id("install");
            });
        }
    }

    fn missing() -> Harness<Missing> {
        let install = Install::package_with("libarchive", machine(&["pacman"]), false).expect("pacman");
        Harness::new(Missing { install, outcomes: Vec::new() }, 72, 16)
    }

    /// Clicks the application's Install button and lets the question open.
    fn ask(h: &mut Harness<Missing>) {
        h.click_text("Install").advance(Duration::from_millis(200));
    }

    #[test]
    fn the_question_shows_the_command_and_nothing_runs_before_the_answer() {
        let mut h = missing();
        ask(&mut h);
        let screen = h.screen();
        assert!(screen.contains("Install libarchive?"), "{screen}");
        assert!(screen.contains("sudo pacman -S --needed libarchive"), "the exact command is shown: {screen}");
        assert!(screen.contains("pacman may ask for your password"), "{screen}");
        assert!(h.handoffs().is_empty(), "nothing is handed over before the person answers");
    }

    #[test]
    fn escape_installs_nothing() {
        let mut h = missing();
        ask(&mut h);
        h.press("esc");
        assert!(!h.screen().contains("Install libarchive?"), "{}", h.screen());
        assert!(h.handoffs().is_empty());
        assert!(h.app().outcomes.is_empty());
    }

    #[test]
    fn confirming_hands_the_terminal_to_the_package_manager_once() {
        let mut h = missing();
        ask(&mut h);
        // The dialog's own Install button sits right of Cancel on the answer row.
        let screen = h.screen();
        let (cancel_x, row) = h.find("Cancel").unwrap_or_else(|| panic!("an answer row: {screen}"));
        let line = screen.lines().nth(usize::try_from(row).unwrap_or(0)).unwrap_or_default();
        let after_cancel = line.chars().skip(usize::try_from(cancel_x).unwrap_or(0)).collect::<String>();
        let offset = after_cancel.find("Install").unwrap_or_else(|| panic!("Install beside Cancel: {line}"));
        let x = cancel_x + i32::try_from(after_cancel[..offset].chars().count()).unwrap_or(0);
        h.click(x + 1, row);
        let asked = h.handoffs();
        assert_eq!(asked.len(), 1, "exactly one handoff");
        assert_eq!(asked[0].program, OsString::from("sudo"));
        assert_eq!(asked[0].args, words("pacman -S --needed libarchive"));
        assert!(asked[0].pause, "the manager's last lines stay until a key is pressed");
        assert_eq!(asked[0].notice.as_deref(), Some("Installing libarchive with pacman…"));
        assert_eq!(h.app().outcomes, [HandoffOutcome::Finished { code: Some(0) }], "the outcome comes back");
    }
}
