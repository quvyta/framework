//! The clipboard: notifications for applications and reading what the user copied.
//!
//! Reading tries three sources in order and takes the first that has text:
//!
//! 1. **The system clipboard tool**: `wl-paste` on Wayland, `xclip` or `xsel` on X11, `pbpaste`
//!    on macOS. Each runs without a shell and is stopped after a short timeout; on a runtime it
//!    runs on its own thread, so drawing never waits for it.
//! 2. **The terminal**, asked with an OSC 52 clipboard query. Many terminals refuse or ignore it,
//!    so the runtime waits only briefly for the answer.
//! 3. **The text this application copied last.**

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// Something that happened on the clipboard, delivered to
/// [`App::clipboard`](crate::runtime::App::clipboard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardEvent {
    /// A widget or a mouse selection copied this text.
    Copied(String),
    /// This text was pasted (from the terminal, with the `paste` key or from a Paste menu entry)
    /// and no focused widget took it.
    Pasted(String),
}

/// How long a clipboard tool may run before it is stopped.
pub(crate) const TOOL_TIMEOUT: Duration = Duration::from_millis(500);

/// How often a running tool is checked for having finished.
const TOOL_POLL: Duration = Duration::from_millis(5);

/// A clipboard program and its arguments, run without a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tool {
    program: String,
    args: Vec<String>,
}

impl Tool {
    pub(crate) fn new(program: &str, args: &[&str]) -> Self {
        Self { program: program.to_owned(), args: args.iter().map(|arg| (*arg).to_owned()).collect() }
    }

    /// Runs the tool and returns what it printed, when it succeeds within `timeout` with text.
    /// A tool that is missing, fails, prints nothing or something that is not UTF-8, or runs too
    /// long gives `None`; a tool that runs too long is killed.
    pub(crate) fn read(&self, timeout: Duration) -> Option<String> {
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        // Read while waiting: a large clipboard would otherwise fill the pipe and stall the tool.
        let mut stdout = child.stdout.take()?;
        // A system that cannot start another thread cannot run the tool safely either.
        let Ok(output) = std::thread::Builder::new().name("quvyta-clipboard-pipe".to_owned()).spawn(move || {
            let mut output = Vec::new();
            stdout.read_to_end(&mut output).map(|_| output)
        }) else {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        };
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() < timeout => std::thread::sleep(TOOL_POLL),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        };
        let text = String::from_utf8(output.join().ok()?.ok()?).ok()?;
        (status.success() && !text.is_empty()).then_some(text)
    }
}

/// The clipboard tools to try on this system, in order. `var` tells whether an environment
/// variable is set.
pub(crate) fn platform_tools(var: impl Fn(&str) -> bool) -> Vec<Tool> {
    if cfg!(target_os = "macos") {
        return vec![Tool::new("pbpaste", &[])];
    }
    let mut tools = Vec::new();
    if var("WAYLAND_DISPLAY") {
        tools.push(Tool::new("wl-paste", &["--no-newline", "--type", "text"]));
    }
    if var("DISPLAY") {
        tools.push(Tool::new("xclip", &["-o", "-selection", "clipboard"]));
        tools.push(Tool::new("xsel", &["--clipboard", "--output"]));
    }
    tools
}

/// Where the first source, the system clipboard, comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SystemClipboard {
    /// The platform's clipboard programs, tried in order.
    Tools(Vec<Tool>),
    /// A fixed answer: the test harness never touches the real clipboard.
    Fixed(Option<String>),
}

/// What reading the clipboard needs next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReadStep {
    /// The system tool is still running on its thread.
    Waiting,
    /// The system clipboard gave nothing: ask the terminal with OSC 52 and pass its answer to
    /// [`ClipboardReader::terminal_answer`].
    AskTerminal,
    /// Reading ended; `None` when neither the system nor the terminal had text, so the text the
    /// application copied last is used.
    Done(Option<String>),
}

/// Reads the clipboard from the system tool, then the terminal; see the module docs.
pub(crate) struct ClipboardReader {
    system: SystemClipboard,
    terminal: bool,
    /// Whether the tools run on a thread (a runtime) or inline.
    threaded: bool,
    state: ReadState,
}

enum ReadState {
    Idle,
    System(Receiver<Option<String>>),
    Terminal,
}

