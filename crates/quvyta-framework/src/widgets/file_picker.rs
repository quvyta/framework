//! Browsing the file system to choose a file or a folder.

use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{Align, EventCx, Length, MeasureCx, NodeMut, PaintCx, View, Widget};

use super::cells;
use super::delayed::DelayedIndicator;
use super::file_browser::{FileBrowser, FilePickerMsg, FolderState, ListingError, PickMode};
use super::{Button, List, ListItem, SpinnerStyle, Switch, Text, TextInput};

/// Browses folders and chooses a file or a folder.
///
/// The picker is built from framework widgets: a clickable path, a name filter, a list of the
/// folder's entries (folders first, a parent row on top), and a footer with a hidden-files
/// switch and the choose button. The application owns a [`FileBrowser`] and passes it every
/// [`FilePickerMsg`]; folders are read on a background thread, and a folder that cannot be opened
/// shows a readable message at once.
///
/// While a folder is read the picker keeps showing the folder it was on, unchanged and usable,
/// and switches to the new one in a single frame when it has been read. A read that takes longer
/// than about 300 ms shows a small spinner right after the path, which then stays at least
/// about 500 ms, so quick reads never flash a loading state and slow ones never blink one.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::{FileBrowser, FilePicker, FilePickerMsg, PickMode};
///
/// struct Open {
///     browser: FileBrowser,
///     chosen: Option<std::path::PathBuf>,
/// }
///
/// #[derive(Clone)]
/// enum Msg {
///     Picker(FilePickerMsg),
/// }
///
/// impl App for Open {
///     type Msg = Msg;
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::Picker(FilePickerMsg::Chosen(path)) => self.chosen = Some(path),
///             Msg::Picker(message) => return self.browser.update(message, Msg::Picker),
///         }
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         FilePicker::new(&self.browser, Msg::Picker).show(ui).fill();
///     }
/// }
/// ```
///
/// Keys: the list's keys (↑/↓, Enter opens a folder or chooses a file) and Tab between the
/// filter, the list, the switch and the button. Style keys: `path-segment` (`fg`, `bg`) with
/// `hover` and `selected` for the current folder; `path-separator`; `spinner` and
/// `spinner-label` for the reading indicator; the styles of `List`, `TextInput`, `Switch` and
/// `Button`. Icons: `folder`, `file`, `arrow-up`,
/// `path-separator`, `error`. Framework strings under `quvyta.file-picker`.
pub struct FilePicker<'a, Msg> {
    browser: &'a FileBrowser,
    wrap: Wrap<Msg>,
}

/// Turns picker messages into the application's; shared by every widget of one picker.
type Wrap<Msg> = Rc<dyn Fn(FilePickerMsg) -> Msg>;

