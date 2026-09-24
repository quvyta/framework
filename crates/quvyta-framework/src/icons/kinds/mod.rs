//! What a file is, told from its name: the icon a list draws for it and the family it belongs to.
//!
//! A person should know what a file is from its icon before reading its name. [`file_kind`] picks
//! the icon key from the name alone, so a folder of ten thousand entries is drawn without opening
//! any of them. The keys are icons of the built-in set: in a Nerd Font each kind draws its own
//! glyph (the Rust logo, a PDF page, a zipper), and without one each draws its family's shape, so
//! code, pictures and archives are still told apart in Unicode and in ASCII.
//!
//! The folders of a person's home, whose names depend on their language, are
//! [`UserFolders`]' to recognise.

mod table;
mod user;

#[cfg(test)]
mod tests;

pub use user::UserFolders;

/// The family a kind of file belongs to: what its shape is outside a Nerd Font, and what colour it
/// takes where kinds are coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KindFamily {
    /// A folder, whatever it holds.
    Folder,
    /// Plain text: notes, logs, a readme, a licence.
    Text,
    /// A document laid out for reading: PDF, a word processor's file, a book.
    Document,
    /// A spreadsheet, a table of values or a presentation.
    Sheet,
    /// Source code and the files that build it.
    Code,
    /// Data and settings: JSON, TOML, YAML, a database, a lock file.
    Data,
    /// A picture, a drawing or a 3D scene.
    Image,
    /// Sound.
    Audio,
    /// Moving pictures.
    Video,
    /// An archive of other files.
    Archive,
    /// A package to install, or the image of a whole disk.
    Package,
    /// A program or a library of one.
    Executable,
    /// A key, a signature or a certificate.
    Key,
    /// A font.
    Font,
    /// A file whose kind is not known.
    File,
}

impl KindFamily {
    /// The theme colour the family takes where kinds are coloured, or `None` for a file whose kind
    /// is not known, which keeps the row's own colour.
    ///
    /// Folders take the accent, and the files four of the theme's series tones: code and writing
    /// one, pictures, sound, video and fonts one, data and keys one, and archives, packages and
    /// programs one. The first series tone is the accent itself in every built-in theme, so the
    /// folders stand for it. The colour only repeats what the shape already says.
    #[must_use]
    pub fn tone(self) -> Option<&'static str> {
        match self {
            Self::Folder => Some("accent"),
            Self::Code | Self::Text | Self::Document | Self::Sheet => Some("series-2"),
            Self::Image | Self::Audio | Self::Video | Self::Font => Some("series-3"),
            Self::Data | Self::Key => Some("series-4"),
            Self::Archive | Self::Package | Self::Executable => Some("series-5"),
            Self::File => None,
        }
    }
}

/// The kind of a file or folder: the icon it is drawn with and the family that icon belongs to.
///
/// Found by [`file_kind`], or by [`UserFolders::kind`] for the folders of a home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileKind {
    icon: &'static str,
    family: KindFamily,
}

impl FileKind {
    /// A file whose kind is not known: the `file` icon.
    const FILE: Self = Self { icon: "file", family: KindFamily::File };

    /// A folder whose name says nothing more: the `folder` icon.
    const FOLDER: Self = Self { icon: "folder", family: KindFamily::Folder };

    /// A file that is run: the `file-executable` icon.
    const EXECUTABLE: Self = Self { icon: "file-executable", family: KindFamily::Executable };

    /// The kind drawn with the icon `key` of the tables, in the family the tables give it.
    fn of(key: &'static str) -> Self {
        let family =
            table::KINDS.binary_search_by(|(kind, _)| kind.cmp(&key)).map_or(KindFamily::File, |at| table::KINDS[at].1);
        Self { icon: key, family }
    }

    /// The icon key of the built-in set, such as `"file-rust"`, `"folder-git"`, or `"file"` and
    /// `"folder"` when nothing more is known. A theme or an application can restyle any of them.
    #[must_use]
    pub fn icon(&self) -> &'static str {
        self.icon
    }

    /// The family the kind belongs to.
    #[must_use]
    pub fn family(&self) -> KindFamily {
        self.family
    }
}

