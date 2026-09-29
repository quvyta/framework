//! Installing a missing program's package: the package manager of the machine, the exact
//! command, and the question that shows it. The page pretends which package manager the machine
//! has and never runs one.

use std::path::PathBuf;

use qframe::install::{Install, Manager};
use qframe::prelude::*;
use qframe::widgets::RadioGroup;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "install";

/// What the demo application misses, and the package that brings it.
const PROGRAM: &str = "bsdtar";
const PACKAGE: &str = "libarchive";

/// What the last question came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Agreed,
    Declined,
}

/// The machine the page pretends and the last answer.
#[derive(Debug)]
pub struct State {
    /// An index into [`Manager::all`], or its length for a machine with none.
    manager: usize,
    root: bool,
    debian_name: bool,
    answer: Option<Answer>,
}

impl Default for State {
    fn default() -> Self {
        Self { manager: 0, root: false, debian_name: true, answer: None }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Ask,
    Install,
    Declined,
    Manager(usize),
    Root(bool),
    DebianName(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Install(message))
}

/// The install on the pretended machine: a lookup that finds only the chosen package manager.
fn install(state: &State) -> Option<Install> {
    let pretended = Manager::all().get(state.manager).copied();
    // region: install-find
    // A real application calls Install::package(PACKAGE), which searches PATH.
    let lookup = |program: &str| {
        pretended
            .filter(|manager| manager.name() == program)
            .map(|manager| PathBuf::from("/usr/bin").join(manager.name()))
    };
    let mut install = Install::package_with(PACKAGE, lookup, state.root)?;
    // endregion
    if state.debian_name {
        // region: install-name
        install = install.name_for(Manager::Apt, "libarchive-tools");
        // endregion
    }
    Some(install)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Ask => {
            let Some(install) = install(state) else {
                return Command::none();
            };
            log.push(PAGE, "Install::confirm", install.command_line());
            state.answer = None;
            // region: install-ask
            return Command::confirm(install.confirm(send(Msg::Install)).on_cancel(send(Msg::Declined)));
            // endregion
        }
        Msg::Install => {
            // An application returns Command::handoff(install.handoff(..)) here; the showcase
            // stops before it, so nothing is ever installed from this page.
            state.answer = Some(Answer::Agreed);
            log.push(PAGE, "Confirm", "agreed, the handoff is not run in the showcase");
        }
        Msg::Declined => {
            state.answer = Some(Answer::Declined);
            log.push(PAGE, "Confirm", "declined, nothing installed");
        }
        Msg::Manager(index) if index <= Manager::all().len() => {
            state.manager = index;
            state.answer = None;
            let name = Manager::all().get(index).map_or("none", |manager| manager.name());
            log.push(PAGE, "Playground", format!("package manager = {name}"));
        }
        Msg::Manager(_) => {}
        Msg::Root(on) => {
            state.root = on;
            state.answer = None;
            log.push(PAGE, "Playground", format!("root = {on}"));
        }
        Msg::DebianName(on) => {
            state.debian_name = on;
            state.answer = None;
            log.push(PAGE, "Playground", format!("name_for(Apt) = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("install.hint")).role("secondary"));
        let warning = ui.env().icons().glyph("warning").into_owned();
        ui.add(Text::rich([
            Span::new(format!("{warning} ")).color("warning"),
            Span::new(t!("install.missing", program = PROGRAM)).role("body"),
        ]));
        match install(state) {
            Some(install) => {
                ui.add(Text::rich([
                    Span::new(t!("install.command")).role("faint"),
                    Span::new(format!("  {}", install.command_line())).role("body"),
                ]));
                // region: install-button
                ui.add(
                    Button::new(t!("install.button", package = install.name()))
                        .variant("primary")
                        .on_press(send(Msg::Ask)),
                )
                .id("install");
                // endregion
            }
            None => {
                ui.add(Text::new(t!("install.none", package = PACKAGE)).role("secondary"));
            }
        }
        match state.answer {
            Some(Answer::Agreed) => {
                let success = ui.env().icons().glyph("success").into_owned();
                ui.add(Text::rich([
                    Span::new(format!("{success} ")).color("success"),
                    Span::new(t!("install.agreed")).role("secondary"),
                ]));
            }
            Some(Answer::Declined) => {
                ui.add(Text::new(t!("install.declined")).role("secondary"));
            }
            None => {}
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("install.manager"), |ui| {
            let mut names = Manager::all().iter().map(|manager| manager.name().to_owned()).collect::<Vec<_>>();
            names.push(t!("install.no-manager"));
            ui.add(RadioGroup::new(names).selected(Some(state.manager)).on_select(|index| send(Msg::Manager(index))))
                .id("manager");
        });
        setting(ui, t!("install.root"), |ui| {
            ui.add(toggle(state.root, |on| send(Msg::Root(on)))).id("root");
        });
        setting(ui, t!("install.debian-name"), |ui| {
            ui.add(toggle(state.debian_name, |on| send(Msg::DebianName(on)))).id("debian-name");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn the_question_shows_the_command_and_confirming_runs_nothing() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("sudo pacman -S --needed libarchive"), "{}", h.screen());
        h.click_text("Install libarchive").advance(Duration::from_millis(200));
        let screen = h.screen();
        assert!(screen.contains("Install libarchive?"), "{screen}");
        assert!(screen.contains("pacman may ask for your password"), "{screen}");
        h.press("esc");
        assert_eq!(h.app().pages.install.answer, Some(Answer::Declined));
        h.click_text("Install libarchive").advance(Duration::from_millis(200));
        h.press("tab").press("enter");
        assert_eq!(h.app().pages.install.answer, Some(Answer::Agreed));
        assert!(h.handoffs().is_empty(), "the showcase never hands the terminal to a package manager");
    }

    #[test]
    fn the_machine_decides_the_command() {
        let mut h = showcase_on(PAGE);
        h.click_text("apt-get");
        assert!(h.screen().contains("sudo apt-get install libarchive-tools"), "{}", h.screen());
        h.send(send(Msg::DebianName(false))).send(send(Msg::Root(true)));
        assert!(h.screen().contains("apt-get install libarchive"), "{}", h.screen());
        assert!(!h.screen().contains("sudo"), "no sudo as root: {}", h.screen());
        h.click_text("none");
        assert!(h.screen().contains("No package manager"), "{}", h.screen());
    }
}
