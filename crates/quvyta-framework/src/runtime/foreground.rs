//! Giving the terminal's foreground to a program for a while, and taking it back.
//!
//! The terminal sends the signals of its keys — `Ctrl-C`, `Ctrl-\`, `Ctrl-Z` — to one process
//! group, its foreground group. A program that shares the application's group would take the
//! application down with it on `Ctrl-C` at a `sudo` prompt, so a handoff starts the program in a
//! group of its own and makes that group the foreground, the way a shell runs a job. The
//! application keeps its signal dispositions untouched; it is simply not the one the keys reach.
//!
//! The group is a process group, not a session: the program keeps the application's session and
//! controlling terminal, which is what `sudo` keys its ticket on (the terminal device and the
//! start time of the session leader), so a warm ticket stays warm.

use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus};
use std::sync::{Mutex, MutexGuard, PoisonError};

use nix::sys::signal::{SigSet, SigmaskHow, Signal as NixSignal};
use rustix::fs::{Mode, OFlags};
use rustix::process::{
    Pid, Signal, WaitId, WaitIdOptions, getpgrp, kill_current_process_group, kill_process_group, waitid,
};
use rustix::termios::{tcgetpgrp, tcsetpgrp};

/// One handoff at a time owns the terminal's foreground. The runtime runs handoffs one after
/// another anyway; this keeps two callers on different threads from taking it from each other.
static FOREGROUND: Mutex<()> = Mutex::new(());

/// Runs `command` to its end with the terminal's foreground handed to it, and returns how it
/// ended.
///
/// When the application has no controlling terminal, or is not in its foreground group (so no
/// key of it reaches the application anyway), the program runs in the application's own group,
/// as any child does.
///
/// # Errors
///
/// Returns an I/O error when the program cannot be started, when the terminal cannot be handed
/// to it, or when the terminal cannot be taken back afterwards.
pub(crate) fn status(command: &mut Command) -> io::Result<ExitStatus> {
    let (mut child, foreground) = Foreground::spawn(command)?;
    let status = foreground.wait(&mut child);
    let taken = foreground.give_back();
    let status = status?;
    taken?;
    Ok(status)
}

/// The terminal's foreground, lent to one program. Whatever happens while it is lent, a panic
/// included, dropping this gives it back to the application.
pub(crate) struct Foreground {
    /// The terminal and the program's group, while the program owns the terminal.
    lent: Option<(Terminal, Pid)>,
    _only_one: MutexGuard<'static, ()>,
}

impl Foreground {
    /// Starts `command` and makes its process group the terminal's foreground.
    ///
    /// When the application has no controlling terminal, or is not in its foreground group,
    /// the program is only started, in the application's own group, as any child is.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the program cannot be started or the terminal cannot be handed
    /// to it; the program is ended then, since in the background it would stop at its first
    /// read of the terminal and wait forever.
    pub(crate) fn spawn(command: &mut Command) -> io::Result<(Child, Self)> {
        let only_one = FOREGROUND.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(terminal) = Terminal::open() else {
            return Ok((command.spawn()?, Self { lent: None, _only_one: only_one }));
        };
        command.process_group(0);
        let mut child = command.spawn()?;
        let group = Pid::from_child(&child);
        if let Err(error) = tcsetpgrp(&terminal.device, group) {
            // A program that already ended has nothing to hand the terminal to.
            if let Ok(Some(_)) = child.try_wait() {
                return Ok((child, Self { lent: None, _only_one: only_one }));
            }
            let _ = kill_process_group(group, Signal::KILL);
            let _ = child.wait();
            return Err(error.into());
        }
        // A program that reached for the terminal in the moment before it was handed over was
        // stopped for it; now it may go on.
        let _ = kill_process_group(group, Signal::CONT);
        Ok((child, Self { lent: Some((terminal, group)), _only_one: only_one }))
    }

    /// Waits for `child` to end. While it owns the terminal, a stop is answered by letting it go
    /// on, see [`wait_through_stops`].
    pub(crate) fn wait(&self, child: &mut Child) -> io::Result<ExitStatus> {
        match &self.lent {
            Some((_, group)) => wait_through_stops(child, *group),
            None => child.wait(),
        }
    }

