//! What kind a file is, as shared-mime-info tells it: from its name first, from its first bytes
//! when the name says nothing. What a kind is called in words is in `comment.rs`.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::{XdgDirs, lines, read_small, warn};
use crate::diagnostics::Diagnostic;

#[cfg(test)]
mod tests;

/// The kind of a folder.
const DIRECTORY: &str = "inode/directory";
/// The kind every text is.
const TEXT: &str = "text/plain";
/// The kind every file is.
const OCTET: &str = "application/octet-stream";

/// How much of a file is read to tell text from anything else. Enough for a text to show itself,
/// little enough that opening a film costs nothing.
const SNIFF: usize = 4096;

/// A glob pattern that stands in `globs2` for "forget the patterns less important folders give
/// this kind".
const NO_GLOBS: &str = "__NOGLOBS__";

/// The shared-mime-info database: name patterns, aliases and which kind is a special case of
/// which.
#[derive(Debug, Clone, Default)]
pub struct MimeDb {
    /// Every pattern, the one that wins first.
    globs: Vec<Glob>,
    /// An old or alternative name of a kind, and the kind's own name.
    aliases: HashMap<String, String>,
    /// A kind and the kinds it is a special case of.
    parents: HashMap<String, Vec<String>>,
    /// The problems found while reading, in the order the files were read.
    diagnostics: Vec<Diagnostic>,
    /// The data folders, the person's own first, where a kind's own file is read from when its
    /// words are asked for.
    pub(super) data: Vec<PathBuf>,
}

/// One line of `globs2`.
#[derive(Debug, Clone)]
struct Glob {
    weight: u32,
    /// The pattern's length as written, the tie-breaker between equal weights: `*.tar.gz` says
    /// more than `*.gz`.
    len: usize,
    /// The pattern, lowercased unless it is case-sensitive.
    pattern: Vec<char>,
    case_sensitive: bool,
    mime: String,
}

impl MimeDb {
    /// Reads `mime/globs2`, `mime/aliases` and `mime/subclasses` from every data folder, the
    /// person's own first. A missing file reads as empty, and a line that makes no sense is
    /// skipped with a [`Diagnostic`]: a damaged database costs the kinds it describes, never the
    /// program.
    #[must_use]
    pub fn load(dirs: &XdgDirs) -> Self {
        let mut db = Self { data: dirs.data().cloned().collect(), ..Self::default() };
        // Kinds a more important folder has said "no patterns from below" for.
        let mut sealed: HashSet<String> = HashSet::new();
        for dir in dirs.data() {
            let dir = dir.join("mime");
            let file = dir.join("globs2");
            if let Some(bytes) = read_small(&file, &mut db.diagnostics) {
                let mut sealing = HashSet::new();
                for (number, line) in lines(&bytes, &file, &mut db.diagnostics) {
                    let Some(glob) = parse_glob(line) else {
                        if !is_blank(line) {
                            warn(
                                &mut db.diagnostics,
                                &file,
                                number,
                                "a glob line is weight:type:pattern; it is skipped",
                            );
                        }
                        continue;
                    };
                    if sealed.contains(&glob.mime) {
                        continue;
                    }
                    if glob.pattern.iter().collect::<String>().eq_ignore_ascii_case(NO_GLOBS) {
                        sealing.insert(glob.mime);
                    } else {
                        db.globs.push(glob);
                    }
                }
                sealed.extend(sealing);
            }
            for (alias, mime) in db.pairs(&dir.join("aliases")) {
                db.aliases.entry(alias).or_insert(mime);
            }
            for (child, parent) in db.pairs(&dir.join("subclasses")) {
                let parents = db.parents.entry(child).or_default();
                if !parents.contains(&parent) {
                    parents.push(parent);
                }
            }
        }
        // A stable sort: between patterns of equal weight and length, the more important folder
        // and the earlier line still win.
        // A case-sensitive pattern is tried before a case-blind one of the same weight, as the
        // specification asks, so `main.C` is C++ whatever order the lines come in.
        db.globs.sort_by(|a, b| {
            b.weight.cmp(&a.weight).then(b.case_sensitive.cmp(&a.case_sensitive)).then(b.len.cmp(&a.len))
        });
        db
    }

