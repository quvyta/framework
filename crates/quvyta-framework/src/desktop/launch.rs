//! Starting a program on a file, the right way for its kind: a terminal program is handed the
//! terminal, a graphical one starts beside the application.

use std::fmt;
use std::path::Path;

use super::apps::DesktopApp;
use crate::runtime::{Command, Handoff, HandoffOutcome, Open, OpenOutcome};

/// Whether there is a graphical session for a program with windows to open on: `DISPLAY` or
/// `WAYLAND_DISPLAY` is set and not empty. Over SSH, in a console or in a desktop drawn inside the
/// terminal there is none.
///
/// `lookup` returns a variable's value, as for [`XdgDirs::from_env`](super::XdgDirs::from_env), so
/// a test decides the answer instead of the machine it runs on.
pub fn graphical_session(lookup: impl Fn(&str) -> Option<String>) -> bool {
    ["DISPLAY", "WAYLAND_DISPLAY"].iter().any(|name| lookup(name).is_some_and(|value| !value.is_empty()))
}

/// How starting a program through [`DesktopApp::launch`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launched {
    /// A terminal program ran and the application has the screen back; `code` is `None` when a
    /// signal ended it.
    Returned {
        /// The exit code, or `None` after a signal.
        code: Option<i32>,
    },
    /// A graphical program was started beside the application. Whether it then showed the file
    /// is out of a terminal's reach.
    Started,
    /// The program could not be started; the reason is the system's.
    Failed(String),
}

/// Why [`DesktopApp::launch`] did not even try.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    /// The program has windows of its own and there is no graphical session to open them on.
    NoGraphicalSession,
    /// The `Exec` line gives no command. [`Apps::load`](super::Apps::load) never offers such a
    /// program; only a [`DesktopApp`] built by hand can have one.
    NoCommand,
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoGraphicalSession => "there is no graphical session to open this program on",
            Self::NoCommand => "the program's Exec line gives no command",
        })
    }
}

impl std::error::Error for LaunchError {}

impl DesktopApp {
    /// Whether this program can start here: a terminal program always can, a graphical one only
    /// in a graphical session (see [`graphical_session`]). A menu shows the others dimmed.
    #[must_use]
    pub fn can_start(&self, graphical: bool) -> bool {
        self.terminal || graphical
    }

    /// The command that opens `file` with this program, delivering `on_done` with how it ended.
    ///
    /// A terminal program (`Terminal=true`) goes through a [`Handoff`]: the application steps
    /// aside, the program owns the terminal, and the screen comes back when it ends. A graphical
    /// one goes through [`Open::program`] and starts quietly beside the application. `graphical`
    /// says whether there is a graphical session; without one a graphical program is refused
    /// instead of tried. The arguments are those of [`DesktopApp::command`], so no shell is
    /// involved.
    ///
    /// Either way the program runs in the file's folder, as desktop file managers start it:
    /// relative paths, "Save as" and a shell opened from the program begin beside the file, not
    /// wherever the application happened to be started. A file given without a folder leaves the
    /// application's own.
    ///
    /// In a [`Harness`](crate::runtime::Harness) nothing runs: the handoff or the opening is
    /// recorded, and a test reads it, folder included, from `handoffs()` or `opens()`.
    ///
    /// # Errors
    ///
    /// [`LaunchError::NoGraphicalSession`] for a graphical program without a graphical session,
    /// [`LaunchError::NoCommand`] when the `Exec` line gives no command.
    pub fn launch<Msg: Send + 'static>(
        &self,
        file: &Path,
        graphical: bool,
        on_done: impl FnOnce(Launched) -> Msg + Send + 'static,
    ) -> Result<Command<Msg>, LaunchError> {
        if !self.can_start(graphical) {
            return Err(LaunchError::NoGraphicalSession);
        }
        Ok(match self.start(file, on_done)? {
            Start::Terminal(handoff) => Command::handoff(handoff),
            Start::Beside(open) => Command::open_with(open),
        })
    }

    /// The handoff or the opening that starts this program on `file` in the file's folder.
    fn start<Msg: Send + 'static>(
        &self,
        file: &Path,
        on_done: impl FnOnce(Launched) -> Msg + Send + 'static,
    ) -> Result<Start<Msg>, LaunchError> {
        let mut words = self.command(file).ok_or(LaunchError::NoCommand)?.into_iter();
        let program = words.next().ok_or(LaunchError::NoCommand)?;
        // `Path::parent` of a bare name is the empty path, which is no folder to start in.
        let folder = file.parent().filter(|folder| !folder.as_os_str().is_empty());
        if self.terminal {
            let mut handoff = Handoff::new(program, move |outcome| {
                on_done(match outcome {
                    HandoffOutcome::Finished { code } => Launched::Returned { code },
                    HandoffOutcome::Failed(reason) => Launched::Failed(reason),
                })
            })
            .args(words);
            if let Some(folder) = folder {
                handoff = handoff.dir(folder);
            }
            return Ok(Start::Terminal(handoff));
        }
        let mut open = Open::program(program).args(words).answer(move |outcome| {
            on_done(match outcome {
                OpenOutcome::Opened => Launched::Started,
                OpenOutcome::Failed(reason) => Launched::Failed(reason),
            })
        });
        if let Some(folder) = folder {
            open = open.dir(folder);
        }
        Ok(Start::Beside(open))
    }
}