    /// How `child` ended, without waiting: `None` while it runs. A stop is answered by letting
    /// it go on, as [`Foreground::wait`] does.
    pub(crate) fn check(&self, child: &mut Child) -> io::Result<Option<ExitStatus>> {
        if let Some((_, group)) = &self.lent {
            let pid = Pid::from_child(child);
            let options = WaitIdOptions::STOPPED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
            if let Ok(Some(state)) = waitid(WaitId::Pid(pid), options)
                && state.stopped()
            {
                let _ = waitid(WaitId::Pid(pid), WaitIdOptions::STOPPED | WaitIdOptions::NOHANG);
                let _ = kill_process_group(*group, Signal::CONT);
            }
        }
        child.try_wait()
    }

    /// Makes the application's group the terminal's foreground again, reporting what went
    /// wrong. Dropping does the same and forgets the error.
    pub(crate) fn give_back(mut self) -> io::Result<()> {
        match self.lent.take() {
            Some((terminal, _)) => terminal.take_back(),
            None => Ok(()),
        }
    }
}

impl Drop for Foreground {
    fn drop(&mut self) {
        if let Some((terminal, _)) = self.lent.take() {
            let _ = terminal.take_back();
        }
    }
}

/// Waits for `child` to end. A stop, from `Ctrl-Z` or from reaching for the terminal too early,
/// is answered by letting the group go on: the application is not a shell that could offer the
/// user a way back to a stopped program, and a stopped program in the foreground would leave the
/// terminal with nobody reading it.
fn wait_through_stops(child: &mut Child, group: Pid) -> io::Result<ExitStatus> {
    let pid = Pid::from_child(child);
    loop {
        match waitid(WaitId::Pid(pid), WaitIdOptions::EXITED | WaitIdOptions::STOPPED | WaitIdOptions::NOWAIT) {
            Ok(Some(state)) if state.stopped() => {
                // The stop was only looked at; it is taken now, so the next wait sees what comes
                // after it.
                let _ = waitid(WaitId::Pid(pid), WaitIdOptions::STOPPED | WaitIdOptions::NOHANG);
                let _ = kill_process_group(group, Signal::CONT);
            }
            Err(rustix::io::Errno::INTR) => {}
            // It ended, or waiting failed; the wait below collects the status or the error.
            _ => return child.wait(),
        }
    }
}

/// The controlling terminal, while the application's group is its foreground.
struct Terminal {
    device: OwnedFd,
    group: Pid,
}

impl Terminal {
    /// The controlling terminal, when there is one and the application is in its foreground.
    fn open() -> Option<Self> {
        let device =
            rustix::fs::open("/dev/tty", OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC, Mode::empty()).ok()?;
        let group = getpgrp();
        (tcgetpgrp(&device).ok()? == group).then_some(Self { device, group })
    }

    /// Makes the application's group the foreground again.
    ///
    /// The application is in the background at this point, and the system answers a background
    /// group that changes the terminal with SIGTTOU, which would stop it. Blocking the signal on
    /// this thread for the one call is what a shell does; the thread's previous mask is put back
    /// right after, exactly as it was.
    fn take_back(self) -> io::Result<()> {
        let mut ttou = SigSet::empty();
        ttou.add(NixSignal::SIGTTOU);
        let previous = ttou.thread_swap_mask(SigmaskHow::SIG_BLOCK).map_err(io::Error::from)?;
        let taken = tcsetpgrp(&self.device, self.group);
        previous.thread_set_mask().map_err(io::Error::from)?;
        taken?;
        // A child of the application that read the terminal while it was the program's was
        // stopped for it; the terminal is the application's again, so it may go on.
        let _ = kill_current_process_group(Signal::CONT);
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    //! These tests need a terminal of their own: the one `cargo test` runs in may be none, and
    //! pressing keys on it would reach whatever else runs there. Each test starts this test binary
    //! again as a new session on a pseudo-terminal, runs one handoff inside it, and presses keys
    //! on that terminal from outside.

    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use crate::runtime::HandoffOutcome;
    use crate::runtime::handoff::{self, Handoff, HandoffScreen};

    /// Where the test running inside the pseudo-terminal leaves its files.
    const DIR_VAR: &str = "QUVYTA_FOREGROUND_TEST_DIR";

    /// How long the machine may take for anything here; generous, because it may be loaded.
    const PATIENCE: Duration = Duration::from_secs(60);

    /// The program the handoff runs: it records its group, session and terminal, waits until
    /// its group owns the terminal, says it is ready and runs until `Ctrl-C` ends it with 42.
    const PROGRAM: &str = r#"
set -- $(cat /proc/$$/stat); echo "$5 $6 $7" > "$D/child"
while set -- $(cat /proc/$$/stat); [ "$5" != "$8" ]; do sleep 0.01; done
trap 'echo interrupted > "$D/interrupted"; exit 42' INT
touch "$D/ready"
while :; do sleep 0.05; done
"#;

    /// The process group, session and terminal device of this process, from `/proc`.
    fn stat_ids(pid: &str) -> String {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).expect("stat");
        // The command name may hold spaces; everything after its closing parenthesis is fields.
        let rest = &stat[stat.rfind(')').expect("name") + 2..];
        let fields: Vec<&str> = rest.split(' ').collect();
        // After the name: state, parent, group, session, terminal.
        format!("{} {} {}", fields[2], fields[3], fields[4])
    }

