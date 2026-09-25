//! The one look at an application's own file that turns shared values written there as fixed ones
//! back into following the ecosystem.

use std::fs;
use std::io;
use std::path::Path;

use super::ecosystem::file_name;
use super::preferences::hold_folder;
use super::{Ecosystem, SettingValue, Settings, Shared};
use crate::diagnostics::Severity;
use crate::icons::IconMode;

impl Ecosystem {
    /// Looks once at the shared keys application `app` wrote into its own file as fixed values,
    /// and makes the application follow the ecosystem again where nothing is lost by it.
    ///
    /// For an application that used to write a choice meant for every member into its own file
    /// alone. For each of language, theme, icons and reduced motion that its file names as a value
    /// of its own:
    ///
    /// - the same value as the [shared file](Self::shared_file) holds: the key becomes the
    ///   ecosystem's id, so the application follows the shared value from now on;
    /// - another value: it stays, since it is what the person chose for this application.
    ///
    /// Reduced motion was each application's own before it was shared, so a shared file may not
    /// hold it yet; it then counts as off, the value every application reads from such a file. An
    /// application that kept motion follows the ecosystem again, one that reduced it keeps that.
    ///
    /// Then the file takes [`Settings::SHARED_CHECKED`]` = true` and every later call changes
    /// nothing. The mark sits in the application's own file, beside the values it speaks for, so it
    /// travels with them when the settings folder is copied to another machine. Without it, a
    /// person who later chose "only here" for the value the ecosystem happens to share would find
    /// the choice undone at the next start. A missing file is created holding the mark alone: an
    /// application that starts without one writes its shared keys the new way, and nothing it
    /// writes afterwards is a leftover to look at. A [`Setup`](crate::widgets::Setup) still counts
    /// such a file as no settings at all.
    ///
    /// Call it at start, before the application's settings are loaded
    /// ([`Settings::load_member`]), so what it loads already holds the change; a copy loaded
    /// earlier would save the old values back. The file is read right before it is written and
    /// only the changed keys and the mark change in it, with the ecosystem's folder held by an
    /// advisory lock on Unix, as [`set`](Self::set) does. Comments do not survive a change, as with
    /// [`Settings::save`]. The shared file is only read.
    ///
    /// Returns the keys that now follow the ecosystem, in the order of [`Shared::ALL`]; empty when
    /// nothing changed or the look was already taken.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`io::ErrorKind::NotFound`] when there is no home folder, of kind
    /// [`io::ErrorKind::InvalidData`] when the application's file cannot be read as settings, which
    /// is then left exactly as it was, and any error from writing.
    pub fn settle(&self, app: &str) -> io::Result<Vec<Shared>> {
        match self.config_dir() {
            Some(dir) => self.settle_in(&dir, app),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no config directory found")),
        }
    }

