//! The programs installed, from their desktop entries, and which of them open which kind, from
//! the entries and from `mimeapps.list`.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use super::exec::{self, Fields};
use super::keyfile::{self, Group};
use super::program::{find_program, is_executable};
use super::{MimeDb, XdgDirs, read_small, warn};
use crate::diagnostics::Diagnostic;

#[cfg(test)]
mod tests;

/// How deep below `applications` desktop entries are looked for. Real trees are one or two
/// levels deep; the limit keeps a folder link pointing back up from being followed forever.
const DEEPEST: usize = 8;

/// A program, as its desktop entry describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopApp {
    /// The desktop file id (`org.gnome.TextEditor.desktop`): the entry's path below
    /// `applications`, folders joined with `-`. `mimeapps.list` names programs by it.
    pub id: String,
    /// The program's name in the person's language when the entry has one.
    pub name: String,
    /// The `Exec` line, with the key file's escapes resolved.
    pub exec: String,
    /// Whether the program runs inside a terminal.
    pub terminal: bool,
    /// The kinds the program says it opens.
    pub mime_types: Vec<String>,
    /// Where the desktop entry is.
    pub path: PathBuf,
    /// The program's icon name, when it has one.
    pub icon: Option<String>,
}

impl DesktopApp {
    /// The command that opens `file` with this program, program first: the `Exec` line with its
    /// field codes filled in. `None` when the line is empty or malformed.
    ///
    /// No shell is involved; the file is always one argument, whatever its name holds.
    #[must_use]
    pub fn command(&self, file: &Path) -> Option<Vec<OsString>> {
        exec::expand(&self.exec, &Fields { file, name: &self.name, icon: self.icon.as_deref(), entry: &self.path })
    }
}

/// The installed programs and the person's and the system's choices of which opens what.
#[derive(Debug, Clone, Default)]
pub struct Apps {
    /// Every program, in the order found: the person's own folder first.
    apps: Vec<DesktopApp>,
    /// Every `mimeapps.list` found, the most important first.
    lists: Vec<MimeAppsList>,
    /// The problems found while reading, in the order the files were read.
    diagnostics: Vec<Diagnostic>,
}

impl Apps {
    /// Reads the desktop entries below `applications` in every data folder and every
    /// `mimeapps.list`.
    ///
    /// Of two entries with the same id the first found wins, so the person's own copy overrides
    /// the system's, and an entry marked `Hidden` hides the system's copy as well as itself. Only
    /// `Type=Application` entries count. An entry whose `TryExec` program is not installed is
    /// dropped: an absolute path must be an executable file, a bare name is looked for in the
    /// folders of `path_var`. Entries marked `NoDisplay` are kept, since they are left out of
    /// menus but still open files. `lang` (`tr_TR.UTF-8`) picks the name: `Name[tr_TR]`, then
    /// `Name[tr]`, then `Name`. An application entry without a name, without an `Exec` line or
    /// with one that cannot be split is skipped with a [`Diagnostic`], as is a line that cannot
    /// be read; its id stays taken, so a broken copy of the person's never brings back the
    /// system's.
    #[must_use]
    pub fn load(dirs: &XdgDirs, lang: &str, path_var: Option<&OsStr>) -> Self {
        let mut seen = HashSet::new();
        let mut apps = Vec::new();
        let mut diagnostics = Vec::new();
        for dir in dirs.data() {
            let root = dir.join("applications");
            let mut found = Vec::new();
            entries(&root, &root, 0, &mut found);
            found.sort();
            for (id, path) in found {
                if seen.insert(id.clone())
                    && let Some(app) = read_entry(id, path, lang, path_var, &mut diagnostics)
                {
                    apps.push(app);
                }
            }
        }
        let mut lists = Vec::new();
        for path in list_paths(dirs) {
            if let Some(bytes) = read_small(&path, &mut diagnostics) {
                lists.push(MimeAppsList::parse(&bytes, &path, &mut diagnostics));
            }
        }
        Self { apps, lists, diagnostics }
    }

    /// The problems found while reading, in the order the files were read.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Every installed program, in the order found: the person's own folder first, each folder
    /// in name order. A launcher leaves out the ones whose entry says `NoDisplay`.
    #[must_use]
    pub fn all(&self) -> &[DesktopApp] {
        &self.apps
    }

