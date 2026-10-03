//! Help layer: every key binding of the screen and the keymap, grouped and searchable.

use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::Task;
use qframe::widgets::{Column, HelpLayer, Table, TableCell, TableRow};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "help-layer";

/// How long a help layer that cannot be dismissed stays before the demo closes it.
const FIRM_FOR: Duration = Duration::from_secs(5);

/// The containers the demo's screen lists.
const CONTAINERS: [&str; 3] = ["web", "api", "worker"];

/// Whether the page's own help layer is open, and the playground.
#[derive(Debug)]
pub struct State {
    open: bool,
    screen_hints: bool,
    narrow: bool,
    dismissable: bool,
    selected: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self { open: false, screen_hints: true, narrow: false, dismissable: true, selected: Some(0) }
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
    Select(usize),
    Opened(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::HelpLayer(message))
}

/// The page's answer to the `screen-keys` action, which opens this page's own layer from wherever
/// the focus is. It is a key and not the button below, so the focus is still on the widget the
/// layer stands over when it opens: a click on the button would leave the focus there, and a
/// layer lists the keys of the widget the keyboard came from.
pub fn action(name: &str) -> Option<AppMsg> {
    (name == "screen-keys").then(|| send(Msg::Open))
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
        Msg::Select(index) => state.selected = Some(index),
        Msg::Opened(index) => {
            let name = CONTAINERS.get(index).copied().unwrap_or_default();
            log.push(PAGE, "Table#containers", format!("open {name}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("help-layer.hint")).role("secondary"));
        // region: containers
        // The screen the layer stands over: a table of containers, whose own keys the layer
        // lists. The application writes hints only for the two keys it reads itself.
        let rows: Vec<TableRow> = CONTAINERS.iter().map(|name| TableRow::new([TableCell::new(*name)])).collect();
        ui.add(
            Table::new([Column::new(t!("help-layer.container"))], rows)
                .selected(state.selected)
                .on_select(|index| send(Msg::Select(index)))
                .on_activate(|index| send(Msg::Opened(index))),
        )
        .height(Length::Cells(4))
        .id("rows");
        // endregion
        ui.add(Button::new(t!("help-layer.open")).variant("primary").on_press(send(Msg::Open))).id("open");
        if state.open {
            // region: help
            let mut help = HelpLayer::new(send(Msg::Close)).dismissable(state.dismissable);
            if state.screen_hints {
                help = help.hint("r", t!("help-layer.restart")).hint("l", t!("help-layer.logs"));
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

    use qframe::runtime::Harness;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_on;

    /// The lines of the "This screen" group, from its title to the next group.
    fn screen_group(h: &Harness<Showcase>) -> String {
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let start = lines.iter().position(|line| line.contains("This screen")).expect("the group is there");
        let end = lines[start + 1..]
            .iter()
            .position(|line| line.contains("Application") || line.contains("General"))
            .map_or(lines.len(), |next| start + 1 + next);
        lines[start..end].join("\n")
    }

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
    fn the_keys_of_the_focused_table_come_first_and_the_hints_follow() {
        let mut h = showcase_on(PAGE);
        h.click_text("web");
        assert!(h.is_focused("rows"), "a click on a row leaves the focus on the table");
        h.press("f5").advance(Duration::from_millis(200));
        let group = screen_group(&h);
        for expected in ["move", "scroll", "open", "Restart the selected container"] {
            assert!(group.contains(expected), "{expected} missing from the screen's keys:\n{group}");
        }
        assert!(
            group.find("move") < group.find("Restart the selected container"),
            "the table's own keys come before the hints:\n{group}"
        );
    }

    #[test]
    fn a_focus_that_declares_no_keys_leaves_the_group_to_the_hints() {
        let mut h = showcase_on(PAGE);
        h.click_text("Show the keys of this screen").advance(Duration::from_millis(200));
        let group = screen_group(&h);
        assert!(group.contains("Restart the selected container"), "the hints are the application's own:\n{group}");
        assert!(!group.contains("enter") && !group.contains("scroll"), "a button takes no keys of its own:\n{group}");
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
