//! Following the folders a [`FileManager`](super::FileManager) shows while other programs change
//! them, so what appears or goes is shown without anyone asking for it.
//!
//! The system tells when an entry of a watched folder appears, goes or is renamed, so nothing is
//! polled and an idle manager costs nothing. Only the folders whose rows are on screen are
//! watched, and a change rereads the one folder it happened in.
//!
//! Where the system has no watch to give (a platform other than Linux, or its limit on watches
//! reached), the folders are read again after the manager's own operations, when it comes back on
//! screen and on Refresh, as they would be without any of this.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::runtime::Command;
use crate::storage::{FolderChange, FolderChangeKind, FolderChanges, FolderWatch};

use super::state::{FileManagerMsg, FileManagerState, Wrap};

/// Whether a manager's folders are watched.
#[derive(Debug, Default)]
pub(super) enum Live {
    /// Not watched: the manager does not follow the disk.
    #[default]
    Off,
    /// The system has no watch to give; asking again for every message would only fail again.
    Unavailable,
    /// Watched.
    On(Watching),
}

/// A watch on the folders a manager shows.
#[derive(Debug)]
pub(super) struct Watching {
    watch: FolderWatch,
    /// Which watch this is, so changes a watch that was let go sent before it noticed are not
    /// taken for this one's.
    run: u64,
    /// The folders it was asked to watch, by key, with the path each was watched under; a folder
    /// the system refused has no path and is not asked for again while it stays on screen.
    folders: BTreeMap<String, Option<PathBuf>>,
}

impl Watching {
    /// The key of the folder a change was reported for.
    fn key_of(&self, folder: &Path) -> Option<String> {
        self.folders.iter().find(|(_, path)| path.as_deref() == Some(folder)).map(|(key, _)| key.clone())
    }

    /// The keys of the folders being watched.
    #[cfg(test)]
    pub(super) fn watched(&self) -> Vec<&str> {
        self.folders.iter().filter(|(_, path)| path.is_some()).map(|(key, _)| key.as_str()).collect()
    }

    /// Which watch this is.
    #[cfg(test)]
    pub(super) fn run(&self) -> u64 {
        self.run
    }

    /// The waiting side of the watch, for a test to take a batch by hand.
    #[cfg(test)]
    pub(super) fn changes(&self) -> FolderChanges {
        self.watch.changes()
    }
}

/// Keeps the watch of `state` on exactly the folders it shows: a watch is made the first time,
/// folders that came on screen are watched and folders that left it are let go.
///
/// A folder that comes back on screen with entries already known is read again, since whatever
/// happened in it while it was not watched went unseen.
pub(super) fn follow<Msg: Clone + Send + 'static>(state: &mut FileManagerState, wrap: &Wrap<Msg>) -> Command<Msg> {
    if !state.follows_changes() {
        return Command::none();
    }
    let mut commands = Vec::new();
    if matches!(state.live, Live::Off) {
        match FolderWatch::new() {
            Ok(watch) => {
                state.runs += 1;
                commands.push(wait(state.runs, watch.changes(), wrap));
                state.live = Live::On(Watching { watch, run: state.runs, folders: BTreeMap::new() });
            }
            Err(_) => state.live = Live::Unavailable,
        }
    }
    let visible = state.visible_folders();
    let Live::On(watching) = &mut state.live else { return Command::batch(commands) };
    let left: Vec<String> = watching.folders.keys().filter(|key| !visible.contains(key)).cloned().collect();
    for key in left {
        if let Some(Some(path)) = watching.folders.remove(&key) {
            watching.watch.unwatch(&path);
        }
    }
    let mut again = Vec::new();
    let mut started = Vec::new();
    for key in visible {
        if watching.folders.contains_key(&key) {
            continue;
        }
        started.push(key);
    }
    for key in started {
        let path = state.path(&key);
        let known = state.children(&key).is_some() && !state.is_loading(&key);
        let Live::On(watching) = &mut state.live else { break };
        let watched = watching.watch.watch(&path).is_ok();
        if watched && known {
            again.push(key.clone());
        }
        watching.folders.insert(key, watched.then_some(path));
    }
    commands.push(state.reread(again, wrap));
    Command::batch(commands)
}

/// Takes a batch of changes of the watch `run`: the folders they happened in are read again, and
/// the next batch is waited for.
///
/// A change of an entry's content leaves the manager as it is, since a tree shows names and not
/// what is in the files; a program saving a file over and over reads nothing. A watched folder
/// that went away, or changes that came faster than the system could keep, mean anything may have
/// changed, so every folder on screen is read again.
pub(super) fn changed<Msg: Clone + Send + 'static>(
    state: &mut FileManagerState,
    run: u64,
    batch: Vec<FolderChange>,
    wrap: &Wrap<Msg>,
) -> Command<Msg> {
    let Live::On(watching) = &mut state.live else { return Command::none() };
    // An empty batch only comes from a watch that was let go, which this one is not; waiting again
    // on it would spin.
    if watching.run != run || batch.is_empty() {
        return Command::none();
    }
    let next = wait(run, watching.watch.changes(), wrap);
    let mut again = Vec::new();
    let mut everything = false;
    for change in batch {
        let Some(key) = watching.key_of(&change.folder) else { continue };
        match change.kind {
            FolderChangeKind::Modified => {}
            FolderChangeKind::Created | FolderChangeKind::Removed | FolderChangeKind::Renamed { .. } => {
                again.push(key);
            }
            FolderChangeKind::Gone => {
                // The system no longer watches it; forgetting it lets it be watched again should
                // it come back on screen.
                watching.folders.remove(&key);
                everything = true;
            }
            FolderChangeKind::Overflow => everything = true,
        }
    }
    let reread = if everything {
        state.refresh(wrap)
    } else {
        again.sort();
        again.dedup();
        state.reread(again, wrap)
    };
    Command::batch([reread, next])
}

/// Waits on a background thread for the next batch of the watch `run`.
fn wait<Msg: Clone + Send + 'static>(run: u64, changes: FolderChanges, wrap: &Wrap<Msg>) -> Command<Msg> {
    let wrap = std::sync::Arc::clone(wrap);
    Command::perform(move || wrap(FileManagerMsg::Changed(run, changes.next())))
}
