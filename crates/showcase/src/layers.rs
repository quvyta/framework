//! The showcase-wide help layer (`?`) and command palette (`ctrl p`).

use qframe::prelude::*;
use qframe::widgets::{CommandPalette, HelpLayer, PaletteCommand};

use crate::app::Msg as AppMsg;
use crate::catalog::Catalog;
use crate::pages::FOUNDATIONS;

/// How many recent commands the palette lists.
const RECENT: usize = 5;

/// Which layer is open and the recently run commands.
#[derive(Debug, Default)]
pub struct State {
    help: bool,
    palette: bool,
    recent: Vec<String>,
}

/// Layer messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Help(bool),
    Palette(bool),
    Ran(String),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Layers(message)
}

/// Applies a layer message.
pub fn update(state: &mut State, message: Msg) {
    match message {
        Msg::Help(open) => state.help = open,
        Msg::Palette(open) => state.palette = open,
        Msg::Ran(id) => {
            state.recent.retain(|recent| *recent != id);
            state.recent.insert(0, id);
            state.recent.truncate(RECENT);
        }
    }
}

/// The open layer, if any, over the whole showcase.
pub fn view(state: &State, catalog: &Catalog, ui: &mut View<'_, AppMsg>) {
    if state.help {
        ui.add(HelpLayer::new(send(Msg::Help(false))).hint("↑↓", t!("hints.move")));
    }
    if state.palette {
        let pages = FOUNDATIONS.iter().map(|page| (*page).to_owned()).chain(
            catalog
                .items
                .iter()
                .filter(|item| item.done && item.page.as_deref() == Some(&item.id))
                .map(|item| item.id.clone()),
        );
        let mut commands: Vec<PaletteCommand<AppMsg>> = pages
            .map(|page| {
                let name = t!(&format!("names.{page}"));
                PaletteCommand::new(format!("page:{page}"), t!("command-palette.go", name = name), AppMsg::Open(page))
            })
            .collect();
        for (id, name) in ui.env().themes() {
            commands.push(PaletteCommand::new(
                format!("theme:{id}"),
                t!("command-palette.theme", name = name),
                AppMsg::Theme(id),
            ));
        }
        ui.add(
            CommandPalette::new(commands, send(Msg::Palette(false)))
                .keymap(true)
                .recent(state.recent.clone())
                .on_run(|id| send(Msg::Ran(id.to_owned()))),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::tests::showcase_on;

    #[test]
    fn question_mark_opens_help_and_ctrl_p_jumps_to_a_page() {
        let mut h = showcase_on("button");
        h.press("?").advance(Duration::from_millis(200));
        assert!(h.screen().contains("Keyboard shortcuts"), "{}", h.screen());
        assert!(h.screen().contains("search"), "showcase actions are listed");
        h.press("esc").press("ctrl+p").advance(Duration::from_millis(200));
        h.type_text("go modal").press("enter");
        assert_eq!(h.app().current(), "modal");
        h.press("ctrl+p").advance(Duration::from_millis(200));
        let screen = h.screen();
        assert!(screen.find("Recent") < screen.find("Go to Modal and dialog"), "{screen}");
    }
}