    /// [`settle`](Self::settle) with `config_dir` as the ecosystem's folder instead of this
    /// platform's, for a test or a demo that must leave the user's own files alone.
    ///
    /// ```
    /// use qframe::storage::{Ecosystem, Shared};
    ///
    /// # let folder = std::env::temp_dir().join(format!("quvyta-settle-doc-{}", std::process::id()));
    /// # std::fs::create_dir_all(&folder).expect("folder");
    /// std::fs::write(folder.join("quvyta.conf"), "theme = \"nordic\"\nlanguage = \"tr\"\n").expect("shared");
    /// std::fs::write(folder.join("desktop.conf"), "theme = \"nordic\"\nlanguage = \"en\"\n").expect("own");
    /// let now_following = Ecosystem::QUVYTA.settle_in(&folder, "desktop").expect("settle");
    /// assert_eq!(now_following, [Shared::Theme]);
    /// let own = std::fs::read_to_string(folder.join("desktop.conf")).expect("read");
    /// assert_eq!(own, "theme = \"quvyta\"\nlanguage = \"en\"\nshared-checked = true\n");
    /// # std::fs::remove_dir_all(&folder).ok();
    /// ```
    ///
    /// # Errors
    ///
    /// As [`settle`](Self::settle), except that there is always a folder.
    pub fn settle_in(&self, config_dir: &Path, app: &str) -> io::Result<Vec<Shared>> {
        fs::create_dir_all(config_dir)?;
        let _held = hold_folder(config_dir)?;
        let mut own = Settings::open(config_dir.join(file_name(app))).member_of(self);
        if let Some(problem) = own.diagnostics().iter().find(|problem| problem.severity == Severity::Error) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, problem.to_string()));
        }
        if own.get::<bool>(Settings::SHARED_CHECKED) == Some(true) {
            return Ok(Vec::new());
        }
        let shared_path = config_dir.join(file_name(self.id()));
        let shared = shared_path.is_file().then(|| Settings::open(&shared_path));
        let mut following = Vec::new();
        for key in Shared::ALL {
            let fixed = key.read(&own).filter(|text| *text != self.id());
            let common = shared
                .as_ref()
                .and_then(|shared| key.read(shared))
                .or_else(|| (key == Shared::ReducedMotion && shared.is_some()).then(|| false.to_string()));
            if let (Some(fixed), Some(common)) = (fixed, common)
                && common != self.id()
                && same(key, &fixed, &common)
            {
                own.store(key.key(), SettingValue::Text(self.id().to_owned()));
                following.push(key);
            }
        }
        own.store(Settings::SHARED_CHECKED, SettingValue::Bool(true));
        own.save()?;
        Ok(following)
    }
}