    /// The program with this desktop file id, when it is installed.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&DesktopApp> {
        self.apps.iter().find(|app| app.id == id)
    }

    /// The programs that open a file of kind `mime`, the most fitting first, each once.
    ///
    /// For `mime` and then each kind it is a special case of ([`MimeDb::ancestors`]): the
    /// defaults `mimeapps.list` names for it, then its `[Added Associations]`, then the programs
    /// whose entries list it. A program in the `[Removed Associations]` of a kind is left out for
    /// that kind, unless a more important file adds it back. The `mimeapps.list` files are read
    /// in this order, the first the most important: `<desktop>-mimeapps.list` for each running
    /// desktop and then `mimeapps.list`, in the person's configuration folder, in each system
    /// configuration folder, then below `applications` in the person's data folder and in each
    /// system data folder. A listed id that is not installed is passed over.
    #[must_use]
    pub fn for_mime(&self, db: &MimeDb, mime: &str) -> Vec<&DesktopApp> {
        let mut out: Vec<&DesktopApp> = Vec::new();
        for kind in db.ancestors(mime) {
            let listed = self.listed(&kind, Section::Default).into_iter().chain(self.listed(&kind, Section::Added));
            let declared = self.apps.iter().filter(|app| {
                app.mime_types.iter().any(|declared| declared == &kind || db.canonical(declared) == kind)
                    && !self.removed_before(self.lists.len(), &kind, &app.id)
            });
            for app in listed.chain(declared) {
                if !out.iter().any(|known| known.id == app.id) {
                    out.push(app);
                }
            }
        }
        out
    }

    /// The program a file of kind `mime` opens with when nothing else is asked for: the one
    /// `[Default Applications]` names for the kind or, failing that, for the nearest kind it is a
    /// special case of; else the first of [`for_mime`](Self::for_mime); else `None`.
    #[must_use]
    pub fn default_for(&self, db: &MimeDb, mime: &str) -> Option<&DesktopApp> {
        db.ancestors(mime)
            .iter()
            .find_map(|kind| self.listed(kind, Section::Default).into_iter().next())
            .or_else(|| self.for_mime(db, mime).into_iter().next())
    }

    /// The installed programs one section of the `mimeapps.list` files names for `kind`, in
    /// order, less those a more important file removed.
    fn listed(&self, kind: &str, section: Section) -> Vec<&DesktopApp> {
        self.lists
            .iter()
            .enumerate()
            .flat_map(|(at, list)| {
                list.section(section)
                    .get(kind)
                    .into_iter()
                    .flatten()
                    .filter(move |id| !self.removed_before(at, kind, id))
            })
            .filter_map(|id| self.get(id))
            .collect()
    }

    /// Whether one of the first `count` `mimeapps.list` files removes program `id` for `kind`.
    ///
    /// A file's own additions are made before its removals, so it only removes what less
    /// important files and the entries themselves say.
    fn removed_before(&self, count: usize, kind: &str, id: &str) -> bool {
        self.lists[..count]
            .iter()
            .any(|list| list.removed.get(kind).is_some_and(|ids| ids.iter().any(|removed| removed == id)))
    }
}

/// The sections of `mimeapps.list` that name programs to use.
#[derive(Debug, Clone, Copy)]
enum Section {
    Default,
    Added,
}

/// One `mimeapps.list`: for each kind, the program ids of each section.
#[derive(Debug, Clone, Default)]
struct MimeAppsList {
    defaults: HashMap<String, Vec<String>>,
    added: HashMap<String, Vec<String>>,
    removed: HashMap<String, Vec<String>>,
}

impl MimeAppsList {
    fn parse(bytes: &[u8], path: &Path, diagnostics: &mut Vec<Diagnostic>) -> Self {
        let mut list = Self::default();
        for group in keyfile::parse(bytes, path, diagnostics) {
            let section = match group.name.as_str() {
                "Default Applications" => &mut list.defaults,
                "Added Associations" => &mut list.added,
                "Removed Associations" => &mut list.removed,
                _ => continue,
            };
            for (kind, ids) in group.entries() {
                let ids = keyfile::list(ids);
                let known = section.entry(kind.to_owned()).or_default();
                for id in ids {
                    if !known.contains(&id) {
                        known.push(id);
                    }
                }
            }
        }
        list
    }

    fn section(&self, section: Section) -> &HashMap<String, Vec<String>> {
        match section {
            Section::Default => &self.defaults,
            Section::Added => &self.added,
        }
    }
}

