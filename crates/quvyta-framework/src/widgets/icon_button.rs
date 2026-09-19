//! Icon buttons: one glyph, pressable, for the small controls at the edge of a header or a row.

use std::time::Duration;

use super::placement::Placement;
use super::press::{self, Press};
use super::tooltip;
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// A button that is one icon: a space, the glyph and a space, three cells in every glyph mode.
///
/// It stands on the ground around it, with no raised surface, so a row of them reads as quiet
/// marks rather than as buttons with labels. The pointer lightens all three cells, keyboard focus
/// lightens them one step further, and a press flashes them one step more; there is no pillar,
/// because three cells have no room for one before the glyph. Enter or Space while focused, or a
/// click released over it, sends its message, like a [`Button`](super::Button).
///
/// Its meaning is only the glyph, so give it a [`tooltip`](Self::tooltip): the words show below it
/// after the pointer rests on it for the theme's hover delay, and at once when it is reached with
/// the keyboard.
///
/// Style keys: `icon-button` (`fg`, `bg`, `bold`) with states `hover`, `focus`, `pressed` and
/// `disabled`; `tooltip` for its words.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::IconButton;
///
/// struct Header;
///
/// impl App for Header {
///     type Msg = ();
///     fn update(&mut self, (): ()) -> Command<()> {
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, ()>) {
///         ui.row(|ui| {
///             ui.add(Text::new("Packages")).fill_width();
///             ui.add(IconButton::new("settings").tooltip("Settings").on_press(()));
///         })
///         .fill_width();
///     }
/// }
///
/// let app = Harness::new(Header, 20, 1);
/// let glyph = app.env().icons().glyph("settings").into_owned();
/// assert_eq!(app.find(&glyph), Some((18, 0)), "a space, the glyph and a space at the end");
/// ```
pub struct IconButton<Msg> {
    icon: String,
    tooltip: Option<String>,
    disabled: bool,
    on_press: Option<Msg>,
}

/// When the pointer came to rest on the button and when its tooltip began to show.
#[derive(Debug, Default)]
struct IconButtonMemory {
    hovered_since: Option<Duration>,
    shown_since: Option<Duration>,
}

/// Cells an icon button takes around its glyph: one space on each side.
const SIDES: u16 = 2;

impl<Msg> IconButton<Msg> {
    /// A button showing the icon `key` of the icon set, such as `"settings"` or `"close"`.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self { icon: key.into(), tooltip: None, disabled: false, on_press: None }
    }

    /// The message sent when the button is pressed.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Words that say what the button does, shown below it after the hover delay and at once
    /// when it is reached with the keyboard.
    #[must_use]
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Greys the button out; it cannot be focused or pressed.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_press.is_some()
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for IconButton<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let glyph = text::width(&cx.env().icons().glyph(&self.icon));
        Size::new(glyph.saturating_add(SIDES), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let active = self.active();
        let mut states = if active { cx.pressable_states() } else { Vec::new() };
        if self.disabled {
            states.push(State::Disabled);
        }
        let style = cx.style("icon-button", None, &states).text();
        // At rest the theme gives no ground, so the button keeps whatever it stands on.
        if let Some(bg) = style.bg {
            cx.fill(area, bg);
        }
        if active {
            cx.register_hit(area);
        }
        let glyph = cx.env().icons().glyph(&self.icon).into_owned();
        let width = text::width(&glyph);
        let x = area.x + i32::from(area.width.saturating_sub(width) / 2);
        cx.text(x, area.y, &glyph, CellStyle { bg: None, ..style }, area.width);
        self.schedule_tooltip(cx, area, &states);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let Some(text) = &self.tooltip else {
            return;
        };
        let since = cx.memory::<IconButtonMemory>().shown_since.unwrap_or_default();
        tooltip::paint_tip(cx, anchor, text, Placement::Below, since);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        match press::read(cx, event) {
            Press::Ignored => false,
            Press::Used => true,
            Press::Key | Press::Click(..) => {
                if let Some(message) = &self.on_press {
                    cx.flash();
                    cx.emit(message.clone());
                }
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.active()
    }
}

impl<Msg: Clone + 'static> IconButton<Msg> {
    /// Tracks how long the pointer has rested on the button and asks for the overlay once its
    /// tooltip is due, or at once while keyboard focus is on it.
    fn schedule_tooltip(&self, cx: &mut PaintCx<'_>, area: Rect, states: &[State]) {
        if self.tooltip.is_none() {
            return;
        }
        let now = cx.now();
        let delay = cx.env().theme().motion().hover_delay;
        let hovered = states.contains(&State::Hover);
        let keyboard = states.contains(&State::Focus);
        let memory = cx.memory::<IconButtonMemory>();
        memory.hovered_since = if hovered { Some(memory.hovered_since.unwrap_or(now)) } else { None };
        let due = memory.hovered_since.map(|since| since + delay);
        let visible = keyboard || due.is_some_and(|due| now >= due);
        memory.shown_since = if visible { Some(memory.shown_since.unwrap_or(now)) } else { None };
        if visible {
            cx.request_overlay(area);
        } else if let Some(due) = due {
            cx.request_frame_in(due.saturating_sub(now));
        }
    }
}
