//! Radio group: one choice among a few visible options, in a column or a row.

use qframe::prelude::*;
use qframe::widgets::{RadioGroup, RadioStyle, Segmented};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "radio-group";

/// Container engines of the first group.
const ENGINES: [&str; 3] = ["Podman", "Docker", "Nerdctl"];

/// Densities of the second group.
const DENSITIES: [&str; 3] = ["compact", "comfortable", "spacious"];

/// Styles the playground offers, the default first, with their locale keys.
const STYLES: [(RadioStyle, &str); 4] =
    [(RadioStyle::Square, "square"), (RadioStyle::Mark, "mark"), (RadioStyle::Box, "box"), (RadioStyle::Dot, "dot")];

/// Choices and the playground.
#[derive(Debug)]
pub struct State {
    engine: Option<usize>,
    density: Option<usize>,
    style: usize,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { engine: Some(0), density: None, style: 0, disabled: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Engine(usize),
    Density(usize),
    Style(usize),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::RadioGroup(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Engine(index) => {
            state.engine = Some(index);
            log.push(PAGE, "RadioGroup#engine", format!("selected {}", ENGINES[index]));
        }
        Msg::Density(index) => {
            state.density = Some(index);
            log.push(PAGE, "RadioGroup#density", format!("selected {}", DENSITIES[index]));
        }
        Msg::Style(index) => {
            state.style = index;
            log.push(PAGE, "Playground", format!("style = {}", STYLES[index].1));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let style = STYLES[state.style].0;
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("radio-group.engine")).role("secondary"));
        // region: vertical
        ui.add(
            RadioGroup::new(ENGINES)
                .style(style)
                .selected(state.engine)
                .disabled(state.disabled)
                .on_select(|index| send(Msg::Engine(index))),
        )
        .id("engine");
        // endregion
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("radio-group.density")).role("secondary"));
        // region: horizontal
        let densities = DENSITIES.map(|density| t!(&format!("radio-group.{density}")));
        ui.add(
            RadioGroup::new(densities)
                .style(style)
                .horizontal(true)
                .selected(state.density)
                .disabled(state.disabled)
                .on_select(|index| send(Msg::Density(index))),
        )
        .id("density");
        // endregion
        ui.add(Text::new(t!("radio-group.nothing")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("radio-group.style"), |ui| {
            let names = STYLES.map(|(_, name)| t!(&format!("radio-group.style-{name}")));
            ui.add(Segmented::new(names).selected(state.style).on_select(|i| send(Msg::Style(i)))).id("style");
        });
        let note = match STYLES[state.style].0 {
            RadioStyle::Square | RadioStyle::Mark => "radio-group.mark-glyphs",
            RadioStyle::Box | RadioStyle::Dot => "radio-group.same-look",
        };
        ui.add(Text::new(t!(note)).role("faint"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn clicks_and_keys_choose() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("🬇🬃  Docker"), "the demo shows the default mark style: {screen}");
        h.click_text("Docker");
        assert_eq!(h.app().pages.radio_group.engine, Some(1));
        h.press("down");
        assert_eq!(h.app().pages.radio_group.engine, Some(2));
        h.click_text("Spacious");
        assert_eq!(h.app().pages.radio_group.density, Some(2));
        h.send(send(Msg::Style(3)));
        let screen = h.screen();
        assert!(screen.contains("style = dot"), "{screen}");
        h.click_text("Podman");
        assert_eq!(h.app().pages.radio_group.engine, Some(0), "the dot style chooses the same way");
    }
}