/// Every `mimeapps.list` that may exist, the most important first.
fn list_paths(dirs: &XdgDirs) -> Vec<PathBuf> {
    // A desktop name is joined into a file name; one holding a `/` could reach outside the folder.
    let desktops: Vec<&String> =
        dirs.desktops.iter().filter(|desktop| !desktop.is_empty() && !desktop.contains('/')).collect();
    let folders = dirs.config().cloned().chain(dirs.data().map(|dir| dir.join("applications")));
    let mut out = Vec::new();
    for folder in folders {
        for desktop in &desktops {
            out.push(folder.join(format!("{desktop}-mimeapps.list")));
        }
        out.push(folder.join("mimeapps.list"));
    }
    out
}

/// Collects the desktop entries below `folder` as `(id, path)`.
fn entries(root: &Path, folder: &Path, depth: usize, out: &mut Vec<(String, PathBuf)>) {
    let Ok(read) = fs::read_dir(folder) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            if depth < DEEPEST {
                entries(root, &path, depth + 1, out);
            }
        } else if meta.is_file()
            && path.extension().is_some_and(|ext| ext == "desktop")
            && let Some(id) = path.strip_prefix(root).ok().and_then(Path::to_str).map(|rel| rel.replace('/', "-"))
        {
            out.push((id, path));
        }
    }
}

/// The program a desktop entry describes, or `None` when it describes none that may be used.
fn read_entry(
    id: String,
    path: PathBuf,
    lang: &str,
    path_var: Option<&OsStr>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<DesktopApp> {
    let bytes = read_small(&path, diagnostics)?;
    let groups = keyfile::parse(&bytes, &path, diagnostics);
    let Some(entry) = groups.iter().find(|group| group.name == "Desktop Entry") else {
        warn(diagnostics, &path, 1, "the file has no [Desktop Entry] group, so it is no program");
        return None;
    };
    let flag = |key: &str| entry.get(key).is_some_and(|value| value.trim() == "true");
    if entry.get("Type").map(str::trim) != Some("Application") || flag("Hidden") {
        return None;
    }
    if let Some(program) = entry.get("TryExec").map(keyfile::string)
        && find_program(program.trim(), path_var, is_executable).is_none()
    {
        return None;
    }
    let Some(name) = localized(entry, "Name", lang).filter(|name| !name.trim().is_empty()) else {
        warn(diagnostics, &path, entry.line, "an application needs a Name; the program is skipped");
        return None;
    };
    let Some(exec) = entry.get("Exec").map(keyfile::string) else {
        warn(diagnostics, &path, entry.line, "an application needs an Exec line; the program is skipped");
        return None;
    };
    let app = DesktopApp {
        id,
        name,
        exec,
        terminal: flag("Terminal"),
        mime_types: entry.get("MimeType").map(keyfile::list).unwrap_or_default(),
        path,
        icon: entry.get("Icon").map(keyfile::string).filter(|icon| !icon.trim().is_empty()),
    };
    // A line that gives no command for a file gives none for any: an unclosed quote, a field code
    // the standard does not know, nothing to run. Such a program is never offered.
    if app.command(Path::new("file")).is_none() {
        let line = entry.line_of("Exec").unwrap_or(entry.line);
        warn(diagnostics, &app.path, line, "the Exec line gives no command; the program is skipped");
        return None;
    }
    Some(app)
}

/// A value in the person's language: `key[lang_COUNTRY@MODIFIER]`, `key[lang_COUNTRY]`,
/// `key[lang@MODIFIER]`, `key[lang]`, then `key`, as the desktop entry standard orders them.
fn localized(entry: &Group, key: &str, lang: &str) -> Option<String> {
    let (base, modifier) = match lang.split_once('@') {
        Some((base, modifier)) => (base, Some(modifier)),
        None => (lang, None),
    };
    let base = base.split('.').next().unwrap_or_default();
    let (language, country) = match base.split_once('_') {
        Some((language, country)) => (language, Some(country)),
        None => (base, None),
    };
    let mut locales = Vec::new();
    if !language.is_empty() && language != "C" && language != "POSIX" {
        if let (Some(country), Some(modifier)) = (country, modifier) {
            locales.push(format!("{language}_{country}@{modifier}"));
        }
        if let Some(country) = country {
            locales.push(format!("{language}_{country}"));
        }
        if let Some(modifier) = modifier {
            locales.push(format!("{language}@{modifier}"));
        }
        locales.push(language.to_owned());
    }
    locales
        .iter()
        .find_map(|locale| entry.get(&format!("{key}[{locale}]")))
        .or_else(|| entry.get(key))
        .map(keyfile::string)
}