    /// The problems found while reading, in the order the files were read.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// The two-word lines of an `aliases` or `subclasses` file; any other line is reported.
    fn pairs(&mut self, file: &Path) -> Vec<(String, String)> {
        let Some(bytes) = read_small(file, &mut self.diagnostics) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (number, line) in lines(&bytes, file, &mut self.diagnostics) {
            match pair(line) {
                Some((first, second)) => out.push((first.to_owned(), second.to_owned())),
                None if is_blank(line) => {}
                None => warn(
                    &mut self.diagnostics,
                    file,
                    number,
                    "the line should be two kinds separated by a space; it is skipped",
                ),
            }
        }
        out
    }

    /// The kind a file name says a file is: the pattern with the highest weight wins, then the
    /// longest. Letter case does not matter unless the pattern says it does, and a pattern that
    /// says so wins a tie with one that does not.
    #[must_use]
    pub fn guess(&self, name: &str) -> Option<String> {
        let exact: Vec<char> = name.chars().collect();
        let lower: Vec<char> = name.to_lowercase().chars().collect();
        self.globs
            .iter()
            .find(|glob| {
                let name = if glob.case_sensitive { &exact } else { &lower };
                glob_match(&glob.pattern, name)
            })
            .map(|glob| glob.mime.clone())
    }

    /// The kind of the file at `path`.
    ///
    /// A folder is `inode/directory`. Otherwise the name decides; when no pattern fits, the first
    /// 4 KiB do: nothing at all, or valid UTF-8 with no NUL byte, is `text/plain`, anything else
    /// is `application/octet-stream`, and so is a file that cannot be read. Pipes, sockets and
    /// devices are named for what they are and never opened, since reading one could wait
    /// forever.
    #[must_use]
    pub fn sniff(&self, path: &Path) -> String {
        let Ok(meta) = fs::metadata(path) else {
            return OCTET.to_owned();
        };
        if let Some(special) = special_kind(&meta.file_type()) {
            return special.to_owned();
        }
        path.file_name()
            .and_then(|name| self.guess(&name.to_string_lossy()))
            .unwrap_or_else(|| contents(path).to_owned())
    }

    /// A kind's own name when `mime` is an alias of it, else `mime` as it is.
    #[must_use]
    pub fn canonical(&self, mime: &str) -> String {
        self.aliases.get(mime).cloned().unwrap_or_else(|| mime.to_owned())
    }

    /// `mime` and every kind it is a special case of, nearest first: a program for any of them
    /// opens a file of this kind.
    ///
    /// The list starts with `mime` itself (and its own name when it is an alias), then the kinds
    /// shared-mime-info names, breadth first. Every `text/*` is also `text/plain`, and every kind
    /// but the `inode/*` ones ends in `application/octet-stream`: a folder is not a stream of
    /// bytes, so a hex editor is no program for it.
    #[must_use]
    pub fn ancestors(&self, mime: &str) -> Vec<String> {
        let mut out = vec![mime.to_owned()];
        let canonical = self.canonical(mime);
        if canonical != mime {
            out.push(canonical.clone());
        }
        let mut next = 0;
        while let Some(current) = out.get(next).cloned() {
            next += 1;
            let named = self.parents.get(&current).into_iter().flatten().map(|parent| self.canonical(parent));
            let text = (current.starts_with("text/") && current != TEXT).then(|| TEXT.to_owned());
            for parent in named.chain(text) {
                if parent != OCTET && !out.contains(&parent) {
                    out.push(parent);
                }
            }
        }
        if !canonical.starts_with("inode/") && !out.iter().any(|kind| kind == OCTET) {
            out.push(OCTET.to_owned());
        }
        out
    }
}

/// Whether a line of a database file says nothing: blank or a comment.
fn is_blank(line: &str) -> bool {
    let line = line.trim();
    line.is_empty() || line.starts_with('#')
}

