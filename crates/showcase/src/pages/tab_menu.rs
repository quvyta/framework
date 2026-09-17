//! The right-click menu of the tab pages: Close, Close others, Close to the right, Pin and
//! Duplicate, working on a page's open tabs.

use qframe::prelude::*;
use qframe::widgets::{ContextItem, TabEdit};

/// An open tab: the page item it shows and whether it is pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenTab {
    /// Index into the page's list of items (files, projects).
    pub item: usize,
    /// Pinned tabs cannot be closed.
    pub pinned: bool,
}

impl OpenTab {
    /// Every item of a page with `count` items, open and unpinned.
    pub fn all(count: usize) -> Vec<Self> {
        (0..count).map(|item| Self { item, pinned: false }).collect()
    }
}

/// An entry of the menu, for the tab at an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAction {
    Close(usize),
    CloseOthers(usize),
    CloseRight(usize),
    TogglePin(usize),
    Duplicate(usize),
}

impl TabAction {
    /// The tab the action was chosen for.
    pub fn index(self) -> usize {
        match self {
            Self::Close(index)
            | Self::CloseOthers(index)
            | Self::CloseRight(index)
            | Self::TogglePin(index)
            | Self::Duplicate(index) => index,
        }
    }

    /// The event log entry of the action on a tab called `name`.
    pub fn describe(self, tabs: &[OpenTab], name: &str) -> String {
        match self {
            Self::Close(_) => format!("menu: close {name}"),
            Self::CloseOthers(_) => format!("menu: close others than {name}"),
            Self::CloseRight(_) => format!("menu: close right of {name}"),
            Self::TogglePin(index) if tabs.get(index).is_some_and(|tab| tab.pinned) => format!("menu: unpin {name}"),
            Self::TogglePin(_) => format!("menu: pin {name}"),
            Self::Duplicate(_) => format!("menu: duplicate {name}"),
        }
    }
}

/// Whether `action` would close tab `index` of `tabs`. Pinned tabs never close.
fn closes(action: TabAction, tabs: &[OpenTab], index: usize) -> bool {
    let open = tabs.get(index).is_some_and(|tab| !tab.pinned);
    open && match action {
        TabAction::Close(target) => index == target,
        TabAction::CloseOthers(target) => index != target,
        TabAction::CloseRight(target) => index > target,
        TabAction::TogglePin(_) | TabAction::Duplicate(_) => false,
    }
}

/// The menu of tab `index`: entries that would do nothing are disabled, and Pin reads Unpin on a
/// pinned tab.
pub fn items<M>(tabs: &[OpenTab], index: usize, send: impl Fn(TabAction) -> M) -> Vec<ContextItem<M>> {
    let any = |action: TabAction| (0..tabs.len()).any(|other| closes(action, tabs, other));
    let entry = |key: &str, action: TabAction| ContextItem::new(t!(key), send(action)).disabled(!any(action));
    let pinned = tabs.get(index).is_some_and(|tab| tab.pinned);
    vec![
        entry("tab-menu.close", TabAction::Close(index)),
        entry("tab-menu.close-others", TabAction::CloseOthers(index)),
        entry("tab-menu.close-right", TabAction::CloseRight(index)),
        ContextItem::gap(),
        ContextItem::new(t!(if pinned { "tab-menu.unpin" } else { "tab-menu.pin" }), send(TabAction::TogglePin(index))),
        ContextItem::new(t!("tab-menu.duplicate"), send(TabAction::Duplicate(index))),
    ]
}

/// Applies `action` to `tabs` and keeps `active` on a tab that stays open. When the open tab
/// closes, the tab the menu was opened for becomes the open one.
pub fn apply(action: TabAction, tabs: &mut Vec<OpenTab>, active: &mut usize) {
    let index = action.index();
    if index >= tabs.len() {
        return;
    }
    match action {
        TabAction::Close(_) | TabAction::CloseOthers(_) | TabAction::CloseRight(_) => {
            let before = tabs.clone();
            let active_closes = closes(action, &before, *active);
            for other in (0..before.len()).rev().filter(|other| closes(action, &before, *other)) {
                TabEdit::Close(other).apply(tabs, active);
            }
            if active_closes && !closes(action, &before, index) {
                *active = (0..index).filter(|other| !closes(action, &before, *other)).count();
            }
        }
        TabAction::TogglePin(_) => tabs[index].pinned = !tabs[index].pinned,
        TabAction::Duplicate(_) => {
            tabs.insert(index + 1, OpenTab { pinned: false, ..tabs[index] });
            *active = index + 1;
        }
    }
}

/// The indices of the pinned tabs.
pub fn pinned(tabs: &[OpenTab]) -> Vec<usize> {
    tabs.iter().enumerate().filter(|(_, tab)| tab.pinned).map(|(index, _)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs(pinned: &[usize]) -> Vec<OpenTab> {
        (0..5).map(|item| OpenTab { item, pinned: pinned.contains(&item) }).collect()
    }

    fn items_of(tabs: &[OpenTab]) -> Vec<usize> {
        tabs.iter().map(|tab| tab.item).collect()
    }

    #[test]
    fn closing_keeps_pinned_tabs_and_an_open_tab() {
        let mut open = tabs(&[0]);
        let mut active = 4;
        apply(TabAction::CloseOthers(2), &mut open, &mut active);
        assert_eq!((items_of(&open), active), (vec![0, 2], 1), "the menu's tab becomes the open one");

        let mut open = tabs(&[]);
        let mut active = 1;
        apply(TabAction::CloseRight(1), &mut open, &mut active);
        assert_eq!((items_of(&open), active), (vec![0, 1], 1));
        apply(TabAction::Close(0), &mut open, &mut active);
        assert_eq!((items_of(&open), active), (vec![1], 0));

        let mut open = tabs(&[3]);
        let mut active = 0;
        apply(TabAction::Close(3), &mut open, &mut active);
        assert_eq!(open.len(), 5, "a pinned tab stays");
    }

    #[test]
    fn pin_toggles_and_duplicate_opens_a_copy() {
        let mut open = tabs(&[]);
        let mut active = 0;
        apply(TabAction::TogglePin(1), &mut open, &mut active);
        assert_eq!(pinned(&open), vec![1]);
        apply(TabAction::Duplicate(1), &mut open, &mut active);
        assert_eq!((items_of(&open), active), (vec![0, 1, 1, 2, 3, 4], 2));
        assert_eq!(pinned(&open), vec![1], "the copy is not pinned");
        apply(TabAction::TogglePin(1), &mut open, &mut active);
        assert!(pinned(&open).is_empty());
    }
}
