//! Reading a document: what is kept, what is reported, where it is reported, and that the file
//! on disk is never touched.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use super::{Document, Shape, ValueKind};

/// One profile a project carries, as an application would keep it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Profile {
    name: String,
    added: Option<String>,
}

/// A project file, the shape qcode's `project.qcode` has.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Project {
    id: String,
    name: Option<String>,
    retries: Option<i64>,
    network: Option<bool>,
    assets: Option<String>,
    profiles: Vec<Profile>,
}

/// The shape of a project file.
fn shape() -> Shape {
    let profile = Shape::new().required("name", ValueKind::text()).optional("added", ValueKind::text());
    Shape::new()
        .required("id", ValueKind::text())
        .optional("name", ValueKind::text())
        .optional("retries", ValueKind::integer())
        .table("mounts", Shape::new().optional("assets", ValueKind::choice(["rw", "ro"])))
        .table("network", Shape::new().optional("full", ValueKind::flag()))
        .entries("profile", profile)
}

/// Reads a project the way an application does: pull what it needs, keep the diagnostics.
fn read(file: &str, text: &str) -> (Option<Project>, Vec<String>) {
    let document = Document::parse(file, text, &shape());
    let root = document.root();
    let project = root.text("id").map(|id| Project {
        id: id.to_owned(),
        name: root.text("name").map(str::to_owned),
        retries: root.integer("retries"),
        network: root.table("network").and_then(|network| network.flag("full")),
        assets: root.table("mounts").and_then(|mounts| mounts.text("assets")).map(str::to_owned),
        profiles: root
            .entries("profile")
            .iter()
            .filter_map(|entry| {
                Some(Profile { name: entry.text("name")?.to_owned(), added: entry.text("added").map(str::to_owned) })
            })
            .collect(),
    });
    (project, document.diagnostics().iter().map(ToString::to_string).collect())
}

/// The file an application writes for `project`; writing stays the application's own step.
fn to_toml(project: &Project) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "id = {:?}", project.id);
    if let Some(name) = &project.name {
        let _ = writeln!(out, "name = {name:?}");
    }
    if let Some(retries) = project.retries {
        let _ = writeln!(out, "retries = {retries}");
    }
    if let Some(full) = project.network {
        let _ = write!(out, "\n[network]\nfull = {full}\n");
    }
    if let Some(assets) = &project.assets {
        let _ = write!(out, "\n[mounts]\nassets = {assets:?}\n");
    }
    for profile in &project.profiles {
        let _ = write!(out, "\n[[profile]]\nname = {:?}\n", profile.name);
        if let Some(added) = &profile.added {
            let _ = writeln!(out, "added = {added:?}");
        }
    }
    out
}

fn project() -> Project {
    Project {
        id: "payments-api".to_owned(),
        name: Some("Payments API".to_owned()),
        retries: Some(3),
        network: Some(false),
        assets: Some("ro".to_owned()),
        profiles: vec![
            Profile { name: "review".to_owned(), added: Some("2026-09-18".to_owned()) },
            Profile { name: "nightly".to_owned(), added: None },
        ],
    }
}

#[test]
fn an_array_of_tables_and_the_tables_around_it_round_trip() {
    let written = to_toml(&project());
    assert_eq!(
        written,
        "id = \"payments-api\"\nname = \"Payments API\"\nretries = 3\n\n[network]\nfull = false\n\n[mounts]\nassets = \"ro\"\n\n[[profile]]\nname = \"review\"\nadded = \"2026-09-18\"\n\n[[profile]]\nname = \"nightly\"\n"
    );
    let (read_back, diagnostics) = read("project.qcode", &written);
    assert_eq!(diagnostics, Vec::<String>::new());
    assert_eq!(read_back.as_ref(), Some(&project()));
    assert_eq!(read_back.as_ref().map(to_toml).as_deref(), Some(written.as_str()), "and out again unchanged");
}

#[test]
fn nested_tables_read_the_same_written_either_way() {
    let sections = "id = \"api\"\n\n[mounts]\nassets = \"rw\"\n\n[network]\nfull = true\n";
    let dotted = "id = \"api\"\nmounts.assets = \"rw\"\nnetwork.full = true\n";
    let (from_sections, clean) = read("project.qcode", sections);
    assert_eq!(clean, Vec::<String>::new());
    let (from_dotted, clean) = read("project.qcode", dotted);
    assert_eq!(clean, Vec::<String>::new());
    assert_eq!(from_sections, from_dotted);
    assert_eq!(from_dotted.map(|project| (project.assets, project.network)), Some((Some("rw".to_owned()), Some(true))));

    // A missing table is not a problem, and its keys read as missing.
    let (bare, clean) = read("project.qcode", "id = \"api\"\n");
    assert_eq!(clean, Vec::<String>::new());
    assert_eq!(bare.map(|project| (project.assets, project.network)), Some((None, None)));
}