impl<'a, Msg: Clone + Send + 'static> FilePicker<'a, Msg> {
    /// A picker showing `browser`; `wrap` turns picker messages into application messages.
    ///
    /// `wrap` is a function such as `Msg::Picker`, or a closure that captures what it needs,
    /// e.g. a screen's own conversion: `move |message| wrap(screen::Msg::Picker(message))`. It
    /// runs while events are handled on the drawing thread, so it need not be `Send`, and the
    /// picker shares it among its widgets, so it need not be `Clone`.
    #[must_use]
    pub fn new(browser: &'a FileBrowser, wrap: impl Fn(FilePickerMsg) -> Msg + 'static) -> Self {
        Self { browser, wrap: Rc::new(wrap) }
    }

    /// Adds the picker to `ui` as a column.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let browser = self.browser;
        let wrap = self.wrap;
        ui.column(|ui| {
            let failed = matches!(browser.state, FolderState::Failed(_));
            let busy = browser.loading.is_some();
            ui.add(PathBar { folder: browser.folder.clone(), busy, failed, wrap: Rc::clone(&wrap) }).fill_width();
            let filter = Rc::clone(&wrap);
            ui.add(
                TextInput::new(&browser.filter)
                    .placeholder(crate::t!("quvyta.file-picker.filter"))
                    .on_change(move |text| filter(FilePickerMsg::Filter(text))),
            )
            .fill_width();
            match &browser.state {
                // Before the first answer there is nothing to keep: the space stays empty and the
                // path line shows the reading indicator if the read is slow.
                FolderState::Unread => {
                    ui.add(Text::new("")).height(Length::Fill(1));
                }
                FolderState::Failed(error) => Self::failure(ui, error, &browser.folder),
                FolderState::Ready(_) => Self::entries(ui, browser, &wrap),
            }
            Self::footer(ui, browser, &wrap);
        })
        .gap(1)
    }

    fn failure(ui: &mut View<'_, Msg>, error: &ListingError, folder: &Path) {
        let (key, detail) = match error {
            ListingError::PermissionDenied => ("permission-denied", None),
            ListingError::NotFound => ("not-found", None),
            ListingError::NotAFolder => ("not-a-folder", None),
            ListingError::Other(message) => ("unreadable", Some(message.clone())),
        };
        ui.column(|ui| {
            let glyph = ui.env().icons().glyph("error").into_owned();
            ui.add(
                Text::rich([
                    super::Span::new(format!("{glyph}  ")).color("danger"),
                    super::Span::new(crate::t!(&format!("quvyta.file-picker.{key}"))).role("body"),
                ])
                .no_wrap(),
            );
            ui.add(Text::new(folder.display().to_string()).role("faint"));
            if let Some(detail) = detail {
                ui.add(Text::new(detail).role("faint"));
            }
        })
        .height(Length::Fill(1))
        .fill_width();
    }

    fn entries(ui: &mut View<'_, Msg>, browser: &FileBrowser, wrap: &Wrap<Msg>) {
        let parent = browser.folder.parent().map(Path::to_path_buf);
        let visible = browser.visible();
        let mut items = Vec::new();
        // What each row does when opened, and which name it selects.
        let mut rows: Vec<(Option<String>, Option<FilePickerMsg>)> = Vec::new();
        if let Some(parent) = &parent {
            items.push(ListItem::new(crate::t!("quvyta.file-picker.parent")).icon("arrow-up", Some("muted")));
            rows.push((None, Some(FilePickerMsg::Open(parent.clone()))));
        }
        for entry in &visible {
            let path = browser.folder.join(entry.name());
            let (item, action) = if entry.is_folder() {
                (ListItem::new(entry.name()).icon("folder", Some("accent")), Some(FilePickerMsg::Open(path)))
            } else {
                let item = ListItem::new(entry.name())
                    .icon("file", Some("muted"))
                    .detail(entry.size().map(format_size).unwrap_or_default())
                    .faint(browser.mode == PickMode::Folders);
                let action = (browser.mode == PickMode::Files).then_some(FilePickerMsg::Chosen(path));
                (item, action)
            };
            items.push(item);
            rows.push((Some(entry.name().to_owned()), action));
        }
        let selected = match &browser.selected {
            Some(name) => rows.iter().position(|(row, _)| row.as_ref() == Some(name)),
            None => None,
        };
        let empty = if browser.filter.is_empty() { "empty" } else { "no-match" };
        let nothing = visible.is_empty();
        let names: Vec<Option<String>> = rows.iter().map(|(name, _)| name.clone()).collect();
        let actions: Vec<FilePickerMsg> = rows
            .into_iter()
            // Rows without an action (a file in a folder picker) only select.
            .map(|(name, action)| action.unwrap_or(FilePickerMsg::Select(name)))
            .collect();
        let (select, activate) = (Rc::clone(wrap), Rc::clone(wrap));
        let list = List::new(items)
            .selected(selected)
            .on_select(move |index| select(FilePickerMsg::Select(names.get(index).cloned().flatten())))
            .on_activate(move |index| activate(actions.get(index).cloned().unwrap_or(FilePickerMsg::Refresh)));
        if nothing {
            // An empty folder still offers the way back up above its empty text.
            ui.add(list).fill_width();
            ui.add(Text::new(crate::t!(&format!("quvyta.file-picker.{empty}"))).role("faint"))
                .height(Length::Fill(1))
                .padding(crate::geometry::Padding { top: 0, right: 0, bottom: 0, left: 2 });
            return;
        }
        ui.add(list).width(Length::Fill(1)).height(Length::Fill(1));
    }

    fn footer(ui: &mut View<'_, Msg>, browser: &FileBrowser, wrap: &Wrap<Msg>) {
        // The selected entry, when it is one the mode can choose: a file, or a folder.
        let wants_folder = browser.mode == PickMode::Folders;
        let selected = browser.selected.as_ref().filter(|name| {
            browser.visible().iter().any(|entry| entry.name() == name.as_str() && entry.is_folder() == wants_folder)
        });
        let chosen = match browser.mode {
            PickMode::Files => selected.map(|name| browser.folder.join(name)),
            // Without a chosen folder inside, the folder shown is the choice.
            PickMode::Folders => {
                Some(selected.map_or_else(|| browser.folder.clone(), |name| browser.folder.join(name)))
            }
        };
        let ready = matches!(browser.state, FolderState::Ready(_));
        ui.row(|ui| {
            let toggle = Rc::clone(wrap);
            ui.add(
                Switch::new(browser.show_hidden)
                    .label(crate::t!("quvyta.file-picker.hidden"))
                    .on_toggle(move |on| toggle(FilePickerMsg::ShowHidden(on))),
            );
            if !browser.extensions.is_empty() {
                ui.add(Text::new(browser.extensions.join(", ")).role("faint").no_wrap());
            }
            ui.spacer();
            let label = match browser.mode {
                PickMode::Files => crate::t!("quvyta.file-picker.choose-file"),
                PickMode::Folders => crate::t!("quvyta.file-picker.choose-folder"),
            };
            let mut button = Button::new(label).variant("primary").disabled(!ready || chosen.is_none());
            if let Some(path) = chosen {
                button = button.on_press(wrap(FilePickerMsg::Chosen(path)));
            }
            ui.add(button);
        })
        .gap(2)
        .align(Align::Center)
        .fill_width();
    }
}

