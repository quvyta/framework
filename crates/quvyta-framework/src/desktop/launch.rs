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
    /// In a [`Harness`](crate::runtime::Harness) nothing runs: the handoff or the opening is
    /// recorded, and a test reads it from `handoffs()` or `opens()`.
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
        let mut words = self.command(file).ok_or(LaunchError::NoCommand)?.into_iter();
        let program = words.next().ok_or(LaunchError::NoCommand)?;
        if self.terminal {
            let handoff = Handoff::new(program, move |outcome| {
                on_done(match outcome {
                    HandoffOutcome::Finished { code } => Launched::Returned { code },
                    HandoffOutcome::Failed(reason) => Launched::Failed(reason),
                })
            });
            return Ok(Command::handoff(handoff.args(words)));
        }
        let open = Open::program(program).args(words).answer(move |outcome| {
            on_done(match outcome {
                OpenOutcome::Opened => Launched::Started,
                OpenOutcome::Failed(reason) => Launched::Failed(reason),
            })
        });
        Ok(Command::open_with(open))
    }
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
