//! Popover: content that opens as a layer next to the widget it belongs to.

use qframe::prelude::*;
use qframe::widgets::{Placement, Popover, Segmented, Select, Switch};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "popover";

/// Container runtimes offered inside the filter popover.
const RUNTIMES: [&str; 3] = ["Any", "Podman", "Docker"];

/// Filters, open popovers and the playground.
#[derive(Debug, Default)]
pub struct State {
    filters_open: bool,
    details_open: bool,
    running_only: bool,
    include_system: bool,
    runtime: usize,
    placement: usize,
    focus_inside: bool,
    apart_open: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    ToggleFilters,
    CloseFilters,
    ToggleDetails,
    CloseDetails,
    RunningOnly(bool),
    IncludeSystem(bool),
    Runtime(usize),
    Placement(usize),
    FocusInside(bool),
    ToggleApart,
    CloseApart,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Popover(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::ToggleFilters => {
            state.filters_open = !state.filters_open;
            log.push(PAGE, "Popover#filters", if state.filters_open { "opened" } else { "closed" });
        }
        Msg::CloseFilters => {
            state.filters_open = false;
            log.push(PAGE, "Popover#filters", "dismissed");
        }
        Msg::ToggleDetails => {
            state.details_open = !state.details_open;
            log.push(PAGE, "Popover#details", if state.details_open { "opened" } else { "closed" });
        }
        Msg::CloseDetails => {
            state.details_open = false;
            log.push(PAGE, "Popover#details", "dismissed");
        }
        Msg::RunningOnly(on) => {
            state.running_only = on;
            log.push(PAGE, "Switch#running", format!("toggled {on}"));
        }
        Msg::IncludeSystem(on) => {
            state.include_system = on;
            log.push(PAGE, "Switch#system", format!("toggled {on}"));
        }
        Msg::Runtime(index) => {
            state.runtime = index;
            log.push(PAGE, "Select#runtime", format!("selected {}", RUNTIMES[index]));
        }
        Msg::Placement(index) => {
            state.placement = index;
            log.push(PAGE, "Playground", format!("placement = {}", Placement::ALL[index].name()));
        }
        Msg::FocusInside(on) => {
            state.focus_inside = on;
            log.push(PAGE, "Playground", format!("focus_inside = {on}"));
        }
        Msg::ToggleApart => {
            state.apart_open = !state.apart_open;
            log.push(PAGE, "Popover#apart", if state.apart_open { "opened" } else { "closed" });
        }
        Msg::CloseApart => {
            state.apart_open = false;
            log.push(PAGE, "Popover#apart", "dismissed");
        }
    }
    Command::none()
}

/// How many of the twelve demo containers the filters leave.
fn matching(state: &State) -> usize {
    let mut count = if state.include_system { 12 } else { 9 };
    if state.running_only {
        count -= 4;
    }
    if state.runtime != 0 {
        count /= 2;
    }
    count
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.row(|ui| {
            // region: popover-filters
            Popover::new(state.filters_open)
                .on_dismiss(send(Msg::CloseFilters))
                .focus_inside(state.focus_inside)
                .anchor(|ui| {
                    ui.add(Button::new(t!("popover.filters")).icon("search").on_press(send(Msg::ToggleFilters)))
                        .id("filters");
                })
                .content(|ui| {
                    ui.column(|ui| {
                        ui.add(Text::new(t!("popover.filters-title")).role("faint").no_wrap());
                        ui.add(
                            Switch::new(state.running_only)
                                .label(t!("popover.running-only"))
                                .on_toggle(|on| send(Msg::RunningOnly(on))),
                        )
                        .id("running");
                        ui.add(
                            Switch::new(state.include_system)
                                .label(t!("popover.include-system"))
                                .on_toggle(|on| send(Msg::IncludeSystem(on))),
                        )
                        .id("system");
                        ui.add(
                            Select::new(RUNTIMES)
                                .selected(Some(state.runtime))
                                .on_select(|index| send(Msg::Runtime(index))),
                        )
                        .width(Length::Cells(24))
                        .id("runtime");
                    })
                    .gap(1);
                })
                .show(ui);
            // endregion
            ui.add(Text::new(t!("popover.matching", n = matching(state))).role("secondary").no_wrap());
        })
        .gap(3);
        ui.row(|ui| {
            // region: popover-placement
            Popover::new(state.details_open)
                .placement(Placement::ALL[state.placement])
                .on_dismiss(send(Msg::CloseDetails))
                .anchor(|ui| {
                    ui.add(Button::new(t!("popover.details")).on_press(send(Msg::ToggleDetails))).id("details");
                })
                .content(|ui| {
                    ui.add(Text::new("api-gateway  v2.14.0").role("title").no_wrap());
                    ui.add(Text::new(t!("popover.deployed")).role("secondary").no_wrap());
                    ui.add(Text::new(t!("popover.region")).role("faint").no_wrap());
                })
                .show(ui);
            // endregion
            ui.add(Text::new(t!("popover.details-hint")).role("faint").no_wrap());
        })
        .gap(3);
        ui.spacer().height(Length::Cells(10));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("popover.placement"), |ui| {
            let names = Placement::ALL.map(|placement| t!(&format!("popover.{}", placement.name())));
            ui.add(Segmented::new(names).selected(state.placement).on_select(|index| send(Msg::Placement(index))))
                .id("placement");
        });
        setting(ui, t!("popover.focus-inside"), |ui| {
            ui.add(toggle(state.focus_inside, |on| send(Msg::FocusInside(on)))).id("focus-inside");
        });
        ui.add(Text::new(t!("popover.keys")).role("faint"));
    })
    .fill_width();

    apart(state, ui);
}