/// A size in bytes as a short human text: `812 B`, `12.4 KiB`, `3.1 GiB`.
fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// Cells kept after the path for the reading spinner: a gap and the spinner, so the segments
/// never move when it shows.
const INDICATOR_CELLS: u16 = 2;

/// The current folder as clickable segments separated by a faint glyph, with the delayed reading
/// indicator after them.
struct PathBar<Msg> {
    folder: PathBuf,
    /// A folder is being read.
    busy: bool,
    /// The folder shown could not be read; its error replaces any indicator at once.
    failed: bool,
    wrap: Wrap<Msg>,
}

impl<Msg> PathBar<Msg> {
    /// Segment labels with the folder each one opens.
    fn segments(&self) -> Vec<(String, PathBuf)> {
        let mut out = Vec::new();
        let mut path = PathBuf::new();
        for component in self.folder.components() {
            path.push(component.as_os_str());
            let label = match component {
                Component::RootDir => std::path::MAIN_SEPARATOR.to_string(),
                other => other.as_os_str().to_string_lossy().into_owned(),
            };
            if matches!(component, Component::Prefix(_)) {
                continue;
            }
            out.push((label, path.clone()));
        }
        out
    }

    /// Where the segments go: the line without the cells kept for the indicator.
    fn segments_area(area: Rect) -> Rect {
        Rect::new(area.x, area.y, area.width.saturating_sub(INDICATOR_CELLS), area.height)
    }

