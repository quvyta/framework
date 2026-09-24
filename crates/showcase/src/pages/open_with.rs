//! Open with: a file's type from the desktop's shared MIME database, and the programs that open
//! it, from their `.desktop` files and the person's defaults. Everything is read from a folder of
//! this run, never from the system's own.

use std::path::{Path, PathBuf};

use qframe::desktop::{Choices, DesktopApp, Launched, Openers, XdgDirs};
use qframe::prelude::*;
use qframe::widgets::{Badge, RadioGroup};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "open-with";

/// The demo's MIME patterns: `*.ts` twice to show that the weight decides.
const GLOBS: &str = "# weight:type:pattern
50:text/x-rust:*.rs
50:text/plain:*.txt
50:text/markdown:*.md
50:application/gzip:*.gz
50:application/x-compressed-tar:*.tar.gz
50:image/png:*.png
50:image/jpeg:*.jpg
50:video/mp2t:*.ts
80:text/x-typescript:*.ts
";

/// Which type is a kind of which.
const SUBCLASSES: &str = "text/x-rust text/plain
text/markdown text/plain
text/x-typescript text/plain
application/x-compressed-tar application/gzip
";

/// What three of the demo's types are called in words, as `update-mime-database` writes them:
/// the path under `data/mime` and the file. Every other type has none, and the page says nothing.
const DESCRIPTIONS: [(&str, &str); 3] = [
    (
        "text/x-rust.xml",
        r#"<?xml version="1.0" encoding="utf-8"?>
<mime-type xmlns="http://www.freedesktop.org/standards/shared-mime-info" type="text/x-rust">
  <comment>Rust source code</comment>
  <comment xml:lang="tr">Rust kaynak kodu</comment>
</mime-type>
"#,
    ),
    (
        "text/markdown.xml",
        r#"<?xml version="1.0" encoding="utf-8"?>
<mime-type xmlns="http://www.freedesktop.org/standards/shared-mime-info" type="text/markdown">
  <comment>Markdown document</comment>
  <comment xml:lang="tr">Markdown belgesi</comment>
</mime-type>
"#,
    ),
    (
        "text/plain.xml",
        r#"<?xml version="1.0" encoding="utf-8"?>
<mime-type xmlns="http://www.freedesktop.org/standards/shared-mime-info" type="text/plain">
  <comment>Plain text document</comment>
  <comment xml:lang="tr">Düz metin belgesi</comment>
</mime-type>
"#,
    ),
];

/// A graphical editor for plain text.
const EDITOR: &str = "[Desktop Entry]
Type=Application
Name=Text Editor
Name[tr]=Metin Düzenleyici
Exec=editor --new-window %F
Icon=accessories-text-editor
Terminal=false
MimeType=text/plain;
";

/// A pager that runs in the terminal.
const PAGER: &str = "[Desktop Entry]
Type=Application
Name=Pager
Name[tr]=Sayfalayıcı
Exec=less %f
Terminal=true
MimeType=text/plain;text/markdown;
";

/// A graphical image viewer whose program name has a space in it.
const IMAGES: &str = "[Desktop Entry]
Type=Application
Name=Image Viewer
Name[tr]=Resim Gösterici
Exec=\"image viewer\" %U
MimeType=image/png;image/jpeg;
";

/// The person's choices: the pager for plain text, the editor added for Rust, and the image
/// viewer taken away from PNG.
const MIMEAPPS: &str = "[Default Applications]
text/plain=pager.desktop

[Added Associations]
text/x-rust=editor.desktop;

[Removed Associations]
image/png=images.desktop;
";

/// The demo's files: a name, and what is in it (`None` for a folder).
const FILES: [(&str, Option<&[u8]>); 7] = [
    ("main.rs", Some(b"fn main() {}\n")),
    ("notes.md", Some(b"# Notes\n")),
    ("holiday photo.jpg", Some(b"\xff\xd8\xff\xe0")),
    ("diagram.png", Some(b"\x89PNG\r\n")),
    ("LICENSE", Some(b"Permission is hereby granted\n")),
    ("data.bin", Some(b"\x00\x01\x02\x03")),
    ("pictures", None),
];

/// The demo's desktop: its folder and what was read from it, with the programs' names in English
/// and in Turkish.
#[derive(Debug)]
struct Demo {
    root: PathBuf,
    openers: [Openers; 2],
}

impl Demo {
    /// Writes the demo's desktop into a folder of this run and reads it.
    fn new() -> Result<Self, String> {
        let root = demo_dir();
        match Self::write(&root) {
            Ok(()) => {
                let openers = read(&root);
                Ok(Self { root, openers })
            }
            Err(problem) => {
                // Whatever was written before the failure goes too.
                let _ = std::fs::remove_dir_all(&root);
                Err(problem)
            }
        }
    }