/// The kind of the entry called `name`: a folder when `folder`, and a file that may be run when
/// `executable`.
///
/// The name alone decides, in this order: the whole name (`Cargo.toml`, `Dockerfile`, `PKGBUILD`,
/// `.bashrc`, and `README` or `LICENSE` with any text ending such as `README.de.md` or
/// `LICENSE-MIT`), then an ending of several parts (`.tar.gz`, `.pkg.tar.zst`, `.d.ts`), then the
/// extension, then, for a folder, a name that says what it holds (`.git`, `node_modules`, `src`),
/// and a folder the platform hides that says nothing more (`.mozilla`).
/// A file that may be run shows as a program only when nothing before that recognised it, so
/// `build.sh` stays a shell script. Anything else is `file` or `folder`: every entry has an icon.
/// Letter case never matters.
///
/// Nothing is read from the disk. The folders of a home, such as `Downloads` or its translation,
/// are [`UserFolders`]' to recognise, because their names are the person's own.
///
/// ```
/// use qframe::icons::{KindFamily, file_kind};
///
/// assert_eq!(file_kind("main.rs", false, false).icon(), "file-rust");
/// assert_eq!(file_kind("backup.TAR.GZ", false, false).family(), KindFamily::Archive);
/// assert_eq!(file_kind("configure", false, true).icon(), "file-executable");
/// assert_eq!(file_kind(".git", true, false).icon(), "folder-git");
/// assert_eq!(file_kind("notes", true, false).icon(), "folder");
/// ```
#[must_use]
pub fn file_kind(name: &str, folder: bool, executable: bool) -> FileKind {
    let lower = name.to_ascii_lowercase();
    if folder {
        let hidden = lower.starts_with('.').then_some("folder-hidden");
        return find(table::FOLDERS, &lower).or(hidden).map_or(FileKind::FOLDER, FileKind::of);
    }
    let known = find(table::NAMES, &lower).or_else(|| by_lead(&lower)).or_else(|| by_ending(&lower));
    match known {
        Some(key) => FileKind::of(key),
        None if executable => FileKind::EXECUTABLE,
        None => FileKind::FILE,
    }
}

/// The value of `key` in the sorted `table`.
fn find(table: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table.binary_search_by(|(name, _)| (*name).cmp(key)).ok().map(|at| table[at].1)
}

/// Names that keep their kind whatever follows them: `README.de.md`, `LICENSE-MIT`, `COPYING`.
const LEADS: &[(&str, &str)] =
    &[("copying", "file-license"), ("licence", "file-license"), ("license", "file-license"), ("readme", "file-readme")];

/// Endings a readme or a licence is written with. Any other ending is a file of its own kind that
/// only starts with the word, such as `license.rs` or `readme_test.py`.
const TEXT_ENDINGS: &[&str] = &["adoc", "markdown", "md", "org", "rst", "txt"];

/// The kind of a readme or a licence, however its name goes on.
fn by_lead(lower: &str) -> Option<&'static str> {
    LEADS.iter().find_map(|&(lead, key)| {
        let rest = lower.strip_prefix(lead)?;
        let joined = rest.is_empty() || rest.starts_with(['.', '-', '_']);
        let ending = rest.rsplit_once('.').map(|(_, ending)| ending);
        (joined && ending.is_none_or(|ending| TEXT_ENDINGS.contains(&ending))).then_some(key)
    })
}

/// The kind the end of the name says: the longest ending of up to three parts that the tables
/// know, so `x.pkg.tar.zst` is a package before it is a `.tar.zst` archive or a `.zst` file.
///
/// The dot a hidden name starts with begins no ending: `.bashrc` has none.
fn by_ending(lower: &str) -> Option<&'static str> {
    // The dots of the last three endings, nearest first; a dot at the very start is not one.
    let mut dots = [0usize; 3];
    let mut count = 0;
    for (at, _) in lower.rmatch_indices('.').filter(|(at, _)| *at > 0).take(dots.len()) {
        dots[count] = at;
        count += 1;
    }
    dots[..count].iter().rev().find_map(|&at| {
        let ending = &lower[at + 1..];
        if ending.contains('.') { find(table::DOUBLES, ending) } else { find(table::EXTENSIONS, ending) }
    })
}
