//! Command palette: container commands plus keymap actions, fuzzy filtered, with recent ones.

use qframe::prelude::*;
use qframe::widgets::{CommandPalette, PaletteCommand};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "command-palette";

/// Containers the generated commands act on.
const CONTAINERS: [&str; 8] = ["web", "worker", "scheduler", "postgres", "redis", "minio", "mailpit", "traefik"];

/// What each container command does, as a locale key suffix.
const VERBS: [&str; 4] = ["restart", "logs", "shell", "stop"];

/// The palette state and the playground.
#[derive(Debug, Default)]
pub struct State {
    open: bool,
    last: Option<String>,
    recent: Vec<String>,
    keymap: bool,
    remember: bool,
    firm: bool,
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Open,
    Close,
    Run(usize, usize),
    Ran(String),
    Keymap(bool),
    Remember(bool),
    Dismissable(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::CommandPalette(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Open => {
            state.open = true;
            log.push(PAGE, "CommandPalette", "open");
        }
        Msg::Close => {
            state.open = false;
            log.push(PAGE, "CommandPalette", "closed");
        }
        Msg::Dismissable(on) => {
            state.firm = !on;
            log.push(PAGE, "Playground", format!("dismissable = {on}"));
        }
        Msg::Run(verb, container) => {
            let done = format!("{} {}", VERBS[verb], CONTAINERS[container]);
            log.push(PAGE, "CommandPalette", format!("ran {done}"));
            state.last = Some(done);
        }
        Msg::Ran(id) => {
            // region: recent
            state.recent.retain(|recent| *recent != id);
            state.recent.insert(0, id);
            state.recent.truncate(3);
            // endregion
        }
        Msg::Keymap(on) => {
            state.keymap = on;
            log.push(PAGE, "Playground", format!("keymap = {on}"));
        }
        Msg::Remember(on) => {
            state.remember = on;
            log.push(PAGE, "Playground", format!("recent = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("command-palette.hint")).role("secondary"));
        ui.row(|ui| {
            ui.add(
                Button::new(t!("command-palette.open")).variant("primary").shortcut("ctrl p").on_press(send(Msg::Open)),
            )
            .id("open");
            let last = state.last.clone().unwrap_or_else(|| t!("command-palette.nothing"));
            ui.add(Text::new(last).role("faint").no_wrap());
        })
        .gap(2);
        if state.open {
            palette(state, ui);
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("command-palette.keymap"), |ui| {
            ui.add(toggle(state.keymap, |on| send(Msg::Keymap(on)))).id("keymap");
        });
        setting(ui, t!("command-palette.remember"), |ui| {
            ui.add(toggle(state.remember, |on| send(Msg::Remember(on)))).id("remember");
        });
        setting(ui, t!("command-palette.dismissable"), |ui| {
            ui.add(toggle(!state.firm, |on| send(Msg::Dismissable(on)))).id("dismissable");
        });
    })
    .fill_width();
}

fn palette(state: &State, ui: &mut View<'_, AppMsg>) {
    // region: palette
    let mut commands = Vec::new();
    for (container, name) in CONTAINERS.iter().enumerate() {
        for (verb, key) in VERBS.iter().enumerate() {
            let label = t!(&format!("command-palette.{key}"), name = *name);
            let mut command = PaletteCommand::new(format!("{key}-{name}"), label, send(Msg::Run(verb, container)));
            if container == 0 && verb == 0 {
                command = command.chord("ctrl r");
            }
            commands.push(command);
        }
    }
    let mut palette = CommandPalette::new(commands, send(Msg::Close)).keymap(state.keymap).dismissable(!state.firm);
    if state.remember {
        palette = palette.recent(state.recent.clone()).on_run(|id| send(Msg::Ran(id.to_owned())));
    }
    ui.add(palette);
    // endregion
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn finds_and_runs_a_command_and_remembers_it() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Remember(true)));
        h.click_text("Open the palette").advance(Duration::from_millis(200));
        h.type_text("logs redis").press("enter");
        assert_eq!(h.app().pages.command_palette.last.as_deref(), Some("logs redis"));
        assert_eq!(h.app().pages.command_palette.recent, ["logs-redis"]);
        h.click_text("Open the palette").advance(Duration::from_millis(200));
        let screen = h.screen();
        assert!(screen.find("Recent") < screen.find("Show logs of redis"), "{screen}");
        h.press("esc");
        assert!(!h.app().pages.command_palette.open);
    }

    #[test]
    fn the_mouse_points_clicks_and_closes_the_palette() {
        let mut h = showcase_on(PAGE);
        h.click_text("Open the palette").advance(Duration::from_millis(200));
        let (x, y) = h.find("Show logs of web").expect("second command");
        h.hover(x + 2, y).press("down").press("enter");
        assert_eq!(h.app().pages.command_palette.last.as_deref(), Some("shell web"), "keys went on from the pointer");
        h.click_text("Open the palette").advance(Duration::from_millis(200));
        h.click_text("Stop web");
        assert_eq!(h.app().pages.command_palette.last.as_deref(), Some("stop web"));
        h.click_text("Open the palette").advance(Duration::from_millis(200));
        let (x, y) = crate::tests::close_mark_beside(&h, "Type a command");
        h.click(x.expect("a close mark on the filter row"), y);
        assert!(!h.app().pages.command_palette.open);
        assert!(h.screen().contains("CommandPalette") && h.screen().contains("closed"), "{}", h.screen());
    }
}
