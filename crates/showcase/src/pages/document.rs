//! An application's own data file: a project file with an array of tables, read against a shape
//! into a real structure, and the same file broken by hand so the located diagnostics show.

use qframe::document::{Document, Shape, ValueKind};
use qframe::prelude::*;
use qframe::widgets::{CodeView, Language, Segmented};

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "document";

/// A complete project file: two profiles as an array of tables, and a nested engine table.
const COMPLETE: &str = "id = \"payments-api\"\nname = \"Payments API\"\ncreated = \"2026-09-18\"\n\n[engine]\nkind = \"podman\"\nretries = 2\n\n[[profile]]\nname = \"review\"\nadded = \"2026-09-18\"\n\n[[profile]]\nname = \"nightly\"\n";

/// The same file broken by hand: a value of another type, a line the parser cannot read, a key
/// nobody declared, a choice that is not one, and an entry without the name it must have.
const BROKEN: &str = "id = \"payments-api\"\nname = 7\ncolour = \ncreated = \"2026-09-18\"\n\n[engine]\nkind = \"vm\"\n\n[[profile]]\nname = \"review\"\n\n[[profile]]\nadded = \"2026-09-18\"\n";

/// A file without the one key a project cannot do without.
const NAMELESS: &str = "name = \"Payments API\"\n\n[[profile]]\nname = \"review\"\n";

/// The documents the playground switches between.
const DOCUMENTS: [&str; 3] = [COMPLETE, BROKEN, NAMELESS];

// region: document-shape
/// What a project file holds: the keys of the project itself, the engine table below it and one
/// entry per `[[profile]]`. No key has a default, because a document is not a settings file.
fn shape() -> Shape {
    let profile = Shape::new().required("name", ValueKind::text()).optional("added", ValueKind::text());
    let engine = Shape::new()
        .optional("kind", ValueKind::choice(["podman", "docker"]))
        .optional("retries", ValueKind::integer());
    Shape::new()
        .required("id", ValueKind::text())
        .optional("name", ValueKind::text())
        .optional("created", ValueKind::text())
        .table("engine", engine)
        .entries("profile", profile)
}
// endregion

/// One profile a project carries.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Profile {
    name: String,
    added: Option<String>,
}

/// A project file as the application keeps it once it is read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Project {
    id: String,
    name: Option<String>,
    created: Option<String>,
    engine: Option<String>,
    retries: Option<i64>,
    profiles: Vec<Profile>,
}

// region: document-read
/// The project the document names, or `None` when it names none: `id` is the one key the
/// application cannot do without, and a document without a readable one is not a project.
fn project(document: &Document) -> Option<Project> {
    let root = document.root();
    let engine = root.table("engine");
    Some(Project {
        id: root.text("id")?.to_owned(),
        name: root.text("name").map(str::to_owned),
        created: root.text("created").map(str::to_owned),
        engine: engine.and_then(|engine| engine.text("kind")).map(str::to_owned),
        retries: engine.and_then(|engine| engine.integer("retries")),
        profiles: root
            .entries("profile")
            .iter()
            .filter_map(|entry| {
                Some(Profile { name: entry.text("name")?.to_owned(), added: entry.text("added").map(str::to_owned) })
            })
            .collect(),
    })
}
// endregion

// region: document-write
/// The file the application writes for a project it holds. Writing stays the application's own
/// step: the loader never touches the file, and a save goes through `storage::atomic_write`.
fn to_toml(project: &Project) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
    // results are dropped for that reason and no other.
    let _ = writeln!(out, "id = {:?}", project.id);
    for (key, value) in [("name", &project.name), ("created", &project.created)] {
        if let Some(value) = value {
            let _ = writeln!(out, "{key} = {value:?}");
        }
    }
    if project.engine.is_some() || project.retries.is_some() {
        out.push_str("\n[engine]\n");
        if let Some(kind) = &project.engine {
            let _ = writeln!(out, "kind = {kind:?}");
        }
        if let Some(retries) = project.retries {
            let _ = writeln!(out, "retries = {retries}");
        }
    }
    for profile in &project.profiles {
        let _ = write!(out, "\n[[profile]]\nname = {:?}\n", profile.name);
        if let Some(added) = &profile.added {
            let _ = writeln!(out, "added = {added:?}");
        }
    }
    out
}
// endregion

