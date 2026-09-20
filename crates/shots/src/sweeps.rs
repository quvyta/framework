//! Questions asked of the framework's own source and its built-in files at every commit.
//!
//! The font sweep beside this one turned "a character nobody can draw is found when someone
//! happens to draw it" into "the gate says so". The same shape is here: a key a widget asks for
//! that no language has, or an icon a widget names that a set does not answer, is a hole nobody
//! sees until the one screen that shows it is opened in the one language or the one icon mode
//! that has it. Each sweep reads what the code asks for out of the source and puts it to every
//! built-in file.
//!
//! This crate asks rather than the framework itself, because these questions are about the
//! repository — source files on disk, read as text — and this is where the repository already
//! reads itself (see [`crate::coverage`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qframe::i18n::I18n;
use qframe::icons::{GlyphMode, IconSetRegistry};

/// The framework crate's source folder.
fn framework_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../quvyta-framework/src")
}

/// The code of a source file: its in-module tests cut off — everything from the first
/// `#[cfg(test)]` at the start of a line onwards, since the convention here puts that module last
/// — and its comment lines dropped. What is left is what a running application takes, which is
/// the only code whose keys and icons have to be answered. A key in a comment is an example being
/// explained, a key in a test is the test's own; neither reaches a screen.
fn code_of(text: &str) -> String {
    let code = match [text.find("\n#[cfg(test)]"), text.find("\n#[cfg(all(test")].into_iter().flatten().min() {
        Some(at) => &text[..at],
        None => text,
    };
    code.lines().filter(|line| !line.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

/// Every `*.rs` file under `dir` that is not itself a test file, each without its in-module tests.
fn source_files(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let entries = std::fs::read_dir(&folder).unwrap_or_else(|e| panic!("read {}: {e}", folder.display()));
        for entry in entries {
            let path = entry.expect("a file of the folder").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_owned();
            if path.is_dir() {
                if name != "tests" {
                    folders.push(path);
                }
                continue;
            }
            if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
                let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
                found.push((path, code_of(&text)));
            }
        }
    }
    found.sort();
    assert!(found.len() > 50, "only {} source files found under {}", found.len(), dir.display());
    found
}

/// Every string literal that opens right after one of `calls`, as `("` does in `t!("some.key"`.
///
/// A call only counts where its name begins: `format!("` ends in `t!("` and names no message, and
/// `pillar_glyph("` ends in `glyph("` and names no icon.
fn named_after(text: &str, calls: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for call in calls {
        let mut from = 0;
        while let Some(at) = text[from..].find(call) {
            let at = from + at;
            from = at + call.len();
            let before = text[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            if let Some(end) = text[from..].find('"') {
                found.push(text[from..from + end].to_owned());
            }
        }
    }
    found
}

/// What the running framework asks for, by the call that names it, with the file it stands in.
fn asked_for(calls: &[&str]) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = source_files(&framework_src())
        .into_iter()
        .flat_map(|(path, text)| named_after(&text, calls).into_iter().map(move |name| (name, path.clone())))
        .collect();
    found.sort();
    found.dedup_by(|a, b| a.0 == b.0);
    found
}

#[test]
fn every_message_the_framework_asks_for_is_in_its_language_files() {
    // A key with no translation is shown as ⟦key⟧ on the screen that asks for it, so it is found
    // by opening that screen and no sooner. `has` is asked of English alone because a test of the
    // framework already requires every other language to carry every key English has.
    let i18n = I18n::builtin();
    let asked = asked_for(&["t!(\""]);
    let missing: Vec<String> = asked
        .iter()
        .filter(|(key, _)| !i18n.has("en", key))
        .map(|(key, path)| format!("{key} ({})", path.display()))
        .collect();
    assert!(missing.is_empty(), "no language file answers: {}", missing.join(", "));
    assert!(asked.len() > 60, "only {} messages named in the source", asked.len());
}

#[test]
fn every_icon_the_framework_names_is_answered_by_every_built_in_set_in_every_mode() {
    // An icon a set does not answer is an empty cell, and only in that set and that glyph mode:
    // the screen looks right in the mode it was built in and loses a shape in the other two.
    let registry = IconSetRegistry::builtin();
    let named = asked_for(&[".glyph(\"", "Icon::new(\"", ".icon(\""]);
    assert!(named.len() > 10, "only {} icons named in the source", named.len());
    let mut missing = Vec::new();
    for (id, _) in registry.list() {
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            let icons = registry.icons(&id, &BTreeMap::new(), mode);
            for (name, path) in &named {
                if !icons.keys().any(|key| key == *name) || icons.glyph(name).is_empty() {
                    missing.push(format!("{name} in {mode:?} of set {id} ({})", path.display()));
                }
            }
        }
    }
    assert!(missing.is_empty(), "no glyph for: {}", missing.join(", "));
}

#[test]
fn a_name_is_read_where_the_program_says_it_and_nowhere_else() {
    let text = "\
/// t!(\"in.a.comment\")
fn a() { t!(\"said\"); format!(\"not.a.message\"); ui.add(Icon::new(\"shape\")); }
#[cfg(test)]
mod tests {
    t!(\"in.a.test\");
}
";
    let code = code_of(text);
    assert_eq!(named_after(&code, &["t!(\""]), vec!["said".to_owned()]);
    assert_eq!(named_after(&code, &["Icon::new(\""]), vec!["shape".to_owned()]);
}
