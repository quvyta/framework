//! Button row: a toolbar that keeps every action, moving the ones that do not fit into a menu.

use qframe::prelude::*;
use qframe::widgets::{ButtonRow, Segmented};

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "button-row";

/// The actions of the demo toolbar, in the order they are added.
const ACTIONS: [&str; 6] = ["Open", "Save", "Rename", "Compare", "Publish", "Settings"];

/// Widths the playground offers; 0 means the whole panel.
const WIDTHS: [u16; 3] = [0, 60, 30];

/// The width under test and the action pressed last.
#[derive(Debug, Default)]
pub struct State {
    width: usize,
    last: Option<usize>,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    /// The button with this index in [`ACTIONS`] was pressed, from the row or from its menu.
    Pressed(usize),
    Width(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ButtonRow(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: button-row-update
        Msg::Pressed(index) => {
            state.last = Some(index);
            log.push(PAGE, "ButtonRow#bar", format!("pressed {}", ACTIONS[index]));
        }
        // endregion
        Msg::Width(index) => {
            state.width = index;
            log.push(PAGE, "Playground", format!("width = {}", WIDTHS[index]));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("button-row.hint")).role("secondary"));
        // region: button-row
        // A toolbar is a row of the buttons it offers; nothing else changes when the row narrows.
        let mut bar = ButtonRow::new();
        for (index, action) in ACTIONS.iter().enumerate() {
            let primary = index == 0;
            let button = Button::new(*action).on_press(send(Msg::Pressed(index))).variant(if primary {
                "primary"
            } else {
                "neutral"
            });
            bar = bar.button(button);
        }
        let bar = ui.add(bar).id("bar");
        // endregion
        // region: button-row-width
        match WIDTHS[state.width] {
            0 => bar.fill_width(),
            cells => bar.width(Length::Cells(cells)),
        };
        // endregion
        let last = state.last.map_or_else(|| t!("button-row.nothing"), |index| ACTIONS[index].to_owned());
        ui.add(Text::new(t!("button-row.last", action = last)).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("button-row.width"), |ui| {
            let names = [
                t!("button-row.full"),
                t!("button-row.columns", n = WIDTHS[1]),
                t!("button-row.columns", n = WIDTHS[2]),
            ];
            ui.add(Segmented::new(names).selected(state.width).on_select(|i| send(Msg::Width(i)))).id("width");
        });
        ui.add(Text::new(t!("button-row.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;
    use std::time::Duration;

    /// The row the demo's own buttons stand in.
    fn bar_row(h: &qframe::runtime::Harness<crate::app::Showcase>) -> i32 {
        h.find(ACTIONS[0]).map_or_else(|| panic!("the toolbar:\n{}", h.screen()), |(_, y)| y)
    }

    /// The row of the live demo, without the panels around it.
    fn bar(h: &qframe::runtime::Harness<crate::app::Showcase>) -> String {
        let screen = h.screen();
        screen.lines().nth(usize::try_from(bar_row(h)).unwrap_or(0)).unwrap_or_default().to_owned()
    }

    /// Presses `action` where it stands in the demo's own row. The page's list names some of the
    /// actions too, and a click on that name there goes to the list.
    fn click_action(h: &mut qframe::runtime::Harness<crate::app::Showcase>, action: &str) {
        let y = bar_row(h);
        let screen = h.screen();
        let line = screen.lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default();
        let byte = line.find(action).unwrap_or_else(|| panic!("{action} is not on the row:\n{screen}"));
        h.click(i32::try_from(line[..byte].chars().count()).expect("on screen"), y);
    }

    /// Opens the menu of the actions that do not fit, as a person does with the control.
    fn open_menu(h: &mut qframe::runtime::Harness<crate::app::Showcase>) {
        click_action(h, "More");
        h.advance(Duration::from_millis(300));
    }

    /// Takes the entry `down` places below the first, walking the menu with the arrow keys and
    /// pressing Enter, as a person does.
    fn choose(h: &mut qframe::runtime::Harness<crate::app::Showcase>, down: usize) {
        for _ in 0..down {
            h.press("down");
        }
        h.press("enter");
    }

    #[test]
    fn every_action_stays_reachable_at_every_width() {
        let mut h = showcase_on(PAGE);
        for action in ACTIONS {
            assert!(bar(&h).contains(action), "{action} is on the row:\n{}", bar(&h));
        }
        assert!(!bar(&h).contains("More"), "and no other control while they all fit:\n{}", bar(&h));
        click_action(&mut h, "Settings");
        assert!(h.screen().contains("pressed Settings"), "{}", h.screen());

        // Sixty columns: the last two actions wait in the menu of the control at the end.
        h.send(send(Msg::Width(1)));
        let row = bar(&h);
        assert!(row.contains("Compare") && !row.contains("Publish"), "the row's end moves into the menu:\n{row}");
        open_menu(&mut h);
        assert!(h.screen().contains("Publish"), "the menu lists what did not fit:\n{}", h.screen());
        choose(&mut h, 0);
        assert!(h.screen().contains("pressed Publish"), "the menu sends the button's own message:\n{}", h.screen());

        // Thirty columns: two buttons and the control, and the rest are all still there.
        h.send(send(Msg::Width(2)));
        let row = bar(&h);
        assert!(row.contains("Open") && row.contains("Save"), "two buttons at the left:\n{row}");
        assert!(!row.contains("Rename"), "the row's end is the control:\n{row}");
        assert_eq!(row.matches("More").count(), 1, "the control stands once, at the end:\n{row}");
        open_menu(&mut h);
        for waiting in ["Rename", "Compare", "Publish", "Settings"] {
            assert!(h.screen().contains(waiting), "{waiting} waits in the menu:\n{}", h.screen());
        }
        choose(&mut h, 1);
        assert!(h.screen().contains("pressed Compare"), "{}", h.screen());
    }
}
