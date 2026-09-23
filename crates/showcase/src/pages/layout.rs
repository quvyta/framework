//! Layout and pages: rows, columns, sizes, spacing, alignment, an arrangement chosen from the
//! terminal's size, and page navigation with a router.

use qframe::prelude::*;
use qframe::widget::NodeMut;
use qframe::widgets::Select;

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "layout";

/// Below this many terminal columns the space-aware demo folds its three columns. The showcase's
/// own menu takes part of the width, so the demo folds well before the page gets cramped.
const FOLD_BELOW: u16 = 110;

/// The three pages of the router demo.
const PAGES: [&str; 3] = ["inbox", "message", "reply"];

/// The buttons of the wrapping row demo, more than a narrow terminal has room for on one line.
const ACTIONS: [&str; 5] = ["install", "command", "log", "folder", "cancel"];

/// Playground settings and the router demo.
#[derive(Debug)]
pub struct State {
    gap: u16,
    justify: usize,
    router: Router<&'static str>,
}

impl Default for State {
    fn default() -> Self {
        Self { gap: 1, justify: 0, router: Router::new(PAGES[0]) }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Gap(u16),
    Justify(usize),
    Open(&'static str),
    Back,
    Tapped(&'static str),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Layout(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Tapped(name) => {
            log.push(PAGE, format!("Button#{name}"), "pressed");
            return Command::none();
        }
        Msg::Gap(gap) => state.gap = gap,
        Msg::Justify(index) => state.justify = index,
        // region: router
        Msg::Open(page) => state.router.push(page),
        Msg::Back => {
            state.router.back();
        } // endregion
    }
    log.push(PAGE, "Router", state.router.history().join(" › "));
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let justify = [Align::Start, Align::Center, Align::End].get(state.justify).copied().unwrap_or(Align::Start);
    ui.add_with(Panel::new().title(t!("layout.sizes")), |ui| {
        // region: sizes
        ui.row(|ui| {
            ui.add_with(Panel::new().variant("inset"), |ui| {
                ui.add(Text::new("Fill(1)").no_wrap());
            })
            .width(Length::Fill(1));
            ui.add_with(Panel::new().variant("inset"), |ui| {
                ui.add(Text::new("Fill(2)").no_wrap());
            })
            .width(Length::Fill(2));
            ui.add_with(Panel::new().variant("inset"), |ui| {
                ui.add(Text::new("Cells(14)").no_wrap());
            })
            .width(Length::Cells(14));
        })
        .gap(state.gap)
        .fill_width();
        // endregion
        ui.add(Text::new(t!("layout.sizes-hint")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("layout.alignment")), |ui| {
        // region: justify
        ui.row(|ui| {
            ui.add(Button::new("one").on_press(send(Msg::Tapped("one"))));
            ui.add(Button::new("two").on_press(send(Msg::Tapped("two"))));
            ui.add(Button::new("three").on_press(send(Msg::Tapped("three"))));
        })
        .gap(state.gap)
        .justify(justify)
        .fill_width();
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("layout.wrapping")), |ui| {
        // region: wrap
        ui.row(|ui| {
            for action in ACTIONS {
                let label = t!(&format!("layout.actions.{action}"));
                ui.add(Button::new(label).on_press(send(Msg::Tapped(action)))).id(format!("action-{action}"));
            }
        })
        .gap(state.gap)
        .justify(justify)
        .wrap(true)
        .fill_width();
        // endregion
        ui.add(Text::new(t!("layout.wrap-hint")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("layout.gap"), |ui| {
            ui.add(
                Select::new(["0", "1", "2", "3"])
                    .selected(Some(usize::from(state.gap)))
                    .on_select(|i| send(Msg::Gap(u16::try_from(i).unwrap_or(0)))),
            )
            .width(Length::Cells(16))
            .id("gap");
        });
        setting(ui, t!("layout.justify"), |ui| {
            let names = ["start", "center", "end"].map(|a| t!(&format!("layout.align.{a}")));
            ui.add(Select::new(names).selected(Some(state.justify)).on_select(|i| send(Msg::Justify(i))))
                .width(Length::Cells(16))
                .id("justify");
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("layout.router")), |ui| {
        ui.add(Text::new(t!("layout.router-hint")).role("secondary"));
        ui.row(|ui| {
            for page in PAGES {
                let active = *state.router.current() == page;
                let button = Button::new(t!(&format!("layout.pages.{page}"))).on_press(send(Msg::Open(page)));
                ui.add(if active { button.variant("primary") } else { button }).id(format!("open-{page}"));
            }
            ui.add(Button::new(t!("layout.back")).disabled(!state.router.can_go_back()).on_press(send(Msg::Back)))
                .id("back");
        })
        .gap(2);
        let history: Vec<String> = state.router.history().iter().map(|p| t!(&format!("layout.pages.{p}"))).collect();
        ui.add(Text::rich([
            Span::new(t!("layout.history")).role("faint"),
            Span::new(history.join("  ›  ")).color("accent"),
        ]));
    })
    .fill_width();

    adaptive(ui);
}

/// Three panes side by side on a wide terminal; below `FOLD_BELOW` columns the groups fold into a
/// strip above a full-width list and the detail pane is left for a page of its own.
fn adaptive(ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("layout.adaptive")), |ui| {
        // region: adaptive
        let size = ui.size();
        let narrow = size.width < FOLD_BELOW;
        let arrangement = if narrow { t!("layout.narrow") } else { t!("layout.wide") };
        ui.add(Text::rich([
            Span::new(t!("layout.terminal")).role("faint"),
            Span::new(t!("layout.size", width = size.width, height = size.height)).color("accent"),
            Span::new(format!("   {arrangement}")).role("secondary"),
        ]));
        if narrow {
            ui.column(|ui| {
                pane(ui, t!("layout.panes.groups")).fill_width();
                pane(ui, t!("layout.panes.items")).fill_width();
            })
            .gap(1)
            .fill_width();
        } else {
            ui.row(|ui| {
                pane(ui, t!("layout.panes.groups")).width(Length::Cells(18));
                pane(ui, t!("layout.panes.items")).width(Length::Fill(2));
                pane(ui, t!("layout.panes.detail")).width(Length::Fill(1));
            })
            .gap(2)
            .fill_width();
        }
        // endregion
        ui.add(Text::new(t!("layout.adaptive-hint", columns = FOLD_BELOW)).role("faint"));
    })
    .fill_width();
}

/// One pane of the space-aware demo: an inset surface with its name.
fn pane<'v>(ui: &'v mut View<'_, AppMsg>, name: String) -> NodeMut<'v, AppMsg> {
    ui.add_with(Panel::new().variant("inset"), |ui| {
        ui.add(Text::new(name).no_wrap());
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::{showcase_on, showcase_tall};

    #[test]
    fn router_keeps_history_and_gap_changes_layout() {
        let mut h = showcase_on(PAGE);
        h.click_text("Message").click_text("Reply");
        assert_eq!(h.app().pages.layout.router.history(), &["inbox", "message", "reply"]);
        h.click_text("Go back");
        assert_eq!(*h.app().pages.layout.router.current(), "message");
        h.send(send(Msg::Gap(3)));
        assert!(h.screen().contains("Fill(2)"));
    }

    #[test]
    fn the_space_aware_demo_folds_below_its_width() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 80);
        let wide = h.screen();
        assert!(wide.contains("140 × 80"), "{wide}");
        let (groups, items, detail) = (h.find("Groups"), h.find("Items"), h.find("Detail"));
        let (Some(groups), Some(items), Some(_)) = (groups, items, detail) else {
            panic!("three panes on a wide terminal:\n{wide}");
        };
        assert_eq!(groups.1, items.1, "side by side:\n{wide}");
        assert!(groups.0 < items.0);

        h.resize(100, 80);
        let narrow = h.screen();
        assert!(narrow.contains("100 × 80"), "{narrow}");
        assert!(h.find("Detail").is_none(), "the detail leaves the narrow layout:\n{narrow}");
        let (Some(groups), Some(items)) = (h.find("Groups"), h.find("Items")) else {
            panic!("groups and items on a narrow terminal:\n{narrow}");
        };
        assert!(groups.1 < items.1, "the groups strip sits above the list:\n{narrow}");
        assert_eq!(groups.0, items.0, "{narrow}");

        h.resize(140, 80);
        assert!(h.find("Detail").is_some(), "{}", h.screen());
    }

    #[test]
    fn the_wrapping_row_moves_its_buttons_down_when_the_terminal_narrows() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 80);
        let (Some(install), Some(cancel)) = (h.find("Install"), h.find("Cancel")) else {
            panic!("the wrapping row on a wide terminal:\n{}", h.screen());
        };
        assert_eq!(install.1, cancel.1, "one line when there is room:\n{}", h.screen());

        h.resize(100, 80);
        let narrow = h.screen();
        let (Some(install), Some(cancel), Some(hint)) = (h.find("Install"), h.find("Cancel"), h.find("With wrap on"))
        else {
            panic!("every button and the hint stay on screen:\n{narrow}");
        };
        assert!(cancel.1 > install.1, "the last button moves to a line below:\n{narrow}");
        let second_line_start = ["Show the command", "Copy the log", "Open the folder", "Cancel"]
            .into_iter()
            .filter_map(|label| h.find(label))
            .filter(|at| at.1 == cancel.1)
            .map(|at| at.0)
            .min();
        assert_eq!(second_line_start, Some(install.0), "the new line starts where the first one does:\n{narrow}");
        assert!(hint.1 > cancel.1, "the hint moves down with the row:\n{narrow}");

        h.click_text("Cancel");
        let last = h.app().log.recent(PAGE, 1).first().map(|entry| entry.source.clone());
        assert_eq!(last.as_deref(), Some("Button#cancel"), "a click where the button moved presses it");
    }

    #[test]
    fn an_unknown_alignment_draws_from_the_start() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Justify(7)));
        assert!(h.screen().contains("three"), "{}", h.screen());
    }
}
