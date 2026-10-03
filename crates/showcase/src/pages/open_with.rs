//! Open with: a file's type from the desktop's shared MIME database, and the programs that open
//! it, from their `.desktop` files and the person's defaults. Everything is read from a folder of
//! this run, never from the system's own.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use qframe::desktop::{
    Apps, Change, Choices, DesktopApp, EntryDetails, Include, Launched, MimeDb, Openers, XdgDirs, set_default,
};
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

/// A graphical editor for plain text, saying in its entry what a launcher lists and searches it
/// by, in English and in Turkish.
const EDITOR: &str = "[Desktop Entry]
Type=Application
Name=Text Editor
Name[tr]=Metin Düzenleyici
GenericName=Editor
GenericName[tr]=Düzenleyici
Comment=Edit text and notes in plain text
Comment[tr]=Düz metni ve notları düzenle
Keywords=editor;text;plain text;
Keywords[tr]=düzenleyici;metin;
Categories=Utility;TextEditor;
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
Comment=Read a long text a screen at a time
Comment[tr]=Uzun bir metni ekran ekran oku
Keywords=reader;viewer;
Categories=Utility;
Path=/home/ada/Documents
Exec=less %f
Terminal=true
MimeType=text/plain;text/markdown;
";

/// A graphical image viewer whose program name has a space in it.
const IMAGES: &str = "[Desktop Entry]
Type=Application
Name=Image Viewer
Name[tr]=Resim Gösterici
Comment=Look at a picture
Comment[tr]=Bir resme bak
Keywords=picture;photo;
Categories=Graphics;Viewer;
Exec=\"image viewer\" %U
MimeType=image/png;image/jpeg;
";

/// A photo program that is left out of the menus and whose own program is not on this machine:
/// its entry is there and a launcher lists it, saying that it cannot start it yet.
const PRO: &str = "[Desktop Entry]
Type=Application
Name=Pro Photos
Name[tr]=Pro Photos
Comment=Develop and print photos
Comment[tr]=Fotoğrafları geliştir ve bas
Keywords=photo;raw;print;
Categories=Graphics;Photography;
NoDisplay=true
TryExec=pro-photo-editor
Exec=pro-photo-editor %U
MimeType=image/jpeg;
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

/// The demo's desktop: its folder, the folders its files are read from, and what was read from
/// them, with the programs' names in English and in Turkish.
#[derive(Debug)]
struct Demo {
    root: PathBuf,
    dirs: XdgDirs,
    openers: [Openers; 2],
}