    /// The signal masks and dispositions of the calling thread: blocked, ignored and caught.
    fn signal_state() -> Vec<String> {
        std::fs::read_to_string("/proc/thread-self/status")
            .expect("status")
            .lines()
            .filter(|line| ["SigBlk", "SigIgn", "SigCgt"].iter().any(|name| line.starts_with(name)))
            .map(str::to_owned)
            .collect()
    }

    /// Whether this process's group is the foreground of its controlling terminal.
    fn in_the_foreground() -> bool {
        let device = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty").expect("a terminal");
        rustix::termios::tcgetpgrp(&device).expect("its foreground") == rustix::process::getpgrp()
    }

    /// The part that runs inside the pseudo-terminal: one handoff, pressed from outside.
    #[test]
    #[ignore = "started inside a pseudo-terminal by the tests below"]
    fn inside_a_terminal() {
        let dir = PathBuf::from(std::env::var_os(DIR_VAR).expect("started by the tests below, not directly"));
        assert!(in_the_foreground(), "the test starts as the terminal's foreground");
        let before = signal_state();
        let ours = stat_ids("self");
        let foreground_at_take = std::cell::Cell::new(false);
        let mut release = |_: Option<&str>| Ok(());
        let mut take = || {
            foreground_at_take.set(in_the_foreground());
            Ok(())
        };
        let mut wait_for_key = || Ok(());
        let outcome = handoff::run(
            Handoff::new("sh", |outcome| outcome).args(["-c", PROGRAM]).env("D", &dir),
            &mut HandoffScreen { release: &mut release, take: &mut take, wait_for_key: &mut wait_for_key },
        );
        // Still here: `Ctrl-C` did not reach this process.
        assert_eq!(outcome, HandoffOutcome::Finished { code: Some(42) }, "the program's own trap ended it");
        assert!(dir.join("interrupted").exists(), "`Ctrl-C` reached the program");
        assert!(foreground_at_take.get(), "the terminal was the application's again when it took the screen back");
        assert_eq!(signal_state(), before, "no signal disposition or mask of the application changed");
        assert_eq!(stat_ids("self"), ours, "the application is still in its own group");
        let child = std::fs::read_to_string(dir.join("child")).expect("the program recorded itself");
        let [group, session, terminal] = child.split_whitespace().collect::<Vec<_>>()[..] else {
            panic!("three fields: {child}");
        };
        let [our_group, our_session, our_terminal] = ours.split(' ').collect::<Vec<_>>()[..] else {
            panic!("three fields: {ours}");
        };
        assert_ne!(group, our_group, "the program ran in a group of its own");
        assert_eq!(session, our_session, "the same session, which a sudo ticket is kept for");
        assert_eq!(terminal, our_terminal, "the same controlling terminal, which a sudo ticket is kept for");
    }

    /// The program a detached handoff runs: like [`PROGRAM`], but `Ctrl-C` makes it say it is
    /// ready and serve its input, echoing every line, until the input ends.
    const DETACHED_PROGRAM: &str = r#"
set -- $(cat /proc/$$/stat); echo "$5 $6 $7" > "$D/child"
while set -- $(cat /proc/$$/stat); [ "$5" != "$8" ]; do sleep 0.01; done
trap 'echo interrupted > "$D/interrupted"; echo ready; exec cat' INT
touch "$D/ready"
while :; do sleep 0.05; done
"#;

