//! Splitter: an editor layout of a file list, a source view and a build log, with the
//! boundaries dragged or moved from the keyboard.

use qframe::prelude::*;
use qframe::widgets::{Segmented, Splitter};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "splitter";

/// Files in the left pane.
const FILES: [&str; 6] = ["main.rs", "app.rs", "config.rs", "deploy.rs", "Cargo.toml", "README.md"];

/// Lines of the build log.
const BUILD: [&str; 4] = [
    "Compiling quvyta-framework v0.1.0",
    "Compiling showcase v0.1.0",
    "Finished dev profile in 4.21s",
    "Running target/debug/showcase",
];

/// Limits of the files pane the playground offers, in columns: none above the minimum, then two
/// ranges.
const LIMITS: [(u16, Option<u16>); 3] = [(4, None), (16, Some(40)), (24, Some(48))];

/// Stacked, the files pane counts rows at a third of its columns.
const ROWS_PER_COLUMN: u16 = 3;

/// Pane sizes (files width, source height) and the playground.
#[derive(Debug)]
pub struct State {
    files: u16,
    log: u16,
    selected: usize,
    resizable: bool,
    stacked: bool,
    /// Index into [`LIMITS`].
    limits: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { files: 24, log: 9, selected: 1, resizable: true, stacked: false, limits: 0 }
    }
}

