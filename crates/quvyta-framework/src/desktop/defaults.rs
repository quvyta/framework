//! `mimeapps.list`: the file a person's desktop keeps their choices of which program opens which
//! kind of file in, and the one line that changes when they say "make this the default".
//!
//! Reading that file is the business of [`Apps`](super::Apps), as the rest of this module is; what
//! is written in it belongs here, because the format is freedesktop's own and is the same for every
//! application that offers "make this the default": a file manager's "Open with" dialog and a
//! desktop's both write one line of the same file.
//!
//! Deliberately never touched: a file under `$XDG_CONFIG_DIRS`, which belongs to the system and not
//! to the person; a `<desktop>-mimeapps.list` of a desktop that is running, which that desktop
//! writes itself; and any `.desktop` entry, which says what a program opens and not what the
//! person chose. The only file written is the person's own `$XDG_CONFIG_HOME/mimeapps.list`, the
//! one every desktop reads first and the only one a program may write.
//!
//! The file is edited, never rebuilt. Every other line, section and comment is put back exactly as
//! it was — `[Added Associations]`, `[Removed Associations]`, blank lines and lines nobody
//! understands included — and only the line of the one kind changes: set to `<type>=<id>;`, with
//! the trailing semicolon the format wants, added when the kind has no line and replaced when it
//! has one. A line for the same kind in `[Removed Associations]` is left alone: that section says
//! which programs a person does *not* want, which is their decision and not this one's to take
//! back.
//!
//! The write goes through a temporary file in the same folder and a rename over the old one
//! ([`atomic_write`]), so a failed write never leaves a truncated file behind. A file that cannot
//! be made sense of is not thrown away either: a file that is not text, a line for the kind that
//! names no program and a file the person has made read-only are all reported and left as they are.
//!
//! [`atomic_write`]: crate::storage::atomic_write

use std::fmt;
use std::fs;
use std::io;
use std::io::ErrorKind;

use super::XdgDirs;
use super::apps::MIMEAPPS_LIST;
use crate::storage::atomic_write;

/// The section the default program of a kind is written in, spelled as every other file spells it.
const DEFAULTS: &str = "Default Applications";

/// What became of the line of one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Change {
    /// The kind had no line of its own and the one written is new.
    Added,
    /// The kind had a line of its own and it now names another program.
    Replaced,
}

/// What kept the line of one kind from being written.
#[derive(Debug)]
#[non_exhaustive]
pub enum SetDefaultError {
    /// The file could not be read or written; the system's own words, unchanged.
    Io(io::Error),
    /// There is no configuration folder of the person's own to write the file into, which is what
    /// an unset `XDG_CONFIG_HOME` with no home folder leaves.
    NoConfigHome,
    /// The file is there and is not UTF-8, so not every line of it can be understood. The
    /// framework's own reader skips a line it cannot read and says so; a file with one is not
    /// rewritten here either, since a rewrite is only as careful as the understanding behind it.
    NotText,
    /// The file is there and the person has made it read-only. A rename would go through the mode
    /// anyway, so the mode is asked instead: a file kept read-only is a decision, not an accident,
    /// and this does not write over a decision.
    ReadOnly,
    /// The kind is named by a line in the `[Default Applications]` section that has no `=`, so it
    /// is not a line that can be replaced. Writing a valid one would mean losing that line, which
    /// is the person's own; the file is left as it is.
    InTheWay {
        /// The line standing in the way, as it is in the file.
        line: String,
    },
    /// The kind or the program's desktop file id cannot be written on a line of this file at all.
    Unstorable,
}

impl fmt::Display for SetDefaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::NoConfigHome => f.write_str("there is no configuration folder of the person's own to write in"),
            Self::NotText => f.write_str("the file is not UTF-8 text, so no line of it can be understood"),
            Self::ReadOnly => f.write_str("the file is read-only"),
            Self::InTheWay { line } => {
                write!(f, "the file has the line `{line}`, which is not a program to replace")
            }
            Self::Unstorable => f.write_str("the kind or the program cannot be named on a line of this file"),
        }
    }
}

impl std::error::Error for SetDefaultError {}