    /// Draws the reading spinner right after the segments, which end at `end`, when the delay
    /// rule says so, with its label where the line has room.
    fn paint_indicator(&self, cx: &mut PaintCx<'_>, area: Rect, end: i32) {
        let now = cx.now();
        let indicator = cx.memory::<DelayedIndicator>();
        if self.failed && !self.busy {
            indicator.cancel();
        }
        let shown = indicator.update(self.busy, now);
        let next = indicator.next_change(self.busy, now);
        if let Some(delay) = next {
            cx.request_frame_in(delay);
        }
        if !shown || area.width <= INDICATOR_CELLS {
            return;
        }
        // Glyph, a space and the label, like a `Spinner`, right after the last segment: the eye is
        // already there. The label only shows where it fits whole; the glyph always has its cells.
        let x = end + 1;
        let style = cx.style("spinner", None, &[]).text();
        let cell = cx.animation(SpinnerStyle::default().animation(), style, Some(std::time::Duration::ZERO));
        cx.text(x, area.y, &cell.glyph, cell.style, 1);
        let label = crate::t!("quvyta.file-picker.loading");
        let width = text::width(&label);
        let label_x = x + 2;
        if label_x + i32::from(width) <= area.right() {
            let style = cx.style("spinner-label", None, &[]).text();
            cx.text(label_x, area.y, &label, style, width);
        }
    }

    /// Screen spans of the segments that fit, from the right; the first span may be the
    /// ellipsis of hidden leading segments (index `None`).
    fn layout(&self, separator_width: u16, area: Rect) -> Vec<(Option<usize>, Rect)> {
        let segments = self.segments();
        let widths: Vec<u16> = segments.iter().map(|(label, _)| text::width(label).saturating_add(2)).collect();
        let step = separator_width + 2;
        let mut first = 0;
        let total = |from: usize| -> u16 {
            let ellipsis = if from > 0 { 3 + step } else { 0 };
            widths[from..].iter().sum::<u16>()
                + step * u16::try_from(widths.len() - from).unwrap_or(0).saturating_sub(1)
                + ellipsis
        };
        while first + 1 < segments.len() && total(first) > area.width {
            first += 1;
        }
        let mut out = Vec::new();
        let mut x = area.x;
        if first > 0 {
            out.push((None, Rect::new(x, area.y, 3, 1)));
            x += 3 + i32::from(step);
        }
        for (index, width) in widths.iter().enumerate().skip(first) {
            out.push((Some(index), Rect::new(x, area.y, *width, 1)));
            x += i32::from(*width + step);
        }
        out
    }
}