#[test]
fn a_missing_required_key_is_an_error_at_the_table_it_is_missing_from() {
    let (project, diagnostics) = read("project.qcode", "name = \"Payments API\"\n");
    assert_eq!(project, None, "the application has nothing to open");
    assert_eq!(diagnostics, vec!["project.qcode:1:1: error: `id` is required and missing"]);

    let text = "id = \"api\"\n\n[[profile]]\nname = \"review\"\n\n[[profile]]\nadded = \"2026-09-18\"\n";
    let (project, diagnostics) = read("project.qcode", text);
    assert_eq!(diagnostics, vec!["project.qcode:6:1: error: `profile[1].name` is required and missing"]);
    let names: Vec<String> =
        project.expect("a project with one readable profile").profiles.into_iter().map(|p| p.name).collect();
    assert_eq!(names, vec!["review"], "the readable entry is still there");
}

#[test]
fn a_key_the_shape_does_not_declare_is_reported_where_it_stands_and_skipped() {
    let text = "id = \"api\"\ncolour = \"red\"\n\n[[profile]]\nname = \"review\"\ntint = 3\n\n[mounts]\nspeed = 1\n";
    let (project, diagnostics) = read("project.qcode", text);
    assert_eq!(
        diagnostics,
        vec![
            "project.qcode:2:1: warning: `colour` is not part of the document; it is ignored",
            "project.qcode:6:1: warning: `profile[0].tint` is not part of the document; it is ignored",
            "project.qcode:9:1: warning: `mounts.speed` is not part of the document; it is ignored",
        ]
    );
    let project = project.expect("the rest of the document is usable");
    assert_eq!(project.profiles.len(), 1);
    assert_eq!(project.id, "api", "the declared keys beside the unknown ones are read");
    let document = Document::parse("project.qcode", text, &shape());
    assert_eq!(document.root().location("colour"), None, "an unknown key is not kept at all");
}

#[test]
fn a_value_of_another_type_is_skipped_and_its_severity_says_whether_the_key_was_required() {
    let text = "id = 7\nname = true\nretries = \"three\"\n\n[mounts]\nassets = \"sideways\"\n\n[network]\nfull = 1\n";
    let (project, diagnostics) = read("project.qcode", text);
    assert_eq!(project, None, "`id` is required, so nothing was opened");
    assert_eq!(
        diagnostics,
        vec![
            "project.qcode:1:6: error: `id` must be a string, found 7; it is ignored",
            "project.qcode:2:8: warning: `name` must be a string, found true; it is ignored",
            "project.qcode:3:11: warning: `retries` must be a whole number, found \"three\"; it is ignored",
            "project.qcode:6:10: warning: `mounts.assets` must be one of rw, ro, found \"sideways\"; it is ignored",
            "project.qcode:9:8: warning: `network.full` must be a boolean, found 1; it is ignored",
        ]
    );

    // A value where a table or an array of tables belongs.
    let (_, diagnostics) = read("p.toml", "id = \"api\"\nmounts = 3\nprofile = 7\n");
    assert_eq!(
        diagnostics,
        vec![
            "p.toml:2:10: warning: `mounts` must be a table, found 3; it is ignored",
            "p.toml:3:11: warning: `profile` must be an array of tables, found 7; it is ignored",
        ]
    );

    // An array of something that is not a table: each item is reported where it stands.
    let (_, diagnostics) = read("p.toml", "id = \"api\"\nprofile = [\"review\", 2]\n");
    assert_eq!(
        diagnostics,
        vec![
            "p.toml:2:12: warning: `profile[0]` must be a table, found \"review\"; it is ignored",
            "p.toml:2:22: warning: `profile[1]` must be a table, found 2; it is ignored",
        ]
    );
}

#[test]
fn a_syntax_error_is_located_and_the_rest_of_the_file_is_still_read() {
    let text = "id = \"api\"\nname = \n\n[[profile]]\nname = \"review\"\n";
    let (project, diagnostics) = read("project.qcode", text);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(diagnostics[0].starts_with("project.qcode:2:8: error: "), "{diagnostics:?}");
    let project = project.expect("the readable part is still a project");
    assert_eq!(project.id, "api");
    // The parser recovers the broken line as an empty value; what it could not read is gone.
    assert_eq!(project.name.as_deref(), Some(""));
    assert_eq!(project.profiles.len(), 1, "the entries after the error are read");
}