impl From<io::Error> for SetDefaultError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Writes the default program of `mime` to the desktop entry `app_id` in the person's own
/// `mimeapps.list`, and says whether the kind's line is new or stood there before.
///
/// The file is `$XDG_CONFIG_HOME/mimeapps.list`, made with its folder when the person has none yet;
/// a file that is already there is read, changed in one line and replaced whole, never partly:
/// either the old one or the new one is on disk, whatever happens in between. Only the kind's own
/// line inside `[Default Applications]` changes. Every other line, section and comment is put
/// back byte for byte, and the write goes through [`atomic_write`](crate::storage::atomic_write), so
/// a crash cannot leave a half-written file behind.
///
/// Reading it again with [`Apps::load`](super::Apps::load) from the same `dirs` then gives
/// `app_id` as the default program of `mime`.
///
/// # Errors
///
/// See [`SetDefaultError`]. The file is left as it was on every one of them, and a line already
/// written for `mime` keeps the program it names.
pub fn set_default(dirs: &XdgDirs, mime: &str, app_id: &str) -> Result<Change, SetDefaultError> {
    let folder = dirs.config_home.as_deref().ok_or(SetDefaultError::NoConfigHome)?;
    let file = folder.join(MIMEAPPS_LIST);
    // A rename replaces whatever stands there whatever its mode is, so a read-only file is only
    // kept if it is asked for before the write begins.
    if fs::metadata(&file).is_ok_and(|meta| meta.is_file() && meta.permissions().readonly()) {
        return Err(SetDefaultError::ReadOnly);
    }
    // A file nobody has written yet is an empty one: the first choice of a kind is a new section and
    // one line. Anything else that cannot be read says so instead of being written over.
    let before = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    if std::str::from_utf8(&before).is_err() {
        return Err(SetDefaultError::NotText);
    }
    let (after, change) = changed(&before, mime, app_id)?;
    // The folder is made last, when the line is known to be writable, so a refused name leaves the
    // machine with nothing new on it.
    fs::create_dir_all(folder)?;
    atomic_write(&file, &after)?;
    Ok(change)
}

/// `bytes` as a `mimeapps.list` in which the default program of `mime` is the desktop entry
/// `app_id`, every other line of it as it was, and whether the kind's line is new.
///
/// The first line of the kind is the one that counts: a key file reader takes the first of a key
/// written twice, and a second one is a line the person or their desktop put there, so it is left
/// alone rather than deleted. A kind named by a line in the section that has no `=` is a different
/// case: that line is not a program anyone can replace, so the file is reported instead of rewritten.
fn changed(bytes: &[u8], mime: &str, app_id: &str) -> Result<(Vec<u8>, Change), SetDefaultError> {
    if !storable(mime) || !storable(app_id) {
        return Err(SetDefaultError::Unstorable);
    }
    let mut lines = Lines::split(bytes);
    let mut place = Place::default();
    for (at, line) in lines.iter() {
        let line = trim(line);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        if let Some(name) = section_of(line) {
            // The section to write into is the first `[Default Applications]`; a second one of the
            // same name is a section of its own, and the line goes on into the first.
            if name == DEFAULTS.as_bytes() && place.header.is_none() {
                place.header = Some(at);
                place.inside = true;
                place.last = None;
            } else {
                place.inside = false;
            }
            continue;
        }
        if place.inside && line == mime.as_bytes() && entry_key(line).is_none() {
            return Err(SetDefaultError::InTheWay { line: mime.to_owned() });
        }
        let (Some(key), true) = (entry_key(line), place.inside) else {
            continue;
        };
        if key == mime.as_bytes() && place.line.is_none() {
            place.line = Some(at);
        }
        place.last = Some(at);
    }

    let line = format!("{mime}={app_id};");
    let change = match place.line {
        Some(at) => {
            lines.replace(at, &line);
            Change::Replaced
        }
        None => {
            // Inside the section, after the last line that is an entry of it; with a section that
            // holds no line of its own, straight under its header; with no section at all, a new
            // one at the end of the file, where a header belongs.
            match (place.last, place.header) {
                (Some(last), _) => lines.insert(last + 1, &line),
                (None, Some(header)) => lines.insert(header + 1, &line),
                (None, None) => lines.section(DEFAULTS, &line),
            }
            Change::Added
        }
    };
    Ok((lines.joined(), change))
}

