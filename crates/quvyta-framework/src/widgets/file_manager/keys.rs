//! Ctrl+X, Ctrl+C and Ctrl+V on a file manager's rows: the keys a desktop file explorer cuts,
//! copies and pastes entries with, claimed on the manager's node so they reach it only while its
//! rows have focus and no text is selected with the mouse.

use crate::widget::{ClipboardKey, NodeMut};

use super::state::ROOT;
use super::{FileManager, FileManagerMsg, FileView, parent_key};

impl<Msg: Clone + 'static> FileManager<'_, Msg> {
    /// Claims the clipboard keys on `node`, the node holding the rows, for what can be done now.
    ///
    /// A key with nothing to act on is left unclaimed, so it does what it does without the
    /// manager: Ctrl+C and Ctrl+X with no entry selected, Ctrl+V with nothing waiting.
    pub(super) fn claim_clipboard<'v>(&self, node: NodeMut<'v, Msg>) -> NodeMut<'v, Msg> {
        if self.disabled {
            return node;
        }
        let mut node = node;
        if let Some(key) = self.clipboard_target() {
            node = node
                .on_clipboard(ClipboardKey::Cut, (self.wrap)(FileManagerMsg::Cut(key.clone())))
                .on_clipboard(ClipboardKey::Copy, (self.wrap)(FileManagerMsg::Copy(key)));
        }
        if !self.state.pending().is_empty() {
            let into = self.paste_folder();
            node = node.on_clipboard(ClipboardKey::Paste, (self.wrap)(FileManagerMsg::Paste(into)));
        }
        node
    }

    /// The key a cut or a copy is sent for, which [`FileManagerState::targets`] widens to the
    /// whole selection: one of the selected entries, or the entry under the cursor when nothing
    /// is selected. The root, and in a flat view the row of the shown folder itself, are where
    /// the person is rather than entries to carry away, so they are never it.
    ///
    /// [`FileManagerState::targets`]: super::FileManagerState::targets
    fn clipboard_target(&self) -> Option<String> {
        let state = self.state;
        let shown = (self.view != FileView::Tree).then(|| state.folder());
        let movable = |key: &&str| *key != ROOT && Some(*key) != shown;
        let chosen = state.chosen().iter().map(String::as_str).find(movable);
        let key = if state.chosen().is_empty() { state.selected().filter(movable) } else { chosen };
        key.map(str::to_owned)
    }

    /// Where Ctrl+V pastes. A flat view pastes into the folder it shows, as a desktop explorer's
    /// window does. The tree shows every folder at once, so it pastes where the cursor is: into
    /// the folder the cursor is on, into the folder holding the file the cursor is on, and into
    /// the root while the cursor is nowhere.
    fn paste_folder(&self) -> String {
        let state = self.state;
        if self.view != FileView::Tree {
            return state.folder().to_owned();
        }
        match state.selected() {
            Some(key) if key == ROOT || state.is_folder(key) => key.to_owned(),
            Some(key) => parent_key(key).to_owned(),
            None => ROOT.to_owned(),
        }
    }
}