impl ClipboardReader {
    /// A reader for a runtime: the platform's tools on a thread, then the terminal.
    pub(crate) fn runtime() -> Self {
        let tools = platform_tools(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()));
        Self { system: SystemClipboard::Tools(tools), terminal: true, threaded: true, state: ReadState::Idle }
    }

    /// A reader for tests: a fixed system clipboard that starts empty and no terminal.
    pub(crate) fn fixed() -> Self {
        Self { system: SystemClipboard::Fixed(None), terminal: false, threaded: false, state: ReadState::Idle }
    }

    /// Replaces the system clipboard source.
    pub(crate) fn set_system(&mut self, system: SystemClipboard) {
        self.system = system;
    }

    /// Whether a read is under way.
    pub(crate) fn is_reading(&self) -> bool {
        !matches!(self.state, ReadState::Idle)
    }

    /// Starts reading. Call only while no read is under way.
    pub(crate) fn start(&mut self) -> ReadStep {
        match &self.system {
            SystemClipboard::Fixed(text) => self.after_system(text.clone()),
            SystemClipboard::Tools(tools) if self.threaded => {
                let tools = tools.clone();
                let (sender, receiver) = mpsc::channel();
                let spawned = std::thread::Builder::new().name("quvyta-clipboard".to_owned()).spawn(move || {
                    let _ = sender.send(tools.iter().find_map(|tool| tool.read(TOOL_TIMEOUT)));
                });
                if spawned.is_err() {
                    // No thread to wait on: skip the system tools and go on to the next source.
                    return self.after_system(None);
                }
                self.state = ReadState::System(receiver);
                ReadStep::Waiting
            }
            SystemClipboard::Tools(tools) => {
                let text = tools.iter().find_map(|tool| tool.read(TOOL_TIMEOUT));
                self.after_system(text)
            }
        }
    }

    /// Checks on a system tool running on its thread.
    pub(crate) fn poll(&mut self) -> ReadStep {
        let ReadState::System(receiver) = &self.state else {
            return ReadStep::Waiting;
        };
        match receiver.try_recv() {
            Ok(text) => self.after_system(text),
            Err(TryRecvError::Empty) => ReadStep::Waiting,
            Err(TryRecvError::Disconnected) => self.after_system(None),
        }
    }

    /// The terminal's answer to the OSC 52 query, `None` when it did not answer in time.
    pub(crate) fn terminal_answer(&mut self, text: Option<String>) -> ReadStep {
        if !matches!(self.state, ReadState::Terminal) {
            return ReadStep::Waiting;
        }
        self.state = ReadState::Idle;
        ReadStep::Done(text.filter(|text| !text.is_empty()))
    }

    fn after_system(&mut self, text: Option<String>) -> ReadStep {
        if text.is_some() {
            self.state = ReadState::Idle;
            return ReadStep::Done(text);
        }
        if self.terminal {
            self.state = ReadState::Terminal;
            ReadStep::AskTerminal
        } else {
            self.state = ReadState::Idle;
            ReadStep::Done(None)
        }
    }
}

/// The OSC 52 query asking the terminal for its clipboard.
pub(crate) const OSC52_QUERY: &str = "\x1b]52;c;?\x07";

