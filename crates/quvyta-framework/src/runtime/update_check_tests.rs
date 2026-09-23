//! Tests for the update check. The registry is a server of the test's own on the loopback
//! address; nothing here reaches crates.io.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use super::*;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;

/// A registry that answers every request with one index text and remembers what it was asked.
struct Registry {
    address: String,
    asked: Arc<Mutex<Vec<(String, String)>>>,
}

impl Registry {
    /// A registry whose index lists `versions`, each `(version, yanked)`.
    fn listing(versions: &[(&str, bool)]) -> Self {
        let index: String = versions
            .iter()
            .map(|(version, yanked)| {
                format!("{{\"name\":\"quvyta-code\",\"vers\":\"{version}\",\"deps\":[],\"cksum\":\"00\",\"features\":{{}},\"yanked\":{yanked}}}\n")
            })
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = format!("http://{}", listener.local_addr().expect("its address"));
        let asked = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&asked);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                let path = line.split_whitespace().nth(1).unwrap_or_default().to_owned();
                let mut agent = String::new();
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = header.split_once(':')
                        && name.eq_ignore_ascii_case("user-agent")
                    {
                        agent = value.trim().to_owned();
                    }
                }
                if let Ok(mut asked) = record.lock() {
                    asked.push((path, agent));
                }
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{index}",
                    index.len()
                );
            }
        });
        Self { address, asked }
    }

    /// How many questions reached the registry.
    fn questions(&self) -> usize {
        self.asked.lock().map(|asked| asked.len()).unwrap_or(0)
    }
}

/// A folder of the test's own, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quvyta-update-check-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The check of qcode at `current`, answering with the update, against `registry`, with the
/// ecosystem's file and the memory of the last question in `dir`.
fn check(dir: &Path, current: &str, registry: &str) -> UpdateCheck<Update> {
    UpdateCheck::new(Ecosystem::QUVYTA, "code", "quvyta-code", current, |update| update)
        .in_folders(dir.join("config"), dir.join("state"))
        .registry(registry)
}

/// A fixed moment, so the day is counted from the test's clock rather than the machine's.
fn morning() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_790_000_000)
}

