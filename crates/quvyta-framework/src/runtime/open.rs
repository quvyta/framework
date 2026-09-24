//! Opening an address, a file or a program on the person's own desktop, without leaving the screen.
//!
//! A [`Handoff`](super::Handoff) gives the terminal away and draws everything again afterwards,
//! which is right when the user is about to talk to the program. Handing a link to the browser on
//! the person's own screen is not that: none of it happens in the terminal, so stepping aside for
//! it only makes the screen blink. An [`Open`] starts the program with its standard streams
//! thrown away and, on Unix, a process group of its own, so the keys' signals never reach it and
//! it lives on after the application. The screen is never touched.

use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command as ChildCommand, Stdio};

/// Starts a program beside the application: the desktop's own opener for an address or a path, or
/// a program named outright. The screen stays where it is and nothing is drawn again.
///
/// The answer is optional. With [`Open::answer`] a message says whether the program was started;
/// whether the desktop then really showed the thing is not something a terminal can know, so
/// [`OpenOutcome::Opened`] says the opener was handed the target and no more.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::runtime::{Open, OpenOutcome};
///
/// enum Msg {
///     SignIn(String),
///     Opened(OpenOutcome),
/// }
///
/// fn update(msg: Msg) -> Command<Msg> {
///     match msg {
///         // The address goes to whatever browser this person uses; the screen never blinks.
///         Msg::SignIn(address) => Command::open_with(Open::new(address).answer(Msg::Opened)),
///         Msg::Opened(_) => Command::none(),
///     }
/// }
/// ```
pub struct Open<Msg> {
    program: OsString,
    args: Vec<OsString>,
    target: Option<OsString>,
    dir: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    on_open: Option<Box<dyn FnOnce(OpenOutcome) -> Msg + Send>>,
}

impl<Msg: Send + 'static> Open<Msg> {
    /// Opens `target` — an address, a file or a folder — with the opener of this desktop:
    /// `xdg-open`, `open` on macOS, `start` on Windows.
    #[must_use]
    pub fn new(target: impl Into<OsString>) -> Self {
        let target = target.into();
        let (program, args) = opener(target.clone());
        Self { program, args, target: Some(target), dir: None, env: Vec::new(), on_open: None }
    }

    /// Starts `program` itself instead of the desktop's opener, the same way: quietly, beside the
    /// application, with nothing drawn again.
    #[must_use]
    pub fn program(program: impl Into<OsString>) -> Self {
        Self { program: program.into(), args: Vec::new(), target: None, dir: None, env: Vec::new(), on_open: None }
    }

    /// Adds one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds several arguments, in order.
    #[must_use]
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Starts the program in `dir` instead of the application's working directory.
    #[must_use]
    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = Some(dir.into());
        self
    }

    /// Sets an environment variable for the program. The rest of the environment is inherited.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Delivers `on_open(outcome)` once the program has been started, or could not be.
    ///
    /// Without it nothing is delivered: an application that has nothing to say about the opening
    /// asks for no message.
    #[must_use]
    pub fn answer(mut self, on_open: impl FnOnce(OpenOutcome) -> Msg + Send + 'static) -> Self {
        self.on_open = Some(Box::new(on_open));
        self
    }

    /// What a test sees of this opening.
    pub(crate) fn request(&self) -> OpenRequest {
        OpenRequest {
            program: self.program.clone(),
            args: self.args.clone(),
            target: self.target.clone(),
            dir: self.dir.clone(),
        }
    }

    /// The message of `outcome`, for a harness that never starts the program.
    pub(crate) fn finish(self, outcome: OpenOutcome) -> Option<Msg> {
        self.on_open.map(|on_open| on_open(outcome))
    }

    /// Starts the program and returns the message of what came of it, with the child itself when
    /// one was started.
    ///
    /// The caller is what waits for that child, and only after the message has been delivered:
    /// an opener may live as long as the window it opened, and nothing waits for that.
    pub(crate) fn start(self) -> (Option<Msg>, Option<Child>) {
        let mut command = ChildCommand::new(&self.program);
        command.args(&self.args);
        if let Some(dir) = &self.dir {
            command.current_dir(dir);
        }
        for (key, value) in &self.env {
            command.env(key, value);
        }
        // Nothing of this program belongs on the screen the application is drawing on.
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        // A group of its own: the keys' signals go to the application's group, never to this.
        #[cfg(unix)]
        command.process_group(0);
        match command.spawn() {
            Ok(child) => (self.finish(OpenOutcome::Opened), Some(child)),
            Err(error) => (self.finish(OpenOutcome::Failed(error.to_string())), None),
        }
    }

    /// The same opening delivering `map(message)` wherever it would deliver `message`.
    pub(crate) fn map<B: Send + 'static>(self, map: impl FnOnce(Msg) -> B + Send + 'static) -> Open<B> {
        let on_open = self.on_open;
        Open {
            program: self.program,
            args: self.args,
            target: self.target,
            dir: self.dir,
            env: self.env,
            on_open: on_open.map(|on_open| -> Box<dyn FnOnce(OpenOutcome) -> B + Send> {
                Box::new(move |outcome| map(on_open(outcome)))
            }),
        }
    }
}

