//! The steps of an install, each done by one of the system's programs through
//! [`Process`]: `curl` downloads, a checksum program verifies, `tar` unpacks, and `fc-cache` or
//! `reg` tell the system.

use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use super::{FAMILY, FONT_FILE, Install, InstallError, Os, Progress};
use crate::i18n::translate_active;
use crate::runtime::{Line, Process, ProcessOutcome};

/// Every key a task translates before it starts, since its thread has no language.
const TEXT_KEYS: [&str; 12] = [
    "quvyta.nerd-font.downloading",
    "quvyta.nerd-font.verifying",
    "quvyta.nerd-font.installing",
    "quvyta.nerd-font.cancelled",
    "quvyta.nerd-font.error-folder",
    "quvyta.nerd-font.error-tool",
    "quvyta.nerd-font.error-download",
    "quvyta.nerd-font.error-verify",
    "quvyta.nerd-font.error-checksum",
    "quvyta.nerd-font.error-extract",
    "quvyta.nerd-font.error-copy",
    "quvyta.nerd-font.error-register",
];

/// Stands in for `{detail}` while a text is translated ahead of time; no translation contains it.
const DETAIL: &str = "\u{1}detail\u{1}";

/// Texts translated on the application's thread, for a task's thread to fill in later.
pub(super) struct Texts(Vec<(&'static str, String)>);

impl Texts {
    pub(super) fn capture() -> Self {
        Self(TEXT_KEYS.iter().map(|key| (*key, translate_active(key, &[("detail", DETAIL.into())]))).collect())
    }

    /// The text of `key` with `detail` in place of `{detail}`.
    pub(super) fn get(&self, key: &str, detail: &str) -> String {
        self.0
            .iter()
            .find(|(known, _)| *known == key)
            .map_or_else(|| format!("⟦{key}⟧"), |(_, text)| text.replace(DETAIL, detail))
    }
}

/// A folder of its own for the download and the unpacked archive, removed when dropped.
struct Staging(PathBuf);

impl Staging {
    fn new(root: &Path) -> Result<Self, InstallError> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = root.join(format!("quvyta-nerd-font-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).map_err(|error| InstallError::Copy(error.to_string()))?;
        Ok(Self(dir))
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs every step; the caller reports the outcome.
pub(super) fn run(
    install: &Install,
    os: Os,
    cancel: &dyn Fn() -> bool,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<PathBuf, InstallError> {
    let target = install.target.clone().ok_or(InstallError::NoFolder)?;
    let stop = || if cancel() { Err(InstallError::Cancelled) } else { Ok(()) };
    stop()?;
    let staging = Staging::new(&install.staging)?;
    let name = install.archive.url.rsplit('/').next().filter(|name| !name.is_empty()).unwrap_or("archive");
    let archive = staging.0.join(name);

    on_progress(Progress::Downloading { fraction: None });
    download(os, &install.archive.url, &archive, cancel, on_progress)?;
    stop()?;

    on_progress(Progress::Verifying);
    let actual = sha256(os, &archive, cancel)?;
    if actual != install.archive.sha256 {
        // The staging folder goes too, but a download that failed its check is deleted first.
        let _ = fs::remove_file(&archive);
        return Err(InstallError::Checksum { expected: install.archive.sha256.clone(), actual });
    }
    stop()?;

    on_progress(Progress::Installing);
    let unpacked = staging.0.join("unpacked");
    fs::create_dir_all(&unpacked).map_err(|error| InstallError::Copy(error.to_string()))?;
    extract(os, &archive, &unpacked, cancel)?;
    stop()?;
    let font = copy_font(&unpacked.join(FONT_FILE), &target)?;
    if install.register {
        register(os, &target, &font, cancel)?;
    }
    Ok(if os == Os::Linux { target } else { font })
}

/// A program of Windows' own `System32` folder, so a `tar` or `curl` another package put first
/// on the `PATH` (Git for Windows ships GNU tar, which cannot open zip files) is not taken.
fn program(os: Os, name: &str) -> OsString {
    if os == Os::Windows
        && let Some(root) = std::env::var_os("SystemRoot")
    {
        let path = Path::new(&root).join("System32").join(format!("{name}.exe"));
        if path.is_file() {
            return path.into_os_string();
        }
    }
    OsString::from(name)
}

/// What a finished program left: its exit code and the lines it wrote.
struct Ran {
    code: Option<i32>,
    out: Vec<String>,
    err: Vec<String>,
}

impl Ran {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }

    /// The program's own explanation: its last error lines, or its exit code.
    fn reason(&self, name: &str) -> String {
        let lines: Vec<&str> =
            self.err.iter().chain(&self.out).map(|line| line.trim()).filter(|line| !line.is_empty()).collect();
        match lines.last() {
            Some(line) => (*line).to_owned(),
            None => format!("{name}: {}", self.code.map_or_else(|| "signal".to_owned(), |code| code.to_string())),
        }
    }
}

/// Runs `process` to its end. A missing program is `MissingTool(name)`.
fn finish(process: Process, name: &str, cancel: &dyn Fn() -> bool) -> Result<Ran, InstallError> {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let outcome = process.no_stdin().run(cancel, &mut |line| match line {
        Line::Out(text) => out.push(text),
        Line::Err(text) => err.push(text),
    });
    match outcome {
        Ok(ProcessOutcome::Finished { code }) => Ok(Ran { code, out, err }),
        Ok(ProcessOutcome::Cancelled) => Err(InstallError::Cancelled),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(InstallError::MissingTool(name.to_owned())),
        Err(error) => Err(InstallError::Copy(format!("{name}: {error}"))),
    }
}

fn download(
    os: Os,
    url: &str,
    to: &Path,
    cancel: &dyn Fn() -> bool,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<(), InstallError> {
    let process = Process::new(program(os, "curl"))
        .args(["--fail", "--location", "--show-error", "--progress-bar", "--output"])
        .arg(to)
        .arg(url)
        // The bar is only parsed, so a short one is enough.
        .env("COLUMNS", "40")
        .no_stdin();
    // Both callbacks report progress, so the state they share sits in a cell.
    let state = RefCell::new((None, Vec::new(), on_progress));
    let frame = |text: String| {
        let mut state = state.borrow_mut();
        let (last, err, on_progress) = &mut *state;
        match parse_percent(&text) {
            Some(fraction) if *last != Some(fraction) => {
                *last = Some(fraction);
                on_progress(Progress::Downloading { fraction: Some(fraction) });
            }
            Some(_) => {}
            None => err.push(text),
        }
    };
    let outcome = process.run_with_overwritten(
        cancel,
        &mut |line| {
            let (Line::Err(text) | Line::Out(text)) = line;
            frame(text);
        },
        &mut |line| {
            let (Line::Err(text) | Line::Out(text)) = line;
            frame(text);
        },
    );
    let (_, err, _) = state.into_inner();
    let code = match outcome {
        Ok(ProcessOutcome::Finished { code }) => code,
        Ok(ProcessOutcome::Cancelled) => return Err(InstallError::Cancelled),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(InstallError::MissingTool("curl".into())),
        Err(error) => return Err(InstallError::Download(format!("curl: {error}"))),
    };
    let ran = Ran { code, out: Vec::new(), err };
    if ran.ok() { Ok(()) } else { Err(InstallError::Download(ran.reason("curl"))) }
}

/// The share a `curl --progress-bar` frame shows, such as `### 21.7%`.
pub(super) fn parse_percent(text: &str) -> Option<f32> {
    let word = text.split_whitespace().last()?.strip_suffix('%')?;
    let percent: f32 = word.parse().ok()?;
    Some((percent / 100.0).clamp(0.0, 1.0))
}

/// The SHA-256 of `file`, from the first checksum program this system has.
fn sha256(os: Os, file: &Path, cancel: &dyn Fn() -> bool) -> Result<String, InstallError> {
    let candidates: Vec<(&str, Process)> = match os {
        Os::Windows => {
            vec![("certutil", Process::new(program(os, "certutil")).arg("-hashfile").arg(file).arg("SHA256"))]
        }
        // macOS has shasum (Perl) and, since 14, sha256sum; most Linux systems have both.
        Os::Mac => vec![
            ("shasum", Process::new("shasum").args(["-a", "256"]).arg(file)),
            ("sha256sum", Process::new("sha256sum").arg(file)),
        ],
        Os::Linux => vec![
            ("sha256sum", Process::new("sha256sum").arg(file)),
            ("shasum", Process::new("shasum").args(["-a", "256"]).arg(file)),
        ],
    };
    let first = candidates.first().map_or("sha256sum", |(name, _)| *name).to_owned();
    for (name, process) in candidates {
        let ran = match finish(process, name, cancel) {
            Err(InstallError::MissingTool(_)) => continue,
            other => other?,
        };
        if !ran.ok() {
            return Err(InstallError::Verify(ran.reason(name)));
        }
        return parse_digest(&ran.out.join("\n")).ok_or_else(|| InstallError::Verify(ran.reason(name)));
    }
    Err(InstallError::MissingTool(first))
}

/// Finds a SHA-256 in a checksum program's output: the first word of `sha256sum` and `shasum`,
/// or the line of `certutil`, whose older versions put a space between the bytes.
pub(super) fn parse_digest(output: &str) -> Option<String> {
    let is_digest = |text: &str| text.len() == 64 && text.chars().all(|c| c.is_ascii_hexdigit());
    output.lines().find_map(|line| {
        let first = line.split_whitespace().next().unwrap_or_default();
        let joined: String = line.split_whitespace().collect();
        [first, joined.as_str()].into_iter().find(|text| is_digest(text)).map(str::to_ascii_lowercase)
    })
}

/// Unpacks only the font file into `into`.
fn extract(os: Os, archive: &Path, into: &Path, cancel: &dyn Fn() -> bool) -> Result<(), InstallError> {
    let process = Process::new(program(os, "tar")).arg("-xf").arg(archive).arg("-C").arg(into).arg(FONT_FILE);
    let ran = finish(process, "tar", cancel)?;
    if !ran.ok() {
        return Err(InstallError::Extract(ran.reason("tar")));
    }
    if !into.join(FONT_FILE).is_file() {
        return Err(InstallError::Extract(FONT_FILE.to_owned()));
    }
    Ok(())
}

/// Copies the font into `target` under a temporary name and renames it into place, so the
/// folder never holds half a font. Returns the installed file.
fn copy_font(font: &Path, target: &Path) -> Result<PathBuf, InstallError> {
    let copy = |error: io::Error| InstallError::Copy(error.to_string());
    fs::create_dir_all(target).map_err(copy)?;
    let partial = target.join(format!(".{FONT_FILE}.part"));
    let installed = target.join(FONT_FILE);
    fs::copy(font, &partial).map_err(copy)?;
    if let Err(error) = fs::rename(&partial, &installed) {
        let _ = fs::remove_file(&partial);
        return Err(copy(error));
    }
    Ok(installed)
}

/// Tells the system about the new font where it needs telling.
fn register(os: Os, target: &Path, font: &Path, cancel: &dyn Fn() -> bool) -> Result<(), InstallError> {
    let (name, process) = match os {
        Os::Mac => return Ok(()),
        Os::Linux => ("fc-cache", Process::new("fc-cache").arg("-f").arg(target)),
        Os::Windows => (
            "reg",
            Process::new(program(os, "reg"))
                .args(["add", r"HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts", "/v"])
                .arg(format!("{FAMILY} Regular (TrueType)"))
                .args(["/t", "REG_SZ", "/d"])
                .arg(font)
                .arg("/f"),
        ),
    };
    let ran = match finish(process, name, cancel) {
        // Without fontconfig there is no cache to refresh; the font is in place all the same.
        Err(InstallError::MissingTool(_)) if os == Os::Linux => return Ok(()),
        other => other?,
    };
    if ran.ok() { Ok(()) } else { Err(InstallError::Register(ran.reason(name))) }
}
