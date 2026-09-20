//! What an entry is besides its name: how big it is, when it changed last and who may do what
//! with it.
//!
//! Every one of these means another call to the system for that one entry, so they are never read
//! for a whole folder. A tree row shows a name and nothing else and reads nothing; the views that
//! show details ask for a page of entries around the cursor, and an application that knows exactly
//! which rows it draws asks for those.

use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::date::{DateTime, local_offset_minutes};

/// How many entries one ask reads: always more than a screen holds and far less than a large
/// folder, so scrolling rarely waits and ten thousand entries never mean ten thousand calls.
pub(super) const PAGE: usize = 200;

/// Units a size is said in, each a thousand of the one before.
const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];

/// A size below this is said in whole units; above it one decimal is kept.
const WHOLE: f64 = 10.0;

/// What one entry is besides its name: its size, when it changed last, and its permissions.
///
/// A symbolic link is read as itself rather than as what it points at, the way the rows show it:
/// a link out of the folder must not be followed for a number on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileDetails {
    /// Its size in bytes. A folder's own size says how big the folder itself is on disk, not what
    /// is in it; adding up a folder means reading all of it, which a manager never does by itself.
    pub size: u64,
    /// When it changed last, in seconds since 1970-01-01 UTC, or `None` where the system does not
    /// say.
    pub modified: Option<i64>,
    /// The permission bits, where the platform has them (unix), and `None` where it does not.
    pub mode: Option<u32>,
    /// Whether it may not be written, which every platform says.
    pub readonly: bool,
}

impl FileDetails {
    /// Reads the details of the entry at `path`, or `None` when the system says nothing about it
    /// — it is gone, or may not be looked at.
    ///
    /// This touches the disk, so it belongs on a background thread;
    /// [`FileManagerState`](super::FileManagerState) asks for it with
    /// [`Command::perform`](crate::runtime::Command::perform) and never while drawing.
    #[must_use]
    pub fn read(path: &Path) -> Option<Self> {
        let data = std::fs::symlink_metadata(path).ok()?;
        let modified = data
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|since| i64::try_from(since.as_secs()).ok());
        Some(Self { size: data.len(), modified, mode: mode_of(&data), readonly: data.permissions().readonly() })
    }

    /// The size in the largest unit that leaves a number worth reading: `840 B`, `9.4 kB`,
    /// `12 MB`. A folder's size is left out, because its own size says nothing about what is in
    /// it and a number that means nothing is worse than none.
    #[must_use]
    pub fn size_text(&self, folder: bool) -> String {
        if folder {
            return String::new();
        }
        let mut size = self.size as f64;
        let mut unit = 0;
        while size >= 1000.0 && unit + 1 < UNITS.len() {
            size /= 1000.0;
            unit += 1;
        }
        let number =
            if unit == 0 || size >= WHOLE { format!("{}", size.round() as u64) } else { crate::i18n::number(size, 1) };
        crate::t!("quvyta.file-manager.size", n = number.as_str(), unit = UNITS[unit])
    }

    /// When it changed last, as the year, the month, the day and the clock the machine stands by:
    /// `2026-09-20 14:32`. Empty where the system does not say.
    #[must_use]
    pub fn modified_text(&self) -> String {
        let Some(seconds) = self.modified else { return String::new() };
        let moment = DateTime::from_unix(seconds, local_offset_minutes());
        let (date, time) = (moment.date, moment.time);
        format!("{date} {:02}:{:02}", time.hour, time.minute)
    }

    /// Who may do what with it: the nine letters unix writes them with (`rwxr-xr-x`), and on a
    /// platform without them the one thing it does say, whether the entry may be written.
    #[must_use]
    pub fn permissions_text(&self, folder: bool) -> String {
        let Some(mode) = self.mode else {
            let key =
                if self.readonly { "quvyta.file-manager.read-only" } else { "quvyta.file-manager.read-and-write" };
            return crate::t!(key);
        };
        let kind = if folder { 'd' } else { '-' };
        let letters = ['r', 'w', 'x'];
        let mut text = String::with_capacity(10);
        text.push(kind);
        for group in 0..3 {
            for (bit, letter) in letters.iter().enumerate() {
                let shift = (2 - group) * 3 + (2 - bit);
                if mode & (1 << shift) == 0 {
                    text.push('-');
                } else {
                    text.push(*letter);
                }
            }
        }
        text
    }
}

/// The permission bits where the platform has them.
#[cfg(unix)]
fn mode_of(data: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(data.permissions().mode())
}

/// Nothing where it does not; [`FileDetails::readonly`] is all such a platform says.
#[cfg(not(unix))]
fn mode_of(_data: &std::fs::Metadata) -> Option<u32> {
    None
}
