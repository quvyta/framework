//! The list of suggestions a text field offers under itself, and the [`Suggestion`] rows it draws.
//!
//! The field keeps the list in its memory and passes its events here first, the way it drives its
//! edit menu: the choice comes back to the field and the application hears only which row it was.

use std::time::Duration;

use super::context_item::{self, ContextItem};
use super::placement::{self, Placement};
use crate::event::{Event, KeyEvent, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::motion::Easing;
use crate::widget::PaintCx;

/// A choice a text field offers under itself, such as a path, a package, an address or a command.
///
/// The field draws the list as one of the framework's other lists of choices, so a row shows its
/// label, an optional note at its right and an optional icon before it, and the row the person has
/// chosen is raised in tone with the `▌` pillar beside it.
///
/// ```
/// use qframe::widgets::Suggestion;
///
/// let rows = [
///     Suggestion::new("~/projects/framework").detail("last opened today"),
///     Suggestion::new("~/projects/qcode").icon("folder"),
/// ];
/// assert_eq!(rows[0].label(), "~/projects/framework");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    label: String,
    detail: Option<String>,
    icon: Option<String>,
}

impl Suggestion {
    /// A suggestion labelled `label`, the text the row shows.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), detail: None, icon: None }
    }

    /// A faint note on the right of the row, e.g. where a path points: `"last opened today"`. The
    /// list is as wide as the field, so where the field has no room the note is cut before the
    /// label is.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Icon key drawn before the label, e.g. `"folder"`. Labels line up after the widest icon in
    /// the list, so a row without one keeps the column empty.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>) -> Self {
        self.icon = Some(key.into());
        self
    }

    /// The text of the row, which is what the application puts in its field when the row is chosen.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// What an event did to the open list.
pub(crate) enum SuggestionAction {
    /// Not for the list; the field deals with the event itself.
    Other,
    /// The list used the event and chose nothing.
    Used,
    /// The list used the event and this row was chosen.
    Chosen(usize),
}

/// The list a field offers under itself: whether the person has typed, the row they have chosen
/// and where the list was drawn last frame. Every part is copyable, so the field takes the list
/// out of its memory, works with it between two frames and puts it back.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SuggestionList {
    /// Whether the person has typed since the field gained focus.
    typed: bool,
    /// The chosen row, or `None` while the typed text stands.
    chosen: Option<usize>,
    /// Whether the person closed the list; typing opens it again.
    closed: bool,
    /// Whether the list was shown last frame, so the frame it opens in starts the entrance.
    shown: bool,
    /// The layer as painted last frame: where a press on a row lands.
    rect: Rect,
    /// When the list opened, for the entrance.
    opened_at: Duration,
}

impl SuggestionList {
    /// Whether the list is open: the field has focus, there is something to choose from, the person
    /// has typed and has not closed the list since.
    pub(crate) fn open(&self, items: &[Suggestion], focused: bool) -> bool {
        focused && self.typed && !self.closed && !items.is_empty()
    }

    /// Notes that the person typed: the chosen row goes and a list they closed opens again.
    pub(crate) fn typed(&mut self) {
        self.typed = true;
        self.chosen = None;
        self.closed = false;
    }

    /// Notes that the field gained focus: nothing is offered before the person asks for it.
    pub(crate) fn gained_focus(&mut self) {
        self.typed = false;
        self.chosen = None;
        self.closed = false;
    }

    /// Closes the list and gives back `choice`.
    fn close(&mut self, choice: SuggestionAction) -> SuggestionAction {
        self.chosen = None;
        self.closed = true;
        choice
    }

    /// The row after `from` in direction `down`, around the typed text that stands at either end of
    /// the list: ↑ from the first row and ↓ from the last one bring it back.
    fn step(len: usize, from: Option<usize>, down: bool) -> Option<usize> {
        let stop = from.map_or(0, |index| index + 1);
        let next = if down { stop + 1 } else { stop + len } % (len + 1);
        // Stop zero is the typed text, which stands wherever no row is chosen.
        match next {
            0 => None,
            row => Some(row - 1),
        }
    }

    /// What a key did to the list. Any other key is the field's, so typing keeps changing the text
    /// and leaves the typed text standing.
    fn key(&mut self, key: &KeyEvent, items: &[Suggestion]) -> SuggestionAction {
        if key.is_plain(Key::Esc) {
            return self.close(SuggestionAction::Used);
        }
        if key.is_plain(Key::Up) || key.is_plain(Key::Down) {
            self.chosen = Self::step(items.len(), self.chosen, key.is_plain(Key::Down));
            return SuggestionAction::Used;
        }
        if key.is_plain(Key::Enter) {
            return self.chosen.map_or(SuggestionAction::Other, |index| self.close(SuggestionAction::Chosen(index)));
        }
        SuggestionAction::Other
    }

    /// What an event did to the list; the caller puts the list back into the field's memory.
    pub(crate) fn event(mut self, event: &Event, items: &[Suggestion], focused: bool) -> (Self, SuggestionAction) {
        let open = self.open(items, focused);
        let choice = match event {
            Event::PointerOutside if open => self.close(SuggestionAction::Used),
            Event::Key(key) if open => self.key(key, items),
            Event::Mouse(mouse) if open && self.rect.contains(mouse.x, mouse.y) => match mouse.kind {
                MouseKind::Down(MouseButton::Left) => {
                    let row = usize::try_from(mouse.y - self.rect.y).unwrap_or(0);
                    self.close(if row < items.len() { SuggestionAction::Chosen(row) } else { SuggestionAction::Used })
                }
                // The tail of a press that began on a row belongs to the list, so a drag out of it
                // does not move the cursor. A right press is the field's: it opens the edit menu.
                MouseKind::Up(_) | MouseKind::Drag(_) => SuggestionAction::Used,
                _ => SuggestionAction::Other,
            },
            _ => SuggestionAction::Other,
        };
        (self, choice)
    }

    /// Draws the list under `anchor` when it is open, as wide as the field: a layer as wide as its
    /// anchor, so its own edges stand where the field's do and a label longer than the field is cut
    /// there. It unfolds from the field over the theme's `motion.enter` and draws the rows as a
    /// menu's, so the row the person chose is raised in tone with its pillar and never in brackets.
    pub(crate) fn paint(&mut self, cx: &mut PaintCx<'_>, anchor: Rect, items: &[Suggestion], focused: bool) {
        if !self.open(items, focused) {
            self.shown = false;
            return;
        }
        if !self.shown {
            self.opened_at = cx.now();
        }
        self.shown = true;
        let rows: Vec<ContextItem<usize>> = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let row = ContextItem::new(item.label.clone(), index);
                let row = match &item.icon {
                    Some(icon) => row.icon(icon.clone()),
                    None => row,
                };
                match &item.detail {
                    Some(detail) => row.detail(detail.clone()),
                    None => row,
                }
            })
            .collect();
        let screen = cx.clip();
        let size = Size::new(
            placement::anchor_width(anchor, screen),
            clamp_u16(i32::try_from(rows.len()).unwrap_or(i32::MAX)),
        );
        let (full, side) = placement::place(anchor, size, screen, Placement::Below);
        let enter = cx.env().theme().motion().enter;
        let progress = cx.progress_since(self.opened_at, enter, Easing::EaseOut);
        let shown = placement::unfold(full, side, progress);
        context_item::paint(cx, &rows, full, shown, self.chosen);
        self.rect = shown;
    }
}