/// Whether `fixed` and `common` are one value of `key`: icon modes by the mode they name, the rest
/// as written, apart from the spaces around them.
fn same(key: Shared, fixed: &str, common: &str) -> bool {
    match key {
        Shared::Icons => IconMode::from_name(fixed).is_some_and(|mode| IconMode::from_name(common) == Some(mode)),
        Shared::ReducedMotion => fixed.parse::<bool>().is_ok_and(|reduced| common.parse() == Ok(reduced)),
        Shared::Language | Shared::Theme => {
            let fixed = fixed.trim();
            !fixed.is_empty() && fixed == common.trim()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A folder of this test's own; the real settings folder is never touched.
    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("quvyta-settle-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the folder");
        path
    }

    fn read(folder: &Path, name: &str) -> String {
        fs::read_to_string(folder.join(name)).unwrap_or_default()
    }

    const SHARED: &str = "language = \"tr\"\ntheme = \"nordic\"\nicons = \"unicode\"\n";

    #[test]
    fn a_value_equal_to_the_shared_one_follows_the_ecosystem() {
        let folder = folder("equal");
        fs::write(folder.join("quvyta.conf"), SHARED).expect("shared");
        fs::write(
            folder.join("desktop.conf"),
            "language = \"tr\"\ntheme = \"nordic\"\nicons = \"unicode\"\ndock = 3\n",
        )
        .expect("own");
        let following = Ecosystem::QUVYTA.settle_in(&folder, "desktop").expect("settle");
        assert_eq!(following, [Shared::Language, Shared::Theme, Shared::Icons]);
        assert_eq!(
            read(&folder, "desktop.conf"),
            "language = \"quvyta\"\ntheme = \"quvyta\"\nicons = \"quvyta\"\ndock = 3\nshared-checked = true\n"
        );
        assert_eq!(read(&folder, "quvyta.conf"), SHARED, "the shared file is only read");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_value_other_than_the_shared_one_stays() {
        let folder = folder("different");
        fs::write(folder.join("quvyta.conf"), SHARED).expect("shared");
        fs::write(folder.join("code.conf"), "language = \"en\"\ntheme = \"nordic\"\nicons = \"nerd\"\n").expect("own");
        let following = Ecosystem::QUVYTA.settle_in(&folder, "code").expect("settle");
        assert_eq!(following, [Shared::Theme]);
        let own = read(&folder, "code.conf");
        assert!(own.contains("language = \"en\""), "the person's language stays: {own}");
        assert!(own.contains("icons = \"nerd\""), "and so do the icons: {own}");
        assert!(own.contains("theme = \"quvyta\""), "{own}");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn reduced_motion_kept_off_follows_and_reduced_stays_when_the_shared_file_never_held_it() {
        let folder = folder("motion");
        fs::write(folder.join("quvyta.conf"), SHARED).expect("shared");
        fs::write(folder.join("code.conf"), "reduced-motion = false\n").expect("code");
        fs::write(folder.join("focus.conf"), "reduced-motion = true\n").expect("focus");
        assert_eq!(Ecosystem::QUVYTA.settle_in(&folder, "code").expect("code"), [Shared::ReducedMotion]);
        assert_eq!(read(&folder, "code.conf"), "reduced-motion = \"quvyta\"\nshared-checked = true\n");
        assert!(Ecosystem::QUVYTA.settle_in(&folder, "focus").expect("focus").is_empty());
        assert!(read(&folder, "focus.conf").contains("reduced-motion = true"), "the person's need stays");
        // Once the ecosystem reduces motion too, the same need follows it.
        fs::write(folder.join("quvyta.conf"), format!("{SHARED}reduced-motion = true\n")).expect("shared");
        fs::write(folder.join("focus.conf"), "reduced-motion = true\n").expect("focus");
        assert_eq!(Ecosystem::QUVYTA.settle_in(&folder, "focus").expect("again"), [Shared::ReducedMotion]);
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_second_run_changes_nothing() {
        let folder = folder("twice");
        fs::write(folder.join("quvyta.conf"), SHARED).expect("shared");
        fs::write(folder.join("code.conf"), "theme = \"iris\"\n").expect("own");
        assert!(Ecosystem::QUVYTA.settle_in(&folder, "code").expect("first").is_empty());
        // The person now keeps the shared theme here alone, on purpose.
        let mut own = Settings::open(folder.join("code.conf")).member_of(&Ecosystem::QUVYTA);
        own.set("theme", "nordic".to_owned());
        own.save().expect("the choice");
        let before = read(&folder, "code.conf");
        assert!(Ecosystem::QUVYTA.settle_in(&folder, "code").expect("second").is_empty());
        assert_eq!(read(&folder, "code.conf"), before, "the choice is not undone");
        assert!(before.contains("theme = \"nordic\""), "{before}");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_missing_file_takes_only_the_mark() {
        let folder = folder("missing");
        fs::write(folder.join("quvyta.conf"), SHARED).expect("shared");
        assert!(Ecosystem::QUVYTA.settle_in(&folder, "code").expect("settle").is_empty());
        assert_eq!(read(&folder, "code.conf"), "shared-checked = true\n");
        let bare = folder.join("bare");
        assert!(Ecosystem::QUVYTA.settle_in(&bare, "code").expect("no shared file either").is_empty());
        assert_eq!(read(&bare, "code.conf"), "shared-checked = true\n");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_broken_file_is_left_as_it_was() {
        let folder = folder("broken");
        fs::write(folder.join("quvyta.conf"), SHARED).expect("shared");
        fs::write(folder.join("code.conf"), "theme = \nicons = \"nerd\"\n").expect("own");
        let error = Ecosystem::QUVYTA.settle_in(&folder, "code").expect_err("broken");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(read(&folder, "code.conf"), "theme = \nicons = \"nerd\"\n");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_mark_survives_a_member_s_self_healing() {
        let text = "theme = \"nordic\"\nshared-checked = true\n";
        let healed = Settings::parse_str("code.conf", text)
            .member_of(&Ecosystem::QUVYTA)
            .schema(super::super::Schema::builtin())
            .self_heal(true);
        assert_eq!(healed.get::<bool>(Settings::SHARED_CHECKED), Some(true));
        assert!(healed.diagnostics().is_empty(), "{:?}", healed.diagnostics());
    }
}
