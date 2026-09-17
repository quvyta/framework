//! Text: typography roles, colour tokens, spans, wrapping to the space it gets or a fixed
//! width, alignment, and opting in to mouse selection.

use qframe::prelude::*;
use qframe::widgets::Select;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "text";

/// Fixed widths the playground offers for the paragraph, after "auto".
const WIDTHS: [u16; 4] = [24, 40, 60, 80];

/// Playground settings.
#[derive(Debug)]
pub struct State {
    /// `None` is auto: the paragraph takes the width it is given.
    width: Option<usize>,
    align: usize,
    wrap: bool,
    selectable: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { width: None, align: 0, wrap: true, selectable: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    /// A fixed width by index into [`WIDTHS`], or auto.
    Width(Option<usize>),
    Align(usize),
    Wrap(bool),
    Selectable(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Text(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let entry = match message {
        Msg::Width(index) => {
            state.width = index;
            index.map_or_else(|| "width = auto".to_owned(), |i| format!("width = {}", WIDTHS[i]))
        }
        Msg::Align(index) => {
            state.align = index;
            format!("align = {index}")
        }
        Msg::Wrap(on) => {
            state.wrap = on;
            format!("wrap = {on}")
        }
        Msg::Selectable(on) => {
            state.selectable = on;
            format!("selectable = {on}")
        }
    };
    log.push(PAGE, "Playground", entry);
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("text.roles")).gap(0), |ui| {
        // region: roles
        ui.add(Text::new(t!("text.title")).role("title"));
        ui.add(Text::new(t!("text.body")).role("body"));
        ui.add(Text::new(t!("text.secondary")).role("secondary"));
        ui.add(Text::new(t!("text.faint")).role("faint"));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("text.spans")), |ui| {
        // region: spans
        ui.add(Text::rich([
            Span::new(t!("text.build")).role("secondary"),
            Span::new(t!("text.passed")).color("success").bold(),
            Span::new("  ·  ").role("faint"),
            Span::new(t!("text.warnings")).color("warning"),
            Span::new("  ·  ").role("faint"),
            Span::new(t!("text.marked")).on("raised").color("accent"),
        ]));
        // endregion
    })
    .fill_width();

    let align = [Align::Start, Align::Center, Align::End][state.align];
    ui.add_with(Panel::new().title(t!("text.paragraph")), |ui| {
        // region: wrapping
        let mut paragraph = Text::new(t!("text.long")).role("secondary").align(align);
        if !state.wrap {
            paragraph = paragraph.no_wrap();
        }
        // Auto takes the whole width the panel gives and wraps again when the terminal resizes.
        let width = state.width.map_or(Length::Fill(1), |index| Length::Cells(WIDTHS[index]));
        ui.add(paragraph).width(width).selectable(state.selectable).id("paragraph");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("text.width"), |ui| {
            let names = std::iter::once(t!("text.auto")).chain(WIDTHS.map(|w| format!("{w}")));
            ui.add(
                Select::new(names)
                    .selected(Some(state.width.map_or(0, |index| index + 1)))
                    .on_select(|i| send(Msg::Width(i.checked_sub(1)))),
            )
            .width(Length::Cells(16))
            .id("width");
        });
        setting(ui, t!("layout.justify"), |ui| {
            let names = ["start", "center", "end"].map(|a| t!(&format!("layout.align.{a}")));
            ui.add(Select::new(names).selected(Some(state.align)).on_select(|i| send(Msg::Align(i))))
                .width(Length::Cells(16))
                .id("align");
        });
        setting(ui, t!("text.wrap"), |ui| {
            ui.add(toggle(state.wrap, |on| send(Msg::Wrap(on)))).id("wrap");
        });
        setting(ui, t!("text.selectable"), |ui| {
            ui.add(toggle(state.selectable, |on| send(Msg::Selectable(on)))).id("selectable");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use qframe::icons::GlyphMode;
    use qframe::runtime::Harness;

    use super::*;
    use crate::app::{Msg as ShowcaseMsg, Showcase};
    use crate::tests::{env, showcase_on};

    #[test]
    fn wrapping_can_be_turned_off() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("passed"));
        h.send(send(Msg::Wrap(false)));
        assert!(h.screen().contains('…'));
    }

    /// The paragraph's rows at a terminal `width` cells wide.
    fn paragraph_rows(width: u16, message: Option<Msg>) -> Vec<String> {
        let mut h = Harness::with_env(Showcase::new(), env(), width, 44);
        h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
        h.send(ShowcaseMsg::Open(PAGE.to_owned()));
        if let Some(message) = message {
            h.send(send(message));
        }
        h.advance(Duration::from_secs(1));
        let (x, start) = h.find("Terminal interfaces").expect("paragraph on screen");
        let (_, end) = h.find("ellipsis").expect("paragraph end on screen");
        let screen = h.screen();
        // The paragraph's column, without the menu on its left and the scrollbar on its right.
        let column = |line: &str| {
            let text: String = line.chars().skip(usize::try_from(x).unwrap_or(0)).collect();
            text.trim_end_matches(['▐', '▕']).trim().to_owned()
        };
        let rows = usize::try_from(start).unwrap_or(0)..=usize::try_from(end).unwrap_or(0);
        screen.lines().enumerate().filter(|(row, _)| rows.contains(row)).map(|(_, line)| column(line)).collect()
    }

    #[test]
    fn auto_width_is_the_default_and_wraps_to_the_space_it_gets() {
        let wide = paragraph_rows(160, None);
        let narrow = paragraph_rows(110, None);
        assert!(wide.len() < narrow.len(), "a wider terminal takes fewer rows: {wide:?} {narrow:?}");
        assert!(wide[0].chars().count() > 60, "auto uses the panel's whole width: {wide:?}");
        let fixed = paragraph_rows(160, Some(Msg::Width(Some(0))));
        assert!(fixed.iter().all(|row| row.chars().count() <= 24), "a fixed width stays: {fixed:?}");
        assert_eq!(fixed, paragraph_rows(110, Some(Msg::Width(Some(0)))), "and ignores the terminal");
    }

    #[test]
    fn text_is_selectable_only_when_asked() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("Terminal").expect("paragraph on screen");
        h.drag((x, y), (x + 7, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), None);
        h.send(send(Msg::Selectable(true)));
        h.drag((x, y), (x + 7, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), Some("Terminal"));
    }
}
