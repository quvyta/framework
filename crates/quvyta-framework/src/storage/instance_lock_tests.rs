use super::*;
use std::fs;
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::mpsc;
#[cfg(unix)]
use std::time::{Duration, Instant};

/// An empty directory of this test's own.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quvyta-instances-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("test directory");
    dir
}

#[cfg(unix)]
const PATIENCE: Duration = Duration::from_secs(5);

/// How long a test waits to see that something does not happen.
#[cfg(unix)]
const A_WHILE: Duration = Duration::from_millis(300);

#[cfg(unix)]
/// `try_exclusive` until it answers with the lock or the deadline passes. A released lock is free
/// in a moment, not in the same instant: another test running beside this one starts child
/// processes, and between `fork` and `exec` a child holds a copy of this process's open files.
fn exclusive_soon(path: &Path) -> Option<InstanceLock> {
    let deadline = Instant::now() + PATIENCE;
    loop {
        match InstanceLock::try_exclusive(path).expect("attempt") {
            Some(lock) => return Some(lock),
            None if Instant::now() >= deadline => return None,
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

#[cfg(unix)]
/// Waits for the exclusive lock on a thread of its own, and says how much processor time that
/// thread spent waiting.
fn wait_on_a_thread(path: &Path) -> mpsc::Receiver<(io::Result<InstanceLock>, Duration)> {
    let (send, receive) = mpsc::channel();
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let busy = || {
            let time = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
            Duration::new(u64::try_from(time.tv_sec).unwrap_or(0), u32::try_from(time.tv_nsec).unwrap_or(0))
        };
        let before = busy();
        let lock = InstanceLock::wait_exclusive(&path);
        let _ = send.send((lock, busy().saturating_sub(before)));
    });
    receive
}

#[cfg(unix)]
#[test]
fn many_shared_locks_are_held_at_once_and_keep_the_exclusive_one_out() {
    let dir = temp_dir("shared");
    let path = dir.join("instances");
    let first = InstanceLock::shared(&path).expect("first instance");
    let second = InstanceLock::shared(&path).expect("second instance, without waiting");
    assert!(InstanceLock::try_exclusive(&path).expect("attempt").is_none(), "two instances are running");
    drop(first);
    assert!(InstanceLock::try_exclusive(&path).expect("attempt").is_none(), "one is still running");
    drop(second);
    let lock = exclusive_soon(&path).expect("with nobody running the lock is free");
    drop(lock);
    fs::remove_dir_all(&dir).expect("clean");
}

#[cfg(unix)]
#[test]
fn the_waiter_wakes_when_the_last_instance_closes_and_not_before() {
    let dir = temp_dir("wait");
    let path = dir.join("instances");
    let first = InstanceLock::shared(&path).expect("first instance");
    let second = InstanceLock::shared(&path).expect("second instance");
    let waiter = wait_on_a_thread(&path);

    assert!(waiter.recv_timeout(A_WHILE).is_err(), "two instances run, so it waits");
    drop(first);
    assert!(waiter.recv_timeout(A_WHILE).is_err(), "one instance still runs");
    drop(second);
    let (lock, busy) = waiter.recv_timeout(PATIENCE).expect("the last close woke it");
    let lock = lock.expect("the exclusive lock");
    assert!(busy < Duration::from_millis(100), "it slept in the kernel, busy for {busy:?} of about 600 ms");

    // While the waiter holds it, a new instance waits for it to finish.
    let (send, receive) = mpsc::channel();
    let starting = path.clone();
    std::thread::spawn(move || {
        let _ = send.send(InstanceLock::shared(&starting));
    });
    assert!(receive.recv_timeout(A_WHILE).is_err(), "an instance waits for the cleanup");
    drop(lock);
    receive.recv_timeout(PATIENCE).expect("the cleanup ended").expect("the instance's lock");
    fs::remove_dir_all(&dir).expect("clean");
}

#[cfg(unix)]
#[test]
fn nobody_running_means_the_exclusive_lock_comes_at_once() {
    let dir = temp_dir("nobody");
    let path = dir.join("instances");
    let waiter = wait_on_a_thread(&path);
    let (lock, _) = waiter.recv_timeout(PATIENCE).expect("no wait");
    lock.expect("the exclusive lock");
    fs::remove_dir_all(&dir).expect("clean");
}

#[cfg(unix)]
/// The environment variable that turns [`a_child_process_holds_a_shared_lock`] into a lock holder.
const CHILD: &str = "QUVYTA_INSTANCE_LOCK_CHILD";

#[cfg(unix)]
/// Run as a child process by the next test: takes a shared lock on the path it is given, says so
/// and holds it until it is killed. Run on its own, without the variable, it does nothing.
#[test]
fn a_child_process_holds_a_shared_lock() {
    let Some(path) = std::env::var_os(CHILD) else {
        return;
    };
    let _lock = InstanceLock::shared(Path::new(&path)).expect("the child's lock");
    println!("holding");
    std::thread::sleep(Duration::from_secs(60));
}

#[cfg(unix)]
#[test]
fn an_instance_that_is_killed_releases_its_lock_and_wakes_the_waiter() {
    use std::io::BufRead;

    let dir = temp_dir("killed");
    let path = dir.join("instances");
    // This test binary again, running only the holder above.
    let mut child = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args(["--exact", "storage::instance_lock::tests::a_child_process_holds_a_shared_lock", "--nocapture"])
        .env(CHILD, &path)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the child process");
    let output = child.stdout.take().expect("the child's output");
    let holding = std::io::BufReader::new(output).lines().map_while(Result::ok).any(|line| line == "holding");
    assert!(holding, "the child took its lock");

    let waiter = wait_on_a_thread(&path);
    assert!(waiter.recv_timeout(A_WHILE).is_err(), "the child's instance runs");
    child.kill().expect("kill the child");
    child.wait().expect("the child ended");
    let (lock, _) = waiter.recv_timeout(PATIENCE).expect("the kernel released the dead child's lock");
    lock.expect("the exclusive lock");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_file_that_cannot_be_opened_is_an_error() {
    let dir = temp_dir("missing");
    let path = dir.join("absent").join("instances");
    let expected = if cfg!(unix) { io::ErrorKind::NotFound } else { io::ErrorKind::Unsupported };
    assert_eq!(InstanceLock::shared(&path).expect_err("no directory").kind(), expected);
    assert_eq!(InstanceLock::try_exclusive(&path).expect_err("no directory").kind(), expected);
    assert_eq!(InstanceLock::wait_exclusive(&path).expect_err("no directory").kind(), expected);
    fs::remove_dir_all(&dir).expect("clean");
}
