//! Fake desktop trees for the tests. Every test builds its own under the temporary folder and it
//! is removed when the test ends, so no test ever reads the machine's own databases.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::XdgDirs;

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A temporary folder holding a person's and a system's data and configuration folders.
pub(super) struct Tree {
    root: PathBuf,
}

impl Tree {
    pub(super) fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("qframe-desktop-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the temporary folder can be made");
        Self { root }
    }

    pub(super) fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// Writes a file below the tree, making its folders.
    pub(super) fn write(&self, rel: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("the folder can be made");
        }
        fs::write(&path, contents).expect("the file can be written");
        path
    }

    /// Writes a file that may be run.
    pub(super) fn program(&self, rel: &str) -> PathBuf {
        let path = self.write(rel, "#!/bin/sh\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("mode can be set");
        }
        path
    }

    /// The person's data folder is `home/data`, the system's are `usr/local` then `usr`; the
    /// person's configuration is `home/config`, the system's `etc`.
    pub(super) fn dirs(&self) -> XdgDirs {
        XdgDirs {
            data_home: Some(self.path("home/data")),
            data_dirs: vec![self.path("usr/local"), self.path("usr")],
            config_home: Some(self.path("home/config")),
            config_dirs: vec![self.path("etc")],
            desktops: Vec::new(),
        }
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