    /// The part of the detached test that runs inside the pseudo-terminal.
    #[test]
    #[ignore = "started inside a pseudo-terminal by the tests below"]
    fn detached_inside_a_terminal() {
        use crate::runtime::task::Delivery;
        use crate::runtime::{ChildLine, DetachedHandoff, DetachedOutcome};

        #[derive(Debug)]
        enum Heard {
            Started(DetachedOutcome),
            Said(ChildLine),
        }

        let dir = PathBuf::from(std::env::var_os(DIR_VAR).expect("started by the tests below, not directly"));
        assert!(in_the_foreground(), "the test starts as the terminal's foreground");
        let before = signal_state();
        let ours = stat_ids("self");
        let foreground_at_take = std::cell::Cell::new(false);
        let mut release = |_: Option<&str>| Ok(());
        let mut take = || {
            foreground_at_take.set(in_the_foreground());
            Ok(())
        };
        let mut wait_for_key = || Ok(());
        let (deliveries, lines) = std::sync::mpsc::channel();
        let message = crate::runtime::detached::run(
            DetachedHandoff::new("sh", Heard::Started)
                .args(["-c", DETACHED_PROGRAM])
                .env("D", &dir)
                .on_line(Heard::Said),
            &mut HandoffScreen { release: &mut release, take: &mut take, wait_for_key: &mut wait_for_key },
            &deliveries,
        );
        let Heard::Started(DetachedOutcome::Detached { child, first_line }) = message else {
            panic!("the program said it was ready after Ctrl-C: {message:?}");
        };
        assert_eq!(first_line, "ready");
        assert!(dir.join("interrupted").exists(), "`Ctrl-C` reached the program, not the application");
        assert!(foreground_at_take.get(), "the terminal was the application's again when it took the screen back");
        assert!(in_the_foreground(), "and it stays the application's while the child runs on");
        assert_eq!(signal_state(), before, "no signal disposition or mask of the application changed");
        assert_eq!(stat_ids("self"), ours, "the application is still in its own group");
        let pid = child.id().expect("a real child").to_string();
        let theirs = stat_ids(&pid);
        let [group, session, _] = theirs.split(' ').collect::<Vec<_>>()[..] else {
            panic!("three fields: {theirs}");
        };
        let [our_group, our_session, _] = ours.split(' ').collect::<Vec<_>>()[..] else {
            panic!("three fields: {ours}");
        };
        assert_ne!(group, our_group, "the child runs on in a group of its own, in the background");
        assert_eq!(session, our_session, "in the application's session");
        child.write_line("still here").expect("the child reads its input");
        let said = |expected: ChildLine| match lines.recv_timeout(PATIENCE) {
            Ok(Delivery::Message(Heard::Said(line))) => assert_eq!(line, expected),
            Ok(Delivery::Message(other)) => panic!("expected {expected:?}, got {other:?}"),
            Ok(Delivery::Ended) => panic!("expected {expected:?}, got the end of background work"),
            Err(error) => panic!("expected {expected:?}: {error}"),
        };
        said(ChildLine::Line("still here".to_owned()));
        drop(child);
        said(ChildLine::Ended { code: Some(0) });
    }