/// Where in the file the kind's line is, and what was found around it.
#[derive(Debug, Default)]
struct Place {
    /// The `[Default Applications]` header the new line goes under.
    header: Option<usize>,
    /// Whether the line being read is in that section.
    inside: bool,
    /// The kind's own line, the first one, when the file has it.
    line: Option<usize>,
    /// The last line of that section which is an entry, so a new line lands among the entries.
    last: Option<usize>,
}

/// The file's lines, each with the line break that ended it, so the file can be put back together
/// byte for byte as it was.
#[derive(Debug)]
struct Lines(Vec<Vec<u8>>);

impl Lines {
    /// `bytes` split at its line breaks. A file ending in a break has no empty last line.
    fn split(bytes: &[u8]) -> Self {
        let mut out = Vec::new();
        let mut rest = bytes;
        while let Some(at) = rest.iter().position(|byte| *byte == b'\n') {
            out.push(rest[..=at].to_vec());
            rest = &rest[at + 1..];
        }
        if !rest.is_empty() {
            out.push(rest.to_vec());
        }
        Self(out)
    }

    /// The file's lines, each without the break that ended it.
    fn iter(&self) -> impl Iterator<Item = (usize, &[u8])> {
        self.0.iter().enumerate().map(|(at, line)| (at, without_break(line)))
    }

    /// The length of the file, in lines.
    fn len(&self) -> usize {
        self.0.len()
    }

    /// Puts `line` where line `at` was, keeping the break that ended it.
    fn replace(&mut self, at: usize, line: &str) {
        let mut new = line.as_bytes().to_vec();
        if let Some(break_at) = self.0[at].iter().position(|byte| *byte == b'\n') {
            new.extend_from_slice(&self.0[at][break_at..]);
        }
        self.0[at] = new;
    }

    /// Puts `line` in front of line `at` as a line of its own.
    fn insert(&mut self, at: usize, line: &str) {
        // A line of its own needs the line before it to have ended, and the last line of a file
        // need not have a break.
        if at == self.len() {
            self.end_with_break();
        }
        let mut new = line.as_bytes().to_vec();
        new.push(b'\n');
        self.0.insert(at, new);
    }

    /// Adds a section named `section` holding `line` at the end of the file.
    fn section(&mut self, section: &str, line: &str) {
        self.end_with_break();
        self.0.push(format!("[{section}]\n").into_bytes());
        self.0.push(format!("{line}\n").into_bytes());
    }

    /// Ends the last line with a line break, when it is not empty and has none of its own.
    fn end_with_break(&mut self) {
        if let Some(last) = self.0.last_mut()
            && !last.is_empty()
            && !last.ends_with(b"\n")
        {
            last.push(b'\n');
        }
    }

    /// The file as it is now.
    fn joined(&self) -> Vec<u8> {
        self.0.concat()
    }
}

