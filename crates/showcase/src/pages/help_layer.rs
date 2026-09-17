//! Help layer: every key binding of the screen and the keymap, grouped and searchable.

use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::Task;
use qframe::widgets::HelpLayer;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "help-layer";

/// How long a help layer that cannot be dismissed stays before the demo closes it.
const FIRM_FOR: Duration = Duration::from_secs(5);

/// Whether the page's own help layer is open, and the playground.
#[derive(Debug)]
pub struct State {
    open: bool,
    screen_hints: bool,
    narrow: bool,
    dismissable: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { open: false, screen_hints: true, narrow: false, dismissable: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Open,
    Close,
    TimedOut,
    ScreenHints(bool),
    Narrow(bool),
    Dismissable(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::HelpLayer(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Open => {
            state.open = true;
            log.push(PAGE, "HelpLayer", "open");
            if !state.dismissable {
                // region: firm
                // Nothing on the layer closes it, so the application does, here after a while.
                return Command::task(Task::new("help-layer", |cx| {
                    if !cx.sleep(FIRM_FOR) {
                        return Err("cancelled".into());
                    }
                    Ok(send(Msg::TimedOut))
                }));
                // endregion
            }
        }
        Msg::Close => {
            state.open = false;
            log.push(PAGE, "HelpLayer", "dismissed with Esc or ×");
        }
        Msg::TimedOut => {
            if state.open {
                state.open = false;
                log.push(PAGE, "HelpLayer", "closed by the application");
            }
        }
        Msg::Dismissable(on) => {
            state.dismissable = on;
            log.push(PAGE, "Playground", format!("dismissable = {on}"));
        }
        Msg::ScreenHints(on) => {
            state.screen_hints = on;
            log.push(PAGE, "Playground", format!("screen hints = {on}"));
        }
        Msg::Narrow(on) => {
            state.narrow = on;
            log.push(PAGE, "Playground", format!("width = {}", if on { 44 } else { 64 }));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("help-layer.hint")).role("secondary"));
        ui.add(Button::new(t!("help-layer.open")).variant("primary").on_press(send(Msg::Open))).id("open");
        if state.open {
            // region: help
            let mut help = HelpLayer::new(send(Msg::Close)).dismissable(state.dismissable);
            if state.screen_hints {
                help = help
                    .hint("↑↓", t!("help-layer.move"))
                    .hint("r", t!("help-layer.restart"))
                    .hint("l", t!("help-layer.logs"));
            }
            if state.narrow {
                help = help.width(44);
            }
            ui.add(help);
            // endregion
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("help-layer.screen-hints"), |ui| {
            ui.add(toggle(state.screen_hints, |on| send(Msg::ScreenHints(on)))).id("screen-hints");
        });
        setting(ui, t!("help-layer.narrow"), |ui| {
            ui.add(toggle(state.narrow, |on| send(Msg::Narrow(on)))).id("narrow");
        });
        setting(ui, t!("help-layer.dismissable"), |ui| {
            ui.add(toggle(state.dismissable, |on| send(Msg::Dismissable(on)))).id("dismissable");
        });
        if !state.dismissable {
            ui.add(Text::new(t!("help-layer.firm")).role("faint"));
        }
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn shows_screen_hints_and_the_showcase_keymap_and_filters() {
        let mut h = showcase_on(PAGE);
        h.click_text("Show the keys of this screen").advance(Duration::from_millis(200));
        let screen = h.screen();
        assert!(screen.contains("Restart the selected container") && screen.contains("Keyboard shortcuts"), "{screen}");
        h.type_text("logs");
        assert!(h.screen().contains("Open its logs"));
        assert!(!h.screen().contains("Restart the selected container"));
        h.press("esc");
        assert!(!h.app().pages.help_layer.open);
        assert!(h.screen().contains("HelpLayer"));
    }

    #[test]
    fn the_close_mark_closes_and_a_layer_that_is_not_dismissable_waits_for_the_application() {
        let mut h = showcase_on(PAGE);
        h.click_text("Show the keys of this screen").advance(Duration::from_millis(200));
        let (x, y) = crate::tests::close_mark_beside(&h, "Keyboard shortcuts");
        h.click(x.expect("a close mark on the title row"), y);
        assert!(!h.app().pages.help_layer.open);
        assert!(h.screen().contains("dismissed with Esc or ×"), "{}", h.screen());
        h.send(send(Msg::Dismissable(false)));
        assert!(h.screen().contains("this demo closes the layer itself"), "{}", h.screen());
        h.click_text("Show the keys of this screen").advance(Duration::from_millis(200));
        assert!(crate::tests::close_mark_beside(&h, "Keyboard shortcuts").0.is_none(), "{}", h.screen());
        h.press("esc").advance(Duration::from_secs(1));
        assert!(h.app().pages.help_layer.open, "Esc does nothing");
        h.advance(FIRM_FOR);
        assert!(!h.app().pages.help_layer.open, "the application closed it");
        assert!(h.screen().contains("closed by the application"), "{}", h.screen());
    }
}
