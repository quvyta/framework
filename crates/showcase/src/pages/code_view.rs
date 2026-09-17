//! Code view: highlighted Rust and TOML, line numbers, copying with `c`, and selecting code with
//! the mouse, whose clean copy leaves the line numbers out.

use qframe::prelude::*;
use qframe::widgets::{CodeView, Language};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "code-view";

/// A Rust sample.
const RUST: &str = r#"#[derive(Debug, Clone)]
enum Msg {
    Save,
}

fn update(app: &mut App, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::Save => Command::perform(|| store::save("notes.md")),
    }
}"#;

/// A TOML sample.
const TOML: &str = r##"[meta]
name = "Aurora"
extends = "monochrome"

[colors]
accent = "#7DD3FC"   # sky

[style."button:focus"]
bg = "pulse($accent, $accent-2)""##;

/// Playground settings.
#[derive(Debug)]
pub struct State {
    numbers: bool,
    selectable: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { numbers: true, selectable: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Numbers(bool),
    Selectable(bool),
    Copied(&'static str),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::CodeView(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Numbers(on) => {
            state.numbers = on;
            log.push(PAGE, "Playground", format!("line_numbers = {on}"));
        }
        Msg::Selectable(on) => {
            state.selectable = on;
            log.push(PAGE, "Playground", format!("selectable = {on}"));
        }
        Msg::Copied(id) => log.push(PAGE, format!("CodeView#{id}"), "copied"),
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("code-view.hint")).role("secondary"));
        // region: rust
        // A code view is selectable by itself; `.selectable(false)` turns that off.
        ui.add(CodeView::new(RUST, Language::Rust).line_numbers(state.numbers).on_copy(send(Msg::Copied("rust"))))
            .fill_width()
            .selectable(state.selectable)
            .id("rust");
        // endregion
        ui.add(CodeView::new(TOML, Language::Toml).line_numbers(state.numbers).on_copy(send(Msg::Copied("toml"))))
            .fill_width()
            .selectable(state.selectable)
            .id("toml");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("code-view.numbers"), |ui| {
            ui.add(toggle(state.numbers, |on| send(Msg::Numbers(on)))).id("numbers");
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("text.selectable"), |ui| {
            ui.add(toggle(state.selectable, |on| send(Msg::Selectable(on)))).id("selectable");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn selecting_code_copies_it_without_line_numbers_unless_raw() {
        use qframe::event::{MouseButton, MouseKind};

        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("enum Msg {").expect("code on screen");
        let (gutter, _) = h.find("2  enum").expect("line number on screen");
        h.drag((gutter, y), (x + 9, y + 1));
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        crate::tests::click_text_below(&mut h, "Copy", y);
        assert_eq!(h.clipboard(), Some("enum Msg {\n    Save,"), "no line numbers");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        crate::tests::click_text_below(&mut h, "Raw copy", y);
        let raw = h.clipboard().unwrap_or_default().to_owned();
        assert!(raw.starts_with("2  enum Msg {") && raw.contains("3      Save,"), "{raw:?}");
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "copied 20 characters"), "{log:?}");
        h.send(send(Msg::Selectable(false)));
        h.drag((x, y), (x + 3, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), Some(raw.as_str()), "selection turned off");
    }

    #[test]
    fn copies_and_hides_numbers() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("Aurora"));
        h.send(send(Msg::Numbers(false)));
        assert!(!h.app().pages.code_view.numbers);
    }
}
