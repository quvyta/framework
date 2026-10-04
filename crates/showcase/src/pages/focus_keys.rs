//! Focus and keys: focus order, the keymap and its labels, and the debug layer.

use qframe::event::Event;
use qframe::geometry::{Rect, Size};
use qframe::keymap::Scope;
use qframe::prelude::*;
use qframe::style::CellStyle;
use qframe::widget::{EventCx, MeasureCx, PaintCx, Widget};
use qframe::widgets::TextInput;

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "focus-keys";

/// The focus order demo fields.
#[derive(Debug, Default)]
pub struct State {
    first: String,
    second: String,
    /// What the key-taking screen was last given, and whether the application owns that key.
    seen: Option<(String, bool)>,
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    First(String),
    Second(String),
    Pressed,
    /// A key the screen took, with the answer the keymap gave for it.
    Seen(String, bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::FocusKeys(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::First(text) => state.first = text,
        Msg::Second(text) => state.second = text,
        Msg::Pressed => {
            log.push(PAGE, "Button#focus-demo", "pressed");
            // region: focus-command
            return Command::focus("first-field");
            // endregion
        }
        Msg::Seen(key, reserved) => {
            state.seen = Some((key.clone(), reserved));
            let verdict = t!(if reserved { "focus-keys.reserved" } else { "focus-keys.own" });
            log.push(PAGE, "Screen", format!("{key} {verdict}"));
        }
    }
    Command::none()
}

// region: reserved-widget
/// A widget that takes every key while it has focus and uses it, the shape an embedded page, an
/// embedded terminal or a coding tool's own screen has: without asking, it would swallow the
/// keys bound to the application's own actions and to the runtime's globals.
struct Screen {
    /// What was last taken, and whether the application owns it.
    seen: Option<(String, bool)>,
}

impl Widget<AppMsg> for Screen {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(available.width, 1)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        let (line, colour) = match &self.seen {
            Some((key, true)) => (format!("{key}  {}", t!("focus-keys.reserved")), "text"),
            Some((key, false)) => (format!("{key}  {}", t!("focus-keys.own")), "muted"),
            None => (t!("focus-keys.screen-idle").to_owned(), "muted"),
        };
        let style = CellStyle { fg: Some(cx.color(colour)), ..CellStyle::default() };
        cx.text(area.x, area.y, &line, style, area.width);
    }

    fn event(&self, cx: &mut EventCx<'_, AppMsg>, event: &Event) -> bool {
        let Event::Key(key) = event else { return false };
        // is_reserved reads the keymap in force when the key arrives, so rebinding an action
        // moves the answer without anything here changing.
        let reserved = cx.is_reserved(&key.chord);
        cx.emit(send(Msg::Seen(key.chord.label(), reserved)));
        // A key the application owns is not this screen's to use, so it goes on as usual.
        !reserved
    }

    fn focusable(&self) -> bool {
        true
    }
}
// endregion

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("focus-keys.order")), |ui| {
        ui.add(Text::new(t!("focus-keys.order-hint")).role("secondary"));
        // region: order
        ui.row(|ui| {
            ui.add(TextInput::new(&state.first).placeholder(t!("focus-keys.first")).on_change(|s| send(Msg::First(s))))
                .width(Length::Fill(1))
                .id("first-field");
            ui.add(
                TextInput::new(&state.second).placeholder(t!("focus-keys.second")).on_change(|s| send(Msg::Second(s))),
            )
            .width(Length::Fill(1))
            .id("second-field");
            ui.add(Button::new(t!("focus-keys.back-to-first")).on_press(send(Msg::Pressed))).id("focus-demo");
        })
        .gap(2)
        .fill_width();
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("focus-keys.keymap")), |ui| {
        let env = ui.env();
        let bindings: Vec<(Scope, String, String)> = env
            .keymap()
            .iter()
            .map(|(scope, action, chords)| {
                let keys = chords.iter().map(qframe::keymap::KeyChord::label).collect::<Vec<_>>().join("  ");
                (scope, action.to_owned(), keys)
            })
            .collect();
        let items = bindings.into_iter().map(|(scope, action, keys)| {
            // region: labels
            let label = env.i18n().translate(&scope.label_key(&action), &[]);
            // endregion
            let scope_name = if scope == Scope::Global { "global" } else { "app" };
            ListItem::new(format!("{keys}   {label}")).detail(format!("{scope_name} · {action}"))
        });
        ui.add(List::new(items.collect::<Vec<_>>())).width(Length::Fill(1)).height(Length::Cells(12)).id("bindings");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("focus-keys.screen")).gap(1), |ui| {
        ui.add(Text::new(t!("focus-keys.screen-hint")).role("secondary"));
        ui.add(Screen { seen: state.seen.clone() }).fill_width().id("screen");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("focus-keys.debug")), |ui| {
        ui.add(Text::new(t!("focus-keys.debug-hint")).role("secondary"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn button_moves_focus_back_and_keymap_is_listed() {
        let mut h = showcase_on(PAGE);
        h.click_text("Back to the first field");
        assert!(h.is_focused("first-field"));
        h.type_text("hi");
        assert_eq!(h.app().pages.focus_keys.first, "hi");
        assert!(h.screen().contains("ctrl q"));
    }

    #[test]
    fn the_screen_lets_the_applications_own_keys_go_on_and_uses_the_rest() {
        let mut h = showcase_on(PAGE);
        h.click_text("Click here and press keys");
        assert!(h.is_focused("screen"), "the screen took the click and the keys with it");
        h.press("ctrl+l");
        assert_eq!(
            h.app().pages.focus_keys.seen,
            Some(("ctrl l".to_owned(), false)),
            "the showcase binds no action to ctrl+l, so the key is the screen's own"
        );
        h.press("ctrl+b");
        assert_eq!(
            h.app().pages.focus_keys.seen,
            Some(("ctrl b".to_owned(), true)),
            "the showcase binds ctrl+b to its own menu action"
        );
        h.press("ctrl+q");
        assert_eq!(
            h.app().pages.focus_keys.seen,
            Some(("ctrl q".to_owned(), true)),
            "quitting is one of the runtime's own actions"
        );
    }
}
