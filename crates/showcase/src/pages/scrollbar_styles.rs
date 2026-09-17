//! Scrollbar styles: the four styles side by side, the theme key that chooses one, and the rule
//! that a scrollbar appears only when content overflows.

use qframe::prelude::*;
use qframe::widgets::{ScrollbarStyle, Segmented};

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "scrollbar-styles";

/// Recent deploys listed in every column.
const DEPLOYS: [&str; 12] = [
    "api-gateway",
    "billing",
    "search-index",
    "auth",
    "web-frontend",
    "mailer",
    "image-resizer",
    "reports",
    "webhooks",
    "scheduler",
    "audit-log",
    "metrics",
];

/// Rows shown when the playground asks for content that fits.
const FITTING_ROWS: usize = 4;

/// Shared selection and the playground.
#[derive(Debug, Default)]
pub struct State {
    selected: usize,
    fits: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Select(ScrollbarStyle, usize),
    Contents(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ScrollbarStyles(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Select(style, index) => {
            state.selected = index;
            log.push(PAGE, format!("List#{}", style.name()), format!("selected {}", DEPLOYS[index]));
        }
        Msg::Contents(index) => {
            state.fits = index == 1;
            state.selected = 0;
            log.push(PAGE, "Playground", format!("fits = {}", state.fits));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let rows = if state.fits { FITTING_ROWS } else { DEPLOYS.len() };
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("scrollbar-styles.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            for style in ScrollbarStyle::ALL {
                ui.column(|ui| {
                    ui.add(Text::new(style.name()).role("faint").no_wrap());
                    let items = DEPLOYS.iter().take(rows).map(|name| ListItem::new(*name));
                    // region: scrollbar-pinned
                    ui.add(
                        List::new(items)
                            .selected(Some(state.selected.min(rows - 1)))
                            .scrollbar(style)
                            .on_select(move |index| send(Msg::Select(style, index))),
                    )
                    .width(Length::Fill(1))
                    .height(Length::Cells(6))
                    .id(style.name());
                    // endregion
                })
                .gap(1)
                .width(Length::Fill(1));
            }
        })
        .gap(2)
        .fill_width();
    })
    .fill_width();

    let current = ui.env().theme().style("scrollbar", None, &[]).word("style").unwrap_or("block");
    ui.add_with(Panel::new().title(t!("scrollbar-styles.theme-title")).gap(0), |ui| {
        ui.add(Text::new(t!("scrollbar-styles.theme", style = current)).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: scrollbar-theme
        // In a theme file:
        //
        //   [style.scrollbar]
        //   style = "thin"
        //
        //   [style."scrollbar.thin"]
        //   thumb = "$dim"
        //
        // Widgets without a pinned style follow it.
        let items = DEPLOYS.iter().take(rows).map(|name| ListItem::new(*name));
        ui.add(List::new(items).selected(Some(state.selected.min(rows - 1))))
            .width(Length::Cells(30))
            .height(Length::Cells(6))
            .id("theme-list");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("scrollbar-styles.contents"), |ui| {
            let options = [t!("scrollbar-styles.overflows"), t!("scrollbar-styles.fits")];
            ui.add(Segmented::new(options).selected(usize::from(state.fits)).on_select(|i| send(Msg::Contents(i))))
                .id("contents");
        });
        ui.add(Text::new(t!("scrollbar-styles.fits-hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    /// The six list rows of the live demo, without the menu and the page's own scrollbar.
    fn list_rows(h: &qframe::runtime::Harness<crate::app::Showcase>) -> String {
        let (_, top) = h.find("api-gateway").expect("the lists are on screen");
        let screen = h.screen();
        let rows = screen.lines().skip(usize::try_from(top).unwrap_or(0)).take(6);
        rows.map(|line| line.chars().skip(31).take(104).collect::<String>() + "\n").collect()
    }

    /// Cells of the live demo in the thumb colour of the block style, which draws no glyph.
    fn block_thumb_cells(h: &qframe::runtime::Harness<crate::app::Showcase>) -> usize {
        let (_, top) = h.find("api-gateway").expect("the lists are on screen");
        let thumb = h.env().theme().color("muted");
        let top = u16::try_from(top).unwrap_or(0);
        (top..top + 6).flat_map(|y| (31..135).map(move |x| (x, y))).filter(|(x, y)| h.bg(*x, *y) == thumb).count()
    }

    #[test]
    fn styles_sit_side_by_side_in_order_and_hide_when_content_fits() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        let names: Vec<usize> = ScrollbarStyle::ALL.iter().filter_map(|style| screen.find(style.name())).collect();
        assert_eq!(names.len(), 4, "{screen}");
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]), "block, half, thin, dots:\n{screen}");
        assert!(!screen.contains("cell"), "the retired cell style is gone:\n{screen}");
        let rows = list_rows(&h);
        for glyph in ['▐', '▕', '•', '·'] {
            assert!(rows.contains(glyph), "{glyph} missing:\n{rows}");
        }
        assert!(!rows.contains('█'), "block draws colour, not glyphs:\n{rows}");
        assert!(block_thumb_cells(&h) > 0, "the block thumb is a coloured cell");
        assert!(screen.contains("chooses \"block\""), "block is the theme default:\n{screen}");
        h.click_text("billing");
        assert_eq!(h.app().pages.scrollbar_styles.selected, 1);
        h.send(send(Msg::Contents(1)));
        let rows = list_rows(&h);
        for glyph in ['▐', '▕', '•', '·'] {
            assert!(!rows.contains(glyph), "{glyph} drawn although the rows fit:\n{rows}");
        }
        assert_eq!(block_thumb_cells(&h), 0, "no block thumb when the rows fit");
    }
}
