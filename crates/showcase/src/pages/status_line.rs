//! Status line: one line of the application's own, saying how things stand, with a sign that
//! gives the tone its meaning.

use qframe::prelude::*;
use qframe::widgets::{Button, Segmented, StatusLine, ToastKind};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "status-line";

/// What each kind says about the host right now: a state, not the news a toast carries.
const REPORTED: [(ToastKind, &str); 4] = [
    (ToastKind::Success, "status-line.healthy"),
    (ToastKind::Warning, "status-line.disk"),
    (ToastKind::Danger, "status-line.unresponsive"),
    (ToastKind::Info, "status-line.release"),
];

/// The kind the playground's line reports, `ToastKind::ALL` order.
fn kind_index(kind: ToastKind) -> usize {
    ToastKind::ALL.iter().position(|known| *known == kind).unwrap_or(0)
}

/// Width of the playground's line when it is drawn narrow, where the sentence wraps under itself.
const NARROW: u16 = 26;

/// The line the playground builds and the host it reports on.
#[derive(Debug)]
pub struct State {
    /// The kind the line reports, as an index into [`ToastKind::ALL`].
    kind: usize,
    /// Whether the line carries a button.
    action: bool,
    /// Whether the line is drawn in a narrow area.
    narrow: bool,
    /// Whether the build is running again, after the button was pressed.
    running: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { kind: kind_index(ToastKind::Danger), action: true, narrow: false, running: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Kind(usize),
    Action(bool),
    Narrow(bool),
    Retry,
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Kind(index) => {
            state.kind = index;
            state.running = false;
            log.push(PAGE, "Playground", format!("tone = {}", ToastKind::ALL[index].name()));
        }
        Msg::Action(on) => {
            state.action = on;
            log.push(PAGE, "Playground", format!("action = {on}"));
        }
        Msg::Narrow(on) => {
            state.narrow = on;
            log.push(PAGE, "Playground", format!("narrow area = {on}"));
        }
        // region: status-line-action
        Msg::Retry => {
            state.running = !state.running;
            let verb = if state.running { "retrying" } else { "failed again" };
            log.push(PAGE, "Button#retry", format!("{verb}: {}", ToastKind::ALL[state.kind].name()));
        } // endregion
    }
    Command::none()
}

/// The line the playground builds: the build's state in its tone, with the way back when the
/// state is worth a button.
fn reported(state: &State) -> StatusLine<AppMsg> {
    if state.running {
        return StatusLine::new(t!("status-line.retrying")).tone(ToastKind::Info);
    }
    let line = StatusLine::new(t!("status-line.failed")).tone(ToastKind::ALL[state.kind]);
    if state.action { line.action(Button::new(t!("status-line.retry")).on_press(send(Msg::Retry))) } else { line }
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::StatusLine(message))
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("status-line.host")).gap(0), |ui| {
        // region: status-line-kinds
        for (kind, sentence) in REPORTED {
            ui.add(StatusLine::new(t!(sentence)).tone(kind));
        }
        // endregion
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("status-line.host-hint")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        // region: status-line-plain
        ui.add(StatusLine::new(t!("status-line.healthy")).tone(ToastKind::Success));
        // endregion
        ui.spacer().height(Length::Cells(1));
        // region: status-line-built
        let line = reported(state);
        if state.narrow {
            ui.add(line).width(Length::Cells(NARROW));
        } else {
            ui.add(line).fill_width();
        }
        // endregion
        if state.narrow {
            ui.add(Text::new(t!("status-line.narrow-hint")).role("faint"));
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("status-line.tone"), |ui| {
            let names = REPORTED.map(|(_, sentence)| t!(sentence));
            ui.add(Segmented::new(names).selected(state.kind).on_select(|index| send(Msg::Kind(index)))).id("tone");
        });
        setting(ui, t!("status-line.action"), |ui| {
            ui.add(toggle(state.action, |on| send(Msg::Action(on)))).id("action");
        });
        setting(ui, t!("status-line.narrow"), |ui| {
            ui.add(toggle(state.narrow, |on| send(Msg::Narrow(on)))).id("narrow");
        });
        ui.add(Text::new(t!("status-line.hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::runtime::Harness;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_on;

    /// Clicks the switch of the playground row labelled `label`; the switch stands after the
    /// label's column of 24 cells.
    fn click_setting(h: &mut Harness<Showcase>, label: &str) {
        let (x, y) = h.find(label).unwrap_or_else(|| panic!("the {label} row:\n{}", h.screen()));
        h.click(x + 25, y);
    }

    #[test]
    fn every_kind_is_shown_with_its_sign_and_the_button_answers() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        for sentence in ["healthy", "almost full", "not responding", "0.2 is available"] {
            assert!(screen.contains(sentence), "{sentence}: {screen}");
        }
        h.click_text("Retry");
        assert!(h.screen().contains("Retrying the build"), "{}", h.screen());
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "retrying: danger");
    }

    #[test]
    fn a_narrow_line_wraps_the_sentence_under_itself() {
        let mut h = showcase_on(PAGE);
        click_setting(&mut h, "Narrow area");
        let (first_column, first_row) = h.find("The build of").expect("the sentence starts on one row");
        let (rest_column, rest_row) = h.find("web-frontend").expect("and goes on to the next");
        assert!(rest_row > first_row, "the sentence wraps: {}", h.screen());
        assert_eq!(first_column, rest_column, "and keeps its column: {}", h.screen());
        let (sign_column, _) = h.find("✕").expect("the danger sign stands beside the words");
        assert!(sign_column < first_column, "never under the sign: {}", h.screen());
        let (_, button_row) = h.find("Retry").expect("the button is still there");
        assert!(button_row > rest_row, "and the button moves below: {}", h.screen());
    }
}
