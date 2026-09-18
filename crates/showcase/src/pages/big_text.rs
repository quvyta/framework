//! Big text: a clock, a counter and a title drawn from block elements.

use qframe::prelude::*;
use qframe::widgets::{BigText, Gradient, Segmented, Select};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "big-text";

/// Words the playground can show.
const WORDS: [&str; 4] = ["14:32", "99.9%", "DEPLOYED", "QUVYTA"];

/// The deploy counter and the playground settings.
#[derive(Debug, Default)]
pub struct State {
    deploys: u32,
    word: usize,
    accent: bool,
    gradient: bool,
    /// 0 blends across the columns, 1 down the rows.
    direction: usize,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Deploy,
    Word(usize),
    Accent(bool),
    Gradient(bool),
    Direction(usize),
}

/// The direction the playground blends in.
fn direction(index: usize) -> Gradient {
    if index == 1 { Gradient::Rows } else { Gradient::Columns }
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::BigText(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Deploy => {
            state.deploys += 1;
            log.push(PAGE, "Button#deploy", format!("deploys = {}", state.deploys));
        }
        Msg::Word(index) => {
            state.word = index;
            log.push(PAGE, "Playground", format!("text = {}", WORDS[index]));
        }
        Msg::Accent(on) => {
            state.accent = on;
            log.push(PAGE, "Playground", format!("accent = {on}"));
        }
        Msg::Gradient(on) => {
            state.gradient = on;
            log.push(PAGE, "Playground", format!("gradient = {on}"));
        }
        Msg::Direction(index) => {
            state.direction = index;
            log.push(PAGE, "Playground", format!("direction = {:?}", direction(index)));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("big-text.figures")).gap(0), |ui| {
        ui.add(Text::new(t!("big-text.figures-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.column(|ui| {
                ui.add(Text::new(t!("big-text.uptime")).role("faint"));
                // region: counter
                ui.add(BigText::new("99.98%").variant("accent"));
                // endregion
            });
            ui.column(|ui| {
                ui.add(Text::new(t!("big-text.deploys")).role("faint"));
                // region: live-counter
                ui.add(BigText::new(state.deploys.to_string()));
                // endregion
            });
        })
        .gap(6);
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("big-text.logo")).role("faint"));
        // region: logo
        ui.add(BigText::new("QUVYTA").variant("accent").gradient("info", Gradient::Columns));
        // endregion
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("big-text.deploy")).on_press(send(Msg::Deploy))).id("deploy");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let mut big = BigText::new(WORDS[state.word]);
        if state.accent {
            big = big.variant("accent");
        }
        if state.gradient {
            big = big.gradient("info", direction(state.direction));
        }
        ui.add(big).id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("big-text.text"), |ui| {
            ui.add(Select::new(WORDS).selected(Some(state.word)).on_select(|i| send(Msg::Word(i))))
                .width(Length::Cells(16))
                .id("word");
        });
        setting(ui, t!("big-text.accent"), |ui| {
            ui.add(toggle(state.accent, |on| send(Msg::Accent(on)))).id("accent");
        });
        setting(ui, t!("big-text.gradient"), |ui| {
            ui.add(toggle(state.gradient, |on| send(Msg::Gradient(on)))).id("gradient");
        });
        setting(ui, t!("big-text.direction"), |ui| {
            let labels = [t!("big-text.columns"), t!("big-text.rows")];
            ui.add(Segmented::new(labels).selected(state.direction).on_select(|i| send(Msg::Direction(i))))
                .id("direction");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::color::Rgb;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn counter_grows_in_big_digits() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains('▀'), "{}", h.screen());
        h.click_text("Record a deploy");
        h.click_text("Record a deploy");
        assert_eq!(h.app().pages.big_text.deploys, 2);
        h.send(send(Msg::Word(2)));
        assert_eq!(h.app().pages.big_text.word, 2);
    }

    #[test]
    fn the_logo_blends_between_two_theme_colours() {
        let h = showcase_on(PAGE);
        let (x, y) = h.find("Logo, accent").expect("the logo caption is on screen");
        let (x, y) = (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0) + 1);
        let theme = h.env().theme();
        // "QUVYTA" is twenty-three columns wide: six letters of three and five gaps.
        assert_eq!(h.fg(x, y), theme.color("accent"), "the first column is the accent:\n{}", h.screen());
        assert_eq!(h.fg(x + 22, y), theme.color("info"), "the last one the info colour");
        assert_eq!(h.fg(x, y + 1), h.fg(x, y), "a column blends across, not down");
    }

    /// What the playground's big text is drawn in: the colour of its leftmost and rightmost lit
    /// cell, the row it starts on and that leftmost column.
    fn ends(h: &Harness<crate::app::Showcase>) -> (Option<Rgb>, Option<Rgb>, u16, u16) {
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let title = lines.iter().position(|line| line.contains("PLAYGROUND")).expect("the playground panel");
        let block = |c: char| "▄▀█".contains(c);
        let (row, line) = lines
            .iter()
            .enumerate()
            .skip(title + 1)
            .find(|(_, line)| line.chars().any(block))
            .expect("a row of block elements under the playground title");
        let cells: Vec<char> = line.chars().collect();
        let column = |index: usize| u16::try_from(index).unwrap_or(0);
        let first = column(cells.iter().position(|c| block(*c)).unwrap_or(0));
        let last = column(cells.iter().rposition(|c| block(*c)).unwrap_or(0));
        let row = u16::try_from(row).unwrap_or(0);
        (h.fg(first, row), h.fg(last, row), row, first)
    }

    #[test]
    fn the_playground_turns_the_gradient_on_and_turns_it_around() {
        let mut h = showcase_on(PAGE);
        let (left, right, row, column) = ends(&h);
        assert_eq!(left, right, "the default is one flat colour");

        h.send(send(Msg::Gradient(true)));
        assert!(h.app().pages.big_text.gradient);
        let (left, right, columns_row, _) = ends(&h);
        assert_ne!(left, right, "the blend runs across the columns");
        assert_eq!(columns_row, row, "and nothing moved");

        h.send(send(Msg::Direction(1)));
        assert_eq!(h.app().pages.big_text.direction, 1);
        let (left, right, rows_row, first) = ends(&h);
        assert_eq!(left, right, "down the rows, a row is one tone");
        assert_ne!(h.fg(first, rows_row), h.fg(first, rows_row + 1), "and the rows differ");
        assert_eq!((rows_row, first), (row, column), "still nothing moved");

        h.send(send(Msg::Gradient(false)));
        let (left, right, ..) = ends(&h);
        assert_eq!(left, right, "turning it off gives the flat colour back");
    }
}
