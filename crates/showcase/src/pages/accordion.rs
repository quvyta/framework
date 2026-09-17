//! Accordion: a container's details in sections that open and close, with one-at-a-time,
//! icons and details turned on in the playground.

use qframe::prelude::*;
use qframe::widgets::{Accordion, Section};

use super::{PageMsg, setting, slide_setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "accordion";

/// Section keys with their icon.
const SECTIONS: [(&str, &str); 4] =
    [("overview", "info"), ("ports", "arrow-right"), ("volumes", "folder"), ("environment", "file")];

/// Open sections and the playground.
#[derive(Debug)]
pub struct State {
    open: [bool; 4],
    single: bool,
    icons: bool,
    details: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { open: [true, false, false, false], single: false, icons: false, details: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Toggle(usize, bool),
    Single(bool),
    Icons(bool),
    Details(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Accordion(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: accordion-update
        Msg::Toggle(index, open) => {
            state.open[index] = open;
            let verb = if open { "opened" } else { "closed" };
            log.push(PAGE, "Accordion#details", format!("{verb} {}", SECTIONS[index].0));
        }
        // endregion
        Msg::Single(on) => {
            state.single = on;
            if on {
                // Keep the first open section when switching to one at a time.
                let first = state.open.iter().position(|open| *open);
                state.open = [false; 4];
                if let Some(first) = first {
                    state.open[first] = true;
                }
            }
            log.push(PAGE, "Playground", format!("single = {on}"));
        }
        Msg::Icons(on) => {
            state.icons = on;
            log.push(PAGE, "Playground", format!("icons = {on}"));
        }
        Msg::Details(on) => {
            state.details = on;
            log.push(PAGE, "Playground", format!("details = {on}"));
        }
    }
    Command::none()
}

/// One label and value row of a section body.
fn fact(ui: &mut View<'_, AppMsg>, label: String, value: &str) {
    ui.row(|ui| {
        ui.add(Text::new(label).role("faint").no_wrap()).width(Length::Cells(14));
        ui.add(Text::new(value).no_wrap());
    });
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("accordion.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: accordion
        let sections = SECTIONS.map(|(key, icon)| {
            let mut section = Section::new(t!(&format!("accordion.{key}")));
            if state.icons {
                section = section.icon(icon);
            }
            if state.details {
                section = section.detail(t!(&format!("accordion.{key}-detail")));
            }
            section
        });
        let accordion = Accordion::new(sections)
            .open(state.open)
            .single(state.single)
            .on_toggle(|index, open| send(Msg::Toggle(index, open)));
        ui.add_with(accordion, |ui| {
            ui.column(|ui| {
                fact(ui, t!("accordion.image"), "quvyta/dev:1.4");
                fact(ui, t!("accordion.uptime"), "3 h 12 min");
                fact(ui, t!("accordion.restarts"), "0");
            });
            ui.column(|ui| {
                fact(ui, "8080 → 80".to_owned(), "http");
                fact(ui, "5432 → 5432".to_owned(), "postgres");
            });
            ui.column(|ui| {
                fact(ui, "workspace".to_owned(), "/work");
                fact(ui, "cargo-cache".to_owned(), "/usr/local/cargo");
                fact(ui, "pg-data".to_owned(), "/var/lib/postgresql");
            });
            ui.column(|ui| {
                for (name, value) in [
                    ("RUST_LOG", "info"),
                    ("DATABASE_URL", "postgres://db"),
                    ("PORT", "8080"),
                    ("TZ", "UTC"),
                    ("CI", "false"),
                ] {
                    fact(ui, name.to_owned(), value);
                }
            });
        })
        .width(Length::Cells(56))
        .id("details");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("accordion.single"), |ui| {
            ui.add(toggle(state.single, |on| send(Msg::Single(on)))).id("single");
        });
        setting(ui, t!("accordion.icons"), |ui| {
            ui.add(toggle(state.icons, |on| send(Msg::Icons(on)))).id("icons");
        });
        setting(ui, t!("accordion.details"), |ui| {
            ui.add(toggle(state.details, |on| send(Msg::Details(on)))).id("details-toggle");
        });
        slide_setting(ui, PAGE);
        ui.add(Text::new(t!("accordion.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn sections_open_close_and_single_keeps_one() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("quvyta/dev:1.4"), "{}", h.screen());
        h.click_text("Ports");
        assert_eq!(h.app().pages.accordion.open, [true, true, false, false]);
        h.send(send(Msg::Single(true)));
        assert_eq!(h.app().pages.accordion.open, [true, false, false, false]);
        h.click_text("Volumes");
        assert_eq!(h.app().pages.accordion.open, [false, false, true, false]);
        h.send(send(Msg::Details(true)));
        assert!(h.screen().contains("3 mounts"), "{}", h.screen());
    }

    #[test]
    fn the_chevron_stays_put_while_icon_and_title_slide() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Icons(true)));
        let (chevron, label) = super::super::mark_and_label(&h, '▸', "▪ Environment");
        assert_eq!(label, chevron + 2);
        let (x, y) = h.find("Environment").expect("environment title");
        h.hover(x + 2, y);
        let moved = super::super::mark_and_label(&h, '▸', "▪ Environment");
        assert_eq!(moved, (chevron, chevron + 3), "only icon and title slide");
    }
}
