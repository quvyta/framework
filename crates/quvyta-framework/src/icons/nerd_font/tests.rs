use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Shell;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    move |name| map.get(name).cloned()
}

/// A folder of its own under the system's temporary folder, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("quvyta-nerd-test-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch folder");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Builds a plain tar archive holding `files` and returns its path and SHA-256, the way a
/// release would publish them.
fn fake_archive(scratch: &Path, files: &[(&str, &[u8])]) -> (PathBuf, String) {
    let content = scratch.join("content");
    fs::create_dir_all(&content).expect("content folder");
    for (name, bytes) in files {
        fs::write(content.join(name), bytes).expect("write archive member");
    }
    let archive = scratch.join("NerdFontsSymbolsOnly.tar");
    let names: Vec<&str> = files.iter().map(|(name, _)| *name).collect();
    let status =
        Shell::new("tar").arg("-cf").arg(&archive).arg("-C").arg(&content).args(&names).status().expect("run tar");
    assert!(status.success());
    let output = Shell::new("sha256sum").arg(&archive).output().expect("run sha256sum");
    let digest = String::from_utf8_lossy(&output.stdout).split_whitespace().next().expect("a digest").to_owned();
    (archive, digest)
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn run(install: Install) -> (Result<PathBuf, InstallError>, Vec<Progress>) {
    let mut seen = Vec::new();
    let result = install.run(&|| false, &mut |progress| seen.push(progress));
    (result, seen)
}

#[test]
fn target_folder_on_linux_honours_xdg_data_home() {
    let dir = target_dir_for(Os::Linux, env(&[("HOME", "/home/ada"), ("XDG_DATA_HOME", "/data/ada")]));
    assert_eq!(dir, Some(PathBuf::from("/data/ada/fonts/QuvytaNerdFont")));
    let dir = target_dir_for(Os::Linux, env(&[("HOME", "/home/ada")]));
    assert_eq!(dir, Some(PathBuf::from("/home/ada/.local/share/fonts/QuvytaNerdFont")));
    let relative = target_dir_for(Os::Linux, env(&[("HOME", "/home/ada"), ("XDG_DATA_HOME", "data")]));
    assert_eq!(relative, Some(PathBuf::from("/home/ada/.local/share/fonts/QuvytaNerdFont")), "relative is ignored");
    assert_eq!(target_dir_for(Os::Linux, env(&[("HOME", "home")])), None, "a relative home is no home");
    assert_eq!(target_dir_for(Os::Linux, env(&[])), None);
}

#[test]
fn target_folder_on_macos_and_windows() {
    let mac = target_dir_for(Os::Mac, env(&[("HOME", "/Users/ada")]));
    assert_eq!(mac, Some(PathBuf::from("/Users/ada/Library/Fonts")));
    let windows = target_dir_for(Os::Windows, env(&[("LOCALAPPDATA", "/c/Users/ada/AppData/Local")]));
    assert_eq!(windows, Some(Path::new("/c/Users/ada/AppData/Local").join("Microsoft").join("Windows").join("Fonts")));
    assert_eq!(target_dir_for(Os::Windows, env(&[("HOME", "/home/ada")])), None);
}

#[test]
fn each_system_downloads_the_archive_its_tar_can_open() {
    let unix = release_for(Os::Linux);
    assert!(unix.url().starts_with("https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/"));
    assert!(unix.url().ends_with("NerdFontsSymbolsOnly.tar.xz"));
    assert_eq!(release_for(Os::Mac), unix);
    let windows = release_for(Os::Windows);
    assert!(windows.url().ends_with("NerdFontsSymbolsOnly.zip"));
    for archive in [unix, windows] {
        assert_eq!(archive.sha256().len(), 64);
        assert!(archive.sha256().chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}

#[test]
fn installed_looks_for_a_nerd_font_file() {
    let scratch = Scratch::new("installed");
    assert!(!installed_in(std::slice::from_ref(&scratch.0)));
    let nested = scratch.0.join("QuvytaNerdFont");
    fs::create_dir_all(&nested).expect("nested");
    fs::write(nested.join(FONT_FILE), b"font").expect("font");
    assert!(installed_in(std::slice::from_ref(&scratch.0)));
}

#[test]
fn digests_are_read_from_every_tool() {
    let hex = "01172f37db8543edb102e5cb5c64101c9f4686630804d49b419aa07b23a69996";
    assert_eq!(steps::parse_digest(&format!("{hex}  /tmp/x.tar.xz")).as_deref(), Some(hex));
    let certutil = format!(
        "SHA256 hash of C:\\x.zip:\n{}\nCertUtil: -hashfile command completed successfully.",
        hex.to_uppercase()
    );
    assert_eq!(steps::parse_digest(&certutil).as_deref(), Some(hex));
    let spaced = hex.as_bytes().chunks(2).map(|pair| String::from_utf8_lossy(pair)).collect::<Vec<_>>().join(" ");
    assert_eq!(steps::parse_digest(&format!("header\n{spaced}\n")).as_deref(), Some(hex));
    assert_eq!(steps::parse_digest("sha256sum: missing: No such file or directory"), None);
}

#[test]
fn curl_progress_frames_become_fractions() {
    let fraction = steps::parse_percent("######                       21.7%").expect("a share");
    assert!((fraction - 0.217).abs() < 1e-6, "{fraction}");
    assert_eq!(steps::parse_percent("#################### 100.0%"), Some(1.0));
    assert_eq!(steps::parse_percent("curl: (22) The requested URL returned error: 404"), None);
}

#[test]
fn installs_from_a_verified_archive_and_reports_each_step() {
    let scratch = Scratch::new("install");
    let (archive, digest) = fake_archive(&scratch.0, &[(FONT_FILE, b"glyphs"), ("LICENSE", b"MIT")]);
    let target = scratch.0.join("fonts").join("QuvytaNerdFont");
    let install = Install::new().archive(Archive::new(file_url(&archive), &digest)).target(&target).register(false);
    let (result, seen) = run(install);
    let path = result.expect("installed");
    assert_eq!(fs::read(target.join(FONT_FILE)).expect("font copied"), b"glyphs");
    assert_eq!(fs::read_dir(&target).expect("target").count(), 1, "only the font file, no leftovers");
    let expected = if cfg!(target_os = "linux") { target.clone() } else { target.join(FONT_FILE) };
    assert_eq!(path, expected);
    assert!(matches!(seen.first(), Some(Progress::Downloading { .. })), "{seen:?}");
    let steps: Vec<&Progress> = seen.iter().filter(|p| !matches!(p, Progress::Downloading { .. })).collect();
    assert_eq!(steps, vec![&Progress::Verifying, &Progress::Installing, &Progress::Done { path: expected }]);
    assert!(installed_in(std::slice::from_ref(&scratch.0.join("fonts"))));
}

#[test]
fn a_wrong_checksum_deletes_the_download_and_installs_nothing() {
    let scratch = Scratch::new("checksum");
    let (archive, _) = fake_archive(&scratch.0, &[(FONT_FILE, b"glyphs")]);
    let wrong = "0".repeat(64);
    let target = scratch.0.join("target");
    let mut install = Install::new().archive(Archive::new(file_url(&archive), &wrong)).target(&target).register(false);
    let staging = scratch.0.join("staging");
    fs::create_dir_all(&staging).expect("staging root");
    install.staging.clone_from(&staging);
    let (result, seen) = run(install);
    let error = result.expect_err("a wrong checksum fails");
    assert!(matches!(&error, InstallError::Checksum { expected, .. } if *expected == wrong), "{error:?}");
    assert_eq!(seen.last(), Some(&Progress::Failed(error)));
    assert!(!target.exists(), "nothing is installed");
    assert!(archive.exists(), "the source itself is left alone");
    let leftovers = fs::read_dir(&staging).expect("staging root").count();
    assert_eq!(leftovers, 0, "the download is deleted");
}

#[test]
fn an_archive_without_the_font_fails_to_extract() {
    let scratch = Scratch::new("extract");
    let (archive, digest) = fake_archive(&scratch.0, &[("README.md", b"no font here")]);
    let target = scratch.0.join("target");
    let install = Install::new().archive(Archive::new(file_url(&archive), &digest)).target(&target).register(false);
    let (result, _) = run(install);
    assert!(matches!(result, Err(InstallError::Extract(_))), "{result:?}");
    assert!(!target.join(FONT_FILE).exists());
}

#[test]
fn a_missing_download_reports_curl_s_reason() {
    let scratch = Scratch::new("download");
    let missing = scratch.0.join("nowhere.tar.xz");
    let install = Install::new()
        .archive(Archive::new(file_url(&missing), &"0".repeat(64)))
        .target(scratch.0.join("target"))
        .register(false);
    let (result, seen) = run(install);
    let Err(InstallError::Download(reason)) = &result else { panic!("{result:?}") };
    assert!(reason.contains("curl"), "{reason}");
    assert!(matches!(seen.last(), Some(Progress::Failed(InstallError::Download(_)))));
}

#[test]
fn no_folder_to_install_into_is_an_error() {
    let mut install = Install::new().register(false);
    install.target = None;
    let (result, _) = run(install);
    assert_eq!(result, Err(InstallError::NoFolder));
}

#[test]
fn cancelling_stops_before_anything_is_written() {
    let scratch = Scratch::new("cancel");
    let (archive, digest) = fake_archive(&scratch.0, &[(FONT_FILE, b"glyphs")]);
    let target = scratch.0.join("target");
    let install = Install::new().archive(Archive::new(file_url(&archive), &digest)).target(&target).register(false);
    let result = install.run(&|| true, &mut |_| {});
    assert_eq!(result, Err(InstallError::Cancelled));
    assert!(!target.exists());
}

#[test]
fn texts_come_from_the_locale_files() {
    let i18n = std::sync::Arc::new(crate::i18n::I18n::builtin());
    crate::i18n::scope(i18n, || {
        assert_eq!(Progress::Verifying.text(), "Checking the download");
        let done = Progress::Done { path: PathBuf::from("/fonts/QuvytaNerdFont") }.text();
        assert!(done.contains("/fonts/QuvytaNerdFont"), "{done}");
        let failed = Progress::Failed(InstallError::MissingTool("curl".into())).text();
        assert!(failed.contains("curl") && !failed.contains('⟦'), "{failed}");
        let after = after_install_text();
        assert!(after.contains("JetBrainsMono Nerd Font"), "{after}");
        assert!(!status_text(true).contains('⟦') && !status_text(false).contains('⟦'));
    });
}

#[test]
fn every_language_has_every_install_text() {
    const KEYS: [&str; 18] = [
        "quvyta.nerd-font.task",
        "quvyta.nerd-font.downloading",
        "quvyta.nerd-font.downloading-share",
        "quvyta.nerd-font.verifying",
        "quvyta.nerd-font.installing",
        "quvyta.nerd-font.done",
        "quvyta.nerd-font.cancelled",
        "quvyta.nerd-font.error-folder",
        "quvyta.nerd-font.error-tool",
        "quvyta.nerd-font.error-download",
        "quvyta.nerd-font.error-verify",
        "quvyta.nerd-font.error-checksum",
        "quvyta.nerd-font.error-extract",
        "quvyta.nerd-font.error-copy",
        "quvyta.nerd-font.error-register",
        "quvyta.nerd-font.after-install",
        "quvyta.nerd-font.found",
        "quvyta.nerd-font.missing",
    ];
    let i18n = crate::i18n::I18n::builtin();
    for (code, _) in crate::assets::LOCALES {
        for key in KEYS {
            assert!(i18n.has(code, key), "{code} lacks {key}");
        }
    }
}

mod in_the_runtime {
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness, TaskEvent, TaskOutcome, Tasks};
    use crate::widget::View;

    #[derive(Debug, Clone)]
    enum Msg {
        Install,
        Progress(Progress),
        Event(TaskEvent),
    }

    struct Demo {
        install: Option<Install>,
        seen: Vec<Progress>,
        tasks: Tasks,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Install => {
                    let install = self.install.take().expect("one install");
                    return Command::task(install.task(Msg::Progress).on_event(Msg::Event));
                }
                Msg::Progress(progress) => self.seen.push(progress),
                Msg::Event(event) => self.tasks.apply(&event),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            if let Some(progress) = self.seen.last() {
                ui.add(crate::widgets::Text::new(progress.text()));
            }
        }
    }

    #[test]
    fn the_task_delivers_progress_and_a_translated_failure() {
        let scratch = Scratch::new("task");
        let (archive, _) = fake_archive(&scratch.0, &[(FONT_FILE, b"glyphs")]);
        let install = Install::new()
            .archive(Archive::new(file_url(&archive), &"f".repeat(64)))
            .target(scratch.0.join("target"))
            .register(false);
        let mut h = Harness::new(Demo { install: Some(install), seen: Vec::new(), tasks: Tasks::new() }, 90, 2);
        h.send(Msg::Install);
        h.advance(Duration::from_millis(0));
        let app = h.app();
        assert!(matches!(app.seen.last(), Some(Progress::Failed(InstallError::Checksum { .. }))), "{:?}", app.seen);
        let entry = &app.tasks.entries()[0];
        assert_eq!(entry.label, "Installing Symbols Nerd Font Mono");
        let Some(TaskOutcome::Failed(reason)) = &entry.outcome else { panic!("{:?}", entry.outcome) };
        assert!(reason.contains("checksum") && !reason.contains('⟦'), "{reason}");
        assert!(h.screen().contains("checksum"), "{}", h.screen());
    }

    #[test]
    fn the_task_installs_into_the_given_folder() {
        let scratch = Scratch::new("task-ok");
        let (archive, digest) = fake_archive(&scratch.0, &[(FONT_FILE, b"glyphs")]);
        let target = scratch.0.join("target");
        let install = Install::new().archive(Archive::new(file_url(&archive), &digest)).target(&target).register(false);
        let mut h = Harness::new(Demo { install: Some(install), seen: Vec::new(), tasks: Tasks::new() }, 90, 2);
        h.send(Msg::Install);
        h.advance(Duration::from_millis(0));
        assert!(matches!(h.app().seen.last(), Some(Progress::Done { .. })), "{:?}", h.app().seen);
        assert_eq!(h.app().tasks.entries()[0].outcome, Some(TaskOutcome::Done));
        assert!(target.join(FONT_FILE).exists());
    }
}

/// Downloads the real release into a temporary folder and checks it against the checksum
/// written in the source. Needs the network, so it runs only when asked for.
#[test]
#[ignore = "downloads from github.com"]
fn the_real_release_matches_its_checksum() {
    let scratch = Scratch::new("network");
    let target = scratch.0.join("QuvytaNerdFont");
    let (result, _) = run(Install::new().target(&target).register(false));
    result.expect("the release downloads, verifies and unpacks");
    assert!(fs::metadata(target.join(FONT_FILE)).expect("font").len() > 1_000_000);
}
