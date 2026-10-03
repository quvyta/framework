//! The programs installed, from their desktop entries, and which of them open which kind, from
//! the entries and from `mimeapps.list`.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use super::exec::{self, Fields};
use super::keyfile::{self, Group};
use super::program::{find_program, is_executable};
use super::{MimeDb, XdgDirs, read_small, warn, warn_at};
use crate::diagnostics::Diagnostic;

#[cfg(test)]
mod tests;

/// How deep below `applications` desktop entries are looked for. Real trees are one or two
/// levels deep; the limit keeps a folder link pointing back up from being followed forever.
const DEEPEST: usize = 8;

/// The name of the file that holds a person's choices, in every folder it is read from and in the
/// one of them that is written.
pub(super) const MIMEAPPS_LIST: &str = "mimeapps.list";

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
        exec::expand(
            &self.exec,
            &Fields { file: Some(file), name: &self.name, icon: self.icon.as_deref(), entry: &self.path },
        )
    }

    /// The command that starts this program with nothing to open, program first: the `Exec` line
    /// with the codes that take a file or a URL left out, `%i`, `%c` and `%k` filled in as in
    /// [`command`](Self::command), and `%%` a percent. `None` when the line is empty or malformed.
    ///
    /// A launcher's own list, a favourite, a panel item or a second command for the same program
    /// starts it with nothing to open, so `foo %U --x` is `foo --x` and not `foo --x ` with an
    /// empty argument where the file would have been. No shell is involved, and a program that is
    /// given a file as well is started once for each.
    #[must_use]
    pub fn launch_command(&self) -> Option<Vec<OsString>> {
        exec::expand(
            &self.exec,
            &Fields { file: None, name: &self.name, icon: self.icon.as_deref(), entry: &self.path },
        )
    }
}

/// What a desktop entry says about a program besides the command that opens a file with it, read
/// from the same keys in the same pass: the words a launcher lists a program by, searches it with
/// and leaves out of a menu.
///
/// One of these stands for each [`DesktopApp`] an [`Apps`] holds, in the same order; ask for it
/// by the program's desktop file id with [`Apps::details`]. The `Name`, `Exec`, `Terminal`,
/// `MimeType` and `Icon` keys are the [`DesktopApp`]'s own.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryDetails {
    /// What the program says it is for, in the person's language (`Comment`): "Edit text files".
    pub comment: Option<String>,
    /// What kind of program this is in a few words, in the person's language (`GenericName`):
    /// "Text Editor". A launcher shows it beside the program's own name.
    pub generic_name: Option<String>,
    /// The menus the program belongs to (`Categories`): `Utility`, `TextEditor`. A launcher
    /// builds its own menu out of these.
    pub categories: Vec<String>,
    /// The words a search finds the program by (`Keywords`): the ones in the person's language
    /// first, then the plain ones, each written once and in the order the entry writes them.
    pub keywords: Vec<String>,
    /// The folder the program works in when it is given none (`Path`).
    pub folder: Option<PathBuf>,
    /// Whether the entry says `NoDisplay=true`: the program is left out of menus but still opens
    /// files, so a launcher leaves it out of its own list rather than out of "Open with".
    pub no_display: bool,
    /// Whether the entry says `Hidden=true`, which says the program was deleted. A plain
    /// [`Apps::load`] leaves such an entry out; a read with [`Include::hidden`] keeps it, says so
    /// here and never offers it as a program that opens a file.
    pub hidden: bool,
    /// The program that has to be there for this entry to be of any use (`TryExec`), as it was
    /// written. Empty when the key is written empty, which names no program at all.
    pub try_exec: Option<String>,
    /// Whether that program was found: an absolute path has to be an executable file, a bare
    /// name is looked for in the folders of the search path the read was given. An entry with no
    /// `TryExec` is installed. `false` only for an entry kept with [`Include::missing`], since a
    /// plain [`Apps::load`] drops the others.
    pub installed: bool,
}

/// Which entries a read keeps that a plain [`Apps::load`] leaves out, as in
/// [`Apps::load_including`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Include {
    /// Whether an entry whose [`TryExec`](EntryDetails::try_exec) program is not installed is kept,
    /// with [`EntryDetails::installed`] false. A program that is not installed can open nothing
    /// yet, so a plain read drops it; a launcher keeps it to say so and to offer the package.
    pub missing: bool,
    /// Whether an entry marked `Hidden=true` is kept, with [`EntryDetails::hidden`] true. Its id
    /// is taken either way, so a program's own copy still hides the system's; keeping it only
    /// lets an application see what the person deleted. A kept entry is never offered by
    /// [`for_mime`](Apps::for_mime) or [`default_for`](Apps::default_for), nor is one kept with
    /// [`missing`](Self::missing).
    pub hidden: bool,
}

