//! Tabs with options: closing, widths, overflow, reordering and a right-click menu, each switched on
//! on its own.

use qframe::prelude::*;
use qframe::widgets::{Overflow, Segmented, TabEdit, TabWidth};

use super::tab_menu::{self, OpenTab, TabAction};
use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "tabs-advanced";

/// Files open in the demo editor: path and line count.
const FILES: [(&str, u32); 9] = [
    ("README.md", 64),
    ("main.rs", 38),
    ("app.rs", 412),
    ("deploy.rs", 187),
    ("containers.rs", 256),
    ("Cargo.toml", 41),
    ("theme.toml", 690),
    ("en.toml", 318),
    ("docker-compose.yml", 72),
];

/// Width choices of the playground.
const WIDTHS: [TabWidth; 3] = [TabWidth::Fit, TabWidth::Fixed(16), TabWidth::Fill];

/// Overflow choices of the playground.
const OVERFLOWS: [Overflow; 2] = [Overflow::Arrows, Overflow::Menu];

/// Open files and the options the playground turned on.
#[derive(Debug)]
pub struct State {
    files: Vec<OpenTab>,
    active: usize,
    closable: bool,
    width: usize,
    overflow: usize,
    reorderable: bool,
    menu: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            files: OpenTab::all(FILES.len()),
            active: 2,
            closable: false,
            width: 0,
            overflow: 0,
            reorderable: false,
            menu: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Open(usize),
    Edit(TabEdit),
    Menu(TabAction),
    Reopen,
    Closable(bool),
    Pinned(bool),
    Width(usize),
    Overflow(usize),
    Reorderable(bool),
    ContextMenu(bool),
    /// A dragged tab scrolled the strip; the first position now in view.
    DragScroll(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::TabsAdvanced(message))
}

