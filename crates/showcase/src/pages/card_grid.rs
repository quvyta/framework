//! Card grid: an app store's cards in columns that follow the width, checks, a very long grid and
//! an empty one.

use qframe::prelude::*;
use qframe::widgets::{CardGrid, EmptyState, Select, Span};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "card-grid";

/// The first word of every sample app's name; the kind's word follows it.
const PREFIXES: [&str; 20] = [
    "Ember", "Tide", "Juniper", "Quill", "Harbor", "Lumen", "Maple", "Orbit", "Pebble", "Rook", "Sable", "Thistle",
    "Vale", "Willow", "Zephyr", "Aster", "Birch", "Cobalt", "Drift", "Fern",
];

/// Kinds of sample apps; each has a name word and a one-line summary in the locale files.
const KINDS: [&str; 15] = [
    "notes", "player", "paint", "mail", "browser", "terminal", "photos", "chat", "office", "maps", "games", "backup",
    "fonts", "monitor", "editor",
];

/// Where a sample app can be installed from, as locale keys.
const SOURCES: [&[&str]; 5] = [&["repo"], &["repo", "flatpak"], &["aur"], &["flatpak"], &["repo", "flatpak", "snap"]];

/// How many cards each choice of contents shows.
const COUNTS: [usize; 3] = [300, 10_000, 0];

/// Selection, checks and playground settings.
#[derive(Debug)]
pub struct State {
    selected: Option<usize>,
    checked: Vec<bool>,
    checks: bool,
    contents: usize,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { selected: None, checked: vec![false; COUNTS[0]], checks: false, contents: 0, disabled: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Select(usize),
    Open(usize),
    Toggle(usize),
    Checks(bool),
    Contents(usize),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::CardGrid(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Select(index) => {
            state.selected = Some(index);
            log.push(PAGE, "CardGrid#apps", format!("selected {index}"));
        }
        Msg::Open(index) => log.push(PAGE, "CardGrid#apps", format!("activated {index}")),
        // region: card-toggle
        Msg::Toggle(index) => {
            if let Some(checked) = state.checked.get_mut(index) {
                *checked = !*checked;
            }
            log.push(PAGE, "CardGrid#apps", format!("toggled {index}"));
        }
        // endregion
        Msg::Checks(on) => {
            state.checks = on;
            log.push(PAGE, "Playground", format!("checks = {on}"));
        }
        Msg::Contents(index) => {
            state.contents = index;
            state.selected = None;
            state.checked = vec![false; COUNTS[index]];
            log.push(PAGE, "Playground", format!("contents = {index}"));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
    }
    Command::none()
}

// region: card-content
/// What card `index` shows: an icon and a name, a one-line summary, and where it comes from.
fn card(ui: &mut View<'_, AppMsg>, index: usize) {
    let kind = KINDS[(index / PREFIXES.len()) % KINDS.len()];
    let mut name = format!("{} {}", PREFIXES[index % PREFIXES.len()], t!(&format!("card-grid.kind.{kind}.name")));
    if index >= PREFIXES.len() * KINDS.len() {
        name = format!("{name} {}", index / (PREFIXES.len() * KINDS.len()) + 1);
    }
    let icon = ui.env().icons().glyph("project").into_owned();
    ui.add(Text::rich([Span::new(format!("{icon} ")).color("accent"), Span::new(name).bold()]).no_wrap());
    ui.add(Text::new(t!(&format!("card-grid.kind.{kind}.summary"))).role("secondary").no_wrap());
    let sources = SOURCES[index % SOURCES.len()].iter().map(|source| t!(&format!("card-grid.source.{source}")));
    let mut spans = vec![Span::new(sources.collect::<Vec<_>>().join(" · ")).role("faint")];
    if index.is_multiple_of(9) {
        let check = ui.env().icons().glyph("check").into_owned();
        spans.push(Span::new(format!(" · {} {check}", t!("card-grid.installed"))).color("success"));
    }
    ui.add(Text::rich(spans).no_wrap());
}
// endregion

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("card-grid.hint")).role("secondary"));
        // region: card-grid
        let mut grid = CardGrid::new(COUNTS[state.contents])
            .card_width(24, 32)
            .card_height(3)
            .gap(2, 1)
            .selected(state.selected)
            .disabled(state.disabled)
            .on_select(|index| send(Msg::Select(index)))
            .on_activate(|index| send(Msg::Open(index)))
            .empty(
                EmptyState::new(t!("card-grid.empty"))
                    .message(t!("card-grid.empty-text"))
                    .action(Button::new(t!("card-grid.show-all")).on_press(send(Msg::Contents(0)))),
            )
            .card(card);
        if state.checks {
            grid = grid.checked(state.checked.clone()).on_toggle(|index| send(Msg::Toggle(index)));
        }
        ui.add(grid).width(Length::Fill(1)).height(Length::Cells(15)).id("apps");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("card-grid.contents"), |ui| {
            let names = [t!("card-grid.apps"), t!("card-grid.many"), t!("card-grid.nothing")];
            ui.add(Select::new(names).selected(Some(state.contents)).on_select(|i| send(Msg::Contents(i))))
                .width(Length::Cells(22))
                .id("contents");
        });
        setting(ui, t!("card-grid.checks"), |ui| {
            ui.add(toggle(state.checks, |on| send(Msg::Checks(on)))).id("checks");
        });
        setting(ui, t!("card-grid.disabled"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("card-grid.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn cards_are_chosen_opened_and_checked() {
        let mut h = showcase_on(PAGE);
        h.click_text("Tide Notes");
        assert_eq!(h.app().pages.card_grid.selected, Some(1));
        assert!(h.screen().contains("activated 1"), "{}", h.screen());
        h.press("right");
        assert_eq!(h.app().pages.card_grid.selected, Some(2));
        h.send(send(Msg::Checks(true)));
        h.press("space");
        assert!(h.app().pages.card_grid.checked[2]);
        assert!(h.screen().contains('✓'), "the checked card carries its mark");
    }

    #[test]
    fn the_long_grid_and_the_empty_one() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Contents(1)));
        h.click_text("Ember Notes");
        h.press("end");
        assert_eq!(h.app().pages.card_grid.selected, Some(9_999));
        assert!(h.screen().contains("Fern Browser 34"), "{}", h.screen());
        h.send(send(Msg::Contents(2)));
        assert!(h.screen().contains("No apps here"), "{}", h.screen());
        h.click_text("Show all apps");
        assert_eq!(h.app().pages.card_grid.contents, 0);
        h.set_locale("tr");
        assert!(h.screen().contains("Düzenleyici"), "the cards follow the language:\n{}", h.screen());
    }
}