    /// The part of the panic test that runs inside the pseudo-terminal.
    #[test]
    #[ignore = "started inside a pseudo-terminal by the tests below"]
    fn panic_inside_a_terminal() {
        assert!(in_the_foreground(), "the test starts as the terminal's foreground");
        let (mut child, foreground) = super::Foreground::spawn(Command::new("sleep").arg("30")).expect("sleep starts");
        assert!(!in_the_foreground(), "the program owns the terminal");
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _lent = foreground;
            panic!("a panic while the program owns the terminal");
        }));
        assert!(unwound.is_err());
        assert!(in_the_foreground(), "unwinding gave the terminal back");
        child.kill().expect("kill");
        child.wait().expect("wait");
    }

    /// A pseudo-terminal pair: our side, and the device the session inside it uses.
    fn pseudo_terminal() -> (std::fs::File, std::os::fd::OwnedFd) {
        use rustix::fs::{Mode, OFlags};
        use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
        let controller = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).expect("openpt");
        grantpt(&controller).expect("grantpt");
        unlockpt(&controller).expect("unlockpt");
        let name = ptsname(&controller, Vec::new()).expect("ptsname");
        let device = rustix::fs::open(name, OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC, Mode::empty())
            .expect("the terminal device");
        (std::fs::File::from(controller), device)
    }

    fn wait_for(path: &Path, what: &str, output: &Mutex<Vec<u8>>) {
        let started = Instant::now();
        while !path.exists() {
            assert!(
                started.elapsed() < PATIENCE,
                "{what} never happened; the terminal showed:\n{}",
                String::from_utf8_lossy(&output.lock().expect("output"))
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Runs [`inside_a_terminal`] as a new session on a pseudo-terminal, through `launcher`
    /// (which receives the test binary and its arguments), presses `Ctrl-Z` and then `Ctrl-C`
    /// once the program owns the terminal, and asserts the inner test passed.
    fn press_keys_during_a_handoff(name: &str, launcher: &[&str]) {
        in_a_terminal("inside_a_terminal", name, launcher, true);
    }

    /// Runs the ignored test `inner` as a new session on a pseudo-terminal, through `launcher`,
    /// with `keys` pressing `Ctrl-Z` and then `Ctrl-C` once its program owns the terminal, and
    /// asserts the inner test passed.
    fn in_a_terminal(inner: &str, name: &str, launcher: &[&str], keys: bool) {
        let dir = std::env::temp_dir().join(format!("quvyta-foreground-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test directory");
        let (mut controller, device) = pseudo_terminal();
        let test = format!("{}::{inner}", module_path!().split_once("::").expect("crate path").1);
        let exe = std::env::current_exe().expect("the test binary");
        let mut command = Command::new("setsid");
        command
            .arg("--ctty")
            .args(launcher)
            .arg(&exe)
            .args([test.as_str(), "--exact", "--ignored", "--test-threads=1"])
            .env(DIR_VAR, &dir)
            .stdin(Stdio::from(device.try_clone().expect("clone")))
            .stdout(Stdio::from(device.try_clone().expect("clone")))
            .stderr(Stdio::from(device));
        let mut session = command.spawn().expect("setsid starts");
        drop(command);
        let output = Arc::new(Mutex::new(Vec::new()));
        let reader = {
            let mut controller = controller.try_clone().expect("clone");
            let output = Arc::clone(&output);
            std::thread::spawn(move || {
                let mut chunk = [0_u8; 4096];
                while let Ok(count @ 1..) = controller.read(&mut chunk) {
                    output.lock().expect("output").extend_from_slice(&chunk[..count]);
                }
            })
        };
        if keys {
            wait_for(&dir.join("ready"), "the program owning the terminal", &output);
            // `Ctrl-Z` first: the program stops, and a handoff that did not let it go on would
            // wait forever, since the interrupt below stays pending on a stopped program.
            controller.write_all(b"\x1a").expect("Ctrl-Z");
            std::thread::sleep(Duration::from_millis(300));
            controller.write_all(b"\x03").expect("Ctrl-C");
        }
        let started = Instant::now();
        let status = loop {
            if let Some(status) = session.try_wait().expect("wait") {
                break status;
            }
            if started.elapsed() > PATIENCE {
                let _ = session.kill();
                let _ = session.wait();
                panic!("the session never ended:\n{}", String::from_utf8_lossy(&output.lock().expect("output")));
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        drop(controller);
        let shown = String::from_utf8_lossy(&output.lock().expect("output")).into_owned();
        drop(reader);
        assert!(status.success(), "the test inside the terminal failed ({status}):\n{shown}");
        assert!(shown.contains("1 passed"), "the test inside the terminal ran:\n{shown}");
        std::fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn keys_reach_the_program_when_the_application_leads_its_session() {
        // Started straight by a terminal emulator or `tmux`, the application leads its session
        // and cannot leave its group; the terminal still has to come back to it.
        press_keys_during_a_handoff("leader", &[]);
    }

    #[test]
    fn keys_reach_the_program_when_a_shell_started_the_application() {
        // Started from a shell script, the application shares the script's group.
        press_keys_during_a_handoff("member", &["sh", "-c", r#""$0" "$@"; exit $?"#]);
    }

    #[test]
    fn a_detached_program_gets_the_keys_until_it_is_ready_and_then_runs_in_the_background() {
        in_a_terminal("detached_inside_a_terminal", "detached", &[], true);
    }

    #[test]
    fn a_panic_while_a_program_owns_the_terminal_gives_it_back() {
        in_a_terminal("panic_inside_a_terminal", "panic", &[], false);
    }
}