impl Demo {
    /// Writes the demo's desktop into a folder of this run and reads it.
    fn new() -> Result<Self, String> {
        let root = demo_dir();
        let dirs = read_dirs(&root);
        match Self::write(&root) {
            Ok(()) => {
                let openers = read(&dirs);
                Ok(Self { root, dirs, openers })
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
        write("data/applications/pro.desktop", PRO.as_bytes())?;
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

    /// Reads the demo's desktop again, after one of its files has changed. The list of programs and
    /// the default beside them are the file's own, not a copy of it kept somewhere else.
    fn reload(&mut self) {
        self.openers = read(&self.dirs);
    }

    /// The path of demo file `index`.
    fn file(&self, index: usize) -> PathBuf {
        self.root.join("files").join(FILES[index].0)
    }
}

/// The folders the demo's own desktop is read from and its `mimeapps.list` is written in: both
/// inside the folder of this run, never the machine's own.
fn read_dirs(root: &Path) -> XdgDirs {
    // region: open-with-load
    // An application reads its own environment: `XdgDirs::from_env(|name| std::env::var(name).ok())`.
    XdgDirs { data_home: Some(root.join("data")), config_home: Some(root.join("config")), ..XdgDirs::default() }
    // endregion
}

/// Reads the kinds and the programs of the demo's folders, in both of the showcase's languages.
fn read(dirs: &XdgDirs) -> [Openers; 2] {
    ["en", "tr"].map(|lang| {
        // region: open-with-keep
        // `Openers::load` drops a program whose own program is not installed; a launcher lists it
        // anyway, so it can say so and offer the package, and reads the two halves itself. The
        // language picks the names, comments and keywords; no search path, so the demo program of
        // `pro.desktop` is nowhere to be found, which is the point of it.
        let apps = Apps::load_including(dirs, lang, None, Include::MISSING);
        Openers { mime: MimeDb::load(dirs), apps }
        // endregion
    })
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
    /// Which of the chosen file's programs is to become the default one.
    chosen: usize,
    /// What the last attempt to write the default program came to: the change and the program it
    /// was made for, or the reason it came to nothing.
    made: Option<Result<(Change, String), String>>,
}

impl Default for State {
    fn default() -> Self {
        Self { demo: Demo::new(), file: 0, graphical: true, launched: None, chosen: 0, made: None }
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
    /// Which of the chosen file's programs is to become the default one.
    Choose(usize),
    /// Make the chosen program the default program of the chosen file's type.
    Default,
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
            state.chosen = 0;
            state.made = None;
            log.push(PAGE, "Openers::for_file", FILES[index].0.to_owned());
        }
        Msg::File(_) => {}
        Msg::Choose(index) => {
            state.chosen = index;
            state.made = None;
        }
        Msg::Default => {
            let Ok(demo) = &state.demo else {
                return Command::none();
            };
            let choices = demo.openers("en").for_file(&demo.file(state.file));
            let Some(app) = choices.apps.get(state.chosen) else {
                return Command::none();
            };
            let (mime, id, name) = (choices.mime.clone(), app.id.clone(), app.name.clone());
            // region: open-with-default
            let made = set_default(&demo.dirs, &mime, &id);
            // endregion
            // The program is remembered with the change rather than looked up in the list again:
            // the list behind the answer is read afresh and may come back in another order.
            state.made = Some(made.map(|change| (change, name.clone())).map_err(|error| error.to_string()));
            if let Ok(demo) = &mut state.demo {
                demo.reload();
            }
            log.push(PAGE, "set_default", name);
        }
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
    arguments(ui, &command, path.parent().unwrap_or(path));
}

/// The programs whose own program is not on this machine. They open nothing, so they are in no
/// file's list above; a launcher still names them, so the person can be offered their package.
fn not_installed(ui: &mut View<'_, AppMsg>, apps: &Apps) {
    let missing: Vec<&DesktopApp> =
        apps.all().iter().filter(|app| apps.details(&app.id).is_some_and(|entry| !entry.installed)).collect();
    if missing.is_empty() {
        return;
    }
    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(t!("open-with.not-installed-yet")).role("faint"));
    for app in missing {
        ui.row(|ui| {
            ui.add(Text::new(app.name.as_str()).role("faint").no_wrap());
            ui.add(Text::new(t!("open-with.not-installed")).role("secondary")).fill_width();
        })
        .gap(2);
    }
}

/// Each argument in its own tone, so a file name with spaces is seen to stay one argument. The
/// demo's own folder is left out of the path: it says nothing and fills the line.
fn arguments(ui: &mut View<'_, AppMsg>, command: &[OsString], folder: &Path) {
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

/// What the entry of one program says besides its command, each line one of the entry's own keys
/// so the label says which one it was read from, and the command the program starts with when it
/// is given nothing to open.
fn entry(ui: &mut View<'_, AppMsg>, app: &DesktopApp, details: &EntryDetails, root: &Path) {
    // region: open-with-details
    // What the entry says besides the command, read in the same pass as the command itself, so a
    // launcher never reads the same file a second time with a parser of its own.
    let words = details.keywords.join(";");
    let categories = details.categories.join(";");
    // endregion
    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(t!("open-with.says")).role("faint"));
    if let Some(comment) = details.comment.as_deref() {
        fact(ui, t!("open-with.comment"), comment);
    }
    if let Some(kind) = details.generic_name.as_deref() {
        fact(ui, t!("open-with.kind"), kind);
    }
    if !words.is_empty() {
        fact(ui, t!("open-with.words"), &words);
    }
    if !categories.is_empty() {
        fact(ui, t!("open-with.categories"), &categories);
    }
    if let Some(folder) = details.folder.as_deref() {
        fact(ui, t!("open-with.folder"), &folder.display().to_string());
    }
    if details.no_display {
        fact(ui, t!("open-with.menu"), &t!("open-with.left-out"));
    }
    // region: open-with-launch-command
    // What a launcher's own list, a favourite or a panel item runs: the same line with no file,
    // so the codes that take one are gone and nothing is added in their place.
    let starts = app.launch_command().unwrap_or_default();
    // endregion
    if !starts.is_empty() {
        ui.add(Text::new(t!("open-with.starts")).role("faint"));
        arguments(ui, &starts, root);
    }
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let lang = ui.env().i18n().active().to_owned();
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("open-with.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
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
                not_installed(ui, &openers.apps);
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
                if let Some(app) = default.map(|index| &apps[index])
                    && let Some(details) = openers.apps.details(&app.id)
                {
                    entry(ui, app, details, &demo.root);
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
        let Ok(demo) = &state.demo else {
            return;
        };
        let choices = demo.openers(&lang).for_file(&demo.file(state.file));
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("open-with.as-default")).role("faint"));
        if choices.apps.is_empty() {
            ui.add(Text::new(t!("open-with.none-to-choose")).role("secondary"));
            return;
        }
        let chosen = state.chosen.min(choices.apps.len() - 1);
        let names = choices.apps.iter().map(|app| app.name.as_str());
        ui.add(RadioGroup::new(names).selected(Some(chosen)).on_select(|index| send(Msg::Choose(index))))
            .id("default-program");
        ui.add(Button::new(t!("open-with.make-default", kind = choices.mime.clone())).on_press(send(Msg::Default)))
            .id("make-default");
        match &state.made {
            Some(Ok((Change::Added, program))) => {
                marked(
                    ui,
                    "success",
                    "success",
                    t!("open-with.line-added", program = program.clone(), kind = choices.mime.clone()),
                    "body",
                );
            }
            Some(Ok((Change::Replaced, program))) => {
                marked(
                    ui,
                    "success",
                    "success",
                    t!("open-with.line-replaced", program = program.clone(), kind = choices.mime.clone()),
                    "body",
                );
            }
            // A kind of change the framework adds later has no words on this page yet.
            Some(Ok(_)) => {}
            Some(Err(reason)) => marked(ui, "warning", "warning", reason.clone(), "body"),
            None => {}
        }
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{click_text_below, showcase_tall};

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

    /// Chooses program `index` of the file on screen as the one to make the default, by clicking
    /// its row in the playground's own group. The same name stands in the panel above, so the click
    /// starts under the playground's own heading.
    fn choose(h: &mut Page, index: usize) {
        let name = {
            let page = &h.app().pages.open_with;
            let demo = page.demo.as_ref().expect("the demo");
            demo.openers("en").for_file(&demo.file(page.file)).apps[index].name.clone()
        };
        let (_, heading) = h.find("MAKE IT THE DEFAULT").expect("the playground's own group is on screen");
        click_text_below(h, &name, heading);
        assert_eq!(h.app().pages.open_with.chosen, index, "clicking {name} in the playground chooses it");
    }

    /// Presses the control that makes the chosen program the default for the type on screen.
    fn make_default(h: &mut Page) {
        h.click_text("Make it the default for");
    }

    /// The demo's own `mimeapps.list`, read back after a press.
    fn list(h: &Page) -> String {
        let demo = h.app().pages.open_with.demo.as_ref().expect("the demo");
        std::fs::read_to_string(demo.root.join("config").join("mimeapps.list"))
            .expect("the demo's own list can be read")
    }

    /// The desktop file ids of the programs that open the file on screen, in the order the panel
    /// shows them.
    fn ids(h: &Page) -> Vec<String> {
        let page = &h.app().pages.open_with;
        let demo = page.demo.as_ref().expect("the demo");
        demo.openers("en").for_file(&demo.file(page.file)).apps.iter().map(|app| app.id.clone()).collect()
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
    fn the_default_programs_entry_says_what_a_launcher_needs() {
        let mut h = page();
        let screen = h.screen();
        assert!(screen.contains("WHAT THE ENTRY SAYS BESIDE THE COMMAND"), "the entry's own keys:\n{screen}");
        for expected in ["Read a long text a screen at a time", "reader;viewer", "Utility", "/home/ada/Documents"] {
            assert!(screen.contains(expected), "{expected} is missing:\n{screen}");
        }
        h.set_locale("tr");
        let screen = h.screen();
        assert!(
            screen.contains("Uzun bir metni ekran ekran oku"),
            "the entry is read in the person's language:\n{screen}"
        );
    }

    #[test]
    fn a_program_that_is_not_installed_is_listed_and_says_so() {
        let h = page();
        let screen = h.screen();
        assert!(screen.contains("PROGRAMS THAT ARE NOT INSTALLED"), "a launcher lists them apart:\n{screen}");
        assert!(screen.contains("Pro Photos"), "a program it cannot start yet:\n{screen}");
        assert!(screen.contains("its program is not installed"), "saying why:\n{screen}");
    }

    #[test]
    fn the_command_it_starts_with_names_no_file() {
        let mut h = page();
        pick(&mut h, 2);
        let screen = h.screen();
        assert!(screen.contains("IT STARTS AS"), "what a launcher runs with no file to open:\n{screen}");
        let under = screen.split_once("IT STARTS AS").expect("the header is on screen").1;
        let mut under = under.lines();
        under.next();
        let line = under.next().expect("a line under the header");
        assert!(line.trim_end().ends_with("image viewer"), "the program stands alone under the header:\n{screen}");
        assert!(
            !line.contains("holiday photo"),
            "the code that takes a file is gone, with no empty argument left:\n{screen}"
        );
        assert!(
            screen.contains("image viewer  holiday photo.jpg"),
            "the command for the file still names it:\n{screen}"
        );
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

    /// A type no program opens has nothing to make the default.
    #[test]
    fn a_type_with_no_program_offers_no_default_to_choose() {
        let mut h = page();
        pick(&mut h, 3);
        let screen = h.screen();
        assert!(screen.contains("nothing to choose as its default"), "{screen}");
        assert!(h.find("Make it the default for").is_none(), "there is no control to press:\n{screen}");
    }

    /// The editor was added for Rust source but has no line of its own under `[Default
    /// Applications]`, so the first press adds one and the second replaces it.
    #[test]
    fn making_a_program_the_default_changes_one_line_of_the_demo_own_list() {
        let mut h = page();
        let programs = ids(&h);
        assert!(programs.len() > 1, "the file on screen has programs to choose between: {programs:?}");
        // The demo's own list as the page wrote it, with the kind's line naming `chosen` and every
        // other line exactly as the demo wrote it.
        let listed = |chosen: &str| {
            format!(
                "[Default Applications]\ntext/plain=pager.desktop\ntext/x-rust={chosen};\n\n\
                 [Added Associations]\ntext/x-rust=editor.desktop;\n\n\
                 [Removed Associations]\nimage/png=images.desktop;\n"
            )
        };

        choose(&mut h, 0);
        make_default(&mut h);
        let screen = h.screen();
        assert!(screen.contains("is now the default for text/x-rust; the line is new"), "{screen}");
        assert_eq!(list(&h), listed(&programs[0]), "one line of the kind, the rest of the file as it was");
        // The list is read again, so the panel's own default follows the program that was written.
        let page = &h.app().pages.open_with;
        let demo = page.demo.as_ref().expect("the demo");
        let written = demo.openers("en").for_file(&demo.file(page.file));
        assert_eq!(written.default.map(|index| written.apps[index].id.clone()), Some(programs[0].clone()));

        choose(&mut h, 1);
        make_default(&mut h);
        let screen = h.screen();
        assert!(screen.contains("is now the default for text/x-rust; the line stood there before"), "{screen}");
        assert_eq!(list(&h), listed(&programs[1]), "the line that stood there before is the one that changed");
    }
}
