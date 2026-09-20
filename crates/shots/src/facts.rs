//! Facts the repository writes down more than once, bound here so the copies cannot part.
//!
//! Nothing is derived: each test reads both places and says they must agree. A fact that only
//! one place could hold is not here at all — it was made single instead.

use std::path::PathBuf;

/// The repository's root, two folders above this crate.
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The value of the first `key = "value"` line of `text` at the top level of section `section`.
///
/// Enough for a manifest, which is written by hand in one shape: the line is looked for after the
/// section's header and before the next one.
fn value_in(text: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == format!("[{section}]");
            continue;
        }
        if inside
            && let Some(rest) = line.strip_prefix(key)
            && let Some(rest) = rest.trim_start().strip_prefix('=')
        {
            return rest.trim().trim_matches('"').split('"').next().map(str::to_owned);
        }
    }
    None
}

/// The `version = "x.y.z"` of the first inline table in `text` whose key is `name`.
fn dependency_version(text: &str, name: &str) -> Option<String> {
    let line = text.lines().find(|line| line.trim_start().starts_with(&format!("{name} = {{")))?;
    let rest = line.split_once("version")?.1.split_once('"')?.1;
    rest.split('"').next().map(str::to_owned)
}

/// Every `Cargo.toml` of the workspace but the root's.
fn crate_manifests() -> Vec<(String, String)> {
    let crates = repo().join("crates");
    let mut found: Vec<(String, String)> = std::fs::read_dir(&crates)
        .unwrap_or_else(|e| panic!("read {}: {e}", crates.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let manifest = path.join("Cargo.toml");
            let text = std::fs::read_to_string(&manifest).ok()?;
            Some((manifest.display().to_string(), text))
        })
        .collect();
    found.sort();
    assert!(found.len() >= 3, "the workspace has three crates, found {}", found.len());
    found
}

#[test]
fn every_crate_asks_for_the_version_the_workspace_publishes() {
    // A path dependency carries a version as well, because a published crate has no path to
    // follow. Raising the workspace version and leaving one of these behind publishes a crate
    // that asks crates.io for a framework older than the one it was built against.
    let root = std::fs::read_to_string(repo().join("Cargo.toml")).expect("the workspace manifest");
    let workspace = value_in(&root, "workspace.package", "version").expect("[workspace.package] version");
    for (path, text) in crate_manifests() {
        let Some(asked) = dependency_version(&text, "quvyta-framework") else { continue };
        assert_eq!(asked, workspace, "{path} depends on quvyta-framework {asked}, the workspace publishes {workspace}");
    }
}

#[test]
fn the_changelog_is_headed_by_the_version_the_workspace_publishes() {
    // A release is a version in the manifest and an entry in the changelog. When the two are
    // written apart, the entry of the version people install can be missing and nobody sees it
    // until someone reads the file looking for a change that was never written down.
    let root = std::fs::read_to_string(repo().join("Cargo.toml")).expect("the workspace manifest");
    let workspace = value_in(&root, "workspace.package", "version").expect("[workspace.package] version");
    let changelog = std::fs::read_to_string(repo().join("CHANGELOG.md")).expect("the changelog");
    // Changes that have not been released yet stand under a heading of their own, which names no
    // version; the newest heading that does is the one this fact is about.
    let newest = changelog
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .map(str::trim)
        .find(|heading| heading.starts_with(|c: char| c.is_ascii_digit()))
        .expect("the changelog has a release heading");
    let named = newest.split_whitespace().next().unwrap_or_default();
    assert_eq!(named, workspace, "the newest released heading is {newest:?}, the workspace publishes {workspace}");
}

#[test]
fn a_manifest_line_is_read_where_it_stands() {
    assert_eq!(value_in("[a]\nx = \"1\"\n[b]\nx = \"2\"\n", "b", "x").as_deref(), Some("2"));
    assert_eq!(dependency_version("dep = { version = \"3.4.5\", path = \"..\" }", "dep").as_deref(), Some("3.4.5"));
}
