//! Real signals reaching a real run. Each test starts this test binary again as a new session on
//! a pseudo-terminal, where [`inside_a_terminal`] runs an application with the terminal
//! [`Runtime`], and sends the signals from outside: `kill` for `SIGTERM` and a `SIGHUP` by hand,
//! closing the terminal for a real hangup. What the application heard and saved comes back as
//! files; what the run left the terminal in is read from the terminal itself.

use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Child, Command as Process, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::process::{Pid, Signal, kill_process};
use rustix::termios::{LocalModes, Winsize, tcgetattr, tcsetwinsize};

use super::signals::Signals;
use crate::geometry::Size;
use crate::runtime::{App, Command, Handoff, HandoffOutcome, Runtime, Termination};
use crate::widget::View;
use crate::widgets::Text;

/// Where the run inside the pseudo-terminal leaves its files.
const DIR_VAR: &str = "QUVYTA_SIGNAL_TEST_DIR";
/// How the application inside answers: `save`, `stubborn`, `stuck`, `handoff` or `wake`.
const MODE_VAR: &str = "QUVYTA_SIGNAL_TEST_MODE";

/// How long the machine may take for anything here; generous, because it may be loaded.
const PATIENCE: Duration = Duration::from_secs(60);

/// The handed-off program: it waits until its group owns the terminal, says it is ready and
/// runs until a `SIGTERM` ends it with 43.
const PROGRAM: &str = r#"
while set -- $(cat /proc/$$/stat); [ "$5" != "$8" ]; do sleep 0.01; done
trap 'echo terminated > "$D/program"; exit 43' TERM
touch "$D/ready"
while :; do sleep 0.05; done
"#;

/// The application inside the terminal. It writes down what it hears, since the test outside
/// cannot see its state.
struct Saver {
    dir: PathBuf,
    mode: String,
}

#[derive(Clone)]
enum Msg {
    /// Saves and quits.
    Save(Termination),
    /// Keeps running, as an answer that opens a question would.
    Stay,
    /// Never returns: an application stuck in `update`.
    Hang,
    HandedOff(HandoffOutcome),
}

impl Saver {
    fn note(&self, file: &str, text: &str) {
        let mut out = std::fs::OpenOptions::new().create(true).append(true).open(self.dir.join(file)).expect("note");
        out.write_all(text.as_bytes()).expect("note");
    }
}

impl App for Saver {
    type Msg = Msg;

    fn init(&mut self) -> Command<Msg> {
        if self.mode == "handoff" {
            return Command::handoff(Handoff::new("sh", Msg::HandedOff).args(["-c", PROGRAM]).env("D", &self.dir));
        }
        self.note("ready", "");
        Command::none()
    }

    fn resized(&self, size: Size) -> Option<Msg> {
        self.note("sizes", &format!("{}x{}\n", size.width, size.height));
        None
    }

    fn terminating(&self, cause: Termination) -> Option<Msg> {
        self.note("heard", &format!("{cause:?}\n"));
        Some(match self.mode.as_str() {
            "stubborn" => Msg::Stay,
            "stuck" => Msg::Hang,
            _ => Msg::Save(cause),
        })
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Save(cause) => {
                self.note("saved", &format!("{cause:?}"));
                Command::quit()
            }
            Msg::Stay => Command::none(),
            Msg::Hang => loop {
                std::thread::sleep(Duration::from_secs(1));
            },
            Msg::HandedOff(outcome) => {
                self.note("handoff", &format!("{outcome:?}"));
                Command::none()
            }
        }
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("running"));
    }
}