/// How [`DesktopApp::launch`] starts a program: handed the terminal, or beside the application.
enum Start<Msg> {
    Terminal(Handoff<Msg>),
    Beside(Open<Msg>),
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::*;
    use crate::runtime::{App, Harness};
    use crate::widget::View;

    /// An application that opens one file with one program, as a file explorer would.
    struct Opener {
        app: DesktopApp,
        graphical: bool,
        file: PathBuf,
        heard: Vec<Launched>,
        refused: Option<LaunchError>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Open,
        Done(Launched),
    }

    impl App for Opener {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Open => match self.app.launch(&self.file, self.graphical, Msg::Done) {
                    Ok(command) => command,
                    Err(error) => {
                        self.refused = Some(error);
                        Command::none()
                    }
                },
                Msg::Done(launched) => {
                    self.heard.push(launched);
                    Command::none()
                }
            }
        }
        fn view(&self, _ui: &mut View<'_, Msg>) {}
    }

    const FILE: &str = "/home/ada/my \"odd\" notes.txt";

    fn app(exec: &str, terminal: bool) -> DesktopApp {
        DesktopApp {
            id: "x.desktop".to_owned(),
            name: "X".to_owned(),
            exec: exec.to_owned(),
            terminal,
            mime_types: Vec::new(),
            path: PathBuf::from("/apps/x.desktop"),
            icon: None,
        }
    }

    fn opener(app: DesktopApp, graphical: bool) -> Harness<Opener> {
        let file = PathBuf::from(FILE);
        Harness::new(Opener { app, graphical, file, heard: Vec::new(), refused: None }, 20, 2)
    }

    #[test]
    fn a_terminal_program_is_a_recorded_handoff() {
        let mut harness = opener(app("less %f", true), false);
        harness.send(Msg::Open);
        let handoffs = harness.handoffs();
        assert_eq!(handoffs.len(), 1, "a terminal program starts without a graphical session");
        assert_eq!(handoffs[0].program, "less");
        assert_eq!(handoffs[0].args, [OsString::from(FILE)]);
        assert!(harness.opens().is_empty());
        assert_eq!(harness.app().heard, [Launched::Returned { code: Some(0) }], "the harness answers it");
    }

    #[test]
    fn a_terminal_program_runs_in_the_files_folder() {
        let mut harness = opener(app("less %f", true), false);
        harness.send(Msg::Open);
        assert_eq!(harness.handoffs()[0].dir, Some(PathBuf::from("/home/ada")), "beside the file, not the app");
    }

    #[test]
    fn a_graphical_program_runs_in_the_files_folder() {
        let mut harness = opener(app("editor %F", false), true);
        harness.send(Msg::Open);
        assert_eq!(harness.opens()[0].dir, Some(PathBuf::from("/home/ada")), "beside the file, not the app");
    }

    #[test]
    fn a_file_named_without_a_folder_leaves_the_applications_own() {
        for (terminal, graphical) in [(true, false), (false, true)] {
            let file = PathBuf::from("notes.txt");
            let app = app("editor %f", terminal);
            let mut harness = Harness::new(Opener { app, graphical, file, heard: Vec::new(), refused: None }, 20, 2);
            harness.send(Msg::Open);
            let dir = if terminal { harness.handoffs()[0].dir.clone() } else { harness.opens()[0].dir.clone() };
            assert_eq!(dir, None, "an empty folder is no folder to start in (terminal: {terminal})");
        }
    }

    /// A folder of this test's own with a file in it, removed when the test ends.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("qframe-launch-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("the temporary folder can be made");
            std::fs::write(path.join("notes.txt"), "notes\n").expect("the file can be written");
            Self(path.canonicalize().expect("the folder is there"))
        }

        /// What `pwd` wrote into the folder it ran in.
        fn heard(&self) -> String {
            std::fs::read_to_string(self.0.join("notes.txt.where")).unwrap_or_default().trim().to_owned()
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// `sh` writes the folder it runs in beside the file it was given (`$0`), wherever it runs.
    const PWD: &str = "sh -c \"pwd >\\\"\\$0.where\\\"\" %f";

    #[test]
    fn a_terminal_program_really_starts_in_the_files_folder() {
        let folder = Folder::new("terminal");
        let Ok(Start::Terminal(handoff)) = app(PWD, true).start(&folder.0.join("notes.txt"), |launched| launched)
        else {
            panic!("a terminal program is handed the terminal");
        };
        let mut release = |_: Option<&str>| Ok(());
        let mut take = || Ok(());
        let mut wait_for_key = || Ok(());
        let launched = crate::runtime::handoff::run(
            handoff,
            &mut crate::runtime::handoff::HandoffScreen {
                release: &mut release,
                take: &mut take,
                wait_for_key: &mut wait_for_key,
            },
        );
        assert_eq!(launched, Launched::Returned { code: Some(0) });
        assert_eq!(folder.heard(), folder.0.to_string_lossy(), "the program ran in the file's folder");
    }

    #[test]
    fn a_graphical_program_really_starts_in_the_files_folder() {
        let folder = Folder::new("graphical");
        let Ok(Start::Beside(open)) = app(PWD, false).start(&folder.0.join("notes.txt"), |launched| launched) else {
            panic!("a graphical program starts beside the application");
        };
        let (launched, child) = open.start();
        assert_eq!(launched, Some(Launched::Started));
        let status = child.expect("a child").wait().expect("it ends");
        assert_eq!(status.code(), Some(0));
        assert_eq!(folder.heard(), folder.0.to_string_lossy(), "the program ran in the file's folder");
    }

    #[test]
    fn a_graphical_program_is_a_recorded_opening() {
        let mut harness = opener(app("\"text editor\" --new %F", false), true);
        harness.send(Msg::Open);
        let opens = harness.opens();
        assert_eq!(opens.len(), 1);
        assert_eq!(opens[0].program, "text editor");
        assert_eq!(opens[0].args, [OsString::from("--new"), OsString::from(FILE)]);
        assert_eq!(opens[0].target, None, "a program of its own, not the desktop's opener");
        assert!(harness.handoffs().is_empty());
        assert_eq!(harness.app().heard, [Launched::Started]);
    }

    #[test]
    fn without_a_graphical_session_a_graphical_program_is_refused() {
        let mut harness = opener(app("editor %f", false), false);
        assert!(!harness.app().app.can_start(false));
        harness.send(Msg::Open);
        assert!(harness.opens().is_empty() && harness.handoffs().is_empty(), "nothing is even asked for");
        assert_eq!(harness.app().refused, Some(LaunchError::NoGraphicalSession));
    }

    #[test]
    fn a_line_that_gives_no_command_is_refused() {
        let mut harness = opener(app("editor \"unclosed %f", true), true);
        harness.send(Msg::Open);
        assert!(harness.handoffs().is_empty());
        assert_eq!(harness.app().refused, Some(LaunchError::NoCommand));
    }

    #[test]
    fn the_session_comes_from_either_display_variable() {
        let with = |pairs: &'static [(&str, &str)]| {
            graphical_session(|name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned()))
        };
        assert!(with(&[("DISPLAY", ":0")]));
        assert!(with(&[("WAYLAND_DISPLAY", "wayland-0")]));
        assert!(!with(&[("DISPLAY", ""), ("WAYLAND_DISPLAY", "")]), "empty is none");
        assert!(!with(&[]));
    }
}
