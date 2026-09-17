//! Layout and pages: rows, columns, sizes, spacing, alignment, and page navigation with a router.

use qframe::prelude::*;
use qframe::widgets::Select;

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "layout";

/// The three pages of the router demo.
const PAGES: [&str; 3] = ["inbox", "message", "reply"];

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

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
    fn an_unknown_alignment_draws_from_the_start() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Justify(7)));
        assert!(h.screen().contains("three"), "{}", h.screen());
    }
}