#[test]
fn a_newer_version_is_announced_and_only_the_package_is_asked_about() {
    let dir = scratch("newer");
    let registry = Registry::listing(&[("0.1.12", false), ("0.1.13", false), ("0.1.14", false)]);
    let update = check(&dir, "0.1.13", &registry.address).ask(morning()).expect("a newer version");
    assert_eq!((update.current(), update.latest()), ("0.1.13", "0.1.14"));
    let asked = registry.asked.lock().expect("the questions").clone();
    assert_eq!(asked, [("/qu/vy/quvyta-code".to_owned(), "quvyta-code/0.1.13".to_owned())], "the name, and no more");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_running_version_or_an_older_one_is_not_announced() {
    let dir = scratch("same");
    let registry = Registry::listing(&[("0.1.12", false), ("0.1.13", false)]);
    assert_eq!(check(&dir, "0.1.13", &registry.address).ask(morning()), None);
    assert_eq!(registry.questions(), 1, "the registry was asked");
    std::fs::remove_dir_all(dir).ok();
    let dir = scratch("ahead");
    assert_eq!(check(&dir, "0.2.0", &registry.address).ask(morning()), None, "a build ahead of the registry");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn yanked_versions_and_pre_releases_are_not_announced() {
    let dir = scratch("yanked");
    let registry = Registry::listing(&[("0.1.13", false), ("0.1.14", true), ("0.2.0-beta.1", false)]);
    assert_eq!(check(&dir, "0.1.13", &registry.address).ask(morning()), None);
    assert_eq!(registry.questions(), 1);
    std::fs::remove_dir_all(dir).ok();
}

/// What a check of qcode at `current` announces against a registry listing `versions`.
fn announced(name: &str, current: &str, versions: &[(&str, bool)]) -> Option<String> {
    let dir = scratch(name);
    let registry = Registry::listing(versions);
    let found = check(&dir, current, &registry.address).ask(morning()).map(|update| update.latest().to_owned());
    std::fs::remove_dir_all(dir).ok();
    found
}

#[test]
fn a_person_on_a_pre_release_hears_of_the_next_one() {
    let listing = [("0.1.0-alpha.1", false), ("0.1.0-alpha.2", false)];
    assert_eq!(announced("alpha-next", "0.1.0-alpha.1", &listing).as_deref(), Some("0.1.0-alpha.2"));
    assert_eq!(announced("alpha-same", "0.1.0-alpha.2", &listing), None, "the one it runs is not news");
}

#[test]
fn pre_release_numbers_count_as_numbers_and_more_parts_are_newer() {
    let tens = [("0.1.0-alpha.9", false), ("0.1.0-alpha.10", false)];
    assert_eq!(announced("alpha-ten", "0.1.0-alpha.9", &tens).as_deref(), Some("0.1.0-alpha.10"));
    let parts = [("0.1.0-alpha", false), ("0.1.0-alpha.1", false)];
    assert_eq!(announced("alpha-parts", "0.1.0-alpha", &parts).as_deref(), Some("0.1.0-alpha.1"));
    let words = [("0.1.0-alpha.3", false), ("0.1.0-beta", false)];
    assert_eq!(announced("alpha-beta", "0.1.0-alpha.3", &words).as_deref(), Some("0.1.0-beta"));
    let numeric_first = [("0.1.0-1", false), ("0.1.0-alpha", false)];
    assert_eq!(announced("numeric", "0.1.0-1", &numeric_first).as_deref(), Some("0.1.0-alpha"));
}

#[test]
fn a_release_is_newer_than_its_pre_releases_and_wins_over_them() {
    let listing = [("0.1.0-alpha.1", false), ("0.1.0-alpha.2", false), ("0.1.0", false), ("0.0.9", false)];
    assert_eq!(announced("alpha-release", "0.1.0-alpha.1", &listing).as_deref(), Some("0.1.0"));
    let older = [("0.0.9", false)];
    assert_eq!(announced("alpha-older", "0.1.0-alpha.1", &older), None, "an older release is not news");
    let next_line = [("0.2.0-alpha.1", false)];
    assert_eq!(announced("alpha-line", "0.1.0-alpha.1", &next_line).as_deref(), Some("0.2.0-alpha.1"));
}

#[test]
fn a_person_on_a_release_never_hears_of_a_pre_release() {
    let listing = [("0.1.0", false), ("0.2.0-alpha.1", false)];
    assert_eq!(announced("stable-pre", "0.1.0", &listing), None);
    let both = [("0.1.0", false), ("0.1.1", false), ("0.2.0-alpha.1", false)];
    assert_eq!(announced("stable-both", "0.1.0", &both).as_deref(), Some("0.1.1"));
}

#[test]
fn a_yanked_pre_release_is_passed_over_and_build_metadata_is_ignored() {
    let listing = [("0.1.0-alpha.2", false), ("0.1.0-alpha.3", true)];
    assert_eq!(announced("alpha-yanked", "0.1.0-alpha.1", &listing).as_deref(), Some("0.1.0-alpha.2"));
    let build = [("0.1.0-alpha.1+abc", false)];
    assert_eq!(announced("alpha-build", "0.1.0-alpha.1+xyz", &build), None, "the same version built twice");
    let junk = [("0.1.0-alpha..1", false), ("0.1.0-", false)];
    assert_eq!(announced("alpha-junk", "0.1.0-alpha.1", &junk), None, "what cannot be read is silence");
}

#[test]
fn without_a_registry_the_check_is_silent_and_waits_for_the_next_day() {
    // A port that was open a moment ago and is not now: the connection is refused, as it is on a
    // machine without network or with the registry down.
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let address = format!("http://{}", listener.local_addr().expect("its address"));
    drop(listener);
    let dir = scratch("offline");
    assert_eq!(check(&dir, "0.1.13", &address).ask(morning()), None, "silence, not an error");
    let registry = Registry::listing(&[("0.1.14", false)]);
    let later = morning() + Duration::from_secs(60 * 60);
    assert_eq!(check(&dir, "0.1.13", &registry.address).ask(later), None, "the failed question counted");
    assert_eq!(registry.questions(), 0);
    let tomorrow = morning() + Duration::from_secs(25 * 60 * 60);
    assert!(check(&dir, "0.1.13", &registry.address).ask(tomorrow).is_some(), "the next day asks again");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_registry_is_asked_at_most_once_a_day() {
    let dir = scratch("daily");
    let registry = Registry::listing(&[("0.1.14", false)]);
    assert!(check(&dir, "0.1.13", &registry.address).ask(morning()).is_some());
    for hours in [1, 6, 23] {
        let later = morning() + Duration::from_secs(hours * 60 * 60);
        assert_eq!(check(&dir, "0.1.13", &registry.address).ask(later), None, "{hours} hours later");
    }
    assert_eq!(registry.questions(), 1, "one question in a day, however many starts");
    let tomorrow = morning() + Duration::from_secs(24 * 60 * 60);
    assert!(check(&dir, "0.1.13", &registry.address).ask(tomorrow).is_some());
    assert_eq!(registry.questions(), 2);
    std::fs::remove_dir_all(dir).ok();
    // A clock set back does not silence the question for good.
    let dir = scratch("clock");
    assert!(check(&dir, "0.1.13", &registry.address).ask(tomorrow).is_some());
    assert!(check(&dir, "0.1.13", &registry.address).ask(morning()).is_some(), "a remembered time ahead of now");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn with_the_ecosystem_s_notice_off_nothing_is_asked_or_written() {
    let dir = scratch("off");
    Ecosystem::QUVYTA.set_update_notice_in(&dir.join("config"), false).expect("turned off");
    let registry = Registry::listing(&[("0.1.14", false)]);
    assert_eq!(check(&dir, "0.1.13", &registry.address).ask(morning()), None);
    assert_eq!(registry.questions(), 0, "not a single request");
    assert!(!dir.join("state").exists(), "nothing remembered either");
    Ecosystem::QUVYTA.set_update_notice_in(&dir.join("config"), true).expect("turned on");
    assert!(check(&dir, "0.1.13", &registry.address).ask(morning()).is_some());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_index_is_read_where_cargo_reads_it() {
    assert_eq!(index_address("https://index.crates.io", "a"), "https://index.crates.io/1/a");
    assert_eq!(index_address("https://index.crates.io", "ab"), "https://index.crates.io/2/ab");
    assert_eq!(index_address("https://index.crates.io", "abc"), "https://index.crates.io/3/a/abc");
    assert_eq!(index_address("https://index.crates.io", "Quvyta-Code"), "https://index.crates.io/qu/vy/quvyta-code");
}

/// An application that asks at start and shows the notice, as a member of the ecosystem does.
struct Code;

#[derive(Debug)]
enum Msg {
    NewVersion(Update),
}

impl App for Code {
    type Msg = Msg;
    fn init(&mut self) -> Command<Msg> {
        Command::check_for_update(UpdateCheck::new(Ecosystem::QUVYTA, "code", "quvyta-code", "0.1.13", Msg::NewVersion))
    }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::NewVersion(update) => Command::toast(update.toast()),
        }
    }
    fn view(&self, _ui: &mut View<'_, Msg>) {}
}

#[test]
fn a_harness_records_the_question_and_shows_the_notice_for_the_version_it_is_told() {
    let mut h = Harness::new(Code, 70, 12);
    assert_eq!(h.update_checks().len(), 1, "asked once, at start");
    assert_eq!((h.update_checks()[0].package(), h.update_checks()[0].current()), ("quvyta-code", "0.1.13"));
    assert!(!h.screen().contains("is out"), "no answer, as without a network: {}", h.screen());
    h.set_latest_version(Some("0.1.13")).advance(Duration::from_millis(300));
    assert!(!h.screen().contains("is out"), "the version it runs: {}", h.screen());

    let mut h = Harness::new(Code, 70, 12);
    h.set_latest_version(Some("0.1.14")).advance(Duration::from_millis(300));
    let screen = h.screen();
    assert!(screen.contains("quvyta-code 0.1.14 is out"), "{screen}");
    assert!(screen.contains("You have 0.1.13"), "{screen}");
    assert!(screen.contains("in quvyta") && screen.contains("cargo install"), "how to update: {screen}");
}

/// Qcli while it is in alpha, asking at start and showing the notice.
struct Alpha;

impl App for Alpha {
    type Msg = Msg;
    fn init(&mut self) -> Command<Msg> {
        Command::check_for_update(UpdateCheck::new(
            Ecosystem::QUVYTA,
            "cli",
            "quvyta-cli",
            "0.1.0-alpha.1",
            Msg::NewVersion,
        ))
    }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::NewVersion(update) => Command::toast(update.toast()),
        }
    }
    fn view(&self, _ui: &mut View<'_, Msg>) {}
}

#[test]
fn an_application_in_alpha_shows_the_notice_for_the_next_alpha() {
    let mut h = Harness::new(Alpha, 70, 12);
    h.set_latest_version(Some("0.1.0-alpha.3")).advance(Duration::from_millis(300));
    let screen = h.screen();
    assert!(screen.contains("quvyta-cli 0.1.0-alpha.3 is out"), "{screen}");
    assert!(screen.contains("You have 0.1.0-alpha.1"), "{screen}");
}

/// Qcode as the terminal runtime runs it, asking a registry of the test's own that takes the
/// connection and never answers.
struct Waiting {
    dir: PathBuf,
    registry: String,
}

impl App for Waiting {
    type Msg = Msg;
    fn init(&mut self) -> Command<Msg> {
        let check = UpdateCheck::new(Ecosystem::QUVYTA, "code", "quvyta-code", "0.1.13", Msg::NewVersion)
            .in_folders(self.dir.join("config"), self.dir.join("state"))
            .registry(self.registry.clone());
        Command::check_for_update(check)
    }
    fn update(&mut self, _msg: Msg) -> Command<Msg> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(crate::widgets::Text::new("qcode"));
    }
}

#[test]
fn the_first_frame_never_waits_for_the_registry() {
    // The registry accepts the connection and says nothing, as a slow or broken network does; the
    // question gives up only after ten seconds. Asked on the loop, the first frame would wait
    // those ten seconds.
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let registry = format!("http://{}", listener.local_addr().expect("its address"));
    std::thread::spawn(move || {
        let held: Vec<_> = listener.incoming().take(1).collect();
        std::thread::sleep(Duration::from_secs(30));
        drop(held);
    });
    let dir = scratch("waiting");
    let app = Waiting { dir: dir.clone(), registry };
    let mut engine =
        super::super::engine::Engine::new(app, crate::env::Env::builtin(), super::super::engine::TaskMode::Threads);
    let mut buffer = ratatui_core::buffer::Buffer::empty(ratatui_core::layout::Rect::new(0, 0, 20, 2));
    let started = std::time::Instant::now();
    engine.render(&mut buffer, Duration::ZERO);
    let spent = started.elapsed();
    assert!(spent < Duration::from_secs(5), "the first frame took {spent:?}");
    std::fs::remove_dir_all(dir).ok();
}
