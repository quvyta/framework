//! Page transitions: a small deploy flow whose steps cross-fade or slide, following the router's
//! direction, with reduced motion.

use qframe::prelude::*;
use qframe::storage::Settings;
use qframe::widgets::{PageTransition, Segmented};

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "page-transitions";

/// The steps of the deploy flow, in order.
const STEPS: [&str; 3] = ["source", "build", "release"];

/// The flow's router and the playground.
#[derive(Debug)]
pub struct State {
    router: Router<String>,
    slide: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { router: Router::new(STEPS[0].to_owned()), slide: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Next,
    Back,
    Style(usize),
    Reduced(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::PageTransitions(message))
}

fn step_index(router: &Router<String>) -> usize {
    STEPS.iter().position(|step| step == router.current()).unwrap_or(0)
}

/// Applies a demo message. Reduced motion is remembered in `settings`, like every switch for it.
pub fn update(state: &mut State, settings: &mut Settings, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: navigate
        Msg::Next => {
            if let Some(next) = STEPS.get(step_index(&state.router) + 1) {
                state.router.push((*next).to_owned());
                log.push(PAGE, "Router", format!("push {next}"));
            }
        }
        Msg::Back => {
            if state.router.back() {
                log.push(PAGE, "Router", format!("back to {}", state.router.current()));
            }
        }
        // endregion
        Msg::Style(index) => {
            state.slide = index == 1;
            log.push(PAGE, "Segmented#style", if state.slide { "slide" } else { "fade" });
        }
        Msg::Reduced(on) => {
            log.push(PAGE, "Playground", format!("reduced motion = {on}"));
            return super::storage::apply_and_remember(
                settings,
                Settings::REDUCED_MOTION,
                on,
                Command::set_reduced_motion(on),
            );
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let index = step_index(&state.router);
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("page-transitions.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            for (position, step) in STEPS.iter().enumerate() {
                let role = if position == index { "title" } else { "faint" };
                ui.add(
                    Text::new(format!("{} {}", position + 1, t!(&format!("page-transitions.{step}"))))
                        .role(role)
                        .no_wrap(),
                );
            }
        })
        .gap(3);
        ui.spacer().height(Length::Cells(1));
        // region: transition
        let step = state.router.current().clone();
        let transition = PageTransition::new(step.clone()).slide(state.slide).direction(state.router.direction());
        ui.add_with(transition, |ui| {
            ui.page(step.clone(), |ui| {
                ui.add(Text::new(t!(&format!("page-transitions.{step}-title"))).role("title"));
                ui.add(Text::new(t!(&format!("page-transitions.{step}-body"))).role("body"));
            });
        })
        .height(Length::Cells(4))
        .fill_width()
        .id("flow");
        // endregion
        ui.row(|ui| {
            ui.add(
                Button::new(t!("page-transitions.back"))
                    .disabled(!state.router.can_go_back())
                    .on_press(send(Msg::Back)),
            )
            .id("back");
            let last = index + 1 == STEPS.len();
            ui.add(
                Button::new(t!("page-transitions.next")).variant("primary").disabled(last).on_press(send(Msg::Next)),
            )
            .id("next");
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("page-transitions.style"), |ui| {
            let styles = [t!("page-transitions.fade"), t!("page-transitions.slide")];
            ui.add(Segmented::new(styles).selected(usize::from(state.slide)).on_select(|i| send(Msg::Style(i))))
                .id("style");
        });
        super::motion::reduced_motion_setting(ui, t!("motion.reduced"), |on| send(Msg::Reduced(on)));
        ui.add(Text::new(t!("page-transitions.shell-hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn steps_move_forward_and_back_through_a_transition() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("Pick the source"), "{}", h.screen());
        h.click_text("Next");
        assert!(h.screen().contains("Pick the source"), "the first frame still shows the old step");
        h.advance(Duration::from_secs(1));
        assert!(h.screen().contains("Build the image"), "{}", h.screen());
        h.click_text("Back").advance(Duration::from_secs(1));
        assert!(h.screen().contains("Pick the source"));
        h.send(send(Msg::Reduced(true)));
        h.click_text("Next");
        assert!(h.screen().contains("Build the image"), "reduced motion changes at once");
    }

    #[test]
    fn reduced_motion_is_remembered_like_on_the_motion_page() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Reduced(true)));
        assert!(h.env().reduced_motion());
        assert_eq!(h.app().pages.storage.settings.reduced_motion(), Some(true));
    }
}
