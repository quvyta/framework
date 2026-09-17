//! Clipboard: copyable values with in-place confirmation, clean and raw copies of selected text,
//! pasting from the system clipboard and the clipboard events an application hears.

use qframe::prelude::*;
use qframe::widgets::{CopyValue, Markdown, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "clipboard";

/// Values a deploy dashboard hands out: `(label key, value, secret)`.
const VALUES: [(&str, &str, bool); 3] = [
    ("install", "cargo add quvyta-framework", false),
    ("image", "ghcr.io/quvyta/deploy-api:2026.9.1", false),
    ("token", "qv_live_8f3a61c2d94b7e05", true),
];

/// The note field, what the paste button read, and the playground.
#[derive(Debug)]
pub struct State {
    note: String,
    read: Option<Option<String>>,
    masked: bool,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { note: String::new(), read: None, masked: true, disabled: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Copied(&'static str),
    Note(String),
    ReadClipboard,
    Read(Option<String>),
    Masked(bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Clipboard(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Copied(name) => log.push(PAGE, format!("CopyValue#{name}"), "copied"),
        Msg::Note(text) => {
            log.push(PAGE, "TextInput#note", format!("changed, {} characters", text.chars().count()));
            state.note = text;
        }
        // region: read-clipboard
        Msg::ReadClipboard => return Command::read_clipboard(|text| send(Msg::Read(text))),
        Msg::Read(text) => {
            let length = text.as_ref().map_or(0, |t| t.chars().count());
            log.push(PAGE, "Command::read_clipboard", format!("{length} characters"));
            state.read = Some(text);
        }
        // endregion
        Msg::Masked(on) => {
            log.push(PAGE, "Playground", format!("masked = {on}"));
            state.masked = on;
        }
        Msg::Disabled(on) => {
            log.push(PAGE, "Playground", format!("disabled = {on}"));
            state.disabled = on;
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("clipboard.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        for (name, value, secret) in VALUES {
            ui.row(|ui| {
                ui.add(Text::new(t!(&format!("clipboard.{name}"))).role("faint").no_wrap()).width(Length::Cells(12));
                // region: copy-value
                ui.add(
                    CopyValue::new(value)
                        .masked(secret && state.masked)
                        .disabled(state.disabled)
                        .on_copy(send(Msg::Copied(name))),
                )
                .width(Length::Cells(46))
                .id(name);
                // endregion
            })
            .fill_width();
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("clipboard.paste")).gap(0), |ui| {
        ui.add(Text::new(t!("clipboard.paste-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: clean-and-raw
        // Markdown is selectable by itself. Select the heading, right click it and paste both
        // copies below: Copy leaves the pillar and the padding out, Raw copy keeps them.
        ui.add(Markdown::new(&t!("clipboard.notes"))).width(Length::Cells(48)).id("notes");
        // endregion
        ui.spacer().height(Length::Cells(1));
        ui.add(
            TextInput::new(&state.note)
                .placeholder(t!("clipboard.note-placeholder"))
                .on_change(|text| send(Msg::Note(text))),
        )
        .width(Length::Cells(48))
        .id("note");
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("clipboard.read")).on_press(send(Msg::ReadClipboard))).id("read");
            let (text, role) = match &state.read {
                None => (t!("clipboard.read-idle"), "faint"),
                Some(None) => (t!("clipboard.read-empty"), "faint"),
                Some(Some(text)) => (text.clone(), "body"),
            };
            ui.add(Text::new(text).role(role).no_wrap()).fill_width();
        })
        .gap(2)
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("clipboard.masked"), |ui| {
            ui.add(toggle(state.masked, |on| send(Msg::Masked(on)))).id("masked");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{click_text_below, right_click, showcase_on};

    #[test]
    fn copies_pastes_and_reads_back() {
        let mut h = showcase_on(PAGE);
        assert!(!h.screen().contains("qv_live"), "the token is masked:\n{}", h.screen());
        h.click_text("cargo add quvyta-framework");
        assert_eq!(h.clipboard(), Some("cargo add quvyta-framework"));
        assert!(h.screen().contains("copied"));
        h.click_text("Paste into the note field");
        h.press("ctrl+v");
        assert_eq!(h.app().pages.clipboard.note, "cargo add quvyta-framework");
        h.click_text("Read the clipboard");
        assert_eq!(h.app().pages.clipboard.read, Some(Some("cargo add quvyta-framework".to_owned())));
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "copied 26 characters"), "{log:?}");
    }

    #[test]
    fn the_system_clipboard_comes_first_and_both_copies_paste_back() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.set_system_clipboard(Some("from the browser"));
        h.click_text("Read the clipboard");
        assert_eq!(h.app().pages.clipboard.read, Some(Some("from the browser".to_owned())));
        h.set_system_clipboard(None);
        let (x, y) = h.find("Release 2026.9.1").expect("notes on screen");
        h.click(x, y).click(x, y).click(x, y);
        right_click(&mut h, x, y);
        click_text_below(&mut h, "Raw copy", y);
        let raw = format!("▌ Release 2026.9.1{}", " ".repeat(30));
        assert_eq!(h.clipboard(), Some(raw.as_str()), "the pillar, its gap and the padding");
        right_click(&mut h, x, y);
        click_text_below(&mut h, "Copy", y);
        assert_eq!(h.clipboard(), Some("Release 2026.9.1"), "the heading's text only");
        h.click_text("Paste into the note field").press("ctrl+v");
        assert_eq!(h.app().pages.clipboard.note, "Release 2026.9.1");
    }
}