#[test]
fn a_document_says_when_nothing_was_wrong_and_where_each_key_was_written() {
    let text = "id = \"api\"\n\n[[profile]]\nname = \"review\"\n";
    let document = Document::parse("project.qcode", text, &shape());
    assert!(document.is_clean());
    assert_eq!(document.root().location("id").map(ToString::to_string).as_deref(), Some("project.qcode:1:1"));
    assert_eq!(document.root().location("name"), None);
    let entry = &document.root().entries("profile")[0];
    assert_eq!(entry.location("name").map(ToString::to_string).as_deref(), Some("project.qcode:4:1"));
    assert!(!Document::parse("project.qcode", "\n", &shape()).is_clean());
}

#[test]
fn a_value_is_located_where_the_value_itself_was_written() {
    // An application's own check lands on the value, like the framework's type checks do.
    let text = "id   =   \"api\"\n\n[[profile]]\n  name = \"review\"\n";
    let document = Document::parse("project.qcode", text, &shape());
    let root = document.root();
    assert_eq!(root.location("id").map(ToString::to_string).as_deref(), Some("project.qcode:1:1"));
    assert_eq!(root.value_location("id").map(ToString::to_string).as_deref(), Some("project.qcode:1:10"));
    assert_eq!(root.value_location("name"), None, "a key that was not written has no value either");
    let entry = &root.entries("profile")[0];
    assert_eq!(entry.value_location("name").map(ToString::to_string).as_deref(), Some("project.qcode:4:10"));
    // The value's place is the one a type mismatch is reported at.
    let wrong = Document::parse("project.qcode", "id = 7\n", &shape());
    assert!(wrong.diagnostics()[0].to_string().starts_with("project.qcode:1:6: "), "{:?}", wrong.diagnostics());
}

#[test]
fn reading_a_file_changes_nothing_on_disk() {
    let dir = std::env::temp_dir().join(format!("quvyta-document-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder for the test file");
    let path = dir.join("project.qcode");
    // A document with every kind of problem: a self-healing loader would rewrite all of it.
    let broken = "id = 7\ncolour = \"red\"\nname = \n\n[[profile]]\nadded = \"2026-09-18\"\n";
    fs::write(&path, broken).expect("the test file is writable");

    let document = Document::open(&path, &shape()).expect("the file is readable");
    assert!(!document.is_clean(), "the document is broken: {:?}", document.diagnostics());
    assert!(
        document.diagnostics().iter().any(|d| d.to_string().starts_with("project.qcode:1:6")),
        "problems are reported against the file's own name: {:?}",
        document.diagnostics()
    );
    assert_eq!(fs::read_to_string(&path).expect("still readable"), broken, "the file is untouched");
    let left: Vec<PathBuf> = fs::read_dir(&dir).expect("list").filter_map(|e| e.ok().map(|e| e.path())).collect();
    assert_eq!(left, vec![path.clone()], "no backup and no temporary file were left behind");

    fs::remove_dir_all(&dir).expect("clean up");
    let missing = Document::open(dir.join("project.qcode"), &shape());
    assert_eq!(
        missing.err().map(|error| error.kind()),
        Some(std::io::ErrorKind::NotFound),
        "a missing file is an error"
    );
}

#[test]
fn hostile_nesting_and_long_lists_are_read_without_a_crash() {
    let shape =
        Shape::new().optional("a", ValueKind::text()).entries("e", Shape::new().optional("x", ValueKind::integer()));
    let deep = vec!["b"; 100_000].join(".");
    let hostile = [
        format!("a = {}{}", "[".repeat(100_000), "]".repeat(100_000)),
        format!("a = {}1{}", "{b=".repeat(100_000), "}".repeat(100_000)),
        format!("a.{deep} = 1"),
        format!("[e.{deep}]\nx = 1\n[[a.{deep}]]\n"),
    ];
    for text in &hostile {
        let document = Document::parse("deep.toml", text, &shape);
        assert!(!document.is_clean(), "nesting this deep is reported, not followed");
        assert_eq!(document.root().text("a"), None);
    }

    let text = "[[e]]\nx = 1\n".repeat(10_000) + &"[[e]]\nx = 'one'\n".repeat(10_000);
    let document = Document::parse("long.toml", &text, &shape);
    let entries = document.root().entries("e");
    assert_eq!(entries.len(), 20_000, "every entry is kept, readable or not");
    assert_eq!(entries.iter().filter(|entry| entry.integer("x") == Some(1)).count(), 10_000);
    assert_eq!(document.diagnostics().len(), 10_000, "one warning per unreadable value");
}