/// Which document the page reads.
#[derive(Debug, Default)]
pub struct State {
    which: usize,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    /// Read another of the demo documents.
    Which(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Document(message))
}

/// The document the page is showing.
fn read(state: &State) -> Document {
    Document::parse("project.qcode", DOCUMENTS[state.which], &shape())
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Which(index) => {
            if index >= DOCUMENTS.len() {
                return Command::none();
            }
            state.which = index;
            let document = read(state);
            let found = document.diagnostics().len();
            let n = i64::try_from(found).unwrap_or(i64::MAX);
            let told = if found == 0 { t!("document.clean") } else { t!("document.logged", n = n) };
            log.push(PAGE, "Document::parse", told);
            for diagnostic in document.diagnostics() {
                log.push(PAGE, "Document::diagnostics", diagnostic.to_string());
            }
            Command::none()
        }
    }
}

/// A row of the demo: a faint label and a value that keeps its column.
fn row(ui: &mut View<'_, AppMsg>, label: String, value: Option<String>) {
    let (text, role) = match value {
        Some(value) => (value, "body"),
        None => (t!("document.unread"), "faint"),
    };
    ui.row(|ui| {
        ui.add(Text::new(label).role("faint").no_wrap()).width(Length::Cells(18));
        ui.add(Text::new(text).role(role)).fill_width();
    })
    .fill_width();
}

/// One diagnostic as a quiet line: an error keeps the danger colour, a warning the warning one.
fn diagnostic_line(ui: &mut View<'_, AppMsg>, diagnostic: &qframe::diagnostics::Diagnostic) {
    let error = diagnostic.severity == qframe::diagnostics::Severity::Error;
    let (mark, color) = if error { ("error", "danger") } else { ("warning", "warning") };
    let glyph = ui.env().icons().glyph(mark).into_owned();
    ui.row(|ui| {
        ui.add(Text::new(glyph).color(color).no_wrap());
        ui.add(Text::new(diagnostic.to_string()).role("secondary")).fill_width();
    })
    .gap(1);
}