    /// Writes the demo's MIME database, programs, choices and files under `root`.
    fn write(root: &Path) -> Result<(), String> {
        let write = |relative: &str, bytes: &[u8]| -> Result<(), String> {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::write(&path, bytes).map_err(|error| error.to_string())
        };
        write("data/mime/globs2", GLOBS.as_bytes())?;
        write("data/mime/subclasses", SUBCLASSES.as_bytes())?;
        for (path, xml) in DESCRIPTIONS {
            write(&format!("data/mime/{path}"), xml.as_bytes())?;
        }
        write("data/applications/editor.desktop", EDITOR.as_bytes())?;
        write("data/applications/pager.desktop", PAGER.as_bytes())?;
        write("data/applications/images.desktop", IMAGES.as_bytes())?;
        write("config/mimeapps.list", MIMEAPPS.as_bytes())?;
        for (name, bytes) in FILES {
            match bytes {
                Some(bytes) => write(&format!("files/{name}"), bytes)?,
                None => std::fs::create_dir_all(root.join("files").join(name)).map_err(|error| error.to_string())?,
            }
        }
        Ok(())
    }

    /// What was read, with the programs' names in `lang`, the language the showcase speaks.
    fn openers(&self, lang: &str) -> &Openers {
        &self.openers[usize::from(lang == "tr")]
    }

    /// The path of demo file `index`.
    fn file(&self, index: usize) -> PathBuf {
        self.root.join("files").join(FILES[index].0)
    }
}

/// Reads the kinds and the programs of the demo's folders, in both of the showcase's languages.
fn read(root: &Path) -> [Openers; 2] {
    // region: open-with-load
    // An application reads its own environment: `XdgDirs::from_env(|name| std::env::var(name).ok())`.
    let dirs =
        XdgDirs { data_home: Some(root.join("data")), config_home: Some(root.join("config")), ..XdgDirs::default() };
    // The language picks the programs' names; no PATH, since no demo program has a TryExec.
    let openers = ["en", "tr"].map(|lang| Openers::load(&dirs, lang, None));
    // endregion
    openers
}

/// Tells the demo folders of two pages in one process apart, which the tests need.
static DEMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A folder of this run for the demo's desktop, never one of the user's own.
fn demo_dir() -> PathBuf {
    let ticket = DEMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("quvyta-showcase-open-with-{}-{ticket}", std::process::id()))
}

/// The demo's desktop, the chosen file and what came of the last opening.
#[derive(Debug)]
pub struct State {
    demo: Result<Demo, String>,
    file: usize,
    graphical: bool,
    launched: Option<Result<Launched, String>>,
}

impl Default for State {
    fn default() -> Self {
        Self { demo: Demo::new(), file: 0, graphical: true, launched: None }
    }
}