/// The part that runs inside the pseudo-terminal.
#[test]
#[ignore = "started inside a pseudo-terminal by the tests below"]
fn inside_a_terminal() {
    let dir = PathBuf::from(std::env::var_os(DIR_VAR).expect("started by the tests below, not directly"));
    let mode = std::env::var(MODE_VAR).expect("a mode");
    if mode == "wake" {
        // The loop's wait alone: a signal must end it long before its timeout.
        let signals = Signals::catch().expect("signals");
        std::fs::write(dir.join("ready"), "").expect("ready");
        let started = Instant::now();
        let woken = signals.wait(Duration::from_secs(600), true).expect("wait");
        let heard = signals.take();
        let text = format!("{:?} {:?} {}", heard.causes, woken.keyboard, started.elapsed().as_millis());
        // Written whole and then named, so the test outside, which waits for the name, never
        // reads it half written.
        std::fs::write(dir.join("woken.part"), text).expect("woken");
        std::fs::rename(dir.join("woken.part"), dir.join("woken")).expect("woken");
        return;
    }
    let result = Runtime::new(Saver { dir: dir.clone(), mode }).run();
    std::fs::write(dir.join("returned"), format!("{result:?}")).expect("returned");
}

/// A pseudo-terminal pair: our side, and the device the session inside it uses.
fn pseudo_terminal() -> (std::fs::File, OwnedFd) {
    use rustix::fs::{Mode, OFlags};
    use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
    let controller = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).expect("openpt");
    grantpt(&controller).expect("grantpt");
    unlockpt(&controller).expect("unlockpt");
    let name = ptsname(&controller, Vec::new()).expect("ptsname");
    let device = rustix::fs::open(name, OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC, Mode::empty())
        .expect("the terminal device");
    tcsetwinsize(&controller, Winsize { ws_row: 24, ws_col: 80, ws_xpixel: 0, ws_ypixel: 0 }).expect("size");
    (std::fs::File::from(controller), device)
}

/// An application running on a pseudo-terminal of its own.
struct Session {
    child: Child,
    dir: PathBuf,
    /// The terminal device, kept open to read its modes after the application is gone.
    device: OwnedFd,
    output: Arc<Mutex<Vec<u8>>>,
    /// Asks the reader to stop and close our side of the terminal, which hangs it up.
    close: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
}

impl Session {
    fn start(name: &str, mode: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("quvyta-signals-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test directory");
        let (controller, device) = pseudo_terminal();
        let test = format!("{}::inside_a_terminal", module_path!().split_once("::").expect("crate path").1);
        let clone = || Stdio::from(device.try_clone().expect("clone"));
        let child = Process::new("setsid")
            .arg("--ctty")
            .arg(std::env::current_exe().expect("the test binary"))
            .args([test.as_str(), "--exact", "--ignored", "--test-threads=1"])
            .env(DIR_VAR, &dir)
            .env(MODE_VAR, mode)
            .stdin(clone())
            .stdout(clone())
            .stderr(clone())
            .spawn()
            .expect("setsid starts");
        let output = Arc::new(Mutex::new(Vec::new()));
        let close = Arc::new(AtomicBool::new(false));
        let reader = {
            let (output, close) = (Arc::clone(&output), Arc::clone(&close));
            std::thread::spawn(move || read_terminal(controller, &output, &close))
        };
        Self { child, dir, device, output, close, reader: Some(reader) }
    }