impl Include {
    /// Keeps nothing a plain [`Apps::load`] leaves out.
    pub const NONE: Self = Self { missing: false, hidden: false };
    /// Keeps the programs whose own program is not installed.
    pub const MISSING: Self = Self { missing: true, hidden: false };
    /// Keeps the entries marked `Hidden`, which say a program was deleted.
    pub const HIDDEN: Self = Self { missing: false, hidden: true };
    /// Keeps both kinds of entry.
    pub const ALL: Self = Self { missing: true, hidden: true };
}

/// The installed programs and the person's and the system's choices of which opens what.
#[derive(Debug, Clone, Default)]
pub struct Apps {
    /// Every program, in the order found: the person's own folder first.
    apps: Vec<DesktopApp>,
    /// What every entry says besides its command, in the order of `apps`.
    details: Vec<EntryDetails>,
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
    /// menus but still open files. `lang` (`tr_TR.UTF-8`) picks the name, the comment and the
    /// keywords: `Name[tr_TR]`, then `Name[tr]`, then `Name`. An application entry without a
    /// name, without an `Exec` line or with one that cannot be split is skipped with a
    /// [`Diagnostic`], as is a line that cannot be read; its id stays taken, so a broken copy of
    /// the person's never brings back the system's.
    ///
    /// A launcher that lists every program, the ones it cannot start yet among them, reads with
    /// [`Apps::load_including`] and a [`Include`] instead.
    #[must_use]
    pub fn load(dirs: &XdgDirs, lang: &str, path_var: Option<&OsStr>) -> Self {
        Self::read(dirs, lang, path_var, Include::NONE)
    }

    /// Reads what [`Apps::load`] reads and keeps the entries `include` asks for: a program whose own
    /// `TryExec` is not installed, and an entry marked `Hidden`. Their [`EntryDetails`] say which
    /// they are, and a program that is not installed is never offered as one that opens a file.
    ///
    /// Everything else is the same: of two entries with one id the first found still wins, a
    /// hidden entry's id is still taken, and a broken entry is still skipped with its
    /// [`Diagnostic`].
    #[must_use]
    pub fn load_including(dirs: &XdgDirs, lang: &str, path_var: Option<&OsStr>, include: Include) -> Self {
        Self::read(dirs, lang, path_var, include)
    }

    /// The read both loaders do.
    fn read(dirs: &XdgDirs, lang: &str, path_var: Option<&OsStr>, include: Include) -> Self {
        let mut seen = HashSet::new();
        let mut apps = Vec::new();
        let mut details = Vec::new();
        let mut diagnostics = Vec::new();
        for dir in dirs.data() {
            let root = dir.join("applications");
            let mut found = Vec::new();
            entries(&root, &root, 0, &mut found);
            found.sort();
            for (id, path) in found {
                if seen.insert(id.clone())
                    && let Some((app, entry)) = read_entry(id, path, lang, path_var, include, &mut diagnostics)
                {
                    apps.push(app);
                    details.push(entry);
                }
            }
        }
        let mut lists = Vec::new();
        for path in list_paths(dirs) {
            if let Some(bytes) = read_small(&path, &mut diagnostics) {
                lists.push(MimeAppsList::parse(&bytes, &path, &mut diagnostics));
            }
        }
        Self { apps, details, lists, diagnostics }
    }

    /// The problems found while reading, in the order the files were read.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Every installed program, in the order found: the person's own folder first, each folder
    /// in name order. A launcher leaves out the ones whose entry says `NoDisplay`, which
    /// [`details`](Self::details) reports.
    #[must_use]
    pub fn all(&self) -> &[DesktopApp] {
        &self.apps
    }