/// What the document gave the application: the project, or the one line that says it gave none.
fn what_was_read(project: Option<&Project>, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("document.read")).gap(0), |ui| {
        let Some(project) = project else {
            let glyph = ui.env().icons().glyph("error").into_owned();
            ui.row(|ui| {
                ui.add(Text::new(glyph).color("danger").no_wrap());
                ui.add(Text::new(t!("document.no-project")).color("danger")).fill_width();
            })
            .gap(1);
            return;
        };
        row(ui, t!("document.id"), Some(project.id.clone()));
        row(ui, t!("document.name"), project.name.clone());
        row(ui, t!("document.created"), project.created.clone());
        row(ui, t!("document.engine"), project.engine.clone());
        row(ui, t!("document.retries"), project.retries.map(|retries| retries.to_string()));
        ui.spacer().height(Length::Cells(1));
        if project.profiles.is_empty() {
            ui.add(Text::new(t!("document.no-profiles")).role("faint"));
            return;
        }
        ui.add(
            Text::new(t!("document.profiles", n = i64::try_from(project.profiles.len()).unwrap_or(i64::MAX)))
                .role("faint"),
        );
        for profile in &project.profiles {
            let added = profile.added.clone().unwrap_or_else(|| t!("document.not-dated"));
            ui.row(|ui| {
                ui.add(Text::new("▌").color("accent").no_wrap());
                ui.add(Text::new(profile.name.clone()).role("body").no_wrap()).width(Length::Cells(16));
                ui.add(Text::new(added).role("secondary")).fill_width();
            })
            .gap(1);
        }
    })
    .fill_width();
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let document = read(state);
    let project = project(&document);

    ui.add_with(Panel::new().title(t!("document.file")).gap(0), |ui| {
        ui.add(Text::new(t!("document.file-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.add(CodeView::new(DOCUMENTS[state.which], Language::Toml)).fill_width().id("file");
    })
    .fill_width();

    what_was_read(project.as_ref(), ui);

    ui.add_with(Panel::new().title(t!("document.reported")).gap(0), |ui| {
        // region: document-diagnostics
        if document.is_clean() {
            let glyph = ui.env().icons().glyph("success").into_owned();
            ui.row(|ui| {
                ui.add(Text::new(glyph).color("success").no_wrap());
                ui.add(Text::new(t!("document.clean")).color("success")).fill_width();
            })
            .gap(1);
        }
        for diagnostic in document.diagnostics() {
            diagnostic_line(ui, diagnostic);
        }
        // endregion
    })
    .fill_width();

    if let Some(project) = &project {
        ui.add_with(Panel::new().title(t!("document.written")).gap(0), |ui| {
            ui.add(Text::new(t!("document.written-hint")).role("secondary"));
            ui.spacer().height(Length::Cells(1));
            ui.add(CodeView::new(to_toml(project), Language::Toml)).fill_width().id("written");
        })
        .fill_width();
    }

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        let labels = [t!("document.complete"), t!("document.broken"), t!("document.nameless")];
        setting(ui, t!("document.which"), |ui| {
            ui.add(Segmented::new(labels).selected(state.which).on_select(|index| send(Msg::Which(index)))).id("which");
        });
        ui.add(Text::new(t!("document.hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_tall;

    /// The page in a terminal tall enough for every panel.
    fn page() -> qframe::runtime::Harness<Showcase> {
        showcase_tall(Showcase::new(), PAGE, 120)
    }

    #[test]
    fn a_complete_document_is_read_into_a_project_with_its_profiles() {
        let h = page();
        let screen = h.screen();
        assert!(screen.contains("nothing was wrong with this document"), "{screen}");
        assert!(screen.contains("payments-api") && screen.contains("Payments API"), "{screen}");
        assert!(screen.contains("podman") && screen.contains("review") && screen.contains("nightly"), "{screen}");
        let project = project(&read(&h.app().pages.document)).expect("the complete document names a project");
        assert_eq!(project.profiles.len(), 2);
        assert_eq!(project.retries, Some(2));
        assert_eq!(to_toml(&project), COMPLETE, "the file the application writes back is the file it read");
    }

    #[test]
    fn a_broken_document_keeps_its_readable_part_and_reports_the_rest_where_it_is() {
        let mut h = page();
        h.send(send(Msg::Which(1)));
        let screen = h.screen();
        for expected in [
            "project.qcode:3:10: error: ",
            "project.qcode:2:8: warning: `name` must be a string, found 7",
            "project.qcode:3:1: warning: `colour` is not part of the document",
            "project.qcode:7:8: warning: `engine.kind` must be one of podman, docker",
            "project.qcode:12:1: error: `profile[1].name` is required and missing",
        ] {
            assert!(screen.contains(expected), "`{expected}` is missing from:\n{screen}");
        }
        let document = read(&h.app().pages.document);
        assert_eq!(document.diagnostics().len(), 5, "{:?}", document.diagnostics());
        let project = project(&document).expect("a broken document still names the project");
        assert_eq!(project.id, "payments-api");
        assert_eq!(project.created.as_deref(), Some("2026-09-18"), "the readable keys are all there");
        assert_eq!((project.name, project.engine, project.retries), (None, None, None));
        assert_eq!(project.profiles.iter().map(|p| p.name.clone()).collect::<Vec<_>>(), vec!["review"]);
        assert!(screen.contains("not read"), "the keys that were dropped show as unread:\n{screen}");

        let log: Vec<String> = h.app().log.recent(PAGE, 10).iter().map(|entry| entry.message.clone()).collect();
        assert!(log.iter().any(|line| line == "5 problems, and the readable part is still there"), "{log:?}");
        assert!(log.iter().any(|line| line.contains("`profile[1].name` is required and missing")), "{log:?}");
    }

    #[test]
    fn a_document_without_the_key_it_must_have_names_no_project() {
        let mut h = page();
        h.send(send(Msg::Which(2)));
        let screen = h.screen();
        assert!(screen.contains("project.qcode:1:1: error: `id` is required and missing"), "{screen}");
        assert!(screen.contains("names no project"), "{screen}");
        assert!(project(&read(&h.app().pages.document)).is_none());
        assert!(!screen.contains("THE FILE IT WOULD WRITE"), "there is nothing to write back:\n{screen}");
    }

    #[test]
    fn a_document_that_is_not_there_changes_nothing() {
        let mut h = page();
        h.send(send(Msg::Which(DOCUMENTS.len())));
        assert_eq!(h.app().pages.document.which, 0, "a document that is not offered is not shown");
    }
}