impl<Msg: 'static> Widget<Msg> for PathBar<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let width = cells::sum(self.segments().iter().map(|(label, _)| text::width(label).saturating_add(5)));
        Size::new(width.saturating_add(INDICATOR_CELLS), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        let separator = cx.env().icons().glyph("path-separator").into_owned();
        let separator_width = text::width(&separator);
        let segments = self.segments();
        let pointer = cx.pointer();
        let last = segments.len().saturating_sub(1);
        let separator_style = cx.style("path-separator", None, &[]).text();
        let spans = self.layout(separator_width, Self::segments_area(area));
        for (position, (index, rect)) in spans.iter().enumerate() {
            let mut states = Vec::new();
            if pointer.is_some_and(|(x, y)| rect.contains(x, y)) && *index != Some(last) && index.is_some() {
                states.push(State::Hover);
            }
            if *index == Some(last) {
                states.push(State::Selected);
            }
            let style = cx.style("path-segment", None, &states).text();
            if let Some(bg) = style.bg {
                cx.fill(*rect, bg);
            }
            let label = index.map_or_else(|| text::ELLIPSIS.to_owned(), |i| segments[i].0.clone());
            cx.text(rect.x + 1, rect.y, &label, CellStyle { bg: None, ..style }, rect.width.saturating_sub(2));
            if position + 1 < spans.len() {
                cx.text(rect.right() + 1, rect.y, &separator, separator_style, separator_width);
            }
        }
        let end = spans.last().map_or(area.x, |(_, rect)| rect.right());
        self.paint_indicator(cx, area, end);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let Event::Mouse(mouse) = event else { return false };
        if mouse.kind != MouseKind::Down(MouseButton::Left) {
            return false;
        }
        let separator = text::width(&cx.env().icons().glyph("path-separator"));
        let segments = self.segments();
        let hit = self
            .layout(separator, Self::segments_area(cx.area()))
            .into_iter()
            .find(|(_, rect)| rect.contains(mouse.x, mouse.y));
        match hit {
            Some((Some(index), _)) if index + 1 < segments.len() => {
                cx.emit((self.wrap)(FilePickerMsg::Open(segments[index].1.clone())));
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widgets::read_folder;

    struct Demo {
        browser: FileBrowser,
        chosen: Option<PathBuf>,
    }

    #[derive(Debug, Clone)]
    enum Msg {
        Picker(FilePickerMsg),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Picker(FilePickerMsg::Chosen(path)) => self.chosen = Some(path),
                Msg::Picker(message) => return self.browser.update(message, Msg::Picker),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            FilePicker::new(&self.browser, Msg::Picker).show(ui).fill();
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-a8-picker-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("deploy/scripts")).expect("scratch folders");
        std::fs::write(dir.join("compose.yaml"), "services:\n").expect("scratch file");
        std::fs::write(dir.join("deploy/release.sh"), vec![b'#'; 2048]).expect("scratch file");
        dir
    }

    fn harness(dir: &Path, mode: PickMode) -> Harness<Demo> {
        let browser = FileBrowser::new(dir, mode);
        let mut h = Harness::new(Demo { browser, chosen: None }, 60, 14);
        h.set_glyph_mode(GlyphMode::Unicode);
        let folder = dir.to_path_buf();
        h.send(Msg::Picker(FilePickerMsg::Open(folder)));
        h
    }

    #[test]
    fn browses_into_folders_and_chooses_a_file() {
        let dir = scratch("choose");
        let mut h = harness(&dir, PickMode::Files);
        let screen = h.screen();
        assert!(screen.contains("■ deploy") && screen.contains("compose.yaml"), "{screen}");
        assert!(screen.contains("Parent folder"), "{screen}");
        h.click_text("deploy");
        let screen = h.screen();
        assert!(screen.contains("release.sh") && screen.contains("2.0 KiB"), "{screen}");
        assert!(screen.contains("scripts"), "{screen}");
        h.click_text("release.sh");
        assert_eq!(h.app().chosen.as_deref(), Some(dir.join("deploy/release.sh").as_path()));
        let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        h.click_text(&name);
        assert_eq!(h.app().browser.folder(), dir.as_path());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn folder_mode_chooses_the_selected_folder_and_filters() {
        let dir = scratch("folders");
        let mut h = harness(&dir, PickMode::Folders);
        h.click_text("Choose folder");
        assert_eq!(h.app().chosen.as_deref(), Some(dir.join("deploy").as_path()));
        h.send(Msg::Picker(FilePickerMsg::Filter("zzz".into())));
        assert!(h.screen().contains("Nothing matches"), "{}", h.screen());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unreadable_folders_show_a_state() {
        let missing = std::env::temp_dir().join("quvyta-a8-picker-missing");
        let h = harness(&missing, PickMode::Files);
        let screen = h.screen();
        assert!(screen.contains("✕  This folder does not exist"), "{screen}");
        assert!(!screen.contains('['));
    }

    /// A picker inside a screen of the application: the screen's messages carry the tab it sits
    /// in, so `wrap` is a closure over that tab rather than a function.
    struct Tabbed {
        tab: usize,
        browser: FileBrowser,
        chosen: Vec<(usize, PathBuf)>,
    }

    #[derive(Debug, Clone)]
    enum TabMsg {
        Picker(usize, FilePickerMsg),
    }

    impl App for Tabbed {
        type Msg = TabMsg;
        fn update(&mut self, msg: TabMsg) -> Command<TabMsg> {
            let TabMsg::Picker(tab, message) = msg;
            if let FilePickerMsg::Chosen(path) = message {
                self.chosen.push((tab, path));
                return Command::none();
            }
            self.browser.update(message, move |message| TabMsg::Picker(tab, message))
        }
        fn view(&self, ui: &mut View<'_, TabMsg>) {
            let tab = self.tab;
            FilePicker::new(&self.browser, move |message| TabMsg::Picker(tab, message)).show(ui).fill();
        }
    }

    #[test]
    fn a_closure_capturing_state_wraps_every_picker_message() {
        let dir = scratch("closure");
        let browser = FileBrowser::new(&dir, PickMode::Files);
        let mut h = Harness::new(Tabbed { tab: 3, browser, chosen: Vec::new() }, 60, 14);
        h.set_glyph_mode(GlyphMode::Unicode);
        // `update` hands the closure to `open`, whose read delivers `Loaded` through it.
        h.send(TabMsg::Picker(3, FilePickerMsg::Open(dir.clone())));
        assert!(h.screen().contains("compose.yaml"), "{}", h.screen());
        h.click_text("deploy").click_text("release.sh");
        assert_eq!(h.app().chosen, [(3, dir.join("deploy/release.sh"))]);
        std::fs::remove_dir_all(dir).ok();
    }

    impl From<FilePickerMsg> for Msg {
        fn from(message: FilePickerMsg) -> Self {
            Msg::Picker(message)
        }
    }

    /// Callers that pass a plain function keep compiling: an item, a pointer held in a value, and
    /// a generic function made concrete by the message type.
    #[test]
    fn plain_functions_still_wrap() {
        fn wrap(message: FilePickerMsg) -> Msg {
            Msg::Picker(message)
        }
        fn picked<M: From<FilePickerMsg>>(message: FilePickerMsg) -> M {
            M::from(message)
        }
        let dir = scratch("plain");
        let pointer: fn(FilePickerMsg) -> Msg = wrap;
        let mut browser = FileBrowser::new(&dir, PickMode::Files);
        let command: Command<Msg> = browser.open(dir.clone(), pointer);
        assert_eq!(command.actions.len(), 1);
        let command: Command<Msg> = browser.update(FilePickerMsg::Refresh, picked::<Msg>);
        assert_eq!(command.actions.len(), 1);
        let _ = browser.update(FilePickerMsg::Loaded(dir.clone(), read_folder(&dir)), wrap);
        let mut h = Harness::new(Demo { browser, chosen: None }, 60, 14);
        h.set_glyph_mode(GlyphMode::Unicode);
        assert!(h.screen().contains("compose.yaml"), "{}", h.screen());
        // `Demo` shows its picker with the variant `Msg::Picker`; a pointer is accepted as well.
        let _ = FilePicker::new(&h.app().browser, pointer);
        h.click_text("compose.yaml");
        assert_eq!(h.app().chosen.as_deref(), Some(dir.join("compose.yaml").as_path()));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(format_size(812), "812 B");
        assert_eq!(format_size(12_700), "12.4 KiB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024 + 1024 * 1024 * 100), "3.1 GiB");
    }

    /// A picker whose folder reads wait until the test delivers them, like a real disk.
    struct Held {
        browser: FileBrowser,
    }

    impl App for Held {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            let Msg::Picker(message) = msg;
            // The read command is dropped: the test sends `Loaded` itself when it wants.
            let _ = self.browser.update(message, Msg::Picker);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            FilePicker::new(&self.browser, Msg::Picker).show(ui).fill();
        }
    }

    /// A picker on `dir`, already showing it, with the list focused.
    fn held(dir: &Path) -> Harness<Held> {
        let mut browser = FileBrowser::new(dir, PickMode::Files);
        let _ = browser.update(FilePickerMsg::Loaded(dir.to_path_buf(), read_folder(dir)), |m| m);
        let mut h = Harness::new(Held { browser }, 60, 14);
        h.set_glyph_mode(GlyphMode::Unicode);
        h.press("tab").press("tab");
        h
    }

    fn ms(value: u64) -> std::time::Duration {
        std::time::Duration::from_millis(value)
    }

    /// Whether the path line shows the reading spinner.
    fn spinning(h: &Harness<Held>) -> bool {
        let screen = h.screen();
        screen.lines().next().is_some_and(|line| line.contains(|c| "◜◠◝◞◡◟".contains(c)))
    }

    fn deliver(h: &mut Harness<Held>, folder: &Path) {
        h.send(Msg::Picker(FilePickerMsg::Loaded(folder.to_path_buf(), read_folder(folder))));
    }

    /// Opening a folder must not flash: the path and the listing stay as they were, keyboard focus
    /// included, until the folder has been read. Every frame before the answer is the frame before
    /// the click.
    #[test]
    fn opening_a_folder_keeps_the_listing_until_it_is_read() {
        let dir = scratch("flash");
        let mut h = held(&dir);
        let before = h.screen();
        h.press("enter");
        assert_eq!(h.app().browser.loading(), Some(dir.join("deploy").as_path()));
        for step in [0, 40, 120, 139] {
            h.advance(ms(step));
            assert_eq!(h.screen(), before, "at +{step} ms");
        }
        deliver(&mut h, &dir.join("deploy"));
        let screen = h.screen();
        assert!(screen.contains("›  deploy") && screen.contains("release.sh"), "{screen}");
        assert!(!spinning(&h), "{screen}");
        // The list kept its focus through the read, so the keys still move in it.
        h.press("down");
        assert!(h.screen().contains("▌  ▪ release.sh"), "{}", h.screen());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_slow_read_shows_the_indicator_after_the_delay_for_at_least_its_minimum() {
        let dir = scratch("slow");
        let mut h = held(&dir);
        h.press("enter").advance(ms(299));
        assert!(!spinning(&h), "{}", h.screen());
        h.advance(ms(1));
        let first = h.screen();
        assert!(spinning(&h), "{first}");
        // Still the old folder underneath, untouched.
        assert!(first.contains("▌  ■ deploy") && first.contains("compose.yaml"), "{first}");
        // The read ends at 350 ms: the new folder shows, the indicator stays until 800 ms.
        h.advance(ms(50));
        deliver(&mut h, &dir.join("deploy"));
        assert!(h.screen().contains("release.sh") && spinning(&h), "{}", h.screen());
        h.advance(ms(449));
        assert!(spinning(&h), "{}", h.screen());
        h.advance(ms(1));
        assert!(!spinning(&h), "{}", h.screen());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_folder_that_cannot_be_read_shows_its_error_at_once() {
        let dir = scratch("error");
        let mut h = held(&dir);
        let missing = dir.join("gone");
        h.send(Msg::Picker(FilePickerMsg::Open(missing.clone()))).advance(ms(320));
        assert!(spinning(&h), "{}", h.screen());
        deliver(&mut h, &missing);
        let screen = h.screen();
        assert!(screen.contains("This folder does not exist"), "{screen}");
        assert!(!spinning(&h), "{screen}");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_first_read_leaves_the_space_empty_until_it_is_slow() {
        let dir = scratch("first");
        // Wide enough for the indicator's label after the path.
        let mut h = Harness::new(Held { browser: FileBrowser::new(&dir, PickMode::Files) }, 90, 12);
        h.set_glyph_mode(GlyphMode::Unicode);
        h.send(Msg::Picker(FilePickerMsg::Open(dir.clone())));
        assert!(!spinning(&h) && !h.screen().contains("deploy"), "{}", h.screen());
        h.advance(ms(300));
        assert!(spinning(&h), "{}", h.screen());
        assert!(
            h.screen().lines().next().is_some_and(|line| line.trim_end().ends_with(" Reading folder…")),
            "{}",
            h.screen()
        );
        deliver(&mut h, &dir);
        assert!(h.screen().contains("compose.yaml"), "{}", h.screen());
        std::fs::remove_dir_all(dir).ok();
    }
}