/// Decodes standard base64, ignoring anything outside the alphabet such as line breaks. Returns
/// `None` for text that is not valid UTF-8 once decoded.
pub(crate) fn decode_base64(encoded: &str) -> Option<String> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut bytes = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for sextet in encoded.bytes().take_while(|c| *c != b'=').filter_map(value) {
        buffer = (buffer << 6) | u32::from(sextet);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push(u8::try_from((buffer >> bits) & 0xff).unwrap_or(0));
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fails() -> Tool {
        Tool::new("quvyta-no-such-clipboard-tool", &[])
    }

    #[test]
    fn tools_run_without_a_shell_and_fail_quietly() {
        assert_eq!(Tool::new("printf", &["%s", "deploy $HOME"]).read(TOOL_TIMEOUT), Some("deploy $HOME".to_owned()));
        assert_eq!(fails().read(TOOL_TIMEOUT), None, "a missing program");
        assert_eq!(Tool::new("false", &[]).read(TOOL_TIMEOUT), None, "a failing program");
        assert_eq!(Tool::new("printf", &[""]).read(TOOL_TIMEOUT), None, "an empty clipboard");
        let started = Instant::now();
        assert_eq!(Tool::new("sleep", &["5"]).read(Duration::from_millis(50)), None, "too slow");
        assert!(started.elapsed() < Duration::from_secs(2), "the slow tool was stopped");
    }

    #[test]
    fn platform_tools_follow_the_display_server() {
        if cfg!(target_os = "macos") {
            return;
        }
        let names = |vars: &[&str]| {
            platform_tools(|name| vars.contains(&name)).into_iter().map(|tool| tool.program).collect::<Vec<_>>()
        };
        assert_eq!(names(&["WAYLAND_DISPLAY"]), ["wl-paste"]);
        assert_eq!(names(&["DISPLAY"]), ["xclip", "xsel"]);
        assert_eq!(names(&["WAYLAND_DISPLAY", "DISPLAY"]), ["wl-paste", "xclip", "xsel"]);
        assert!(names(&[]).is_empty(), "over SSH without a display there is no tool");
    }

    #[test]
    fn sources_are_tried_in_order() {
        let reader = |tools: Vec<Tool>, terminal: bool| ClipboardReader {
            system: SystemClipboard::Tools(tools),
            terminal,
            threaded: false,
            state: ReadState::Idle,
        };
        let mut system = reader(vec![fails(), Tool::new("printf", &["from wl-paste"])], true);
        assert_eq!(system.start(), ReadStep::Done(Some("from wl-paste".into())), "the first tool with text wins");
        assert!(!system.is_reading());

        let mut terminal = reader(vec![fails()], true);
        assert_eq!(terminal.start(), ReadStep::AskTerminal, "no tool had text: ask the terminal");
        assert!(terminal.is_reading());
        assert_eq!(terminal.terminal_answer(Some("from OSC 52".into())), ReadStep::Done(Some("from OSC 52".into())));

        let mut silent = reader(vec![fails()], true);
        silent.start();
        assert_eq!(silent.terminal_answer(None), ReadStep::Done(None), "then the application's own copy");
        assert_eq!(silent.terminal_answer(Some("late".into())), ReadStep::Waiting, "a late answer is ignored");

        let mut without_terminal = reader(Vec::new(), false);
        assert_eq!(without_terminal.start(), ReadStep::Done(None));
    }

    #[test]
    fn a_threaded_tool_is_polled_until_it_answers() {
        let mut reader = ClipboardReader {
            system: SystemClipboard::Tools(vec![Tool::new("printf", &["threaded"])]),
            terminal: false,
            threaded: true,
            state: ReadState::Idle,
        };
        assert_eq!(reader.start(), ReadStep::Waiting);
        let started = Instant::now();
        let step = loop {
            match reader.poll() {
                ReadStep::Waiting if started.elapsed() < Duration::from_secs(5) => std::thread::yield_now(),
                step => break step,
            }
        };
        assert_eq!(step, ReadStep::Done(Some("threaded".into())));
    }

    #[derive(Default)]
    struct Reader {
        read: Vec<Option<String>>,
        pasted: Vec<String>,
    }

    enum Msg {
        Read,
        Got(Option<String>),
        Copy,
        Pasted(String),
    }

    impl crate::runtime::App for Reader {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> crate::runtime::Command<Msg> {
            match msg {
                Msg::Read => return crate::runtime::Command::read_clipboard(Msg::Got),
                Msg::Got(text) => self.read.push(text),
                Msg::Copy => return crate::runtime::Command::copy("inside the app"),
                Msg::Pasted(text) => self.pasted.push(text),
            }
            crate::runtime::Command::none()
        }
        fn view(&self, _ui: &mut crate::widget::View<'_, Msg>) {}
        fn clipboard(&self, event: &ClipboardEvent) -> Option<Msg> {
            match event {
                ClipboardEvent::Pasted(text) => Some(Msg::Pasted(text.clone())),
                ClipboardEvent::Copied(_) => None,
            }
        }
    }

    #[test]
    fn read_clipboard_and_the_paste_key_share_the_order_of_sources() {
        let mut h = crate::runtime::Harness::new(Reader::default(), 20, 2);
        h.send(Msg::Read).press("ctrl+v");
        assert_eq!((h.app().read.clone(), h.app().pasted.len()), (vec![None], 0), "nothing anywhere");
        h.send(Msg::Copy).send(Msg::Read).press("ctrl+v");
        assert_eq!(h.app().read.last(), Some(&Some("inside the app".to_owned())), "the last copy inside");
        assert_eq!(h.app().pasted, ["inside the app"]);
        h.set_system_clipboard(Some("from another program")).send(Msg::Read).press("ctrl+v");
        assert_eq!(h.app().read.last(), Some(&Some("from another program".to_owned())), "the system first");
        assert_eq!(h.app().pasted.last().map(String::as_str), Some("from another program"));
    }

    #[test]
    fn decodes_base64_answers() {
        assert_eq!(decode_base64("ZGVwbG95LWFwaQ=="), Some("deploy-api".into()));
        assert_eq!(decode_base64("w6dheQ=="), Some("çay".into()));
        assert_eq!(decode_base64(""), Some(String::new()));
        assert_eq!(decode_base64("//79"), None, "not UTF-8");
    }
}
