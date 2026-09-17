//! The press behaviour shared by buttons and toggles: Enter or Space while focused, or a click
//! released over the widget.

use crate::event::{Event, MouseButton, MouseKind};
use crate::keymap::Key;
use crate::widget::EventCx;

/// What an event meant to a pressable widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Press {
    /// Not a press; let it bubble.
    Ignored,
    /// Part of a press that is not complete yet, such as the mouse going down.
    Used,
    /// Enter or Space while focused.
    Key,
    /// A left click released over the widget, at this cell.
    Click(i32, i32),
}

/// Reads `event` as a press. The mouse is captured on the way down so the release is seen even
/// when it happens outside the widget; releasing outside cancels.
pub(crate) fn read<Msg>(cx: &mut EventCx<'_, Msg>, event: &Event) -> Press {
    match event {
        Event::Key(key) if key.is_plain(Key::Enter) || key.is_plain(Key::Space) => Press::Key,
        Event::Mouse(mouse) => match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                cx.capture_pointer();
                Press::Used
            }
            MouseKind::Up(MouseButton::Left) if cx.area().contains(mouse.x, mouse.y) => Press::Click(mouse.x, mouse.y),
            MouseKind::Up(MouseButton::Left) => Press::Used,
            _ => Press::Ignored,
        },
        _ => Press::Ignored,
    }
}
