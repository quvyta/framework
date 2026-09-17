//! The menus a right click opens on text: Cut, Copy, Paste and Select all in fields, and Copy
//! and Raw copy on selected text. Both are a [`ContextMenu`] that the widget owning the text
//! drives itself: the menu lives in that widget's memory and its choices come back to that
//! widget instead of reaching the application.

use super::context_item::ContextItem;
use super::context_menu::{self, ContextMenu};
use crate::env::Env;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::Rect;
use crate::keymap::{KeyChord, Scope};
use crate::runtime::CopyKind;
use crate::widget::{EventCx, PaintCx, Widget};

/// An entry of a field's edit menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditAction {
    Cut,
    Copy,
    Paste,
    SelectAll,
}

/// A menu of text actions `A` owned by a widget.
pub(crate) struct TextMenu<A> {
    menu: ContextMenu<A>,
}

/// The label of a fixed chord such as `ctrl+x`, or of the first key bound to a global action.
fn shortcut(env: &Env, chord: Option<&str>, action: Option<&str>) -> String {
    let chord = match (chord, action) {
        (Some(chord), _) => chord.parse::<KeyChord>().ok(),
        (None, Some(action)) => env.keymap().chords_for(Scope::Global, action).first().copied(),
        (None, None) => None,
    };
    chord.map(|chord| chord.label()).unwrap_or_default()
}

/// Whether `event` asks for a menu: a right press, Shift+F10 or the menu key.
pub(crate) fn asks(event: &Event) -> bool {
    match event {
        Event::Mouse(mouse) => mouse.kind == MouseKind::Down(MouseButton::Right),
        Event::Key(key) => context_menu::is_menu_key(key),
        _ => false,
    }
}

impl TextMenu<EditAction> {
    /// Cut, Copy, Paste and Select all. Cut and Copy are disabled without `selection`, Paste
    /// without text to paste.
    pub(crate) fn edit(env: &Env, selection: bool, can_paste: bool) -> Self {
        let i18n = env.i18n();
        let item = |key: &str, action, chord: Option<&str>, bound: Option<&str>| {
            ContextItem::new(i18n.translate(key, &[]), action).shortcut(shortcut(env, chord, bound))
        };
        let items = [
            item("quvyta.edit.cut", EditAction::Cut, Some("ctrl+x"), None).disabled(!selection),
            item("quvyta.edit.copy", EditAction::Copy, Some("ctrl+c"), None).disabled(!selection),
            item("quvyta.edit.paste", EditAction::Paste, None, Some("paste")).disabled(!can_paste),
            item("quvyta.edit.select-all", EditAction::SelectAll, Some("ctrl+a"), None),
        ];
        Self { menu: ContextMenu::new(items) }
    }
}

impl TextMenu<CopyKind> {
    /// Copy, which copies clean text, and Raw copy, which copies every cell as shown.
    pub(crate) fn copy(env: &Env) -> Self {
        let i18n = env.i18n();
        let items = [
            ContextItem::new(i18n.translate("quvyta.edit.copy", &[]), CopyKind::Clean).shortcut(shortcut(
                env,
                None,
                Some("copy"),
            )),
            ContextItem::new(i18n.translate("quvyta.edit.raw-copy", &[]), CopyKind::Raw),
        ];
        Self { menu: ContextMenu::new(items) }
    }
}

/// Whether the menu of the widget handling an event is open.
pub(crate) fn is_open<Msg>(cx: &mut EventCx<'_, Msg>) -> bool {
    context_menu::is_open_in(cx)
}

/// Asks for the overlay while the menu of the widget being painted is open; call it from `paint`.
pub(crate) fn request_overlay(cx: &mut PaintCx<'_>, area: Rect) {
    if context_menu::is_open(cx) {
        cx.request_overlay(area);
    }
}

impl<A: Clone + 'static> TextMenu<A> {
    /// Offers `event` to the menu. Returns whether the menu used it and the entry chosen with it.
    /// Opening takes the keys until the menu closes.
    pub(crate) fn event<Msg>(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> (bool, Option<A>) {
        let (used, chosen) = cx.with_messages(|cx| self.menu.event(cx, event));
        (used, chosen.into_iter().last())
    }

    /// Draws the open menu; call it from `paint_overlay`.
    pub(crate) fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        if context_menu::is_open(cx) {
            self.menu.paint_overlay(cx, anchor);
        }
    }
}