    fn file(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(name)).ok()
    }

    fn shown(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().expect("output")).into_owned()
    }

    fn wait_for(&self, name: &str) -> String {
        let started = Instant::now();
        loop {
            if let Some(text) = self.file(name) {
                return text;
            }
            assert!(started.elapsed() < PATIENCE, "`{name}` never appeared; the terminal showed:\n{}", self.shown());
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn signal(&self, signal: Signal) {
        let pid = Pid::from_child(&self.child);
        kill_process(pid, signal).expect("kill");
    }

    /// Closes our side of the terminal: the device hangs up, as when an SSH connection drops.
    fn hang_up(&mut self) {
        self.close.store(true, Ordering::SeqCst);
        if let Some(reader) = self.reader.take() {
            reader.join().expect("reader");
        }
    }

    /// Waits for the application to end and returns how, and how long it took.
    fn ended(&mut self) -> (ExitStatus, Duration) {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("wait") {
                return (status, started.elapsed());
            }
            if started.elapsed() > PATIENCE {
                let _ = self.child.kill();
                let _ = self.child.wait();
                panic!("the application never ended:\n{}", self.shown());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Asserts the terminal is as the user had it: its modes are the shell's again (not raw)
    /// and the last thing written left the alternate screen and showed the cursor.
    fn assert_restored(&mut self) {
        // Read before our side closes: a terminal that hung up has no modes.
        let modes = tcgetattr(&self.device).expect("modes").local_modes;
        // Stops the reader once it has read everything the application wrote.
        self.hang_up();
        assert!(modes.contains(LocalModes::ICANON | LocalModes::ECHO), "raw mode was left: {modes:?}");
        let shown = self.shown();
        let entered = shown.rfind("\x1b[?1049h").expect("the application took the screen");
        let left = shown.rfind("\x1b[?1049l").unwrap_or_else(|| panic!("the screen was never given back:\n{shown}"));
        assert!(left > entered, "the alternate screen was left last");
        assert!(shown[left..].contains("\x1b[?25h"), "the cursor is shown again");
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.hang_up();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Reads what the application writes until `close` is set and everything written is read, then
/// closes our side. Answers the terminal's device attributes query at once, as a terminal does,
/// so the application does not wait for a keyboard enhancement answer that never comes.
fn read_terminal(mut controller: std::fs::File, output: &Mutex<Vec<u8>>, close: &AtomicBool) {
    let mut chunk = [0_u8; 4096];
    let mut answered = 0;
    loop {
        let closing = close.load(Ordering::SeqCst);
        let limit =
            Timespec::try_from(if closing { Duration::ZERO } else { Duration::from_millis(20) }).expect("timespec");
        let mut fds = [PollFd::new(&controller, PollFlags::IN)];
        let readable = poll(&mut fds, Some(&limit)).is_ok() && fds[0].revents().contains(PollFlags::IN);
        let count = match readable.then(|| controller.read(&mut chunk)) {
            Some(Ok(count @ 1..)) => count,
            _ if closing => return,
            None => continue,
            // Nobody holds the device open for a moment: nothing to read yet.
            Some(_) => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
        };
        let mut output = output.lock().expect("output");
        output.extend_from_slice(&chunk[..count]);
        let asked = String::from_utf8_lossy(&output).matches("\x1b[c").count();
        drop(output);
        while answered < asked {
            let _ = controller.write_all(b"\x1b[?62c");
            answered += 1;
        }
    }
}

#[test]
fn a_signal_wakes_the_wait_at_once() {
    let mut session = Session::start("wake", "wake");
    session.wait_for("ready");
    session.signal(Signal::TERM);
    let woken = session.wait_for("woken");
    let (status, _) = session.ended();
    assert!(status.success(), "{}", session.shown());
    let [heard, keyboard, millis] = woken.split(' ').collect::<Vec<_>>()[..] else {
        panic!("three fields: {woken}");
    };
    assert_eq!(heard, "[Terminate]");
    assert_eq!(keyboard, "false", "a signal, not a key");
    let millis: u64 = millis.parse().expect("milliseconds");
    assert!(millis < 60_000, "woken by the signal, not by the ten minute timeout: {millis} ms");
}

#[test]
fn sigterm_and_sigint_let_the_application_save_and_restore_the_terminal() {
    for (name, signal) in [("term", Signal::TERM), ("int", Signal::INT)] {
        let mut session = Session::start(name, "save");
        session.wait_for("ready");
        session.signal(signal);
        let (status, _) = session.ended();
        assert!(status.success(), "{name}: {}", session.shown());
        assert_eq!(session.file("heard").as_deref(), Some("Terminate\n"), "{name}");
        assert_eq!(session.file("saved").as_deref(), Some("Terminate"), "{name}");
        assert_eq!(session.file("returned").as_deref(), Some("Ok(())"), "{name}: run returned normally");
        session.assert_restored();
    }
}

#[test]
fn a_resize_still_reaches_the_application_while_the_loop_waits_on_signals() {
    let mut session = Session::start("resize", "save");
    session.wait_for("ready");
    let size = Winsize { ws_row: 30, ws_col: 100, ws_xpixel: 0, ws_ypixel: 0 };
    tcsetwinsize(&session.device, size).expect("resize");
    let started = Instant::now();
    while session.file("sizes").is_none_or(|sizes| !sizes.contains("100x30")) {
        assert!(started.elapsed() < PATIENCE, "the resize never arrived: {:?}", session.file("sizes"));
        std::thread::sleep(Duration::from_millis(10));
    }
    session.signal(Signal::TERM);
    session.ended();
    assert_eq!(session.file("sizes").as_deref(), Some("80x24\n100x30\n"));
    session.assert_restored();
}

#[test]
fn a_hangup_by_hand_restores_the_terminal_that_is_still_there() {
    let mut session = Session::start("hup", "save");
    session.wait_for("ready");
    session.signal(Signal::HUP);
    let (status, _) = session.ended();
    assert!(status.success(), "{}", session.shown());
    assert_eq!(session.file("saved").as_deref(), Some("Hangup"));
    assert_eq!(session.file("returned").as_deref(), Some("Ok(())"));
    session.assert_restored();
}

#[test]
fn a_real_hangup_lets_the_application_save_without_its_terminal() {
    let mut session = Session::start("hangup", "save");
    session.wait_for("ready");
    session.hang_up();
    let (status, took) = session.ended();
    assert_eq!(
        session.file("heard").as_deref(),
        Some("Hangup\n"),
        "told once, however many hangups came; ended {status} after {took:?}"
    );
    assert_eq!(session.file("saved").as_deref(), Some("Hangup"));
    assert_eq!(session.file("returned").as_deref(), Some("Ok(())"), "no write to the gone terminal failed the run");
}

#[test]
fn a_second_sigterm_ends_the_run_at_once() {
    let mut session = Session::start("twice", "stubborn");
    session.wait_for("ready");
    session.signal(Signal::TERM);
    session.wait_for("heard");
    session.signal(Signal::TERM);
    let (status, took) = session.ended();
    assert!(status.success(), "{}", session.shown());
    assert!(took < Termination::Terminate.grace(), "the second signal, not the grace, ended it: {took:?}");
    assert_eq!(session.file("heard").as_deref(), Some("Terminate\n"), "not asked again");
    assert_eq!(session.file("returned").as_deref(), Some("Ok(())"));
    session.assert_restored();
}

#[test]
fn the_grace_ends_a_run_the_application_keeps_open() {
    let mut session = Session::start("grace", "stubborn");
    session.wait_for("ready");
    session.signal(Signal::HUP);
    let (status, took) = session.ended();
    assert!(status.success(), "{}", session.shown());
    assert!(took >= Termination::Hangup.grace() - Duration::from_millis(100), "the grace was given: {took:?}");
    assert_eq!(session.file("returned").as_deref(), Some("Ok(())"), "the loop quit by itself");
    session.assert_restored();
}

#[test]
fn a_stuck_application_is_ended_by_the_signal_with_the_terminal_restored() {
    let mut session = Session::start("stuck", "stuck");
    session.wait_for("ready");
    session.signal(Signal::TERM);
    session.wait_for("heard");
    session.signal(Signal::TERM);
    let (status, _) = session.ended();
    assert_eq!(status.signal(), Some(Signal::TERM.as_raw()), "ended by the signal: {status}");
    assert_eq!(session.file("returned"), None, "run never returned");
    session.assert_restored();
}

#[test]
fn a_signal_during_a_handoff_reaches_the_program_and_then_the_application() {
    let mut session = Session::start("handoff", "handoff");
    session.wait_for("ready");
    session.signal(Signal::TERM);
    let (status, _) = session.ended();
    assert!(status.success(), "{}", session.shown());
    assert_eq!(session.file("program").as_deref(), Some("terminated\n"), "the program heard it");
    assert_eq!(session.file("handoff").as_deref(), Some("Finished { code: Some(43) }"));
    assert_eq!(session.file("saved").as_deref(), Some("Terminate"), "then the application");
    assert_eq!(session.file("returned").as_deref(), Some("Ok(())"));
    session.assert_restored();
}