impl Drop for State {
    /// Takes the demo folder away with the page, so a run leaves nothing in the temporary folder.
    fn drop(&mut self) {
        if let Ok(demo) = &self.demo {
            let _ = std::fs::remove_dir_all(&demo.root);
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    File(usize),
    Graphical(bool),
    Open,
    Done(Launched),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::OpenWith(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::File(index) if index < FILES.len() => {
            state.file = index;
            state.launched = None;
            log.push(PAGE, "Openers::for_file", FILES[index].0.to_owned());
        }
        Msg::File(_) => {}
        Msg::Graphical(on) => {
            state.graphical = on;
            state.launched = None;
            log.push(PAGE, "graphical_session", on.to_string());
        }
        Msg::Open => {
            let Ok(demo) = &state.demo else {
                return Command::none();
            };
            let path = demo.file(state.file);
            let choices = demo.openers("en").for_file(&path);
            let Some(app) = choices.default.map(|index| &choices.apps[index]) else {
                return Command::none();
            };
            // region: open-with-launch
            match app.launch(&path, state.graphical, |launched| send(Msg::Done(launched))) {
                Ok(command) => {
                    log.push(PAGE, "DesktopApp::launch", app.name.clone());
                    return command;
                }
                Err(error) => {
                    log.push(PAGE, "DesktopApp::launch", error.to_string());
                    state.launched = Some(Err(error.to_string()));
                }
            }
            // endregion
        }
        Msg::Done(launched) => {
            log.push(PAGE, "Launched", format!("{launched:?}"));
            state.launched = Some(Ok(launched));
        }
    }
    Command::none()
}

/// One line with a marker in its own column.
fn marked(ui: &mut View<'_, AppMsg>, marker: &str, color: &'static str, text: String, role: &'static str) {
    let glyph = ui.env().icons().glyph(marker).into_owned();
    ui.row(|ui| {
        ui.add(Text::new(glyph).color(color).no_wrap());
        ui.add(Text::new(text).role(role)).fill_width();
    })
    .gap(1);
}

/// A label and a value on one line.
fn fact(ui: &mut View<'_, AppMsg>, label: String, value: &str) {
    ui.add(Text::rich([Span::new(label).role("faint"), Span::new(format!("  {value}")).role("body")]).no_wrap());
}

/// One program: its name, whether it is the default, how it would start, and the command.
fn program(ui: &mut View<'_, AppMsg>, app: &DesktopApp, default: bool, graphical: bool, path: &Path) {
    // region: open-with-program
    let startable = app.can_start(graphical);
    let route = if !startable {
        t!("open-with.no-session")
    } else if app.terminal {
        t!("open-with.terminal")
    } else {
        t!("open-with.window")
    };
    let command = app.command(path).unwrap_or_default();
    // endregion
    ui.row(|ui| {
        ui.add(Text::new(app.name.as_str()).role(if startable { "body" } else { "faint" }).no_wrap());
        if default {
            ui.add(Badge::new(t!("open-with.default")).variant("accent"));
        }
        ui.add(Text::new(route).role("faint")).fill_width();
    })
    .gap(2);
    // Each argument in its own tone, so a file name with spaces is seen to stay one argument. The
    // demo's own folder is left out of the path: it says nothing and fills the line.
    let folder = path.parent().unwrap_or(path);
    let spans = command.iter().enumerate().flat_map(|(index, word)| {
        let role = if index == 0 {
            "accent"
        } else if index % 2 == 1 {
            "body"
        } else {
            "secondary"
        };
        let shown = Path::new(word).strip_prefix(folder).map_or_else(|_| word.as_os_str(), Path::as_os_str);
        [Span::new("  "), Span::new(shown.to_string_lossy().into_owned()).role(role)]
    });
    ui.add(Text::rich(spans));
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("open-with.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let lang = ui.env().i18n().active().to_owned();
        let demo = match &state.demo {
            Ok(demo) => demo,
            Err(problem) => {
                marked(ui, "error", "danger", problem.clone(), "body");
                return;
            }
        };
        ui.row(|ui| {
            let names = FILES.map(|(name, _)| name);
            ui.add(RadioGroup::new(names).selected(Some(state.file)).on_select(|index| send(Msg::File(index))))
                .width(Length::Cells(24))
                .id("file");
            ui.column(|ui| {
                let path = demo.file(state.file);
                let openers = demo.openers(&lang);
                // region: open-with-type
                let Choices { mime, apps, default } = openers.for_file(&path);
                let kinds = openers.mime.ancestors(&mime);
                // The type in words, in the language the page speaks: "Rust source code".
                let called = openers.mime.comment(&mime, &lang);
                // endregion
                let how = if path.is_dir() {
                    t!("open-with.is-folder")
                } else if openers.mime.guess(FILES[state.file].0).is_some() {
                    t!("open-with.by-name")
                } else {
                    t!("open-with.by-contents")
                };
                if let Some(called) = &called {
                    fact(ui, t!("open-with.called"), called);
                }
                fact(ui, t!("open-with.type"), &format!("{mime}  {how}"));
                let kinds = if kinds.len() > 1 { kinds[1..].join("  ") } else { t!("open-with.nothing-more") };
                fact(ui, t!("open-with.kind-of"), &kinds);
                ui.spacer().height(Length::Cells(1));
                ui.add(Text::new(t!("open-with.programs")).role("faint"));
                if apps.is_empty() {
                    marked(ui, "info", "muted", t!("open-with.none"), "secondary");
                }
                for (index, app) in apps.iter().enumerate() {
                    program(ui, app, default == Some(index), state.graphical, &path);
                }
                ui.spacer().height(Length::Cells(1));
                let can_open = default.is_some_and(|index| apps[index].can_start(state.graphical));
                ui.add(Button::new(t!("open-with.open")).disabled(!can_open).on_press(send(Msg::Open))).id("open");
                match &state.launched {
                    Some(Ok(Launched::Returned { code })) => {
                        let code = code.map_or_else(|| t!("open-with.signal"), |code| code.to_string());
                        marked(ui, "success", "success", t!("open-with.returned", code = code), "body");
                    }
                    Some(Ok(Launched::Started)) => marked(ui, "success", "success", t!("open-with.started"), "body"),
                    Some(Ok(Launched::Failed(reason))) => {
                        marked(ui, "error", "danger", t!("open-with.failed", reason = reason.clone()), "body");
                    }
                    Some(Err(reason)) => marked(ui, "warning", "warning", reason.clone(), "body"),
                    None => {}
                }
            })
            .fill_width();
        })
        .gap(2);
        for diagnostic in demo.openers(&lang).diagnostics() {
            marked(ui, "warning", "warning", diagnostic.to_string(), "secondary");
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("open-with.graphical"), |ui| {
            ui.add(toggle(state.graphical, |on| send(Msg::Graphical(on)))).id("graphical");
        });
        ui.add(Text::new(t!("open-with.graphical-hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_tall;

    type Page = qframe::runtime::Harness<crate::app::Showcase>;

    fn page() -> Page {
        showcase_tall(crate::app::Showcase::new(), PAGE, 60)
    }

    /// Picks demo file `index` the way a person does: by clicking its row in the list.
    fn pick(h: &mut Page, index: usize) {
        let name = FILES[index].0;
        assert!(h.find(name).is_some(), "the row of {name} is on screen:\n{}", h.screen());
        h.click_text(name);
        assert_eq!(h.app().pages.open_with.file, index, "clicking {name} picks it");
    }

    /// Presses the Open control.
    fn open(h: &mut Page) {
        h.click_text("Open with the default");
    }

    /// Flips the graphical-session switch, which sits after the playground's label column.
    fn flip_graphical(h: &mut Page) {
        let (x, y) = h.find("Graphical session").expect("the switch's label");
        h.click(x + 25, y);
    }

    #[test]
    fn rust_source_shows_its_type_and_the_programs_for_plain_text() {
        let h = page();
        let screen = h.screen();
        assert!(screen.contains("text/x-rust"), "{screen}");
        assert!(screen.contains("Rust source code"), "the type in words, from the demo's own file:\n{screen}");
        assert!(screen.contains("text/plain"), "the type it is a kind of:\n{screen}");
        for name in ["Pager", "Text Editor"] {
            assert!(screen.contains(name), "{name} is missing:\n{screen}");
        }
        assert!(screen.contains("default"), "the default is marked:\n{screen}");
        assert!(!screen.contains("Image Viewer"), "{screen}");
        assert!(!screen.contains("warning") && !screen.contains("error"), "the demo loads cleanly:\n{screen}");
    }

    #[test]
    fn a_spaced_name_is_one_argument_and_a_removed_program_is_gone() {
        let mut h = page();
        pick(&mut h, 2);
        let screen = h.screen();
        assert!(screen.contains("image/jpeg") && screen.contains("Image Viewer"), "{screen}");
        assert!(screen.contains("image viewer  holiday photo.jpg"), "the command, one argument each:\n{screen}");
        let demo = h.app().pages.open_with.demo.as_ref().expect("the demo");
        let path = demo.file(2);
        let app = demo.openers("en").apps.get("images.desktop").expect("the image viewer");
        let command = app.command(&path).expect("a command");
        assert_eq!(command, [std::ffi::OsString::from("image viewer"), path.into_os_string()]);
        pick(&mut h, 3);
        let screen = h.screen();
        assert!(screen.contains("image/png") && screen.contains("No program opens this type"), "{screen}");
    }

    #[test]
    fn a_name_nobody_knows_is_told_by_its_contents() {
        let mut h = page();
        for (index, expected) in [(4, "text/plain"), (5, "application/octet-stream"), (6, "inode/directory")] {
            pick(&mut h, index);
            let screen = h.screen();
            assert!(screen.contains(expected), "{expected} for file {index}:\n{screen}");
            let words = index == 4;
            assert_eq!(screen.contains("Plain text document"), words, "only plain text is described:\n{screen}");
            assert_eq!(screen.contains("Called"), words, "a type without words has no such line:\n{screen}");
        }
    }

    #[test]
    fn opening_is_recorded_and_nothing_runs() {
        let mut h = page();
        open(&mut h);
        let handoffs = h.handoffs();
        assert_eq!(handoffs.len(), 1, "the default for Rust source is the pager, a terminal program");
        assert_eq!(handoffs[0].program, "less");
        let files = h.app().pages.open_with.demo.as_ref().expect("the demo").root.join("files");
        assert_eq!(handoffs[0].dir.as_ref(), Some(&files), "the pager runs in the file's folder");
        assert!(h.screen().contains("came back"), "{}", h.screen());
        pick(&mut h, 2);
        open(&mut h);
        assert_eq!(h.opens().len(), 1, "a graphical program starts beside the application");
        assert_eq!(h.opens()[0].program, "image viewer");
        assert_eq!(h.opens()[0].dir.as_ref(), Some(&files), "so does the image viewer");
    }

    #[test]
    fn without_a_graphical_session_a_window_program_cannot_start() {
        let mut h = page();
        flip_graphical(&mut h);
        assert!(!h.app().pages.open_with.graphical, "the switch turns the session off");
        pick(&mut h, 2);
        let screen = h.screen();
        assert!(screen.contains("no graphical session"), "{screen}");
        open(&mut h);
        assert!(h.opens().is_empty(), "nothing is asked of the desktop");
        pick(&mut h, 0);
        assert!(h.screen().contains("in the terminal"), "a terminal program still can:\n{}", h.screen());
    }
}