    /// The program with this desktop file id, when it is installed.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&DesktopApp> {
        self.index_of(id).map(|index| &self.apps[index])
    }

    /// What the entry of the program with this desktop file id says besides its command, when it
    /// is installed.
    #[must_use]
    pub fn details(&self, id: &str) -> Option<&EntryDetails> {
        self.index_of(id).map(|index| &self.details[index])
    }

    /// Where the program with this id stands among [`all`](Self::all).
    fn index_of(&self, id: &str) -> Option<usize> {
        self.apps.iter().position(|app| app.id == id)
    }

    /// Whether the program an entry describes may open a file: neither one the person deleted nor
    /// one whose own program is missing can start, so neither is ever a choice, whatever kinds it
    /// declared and whatever a `mimeapps.list` file names.
    fn opens(entry: &EntryDetails) -> bool {
        !entry.hidden && entry.installed
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
    /// system data folder. A listed id that is not installed is passed over, and so is one a
    /// read with [`Include`] kept that may not open anything: a program the person deleted, and one
    /// whose own program is not installed, are in the list of programs but not choices for a file.
    #[must_use]
    pub fn for_mime(&self, db: &MimeDb, mime: &str) -> Vec<&DesktopApp> {
        let mut out: Vec<&DesktopApp> = Vec::new();
        for kind in db.ancestors(mime) {
            let listed = self.listed(&kind, Section::Default).into_iter().chain(self.listed(&kind, Section::Added));
            let declared = self
                .apps
                .iter()
                .zip(&self.details)
                .filter(|(_, entry)| Self::opens(entry))
                .filter(|(app, _)| {
                    app.mime_types.iter().any(|declared| declared == &kind || db.canonical(declared) == kind)
                        && !self.removed_before(self.lists.len(), &kind, &app.id)
                })
                .map(|(app, _)| app);
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
            .filter_map(|id| self.index_of(id).filter(|index| Self::opens(&self.details[*index])))
            .map(|index| &self.apps[index])
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
            out.push(folder.join(format!("{desktop}-{MIMEAPPS_LIST}")));
        }
        out.push(folder.join(MIMEAPPS_LIST));
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

/// The program a desktop entry describes, with what else it says, or `None` when it describes
/// none that may be used.
fn read_entry(
    id: String,
    path: PathBuf,
    lang: &str,
    path_var: Option<&OsStr>,
    include: Include,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(DesktopApp, EntryDetails)> {
    let bytes = read_small(&path, diagnostics)?;
    let groups = keyfile::parse(&bytes, &path, diagnostics);
    let Some(entry) = groups.iter().find(|group| group.name == "Desktop Entry") else {
        warn(diagnostics, &path, 1, "the file has no [Desktop Entry] group, so it is no program");
        return None;
    };
    let flag = |key: &str| entry.get(key).is_some_and(|value| value.trim() == "true");
    if entry.get("Type").map(str::trim) != Some("Application") {
        return None;
    }
    // `Hidden` says the program was deleted: it takes its id, so the copies below it stay out, and
    // only a read that keeps it can say what was removed.
    let hidden = flag("Hidden");
    if hidden && !include.hidden {
        return None;
    }
    let try_exec = entry.get("TryExec").map(keyfile::string).map(|program| program.trim().to_owned());
    let installed = try_exec.as_ref().is_none_or(|program| find_program(program, path_var, is_executable).is_some());
    if !installed && !include.missing {
        return None;
    }
    let Some(name) = localized(entry, "Name", lang).filter(|name| !name.trim().is_empty()) else {
        warn_at(diagnostics, &path, entry.line, entry.column, "an application needs a Name; the program is skipped");
        return None;
    };
    let Some(exec) = entry.get("Exec").map(keyfile::string) else {
        warn_at(
            diagnostics,
            &path,
            entry.line,
            entry.column,
            "an application needs an Exec line; the program is skipped",
        );
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
        let (line, column) = entry.at("Exec").unwrap_or((entry.line, entry.column));
        warn_at(diagnostics, &app.path, line, column, "the Exec line gives no command; the program is skipped");
        return None;
    }
    let details = EntryDetails {
        comment: described(entry, "Comment", lang),
        generic_name: described(entry, "GenericName", lang),
        categories: entry.get("Categories").map(keyfile::list).unwrap_or_default(),
        keywords: keywords(entry, lang),
        folder: entry
            .get("Path")
            .map(keyfile::string)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        no_display: flag("NoDisplay"),
        hidden,
        try_exec,
        installed,
    };
    Some((app, details))
}

/// A value in the person's language that is written once, read as `name` is: `None` when the key
/// is missing or holds nothing but space.
fn described(entry: &Group, key: &str, lang: &str) -> Option<String> {
    localized(entry, key, lang).map(|value| value.trim().to_owned()).filter(|value| !value.is_empty())
}

/// The `Keywords` in the person's language and then the plain ones, each written once and in the
/// order the entry writes them: a search offers all of them, so none of them is left out.
fn keywords(entry: &Group, lang: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let both = [localized(entry, "Keywords", lang), entry.get("Keywords").map(keyfile::string)];
    for raw in both.into_iter().flatten() {
        for keyword in keyfile::list(&raw) {
            if !out.contains(&keyword) {
                out.push(keyword);
            }
        }
    }
    out
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
