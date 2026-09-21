//! The family's one switch for the update notice: whether its applications ask once a day if a
//! newer version of themselves is out.
//!
//! It lives in the family's shared file only, as `update-notice = false`, because it is a choice
//! about the family rather than one application: a person who does not want to be told turns it
//! off once. A missing key is on, so a family that never heard of the notice has it.

use std::fs;
use std::io;
use std::path::Path;

use super::{Family, SettingValue, Settings};

/// The switch as the shared file `settings` has it: on unless it says `false`.
pub(super) fn from_shared(settings: &Settings) -> bool {
    settings.get::<bool>(Settings::UPDATE_NOTICE).unwrap_or(true)
}

impl Family {
    /// Whether the family's applications say when a newer version is out: the shared file's
    /// `update-notice`, on when the file or the key is missing. Off without a home folder, where
    /// nothing could remember that it was asked.
    #[must_use]
    pub fn update_notice(&self) -> bool {
        self.config_dir().is_some_and(|dir| self.update_notice_in(&dir))
    }

    /// [`update_notice`](Self::update_notice) with `config_dir` as the family's folder instead of
    /// this platform's, for a test or a demo.
    #[must_use]
    pub fn update_notice_in(&self, config_dir: &Path) -> bool {
        let path = config_dir.join(super::family::file_name(self.id()));
        !path.exists() || from_shared(&Settings::open(path))
    }

    /// Turns the update notice on or off for every application of the family, in the shared file.
    /// The file is read right before it is written and only this key changes in it, as
    /// [`set`](Self::set) does.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`io::ErrorKind::NotFound`] when there is no home folder, and any
    /// error from writing.
    pub fn set_update_notice(&self, on: bool) -> io::Result<()> {
        match self.config_dir() {
            Some(dir) => self.set_update_notice_in(&dir, on),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no config directory found")),
        }
    }

    /// [`set_update_notice`](Self::set_update_notice) with `config_dir` as the family's folder
    /// instead of this platform's.
    ///
    /// ```
    /// use qframe::storage::Family;
    ///
    /// # let folder = std::env::temp_dir().join(format!("quvyta-update-notice-doc-{}", std::process::id()));
    /// assert!(Family::QUVYTA.update_notice_in(&folder), "on until someone turns it off");
    /// Family::QUVYTA.set_update_notice_in(&folder, false).expect("saved");
    /// assert!(!Family::QUVYTA.update_notice_in(&folder));
    /// # std::fs::remove_dir_all(&folder).ok();
    /// ```
    ///
    /// # Errors
    ///
    /// Any error from writing.
    pub fn set_update_notice_in(&self, config_dir: &Path, on: bool) -> io::Result<()> {
        fs::create_dir_all(config_dir)?;
        let _held = super::preferences::hold_folder(config_dir)?;
        let shared = config_dir.join(super::family::file_name(self.id()));
        super::preferences::rewrite(&shared, Settings::UPDATE_NOTICE, SettingValue::Bool(on), None)
    }
}

#[cfg(test)]
mod tests {
    use crate::i18n::I18n;
    use crate::storage::Family;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-update-notice-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_notice_is_on_until_the_family_turns_it_off_and_other_keys_stay() {
        let dir = scratch("switch");
        let family = Family::QUVYTA;
        assert!(family.update_notice_in(&dir), "a family that never chose has it");
        std::fs::create_dir_all(&dir).expect("folder");
        std::fs::write(dir.join("quvyta.conf"), "theme = \"amber\"\n").expect("shared file");
        assert!(family.update_notice_in(&dir), "a shared file without the key has it");
        family.set_update_notice_in(&dir, false).expect("saved");
        assert!(!family.update_notice_in(&dir));
        assert!(!family.preferences_in(&dir, "code", &I18n::builtin()).update_notice(), "preferences read it too");
        let text = std::fs::read_to_string(dir.join("quvyta.conf")).expect("read");
        assert!(text.contains("theme = \"amber\"") && text.contains("update-notice = false"), "{text}");
        family.set_update_notice_in(&dir, true).expect("saved");
        assert!(family.preferences_in(&dir, "code", &I18n::builtin()).update_notice());
        std::fs::remove_dir_all(dir).ok();
    }
}