/// The line without the break that ended it.
fn without_break(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// The line with the spaces at both ends taken off, as a key file reader reads it.
fn trim(line: &[u8]) -> &[u8] {
    let start = line.iter().position(|byte| !byte.is_ascii_whitespace());
    let end = line.iter().rposition(|byte| !byte.is_ascii_whitespace());
    match (start, end) {
        (Some(start), Some(end)) => &line[start..=end],
        _ => &[],
    }
}

/// The name of a `[Section]` header, or `None` for a line that is not one.
fn section_of(line: &[u8]) -> Option<&[u8]> {
    line.strip_prefix(b"[").and_then(|rest| rest.strip_suffix(b"]"))
}

/// The key of a `key=value` line, with the spaces in front of the `=` taken off; `None` for a line
/// that is not an entry at all.
fn entry_key(line: &[u8]) -> Option<&[u8]> {
    let at = line.iter().position(|byte| *byte == b'=')?;
    let key = trim_end(&line[..at]);
    (!key.is_empty()).then_some(key)
}

/// The line with the spaces at its end taken off.
fn trim_end(line: &[u8]) -> &[u8] {
    let end = line.iter().rposition(|byte| !byte.is_ascii_whitespace()).map_or(0, |end| end + 1);
    &line[..end]
}

/// Whether `part` can be written on a line of a `mimeapps.list` as itself: a kind or a desktop file
/// id, with nothing in it the format reads as a separator, and with no room for the spaces that
/// would come off it again.
fn storable(part: &str) -> bool {
    !part.is_empty()
        && trim(part.as_bytes()) == part.as_bytes()
        && !part.contains(|c: char| c.is_whitespace() || matches!(c, '=' | ';' | '[' | ']' | '#' | '\\'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::Apps;
    use crate::desktop::fixture::Tree;

    /// A `mimeapps.list` as `set_default` wrote it, read back as text.
    fn written(tree: &Tree, mime: &str, app_id: &str) -> String {
        let change = set_default(&tree.dirs(), mime, app_id).expect("the line can be written");
        assert!(matches!(change, Change::Added | Change::Replaced));
        let after = fs::read_to_string(tree.path("home/config").join(MIMEAPPS_LIST)).expect("the file can be read");
        // A write leaves nothing of its own behind next to the file.
        let names: Vec<String> = fs::read_dir(tree.path("home/config"))
            .expect("list")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, [MIMEAPPS_LIST], "only the file itself stays in its folder");
        after
    }

    #[test]
    fn a_kind_with_no_line_gains_one_under_its_section() {
        let tree = Tree::new();
        tree.write(
            "home/config/mimeapps.list",
            "# written by the desktop\n[Default Applications]\napplication/pdf=viewer.desktop\n\n\
             [Added Associations]\ntext/plain=vim.desktop;\n",
        );
        let after = written(&tree, "text/markdown", "reader.desktop");
        assert_eq!(
            after,
            "# written by the desktop\n[Default Applications]\napplication/pdf=viewer.desktop\n\
             text/markdown=reader.desktop;\n\n[Added Associations]\ntext/plain=vim.desktop;\n"
        );
    }

    #[test]
    fn a_kind_with_a_line_of_its_own_has_it_replaced_and_the_rest_left_alone() {
        let tree = Tree::new();
        tree.write(
            "home/config/mimeapps.list",
            "# the person's own choices\n[Default Applications]\ntext/markdown=reader.desktop\n\
             application/pdf=viewer.desktop\n\n[Added Associations]\ntext/markdown=writer.desktop;\n\
             [Removed Associations]\ntext/markdown=editor.desktop;\n",
        );
        let after = written(&tree, "text/markdown", "writer.desktop");
        assert_eq!(
            after,
            "# the person's own choices\n[Default Applications]\ntext/markdown=writer.desktop;\n\
             application/pdf=viewer.desktop\n\n[Added Associations]\ntext/markdown=writer.desktop;\n\
             [Removed Associations]\ntext/markdown=editor.desktop;\n",
            "only the kind's own line changes; the removal is the person's and stays"
        );
    }

    #[test]
    fn the_only_file_that_changes_is_the_persons_own() {
        let tree = Tree::new();
        tree.write("usr/mime/globs2", "50:text/markdown:*.md\n");
        tree.write(
            "usr/applications/reader.desktop",
            "[Desktop Entry]\nType=Application\nName=Reader\nExec=reader %f\nMimeType=text/markdown;\n",
        );
        tree.write("etc/mimeapps.list", "[Default Applications]\ntext/markdown=system.desktop;\n");
        tree.write("home/config/kde-mimeapps.list", "[Default Applications]\ntext/markdown=desktop.desktop;\n");
        let mut dirs = tree.dirs();
        dirs.desktops = vec!["kde".to_owned()];
        let change = set_default(&dirs, "text/markdown", "reader.desktop").expect("the line can be written");
        assert_eq!(change, Change::Added, "the person's own file had no line for the kind at all");
        assert_eq!(
            fs::read_to_string(tree.path("etc/mimeapps.list")).expect("read"),
            "[Default Applications]\ntext/markdown=system.desktop;\n",
            "a file under the system's configuration folders is never written"
        );
        assert_eq!(
            fs::read_to_string(tree.path("home/config/kde-mimeapps.list")).expect("read"),
            "[Default Applications]\ntext/markdown=desktop.desktop;\n",
            "a running desktop's own list is its own to write"
        );
        assert_eq!(
            fs::read_to_string(tree.path("home/config/mimeapps.list")).expect("read"),
            "[Default Applications]\ntext/markdown=reader.desktop;\n"
        );
        assert!(!tree.path("home/config/reader.desktop").exists(), "no .desktop file is written either");
    }

    #[test]
    fn a_file_that_is_not_text_is_not_written_over() {
        let tree = Tree::new();
        let before = b"[Default Applications]\ntext/markdown=\xff\xfe\n";
        tree.write("home/config/mimeapps.list", before);
        let error =
            set_default(&tree.dirs(), "text/markdown", "reader.desktop").expect_err("a file of bytes is not rewritten");
        assert!(matches!(error, SetDefaultError::NotText), "{error}");
        assert_eq!(
            fs::read(tree.path("home/config").join(MIMEAPPS_LIST)).expect("read"),
            before,
            "the file is left exactly as it was"
        );
    }

    #[test]
    fn a_file_that_was_never_written_is_made_with_its_folder() {
        let tree = Tree::new();
        let mut dirs = tree.dirs();
        dirs.config_home = Some(tree.path("home/deeper/config"));
        let change = set_default(&dirs, "text/markdown", "reader.desktop").expect("the file and its folder are made");
        assert_eq!(change, Change::Added);
        assert_eq!(
            fs::read_to_string(tree.path("home/deeper/config").join(MIMEAPPS_LIST)).expect("read"),
            "[Default Applications]\ntext/markdown=reader.desktop;\n"
        );
    }

    #[test]
    fn without_a_configuration_folder_of_the_persons_own_there_is_nothing_to_write_in() {
        let tree = Tree::new();
        tree.write("etc/mimeapps.list", "[Default Applications]\n");
        let dirs = XdgDirs { config_home: None, ..tree.dirs() };
        let error =
            set_default(&dirs, "text/markdown", "reader.desktop").expect_err("the system's folder is not the person's");
        assert!(matches!(error, SetDefaultError::NoConfigHome), "{error}");
    }

    #[test]
    fn apps_read_back_the_program_that_was_written() {
        let tree = Tree::new();
        tree.write("usr/mime/subclasses", "text/x-rust text/plain\n");
        tree.write(
            "usr/applications/a.desktop",
            "[Desktop Entry]\nType=Application\nName=A\nExec=a %f\nMimeType=text/plain;\n",
        );
        tree.write(
            "usr/applications/b.desktop",
            "[Desktop Entry]\nType=Application\nName=B\nExec=b %f\nMimeType=text/plain;\n",
        );
        tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/plain=a.desktop\n");
        let dirs = tree.dirs();
        let db = crate::desktop::MimeDb::load(&dirs);
        assert_eq!(
            Apps::load(&dirs, "C", None).default_for(&db, "text/plain").map(|app| app.id.clone()),
            Some("a.desktop".to_owned()),
            "the file the person's desktop wrote is what the person chose"
        );
        assert_eq!(set_default(&dirs, "text/plain", "b.desktop").expect("written"), Change::Replaced);
        let apps = Apps::load(&dirs, "C", None);
        assert_eq!(apps.default_for(&db, "text/plain").map(|app| app.id.clone()), Some("b.desktop".to_owned()));
        assert_eq!(apps.default_for(&db, "text/x-rust").map(|app| app.id.clone()), Some("b.desktop".to_owned()));
    }

    #[test]
    fn a_kind_with_no_section_at_all_gets_one_at_the_end_of_the_file() {
        let tree = Tree::new();
        tree.write("home/config/mimeapps.list", "[Added Associations]\ntext/plain=vim.desktop;\n");
        let after = written(&tree, "text/markdown", "reader.desktop");
        assert_eq!(
            after,
            "[Added Associations]\ntext/plain=vim.desktop;\n[Default Applications]\ntext/markdown=reader.desktop;\n"
        );
    }

    #[test]
    fn a_file_without_a_break_at_its_end_stays_a_file_without_one() {
        let tree = Tree::new();
        tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/markdown=reader.desktop");
        let after = written(&tree, "text/markdown", "writer.desktop");
        assert_eq!(after, "[Default Applications]\ntext/markdown=writer.desktop;");
        tree.write("home/config/mimeapps.list", "[Default Applications]");
        let after = written(&tree, "text/markdown", "reader.desktop");
        assert_eq!(after, "[Default Applications]\ntext/markdown=reader.desktop;\n");
    }

    #[test]
    fn the_first_of_two_lines_of_a_kind_is_the_one_that_counts_and_the_other_stays() {
        let tree = Tree::new();
        tree.write(
            "home/config/mimeapps.list",
            "[Default Applications]\ntext/markdown=reader.desktop\ntext/markdown=writer.desktop\n",
        );
        let after = written(&tree, "text/markdown", "writer.desktop");
        assert_eq!(
            after, "[Default Applications]\ntext/markdown=writer.desktop;\ntext/markdown=writer.desktop\n",
            "a second line of the same kind is the person's or their desktop's, and is left there"
        );
    }

    #[test]
    fn lines_nobody_understands_are_kept_where_they_are() {
        let tree = Tree::new();
        tree.write(
            "home/config/mimeapps.list",
            "a line before any section\n[Default Applications]\nnot an entry\n\
             text/markdown=reader.desktop\n[unclosed\n",
        );
        let after = written(&tree, "text/markdown", "writer.desktop");
        assert_eq!(
            after,
            "a line before any section\n[Default Applications]\nnot an entry\n\
             text/markdown=writer.desktop;\n[unclosed\n"
        );
    }

    #[test]
    fn a_line_of_the_kind_with_no_equals_sign_is_not_touched_away() {
        let tree = Tree::new();
        tree.write(
            "home/config/mimeapps.list",
            "[Default Applications]\ntext/markdown\napplication/pdf=viewer.desktop\n",
        );
        let error =
            set_default(&tree.dirs(), "text/markdown", "reader.desktop").expect_err("a line is standing in the way");
        assert!(matches!(&error, SetDefaultError::InTheWay { line } if line == "text/markdown"), "{error}");
        assert_eq!(
            fs::read_to_string(tree.path("home/config").join(MIMEAPPS_LIST)).expect("read"),
            "[Default Applications]\ntext/markdown\napplication/pdf=viewer.desktop\n"
        );
    }

    #[test]
    fn a_name_that_cannot_be_written_on_a_line_is_refused() {
        let tree = Tree::new();
        for app_id in ["a;b", "", " reader.desktop", "reader\n.desktop"] {
            let error = set_default(&tree.dirs(), "text/markdown", app_id).expect_err("no line can hold it");
            assert!(matches!(error, SetDefaultError::Unstorable), "{app_id:?}: {error}");
        }
        assert!(matches!(
            set_default(&tree.dirs(), "text/markdown=editor.desktop", "reader.desktop"),
            Err(SetDefaultError::Unstorable)
        ));
        assert!(storable("text/markdown") && storable("org.gnome.TextEditor.desktop"));
        assert!(!tree.path("home/config/mimeapps.list").exists(), "nothing is written for a refused name");
    }

    #[cfg(unix)]
    #[test]
    fn a_read_only_file_is_never_written_and_nothing_is_lost() {
        use std::os::unix::fs::PermissionsExt as _;

        let tree = Tree::new();
        let before = "[Default Applications]\ntext/markdown=reader.desktop\n";
        let file = tree.write("home/config/mimeapps.list", before);
        fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).expect("the mode can be set");
        let error =
            set_default(&tree.dirs(), "text/markdown", "writer.desktop").expect_err("a read-only file is left alone");
        assert!(matches!(error, SetDefaultError::ReadOnly), "{error}");
        assert_eq!(fs::read_to_string(&file).expect("read"), before);
        assert_eq!(
            fs::read_dir(tree.path("home/config")).expect("list").count(),
            1,
            "no temporary file is left behind either"
        );
    }
}
