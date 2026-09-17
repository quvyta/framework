//! Getting started: a tiny application with state, messages, a view and background work.

use qframe::prelude::*;

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "getting-started";

// region: state
/// Everything the demo remembers. The runtime draws from it and changes it only through messages.
#[derive(Debug, Default)]
pub struct State {
    count: i64,
    files: Option<usize>,
    counting: bool,
}

/// Everything that can happen in the demo.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Increment,
    Decrement,
    Reset,
    CountFiles,
    FilesCounted(usize),
}
// endregion

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::GettingStarted(message))
}

// region: update
/// Applies a message. Slow work never runs here: it is handed to the runtime as a command.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Increment => state.count += 1,
        Msg::Decrement => state.count -= 1,
        Msg::Reset => state.count = 0,
        Msg::CountFiles => {
            state.counting = true;
            log.push(PAGE, "Button#count-files", "CountFiles");
            return Command::perform(|| {
                let entries = std::fs::read_dir(".").map_or(0, |dir| dir.flatten().count());
                send(Msg::FilesCounted(entries))
            });
        }
        Msg::FilesCounted(count) => {
            state.counting = false;
            state.files = Some(count);
            log.push(PAGE, "Command::perform", format!("FilesCounted({count})"));
            return Command::none();
        }
    }
    log.push(PAGE, "Button#counter", format!("count = {}", state.count));
    Command::none()
}
// endregion

// region: view
/// Describes the screen. It runs after every change and only reads the state.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::rich([
            Span::new(t!("getting-started.count")).role("secondary"),
            Span::new(format!("  {}", state.count)).role("title"),
        ]));
        ui.row(|ui| {
            ui.add(Button::new("−").on_press(send(Msg::Decrement))).id("decrement");
            ui.add(Button::new("+").variant("primary").on_press(send(Msg::Increment))).id("increment");
            ui.add(Button::new(t!("getting-started.reset")).on_press(send(Msg::Reset))).id("reset");
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("getting-started.background")), |ui| {
        ui.add(Text::new(t!("getting-started.background-text")).role("secondary"));
        ui.row(|ui| {
            ui.add(
                Button::new(t!("getting-started.count-files"))
                    .icon("folder")
                    .loading(state.counting)
                    .on_press(send(Msg::CountFiles)),
            )
            .id("count-files");
            if let Some(files) = state.files {
                ui.add(Text::new(t!("getting-started.files", n = files)).color("success").no_wrap());
            }
        })
        .gap(3);
    })
    .fill_width();
}
// endregion

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn counts_and_runs_background_work() {
        let mut h = showcase_on(PAGE);
        h.click_text("+").click_text("+").click_text("−");
        assert_eq!(h.app().pages.getting_started.count, 1);
        h.click_text("Count entries");
        assert!(h.app().pages.getting_started.files.is_some());
        assert!(h.screen().contains("FilesCounted"));
    }
}