/// How a limit choice reads: the range, or the word for no upper limit.
fn limits_name(index: usize) -> String {
    match LIMITS[index] {
        (_, None) => t!("splitter.unlimited"),
        (min, Some(max)) => format!("{min}–{max}"),
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Files(u16),
    Log(u16),
    Select(usize),
    Resizable(bool),
    Stacked(usize),
    Limits(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Splitter(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: splitter-update
        Msg::Files(width) => {
            state.files = width;
            log.push(PAGE, "Splitter#files", format!("resized to {width}"));
        }
        Msg::Log(height) => {
            state.log = height;
            log.push(PAGE, "Splitter#log", format!("source resized to {height}"));
        }
        // endregion
        Msg::Select(index) => {
            state.selected = index;
            log.push(PAGE, "List#files", format!("selected {}", FILES[index]));
        }
        Msg::Resizable(on) => {
            state.resizable = on;
            log.push(PAGE, "Playground", format!("resizable = {on}"));
        }
        Msg::Stacked(index) => {
            state.stacked = index == 1;
            log.push(PAGE, "Playground", format!("stacked = {}", state.stacked));
        }
        Msg::Limits(index) => {
            state.limits = index;
            // The pane never shows a size outside its limits; keep the state honest too.
            let (min, max) = LIMITS[index];
            state.files = max.map_or(state.files.max(min), |max| state.files.clamp(min, max));
            log.push(PAGE, "Playground", format!("limits = {}, files = {}", limits_name(index), state.files));
        }
    }
    Command::none()
}

/// The source pane.
fn source(ui: &mut View<'_, AppMsg>, file: &str) {
    ui.add_with(Panel::new().variant("inset"), |ui| {
        ui.add(Text::new(file).role("title").no_wrap());
        ui.add(Text::new(t!("splitter.source")).role("secondary"));
    })
    .fill();
}

/// The build log pane.
fn build_log(ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().variant("inset").gap(0), |ui| {
        for line in BUILD {
            ui.add(Text::new(line).role("faint").no_wrap());
        }
    })
    .fill();
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("splitter.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let file = FILES[state.selected];
        ui.column(|ui| {
            // region: splitter
            // Stacked, the same setting counts rows at a third of the columns.
            let scale = if state.stacked { ROWS_PER_COLUMN } else { 1 };
            let (min, max) = LIMITS[state.limits];
            let files =
                if state.stacked { Splitter::rows(state.files / scale) } else { Splitter::columns(state.files) };
            // `max` is a number of cells, or `None` for no upper limit.
            let mut files = files.limits(min / scale, max.map(|max| max / scale));
            if state.resizable {
                files = files.on_resize(move |size| send(Msg::Files(size * scale)));
            }
            files
                .first(|ui| {
                    let items = FILES.iter().map(|name| ListItem::new(*name).icon("file", None));
                    ui.add_with(Panel::new().variant("inset"), |ui| {
                        let list = List::new(items).selected(Some(state.selected)).on_select(|i| send(Msg::Select(i)));
                        ui.add(list).fill().id("files");
                    })
                    .fill();
                })
                .second(|ui| {
                    // Splitters nest: the source on top, the build log below.
                    let mut editor = Splitter::rows(state.log).limits(3, 14);
                    if state.resizable {
                        editor = editor.on_resize(|size| send(Msg::Log(size)));
                    }
                    editor.first(|ui| source(ui, file)).second(build_log).show(ui).id("log");
                })
                .show(ui)
                .id("files-split");
            // endregion
        })
        .height(Length::Cells(18))
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("splitter.resizable"), |ui| {
            ui.add(toggle(state.resizable, |on| send(Msg::Resizable(on)))).id("resizable");
        });
        setting(ui, t!("splitter.direction"), |ui| {
            let options = [t!("splitter.columns"), t!("splitter.rows")];
            ui.add(Segmented::new(options).selected(usize::from(state.stacked)).on_select(|i| send(Msg::Stacked(i))))
                .id("direction");
        });
        setting(ui, t!("splitter.limits"), |ui| {
            let names: Vec<String> = (0..LIMITS.len()).map(limits_name).collect();
            ui.add(Segmented::new(names).selected(state.limits).on_select(|i| send(Msg::Limits(i)))).id("limits");
        });
        ui.add(Text::new(t!("splitter.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::event::{MouseButton, MouseKind};
    use qframe::runtime::Harness;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn limit_choices_bound_the_files_pane_and_are_logged() {
        let mut h = showcase_on(PAGE);
        assert_eq!(h.app().pages.splitter.limits, 0, "no upper limit unless the app sets one");
        let (x, y) = h.find("main.rs").expect("files pane");
        let drag = |h: &mut Harness<crate::app::Showcase>, from: i32, to: i32| {
            h.mouse(MouseKind::Down(MouseButton::Left), from, y);
            h.mouse(MouseKind::Drag(MouseButton::Left), to, y);
            h.mouse(MouseKind::Up(MouseButton::Left), to, y);
        };
        // The name starts after the panel padding, the pillar gap and the icon.
        let left = x - 6;
        drag(&mut h, left + 24, left + 60);
        assert_eq!(h.app().pages.splitter.files, 60, "unlimited: the pane grows past every preset");

        h.click_text("16–40");
        assert_eq!((h.app().pages.splitter.limits, h.app().pages.splitter.files), (1, 40));
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "limits = 16–40, files = 40");
        drag(&mut h, left + 40, left + 2);
        assert_eq!(h.app().pages.splitter.files, 16, "held at the lower limit");

        h.click_text("24–48");
        assert_eq!(h.app().pages.splitter.files, 24);
        drag(&mut h, left + 24, left + 70);
        assert_eq!(h.app().pages.splitter.files, 48, "held at the upper limit");

        h.click_text("Unlimited");
        assert_eq!(h.app().pages.splitter.files, 48, "lifting the limit keeps the size");
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "limits = Unlimited, files = 48");
    }

    #[test]
    fn dragging_the_invisible_boundary_resizes_the_files_pane() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("main.rs").expect("files pane");
        // The name starts after the panel padding, the pillar gap and the icon.
        let edge = x - 6 + 24;
        h.hover(edge, y);
        h.mouse(MouseKind::Down(MouseButton::Left), edge, y);
        h.mouse(MouseKind::Drag(MouseButton::Left), edge + 6, y);
        h.mouse(MouseKind::Up(MouseButton::Left), edge + 6, y);
        assert_eq!(h.app().pages.splitter.files, 30);
        h.send(send(Msg::Resizable(false)));
        h.send(send(Msg::Stacked(1)));
        assert!(h.screen().contains("Sizes live in the application"), "{}", h.screen());
    }
}