/// What came of an opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    /// The program was started with what it was given. What the desktop did with it afterwards is
    /// out of reach from a terminal, so this is as far as the answer goes.
    Opened,
    /// The program could not be started at all: this desktop has no opener installed, or the
    /// program was not found.
    Failed(String),
}

/// An opening a [`Harness`](super::Harness) recorded instead of carrying out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequest {
    /// The program asked for: the desktop's opener, or the one [`Open::program`] named.
    pub program: OsString,
    /// Its arguments, in order. For an opener the target is the only one.
    pub args: Vec<OsString>,
    /// What [`Open::new`] was given, so a test can read the address or the path without knowing
    /// which opener this system has. `None` after [`Open::program`].
    pub target: Option<OsString>,
    /// The working folder [`Open::dir`] gave; `None` when the program starts in the application's
    /// own.
    pub dir: Option<PathBuf>,
}

/// The opener of this desktop and the arguments that give it `target`.
#[cfg(target_os = "macos")]
fn opener(target: OsString) -> (OsString, Vec<OsString>) {
    (OsString::from("open"), vec![target])
}

/// The opener of this desktop and the arguments that give it `target`.
#[cfg(all(unix, not(target_os = "macos")))]
fn opener(target: OsString) -> (OsString, Vec<OsString>) {
    (OsString::from("xdg-open"), vec![target])
}

/// The opener of this desktop and the arguments that give it `target`. The empty argument is the
/// window title `start` would otherwise read the target as.
#[cfg(windows)]
fn opener(target: OsString) -> (OsString, Vec<OsString>) {
    (OsString::from("cmd"), vec![OsString::from("/C"), OsString::from("start"), OsString::new(), target])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_goes_to_the_opener_of_this_desktop_with_the_target_kept_for_the_test() {
        let request = Open::new("https://example.com/sign-in").answer(|outcome| outcome).request();
        assert_eq!(request.target.as_deref(), Some(std::ffi::OsStr::new("https://example.com/sign-in")));
        assert_eq!(request.args, [OsString::from("https://example.com/sign-in")]);
        assert!(!request.program.is_empty(), "the desktop's opener is named: {:?}", request.program);
    }

    #[test]
    fn a_program_of_its_own_carries_its_arguments_and_no_target() {
        let request = Open::<OpenOutcome>::program("gimp").arg("--new-instance").args(["a.png", "b.png"]).request();
        assert_eq!(request.program, OsString::from("gimp"));
        assert_eq!(request.args, ["--new-instance", "a.png", "b.png"].map(OsString::from));
        assert_eq!(request.target, None, "nothing was handed to an opener");
        assert_eq!(request.dir, None, "without `dir` the program starts where the application runs");
        let placed = Open::<OpenOutcome>::program("gimp").dir("/srv/pictures").request();
        assert_eq!(placed.dir, Some(PathBuf::from("/srv/pictures")), "the folder is recorded");
    }

    #[test]
    fn a_program_that_starts_answers_opened_and_leaves_a_child_to_wait_for() {
        let (message, child) = Open::program("sh").args(["-c", "exit 0"]).answer(|outcome| outcome).start();
        assert_eq!(message, Some(OpenOutcome::Opened));
        let mut child = child.expect("a program that started has a child");
        assert!(child.wait().is_ok(), "the caller is what reaps it");
    }

    #[test]
    fn a_program_that_is_not_there_fails_with_the_reason_and_leaves_no_child() {
        let (message, child) = Open::program("quvyta-no-such-program").answer(|outcome| outcome).start();
        let Some(OpenOutcome::Failed(reason)) = message else {
            panic!("a program that is not there cannot have been opened: {message:?}");
        };
        assert!(!reason.is_empty(), "the reason names what went wrong");
        assert!(child.is_none(), "nothing was started");
    }

    #[test]
    fn the_directory_and_the_environment_reach_the_program() {
        let path = std::env::temp_dir().join(format!(
            "quvyta-open-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        let (message, child) = Open::program("sh")
            .args(["-c", r#"test "$QUVYTA_OPEN_TEST" = ok && pwd > "$0""#, &path.to_string_lossy()])
            .dir("/")
            .env("QUVYTA_OPEN_TEST", "ok")
            .answer(|outcome| outcome)
            .start();
        assert_eq!(message, Some(OpenOutcome::Opened));
        let status = child.expect("a child").wait().expect("it ends");
        assert_eq!(status.code(), Some(0), "the environment reached it");
        assert_eq!(std::fs::read_to_string(&path).unwrap_or_default().trim(), "/", "it ran in the directory");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_opening_without_an_answer_delivers_nothing() {
        let (message, child) = Open::<OpenOutcome>::program("sh").args(["-c", "exit 0"]).start();
        assert!(message.is_none(), "no message was asked for");
        let _ = child.expect("a child").wait();
    }

    #[test]
    fn a_mapped_opening_delivers_the_converted_message() {
        let open = Open::new("https://example.com").answer(|outcome| outcome);
        let mapped = open.map(|outcome| format!("{outcome:?}"));
        assert_eq!(mapped.finish(OpenOutcome::Opened), Some("Opened".to_owned()));
    }
}
