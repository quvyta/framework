//! Text area: multi-line editing with wrapping and scrolling, a length limit with a counter,
//! line numbers, validation and submitting.

use qframe::prelude::*;
use qframe::widgets::TextArea;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "text-area";

/// The most characters a release summary may have.
const SUMMARY_LIMIT: usize = 280;

/// The deploy script the playground starts with.
const SCRIPT: &str = "podman pull ghcr.io/quvyta/api:2.4.1\npodman stop api\npodman run -d --name api -p 8080:8080 ghcr.io/quvyta/api:2.4.1\ncurl --fail http://localhost:8080/health";

/// Field values and playground settings.
#[derive(Debug)]
pub struct State {
    summary: String,
    script: String,
    submitted: Option<String>,
    line_numbers: bool,
    counter: bool,
    plain: bool,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            summary: String::new(),
            script: SCRIPT.to_owned(),
            submitted: None,
            line_numbers: false,
            counter: false,
            plain: false,
            disabled: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Summary(String),
    Submit(String),
    Script(String),
    LineNumbers(bool),
    Counter(bool),
    Plain(bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::TextArea(message))
}

// region: validation
/// A summary that is only whitespace would publish an empty release note.
fn blank(summary: &str) -> bool {
    !summary.is_empty() && summary.trim().is_empty()
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Summary(value) => {
            let text = format!("changed, {} characters", value.chars().count());
            state.summary = value;
            ("TextArea#summary", text)
        }
        Msg::Submit(value) => {
            let text = format!("submitted, {} lines", value.lines().count());
            state.submitted = Some(value);
            ("TextArea#summary", text)
        }
        Msg::Script(value) => {
            let text = format!("changed, {} lines", value.lines().count());
            state.script = value;
            ("TextArea#script", text)
        }
        Msg::LineNumbers(on) => {
            state.line_numbers = on;
            ("Playground", format!("line numbers = {on}"))
        }
        Msg::Counter(on) => {
            state.counter = on;
            ("Playground", format!("counter = {on}"))
        }
        Msg::Plain(on) => {
            state.plain = on;
            ("Playground", format!("plain = {on}"))
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            ("Playground", format!("disabled = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("text-area.summary")).role("secondary"));
        // region: summary
        let invalid = blank(&state.summary);
        ui.add(
            TextArea::new(&state.summary)
                .placeholder(t!("text-area.summary-placeholder"))
                .max_length(SUMMARY_LIMIT)
                .counter(true)
                .invalid(invalid)
                .on_change(|value| send(Msg::Summary(value)))
                .on_submit(|value| send(Msg::Submit(value))),
        )
        .width(Length::Cells(64))
        .id("summary");
        if invalid {
            ui.add(Text::new(t!("text-area.blank")).color("danger"));
        }
        // endregion
        ui.add(Text::new(t!("text-area.submit-hint")).role("faint"));
        if let Some(submitted) = &state.submitted {
            let lines = submitted.lines().count();
            ui.add(Text::new(t!("text-area.submitted", lines = lines)).color("success"));
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.add(Text::new(t!("text-area.playground")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: configured
        let mut script = TextArea::new(&state.script)
            .line_numbers(state.line_numbers)
            .counter(state.counter)
            .disabled(state.disabled)
            .on_change(|value| send(Msg::Script(value)));
        if state.plain {
            // Paper: the panel's own tone instead of a raised field.
            script = script.variant("plain");
        }
        ui.add(script).width(Length::Cells(64)).height(Length::Cells(5)).id("script");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("text-area.line-numbers"), |ui| {
            ui.add(toggle(state.line_numbers, |on| send(Msg::LineNumbers(on)))).id("line-numbers");
        });
        setting(ui, t!("text-area.counter"), |ui| {
            ui.add(toggle(state.counter, |on| send(Msg::Counter(on)))).id("counter");
        });
        setting(ui, t!("text-area.plain"), |ui| {
            ui.add(toggle(state.plain, |on| send(Msg::Plain(on)))).id("plain");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("text-area.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn writes_lines_counts_validates_and_submits() {
        let mut h = showcase_on(PAGE);
        h.click_text("What changed");
        h.type_text("Faster deploys").press("enter").type_text("Fewer restarts");
        assert_eq!(h.app().pages.text_area.summary, "Faster deploys\nFewer restarts");
        assert!(h.screen().contains("29 / 280"), "{}", h.screen());
        h.press("ctrl+enter");
        assert_eq!(h.app().pages.text_area.submitted.as_deref(), Some("Faster deploys\nFewer restarts"));
        h.press("ctrl+a").type_text(" ");
        assert!(h.screen().contains("only spaces"), "{}", h.screen());
        h.send(send(Msg::LineNumbers(true)));
        assert!(h.screen().contains(" 1 podman pull"), "{}", h.screen());
    }

    #[test]
    fn the_plain_script_sits_on_the_panel_tone() {
        // Tall enough to show the playground's switch under the demo.
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::new(), PAGE, 90);
        let (x, y) = h.find("podman pull").expect("the script is drawn");
        let cell = (u16::try_from(x).unwrap(), u16::try_from(y).unwrap());
        let surface = h.env().theme().color("surface");
        assert_ne!(h.bg(cell.0, cell.1), surface, "a raised field by default");
        // The playground's switches stand after their labels' column of 24 cells.
        let (sx, sy) =
            h.find("Plain, in the panel").unwrap_or_else(|| panic!("the switch is on screen:\n{}", h.screen()));
        h.click(sx + 25, sy);
        assert!(h.app().pages.text_area.plain, "the click turned the plain look on");
        assert_eq!(h.bg(cell.0, cell.1), surface);
    }
}