/// The same layer twice, opened by one button: on the screen ground and inside a panel, where
/// it steps away from the panel's tone.
fn apart(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.row(|ui| {
        // region: popover-apart
        // On the screen ground the layer keeps the theme's overlay tone. It opens and closes with
        // the one on the panel, which is painted later and so sits on top: that one takes Esc and
        // the dismissing click.
        ui.column(|ui| {
            ui.add(Text::new(t!("popover.on-screen")).role("faint").no_wrap());
            Popover::new(state.apart_open)
                .anchor(|ui| {
                    ui.add(Text::new(t!("popover.same-layer")).role("secondary").no_wrap());
                })
                .content(|ui| {
                    ui.add(Text::new(t!("popover.overlay-tone")).no_wrap());
                    ui.add(Text::new(t!("popover.overlay-tone-detail")).role("faint").no_wrap());
                })
                .show(ui);
        })
        .width(Length::Cells(40));
        // Inside a panel of nearly the same tone the layer is lifted apart from it.
        ui.add_with(Panel::new().title(t!("popover.on-panel")), |ui| {
            Popover::new(state.apart_open)
                .on_dismiss(send(Msg::CloseApart))
                .anchor(|ui| {
                    ui.add(Button::new(t!("popover.open-both")).on_press(send(Msg::ToggleApart))).id("apart");
                })
                .content(|ui| {
                    ui.add(Text::new(t!("popover.lifted-tone")).no_wrap());
                    ui.add(Text::new(t!("popover.lifted-tone-detail")).role("faint").no_wrap());
                })
                .show(ui);
            ui.spacer().height(Length::Cells(4));
        })
        .fill_width();
        // endregion
    })
    .gap(2);
    ui.add(Text::new(t!("popover.apart-hint")).role("faint"));
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::{showcase_on, showcase_tall};

    #[test]
    fn opens_filters_changes_them_and_dismisses() {
        // Tall enough for the event log below the side-by-side layers.
        let mut h = showcase_tall(crate::app::Showcase::new(), PAGE, 80);
        h.click_text("Filters").advance(Duration::from_millis(300));
        assert!(h.screen().contains("Only running"), "{}", h.screen());
        h.click_text("Only running");
        assert!(h.app().pages.popover.running_only);
        h.press("esc");
        assert!(!h.app().pages.popover.filters_open);
        assert!(h.screen().contains("Popover#filters"));
    }

    #[test]
    fn the_layer_on_the_panel_stands_apart_and_the_one_on_the_screen_keeps_its_tone() {
        for theme in ["monochrome", "nordic", "amber", "iris"] {
            let mut h = showcase_tall(crate::app::Showcase::new(), PAGE, 80);
            h.set_theme(theme);
            h.click_text("Open both").advance(Duration::from_millis(300));
            let overlay = h.env().theme().color("overlay");
            let at = |h: &qframe::runtime::Harness<crate::app::Showcase>, text: &str| {
                let (x, y) = h.find(text).unwrap_or_else(|| panic!("{text} is on screen: {}", h.screen()));
                h.bg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0))
            };
            assert_eq!(at(&h, "The overlay tone as the theme sets it"), overlay, "{theme}");
            let lifted = at(&h, "One small step apart").expect("true colour");
            let (px, py) = h.find("On a panel").expect("the panel title");
            let panel = h.bg(u16::try_from(px).unwrap_or(0), u16::try_from(py + 1).unwrap_or(0)).expect("panel");
            assert!(lifted.perceptual_distance(panel) >= 0.05, "{theme}: {lifted} on {panel}");
            h.press("esc");
            assert!(!h.app().pages.popover.apart_open, "Esc closes both");
        }
    }

    #[test]
    fn details_follow_the_chosen_placement() {
        let mut h = showcase_on(PAGE);
        h.click_text("Right");
        h.click_text("Deploy details").advance(Duration::from_millis(300));
        let (x, y) = h.find("Deploy details").expect("anchor on screen");
        let (dx, dy) = h.find("api-gateway").expect("details open");
        assert!(dx > x + 14 && dy >= y - 1, "{}", h.screen());
    }
}