/// The kind of anything that is not a regular file: a folder, and on Unix a pipe, a socket or a
/// device.
fn special_kind(kind: &fs::FileType) -> Option<&'static str> {
    if kind.is_dir() {
        return Some(DIRECTORY);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if kind.is_fifo() {
            return Some("inode/fifo");
        }
        if kind.is_socket() {
            return Some("inode/socket");
        }
        if kind.is_char_device() {
            return Some("inode/chardevice");
        }
        if kind.is_block_device() {
            return Some("inode/blockdevice");
        }
    }
    None
}

/// A `globs2` line: `weight:type:pattern[:flags]`.
fn parse_glob(line: &str) -> Option<Glob> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut fields = line.splitn(4, ':');
    let weight = fields.next()?.trim().parse().ok()?;
    let mime = fields.next()?.trim();
    let pattern = fields.next()?;
    let case_sensitive = fields.next().is_some_and(|flags| flags.split(',').any(|flag| flag.trim() == "cs"));
    if !mime.contains('/') || pattern.is_empty() {
        return None;
    }
    let pattern = if case_sensitive { pattern.to_owned() } else { pattern.to_lowercase() };
    Some(Glob {
        weight,
        len: pattern.chars().count(),
        pattern: pattern.chars().collect(),
        case_sensitive,
        mime: mime.to_owned(),
    })
}

/// A line of two words, as in `aliases` and `subclasses`.
fn pair(line: &str) -> Option<(&str, &str)> {
    if line.trim_start().starts_with('#') {
        return None;
    }
    let mut words = line.split_whitespace();
    let pair = (words.next()?, words.next()?);
    words.next().is_none().then_some(pair)
}

/// Whether `name` fits a shell-style pattern: `*` is any run of characters, `?` any one, `[...]`
/// one of a set.
fn glob_match(pattern: &[char], name: &[char]) -> bool {
    let (mut p, mut n) = (0, 0);
    // Where the last `*` was, and how much of the name it has taken so far.
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        if let Some(&c) = pattern.get(p) {
            let step = match c {
                '*' => {
                    star = Some((p, n));
                    p += 1;
                    continue;
                }
                '?' => Some(p + 1),
                '[' => match set(pattern, p, name[n]) {
                    Some((true, end)) => Some(end),
                    Some((false, _)) => None,
                    None => (name[n] == '[').then_some(p + 1),
                },
                _ => (c == name[n]).then_some(p + 1),
            };
            if let Some(step) = step {
                p = step;
                n += 1;
                continue;
            }
        }
        let Some((star_p, star_n)) = star else {
            return false;
        };
        p = star_p + 1;
        n = star_n + 1;
        star = Some((star_p, star_n + 1));
    }
    pattern[p..].iter().all(|&c| c == '*')
}

/// Whether `c` is in the set that opens at `pattern[start]`, and where the pattern goes on after
/// it; `None` when the set is never closed, and the `[` is then an ordinary character.
fn set(pattern: &[char], start: usize, c: char) -> Option<(bool, usize)> {
    let mut i = start + 1;
    let negate = matches!(pattern.get(i), Some('!' | '^'));
    if negate {
        i += 1;
    }
    let mut found = false;
    let mut first = true;
    loop {
        let &low = pattern.get(i)?;
        if low == ']' && !first {
            return Some((found != negate, i + 1));
        }
        first = false;
        match (pattern.get(i + 1), pattern.get(i + 2)) {
            (Some('-'), Some(&high)) if high != ']' => {
                found |= (low..=high).contains(&c);
                i += 3;
            }
            _ => {
                found |= low == c;
                i += 1;
            }
        }
    }
}

/// The kind the first bytes of a file show: text or not.
fn contents(path: &Path) -> &'static str {
    let Ok(file) = File::open(path) else {
        return OCTET;
    };
    let mut head = Vec::with_capacity(SNIFF);
    if file.take(SNIFF as u64).read_to_end(&mut head).is_err() || head.contains(&0) {
        return OCTET;
    }
    match std::str::from_utf8(&head) {
        Ok(_) => TEXT,
        // Cutting the file at 4 KiB may split a character in two; that is still text.
        Err(error) if error.error_len().is_none() && head.len() == SNIFF => TEXT,
        Err(_) => OCTET,
    }
}