/// The name of the open file at tab `index`.
fn name(state: &State, index: usize) -> &'static str {
    state.files.get(index).map_or("", |tab| FILES[tab.item].0)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Open(index) => {
            state.active = index;
            log.push(PAGE, "Tabs#files", format!("selected {}", name(state, index)));
        }
        // region: tabs-advanced-edit
        Msg::Edit(edit) => {
            let entry = match edit {
                TabEdit::Close(index) => format!("close {}", name(state, index)),
                TabEdit::Move { from, to } => format!("move {} from {from} to {to}", name(state, from)),
            };
            edit.apply(&mut state.files, &mut state.active);
            log.push(PAGE, "Tabs#files", entry);
        }
        // endregion
        Msg::Menu(action) => {
            let entry = action.describe(&state.files, name(state, action.index()));
            tab_menu::apply(action, &mut state.files, &mut state.active);
            log.push(PAGE, "Tabs#files", entry);
        }
        Msg::Reopen => {
            let active = state.files.get(state.active).map_or(0, |tab| tab.item);
            state.files = OpenTab::all(FILES.len());
            state.active = active;
            log.push(PAGE, "Button#reopen", "reopened every file");
        }
        Msg::Closable(on) => {
            state.closable = on;
            log.push(PAGE, "Playground", format!("closable = {on}"));
        }
        Msg::Pinned(on) => {
            if let Some(first) = state.files.first_mut() {
                first.pinned = on;
            }
            log.push(PAGE, "Playground", format!("pinned = {on}"));
        }
        Msg::Width(index) => {
            state.width = index;
            log.push(PAGE, "Playground", format!("tab_width = {:?}", WIDTHS[index]));
        }
        Msg::Overflow(index) => {
            state.overflow = index;
            log.push(PAGE, "Playground", format!("overflow = {:?}", OVERFLOWS[index]));
        }
        Msg::Reorderable(on) => {
            state.reorderable = on;
            log.push(PAGE, "Playground", format!("reorderable = {on}"));
        }
        Msg::ContextMenu(on) => {
            state.menu = on;
            log.push(PAGE, "Playground", format!("context menu = {on}"));
        }
        Msg::DragScroll(first) => {
            log.push(PAGE, "Tabs#files", format!("drag scroll: {} first in view", name(state, first)));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("tabs-advanced.hint")).role("secondary"));
        if state.files.is_empty() {
            ui.add(Text::new(t!("tabs-advanced.empty")).role("faint"));
        } else {
            let labels: Vec<&str> = state.files.iter().map(|tab| FILES[tab.item].0).collect();
            // region: tabs-advanced-options
            let mut tabs = Tabs::new(labels)
                .active(state.active)
                .on_select(|index| send(Msg::Open(index)))
                .tab_width(WIDTHS[state.width])
                .overflow(OVERFLOWS[state.overflow])
                .pinned(tab_menu::pinned(&state.files));
            if state.closable {
                tabs = tabs.closable(|index| send(Msg::Edit(TabEdit::Close(index))));
            }
            if state.reorderable {
                tabs = tabs
                    .reorderable(|from, to| send(Msg::Edit(TabEdit::Move { from, to })))
                    .on_drag_scroll(|first| send(Msg::DragScroll(first)));
            }
            // endregion
            // region: tabs-advanced-menu
            if state.menu {
                let files = state.files.clone();
                tabs = tabs.context_menu(move |index| tab_menu::items(&files, index, |action| send(Msg::Menu(action))));
            }
            // endregion
            ui.add(tabs).width(Length::Cells(72)).id("files");
            let (name, lines) = FILES[state.files[state.active.min(state.files.len() - 1)].item];
            ui.add(Text::new(t!("tabs-advanced.file", name = name, n = lines)).role("faint"));
        }
        ui.add(Button::new(t!("tabs-advanced.reopen")).on_press(send(Msg::Reopen))).id("reopen");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("tabs-advanced.closable"), |ui| {
            ui.add(toggle(state.closable, |on| send(Msg::Closable(on)))).id("closable");
        });
        setting(ui, t!("tabs-advanced.pinned"), |ui| {
            let pinned = state.files.first().is_some_and(|tab| tab.pinned);
            ui.add(toggle(pinned, |on| send(Msg::Pinned(on)))).id("pinned");
        });
        setting(ui, t!("tabs-advanced.width"), |ui| {
            let names = [t!("tabs-advanced.fit"), t!("tabs-advanced.fixed"), t!("tabs-advanced.fill")];
            ui.add(Segmented::new(names).selected(state.width).on_select(|i| send(Msg::Width(i)))).id("width");
        });
        setting(ui, t!("tabs-advanced.overflow"), |ui| {
            let names = [t!("tabs-advanced.arrows"), t!("tabs-advanced.menu")];
            ui.add(Segmented::new(names).selected(state.overflow).on_select(|i| send(Msg::Overflow(i)))).id("overflow");
        });
        setting(ui, t!("tabs-advanced.reorderable"), |ui| {
            ui.add(toggle(state.reorderable, |on| send(Msg::Reorderable(on)))).id("reorderable");
        });
        setting(ui, t!("tabs-advanced.context-menu"), |ui| {
            ui.add(toggle(state.menu, |on| send(Msg::ContextMenu(on)))).id("context-menu");
        });
        ui.add(Text::new(t!("tabs-advanced.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;
    use qframe::event::{MouseButton, MouseKind};

    #[test]
    fn options_turn_on_one_at_a_time() {
        let mut h = showcase_on(PAGE);
        assert!(!h.screen().contains('×'), "plain tabs show no close marks");
        h.send(send(Msg::Closable(true)));
        assert!(h.screen().contains("app.rs") && h.screen().contains('×'), "{}", h.screen());
        h.send(send(Msg::Edit(TabEdit::Close(0))));
        assert_eq!(h.app().pages.tabs_advanced.files.len(), FILES.len() - 1);
        h.send(send(Msg::Overflow(1)));
        h.send(send(Msg::Reorderable(true)));
        h.send(send(Msg::Edit(TabEdit::Move { from: 0, to: 2 })));
        assert_eq!(h.app().pages.tabs_advanced.files[2].item, 1);
        h.click_text("Reopen every file");
        assert_eq!(h.app().pages.tabs_advanced.files.len(), FILES.len());
        for index in 0..FILES.len() {
            h.send(send(Msg::Edit(TabEdit::Close(0))));
            assert_eq!(h.app().pages.tabs_advanced.files.len(), FILES.len() - index - 1);
        }
        assert!(h.screen().contains("Every file is closed"), "{}", h.screen());
    }

    /// Row `y` of the screen.
    fn line(h: &qframe::runtime::Harness<crate::app::Showcase>, y: i32) -> String {
        h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned()
    }

    /// The cell of the first `glyph` in `line`.
    fn column(line: &str, glyph: char) -> i32 {
        i32::try_from(line.chars().position(|c| c == glyph).expect("glyph on the strip")).expect("on screen")
    }

    #[test]
    fn every_overflow_mode_and_closing_work_with_the_mouse() {
        let mut h = showcase_on(PAGE);
        let (_, y) = h.find("README.md").expect("the strip");
        h.click(column(&line(&h, y), '▶'), y);
        assert!(!line(&h, y).contains("README.md"), "the arrow scrolls:\n{}", h.screen());

        h.click_text("Menu");
        assert!(h.screen().contains("overflow = Menu"), "{}", h.screen());
        h.click(column(&line(&h, y), '▾'), y).advance(std::time::Duration::from_millis(300));
        h.click_text("docker-compose.yml");
        assert!(
            h.screen().contains("selected docker-compose.yml"),
            "a hidden file opens from the menu:\n{}",
            h.screen()
        );

        h.send(send(Msg::Closable(true)));
        let strip = line(&h, y);
        let open = strip.find("docker-compose.yml").map(|byte| strip[..byte].chars().count()).expect("the open file");
        let mark = strip.chars().skip(open).position(|c| c == '×').map(|x| x + open).expect("its close mark");
        h.click(i32::try_from(mark).expect("on screen"), y);
        assert!(h.screen().contains("close docker-compose.yml"), "{}", h.screen());
        assert_eq!(h.app().pages.tabs_advanced.files.len(), FILES.len() - 1);
    }

    #[test]
    fn a_dragged_file_held_on_an_arrow_scrolls_the_strip_and_logs_each_step() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Reorderable(true)));
        let (_, y) = h.find("README.md").expect("the strip");
        let (x, _) = h.find("app.rs").expect("the open file");
        let forward = column(&line(&h, y), '▶');
        let logged = |h: &qframe::runtime::Harness<crate::app::Showcase>, message: &str| {
            h.app().log.recent(PAGE, 20).iter().any(|entry| entry.message == message)
        };
        h.mouse(MouseKind::Down(MouseButton::Left), x, y);
        h.mouse(MouseKind::Drag(MouseButton::Left), forward, y);
        h.advance(std::time::Duration::from_millis(399));
        assert!(!h.app().log.recent(PAGE, 20).iter().any(|entry| entry.message.starts_with("drag scroll")));
        h.advance(std::time::Duration::from_millis(1));
        assert!(logged(&h, "drag scroll: main.rs first in view"), "one file at the delay:\n{}", h.screen());
        h.advance(std::time::Duration::from_millis(150));
        assert!(logged(&h, "drag scroll: app.rs first in view"), "then one every 150 ms");
        h.mouse(MouseKind::Up(MouseButton::Left), forward, y);
        assert!(!line(&h, y).contains("README.md"), "the strip stays scrolled:\n{}", h.screen());
        let moved = h.app().pages.tabs_advanced.files.iter().position(|tab| tab.item == 2).expect("app.rs");
        assert!(moved > 2, "app.rs landed further right, at {moved}");
    }

    #[test]
    fn the_right_click_menu_closes_pins_and_duplicates_files() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("main.rs").expect("the strip");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y);
        assert!(!h.screen().contains("Close others"), "no menu until it is turned on:\n{}", h.screen());

        h.send(send(Msg::ContextMenu(true)));
        let right_click = |h: &mut qframe::runtime::Harness<crate::app::Showcase>, x: i32| {
            h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        };
        right_click(&mut h, x);
        h.click_text("Duplicate");
        assert!(h.screen().contains("menu: duplicate main.rs"), "{}", h.screen());
        let files = |h: &qframe::runtime::Harness<crate::app::Showcase>| {
            h.app().pages.tabs_advanced.files.iter().map(|tab| tab.item).collect::<Vec<_>>()
        };
        assert_eq!(files(&h)[..3], [0, 1, 1]);

        let (readme, _) = h.find("README.md").expect("the first file");
        right_click(&mut h, readme);
        h.click_text("Pin");
        assert!(h.app().pages.tabs_advanced.files[0].pinned);

        right_click(&mut h, x);
        h.click_text("Close to the right");
        assert_eq!(files(&h), vec![0, 1], "everything right of main.rs closes");
        right_click(&mut h, x);
        h.click_text("Close others");
        assert_eq!(files(&h), vec![0, 1], "the pinned README stays");
    }
}
