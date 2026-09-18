//! Showcase pages: each one has a live demo written in its own module, the source regions the
//! Code section shows, and an English and Turkish guide and reference.
//!
//! This module is the page registry: `PAGES`, `Pages`, `PageMsg`, `update` and `demo` list the
//! pages in the same order. The helpers pages share follow it: playground rows and switches, the
//! application-wide slide switch, and test helpers.

pub mod accordion;
pub mod async_tasks;
pub mod badge;
pub mod bar_chart;
pub mod big_text;
pub mod breadcrumb;
pub mod button;
pub mod cell_animation;
pub mod checkbox;
pub mod clipboard;
pub mod code_view;
pub mod command_palette;
pub mod confirm;
pub mod context_menu;
pub mod date_picker;
pub mod date_time;
pub mod divider;
pub mod document;
pub mod duration_input;
pub mod empty_state;
pub mod example_dashboard;
pub mod example_file_explorer;
pub mod example_setup_wizard;
pub mod file_picker;
pub mod focus_keys;
pub mod form;
pub mod gauge;
pub mod getting_started;
pub mod handoff;
pub mod heatmap;
pub mod help_layer;
pub mod hold_to_confirm;
pub mod key_hints;
pub mod layout;
pub mod list;
pub mod log_view;
pub mod markdown;
pub mod menu;
pub mod modal;
pub mod motion;
pub mod number_input;
pub mod page_transitions;
pub mod panel;
pub mod popover;
pub mod progress_bar;
pub mod radio_group;
pub mod samples;
pub mod scroll_view;
pub mod scrollbar_styles;
pub mod segmented;
pub mod select;
pub mod settings_list;
pub mod shimmer_text;
pub mod side_panel;
pub mod skeleton;
pub mod slider;
pub mod sparkline;
pub mod spinner;
pub mod splitter;
pub mod steps;
pub mod storage;
pub mod switch;
pub mod tab_menu;
pub mod tab_rail;
pub mod table;
pub mod tabs;
pub mod tabs_advanced;
pub mod terminal;
pub mod text;
pub mod text_area;
pub mod text_input;
pub mod text_selection;
pub mod theme_icons_language;
pub mod time_input;
pub mod timeline;
pub mod toast;
pub mod tooltip;
pub mod tree;
pub mod widget_dock;
pub mod wizard;

use qframe::prelude::*;
use qframe::runtime::ClipboardEvent;
use qframe::storage::Settings;
use qframe::widgets::Switch;

use crate::app::Msg;
use crate::log::EventLog;

/// The folder the disk demos (tree, file picker, file explorer, terminal) start in: the user's
/// home folder, which every machine has. The demos only list and read what is there; nothing in
/// it is written or removed. Without a home folder they start in the system's temporary folder.
#[cfg(not(test))]
pub fn home_folder() -> std::path::PathBuf {
    std::env::home_dir().filter(|home| home.is_absolute()).unwrap_or_else(std::env::temp_dir)
}

#[cfg(test)]
pub use crate::tests::home_folder;

/// Foundation pages, in menu order.
pub const FOUNDATIONS: [&str; 4] = ["getting-started", "theme-icons-language", "layout", "focus-keys"];

/// Everything a page shows besides its live demo.
pub struct PageContent {
    pub id: &'static str,
    /// The page module's source; the Code section shows its regions.
    pub source: &'static str,
    /// Guide in English and Turkish.
    pub guide: [&'static str; 2],
    /// Reference in English and Turkish.
    pub reference: [&'static str; 2],
}

macro_rules! page {
    ($id:literal, $module:literal) => {
        PageContent {
            id: $id,
            source: include_str!(concat!($module, ".rs")),
            guide: [
                include_str!(concat!("../../assets/pages/", $id, "/guide.en.md")),
                include_str!(concat!("../../assets/pages/", $id, "/guide.tr.md")),
            ],
            reference: [
                include_str!(concat!("../../assets/pages/", $id, "/reference.en.md")),
                include_str!(concat!("../../assets/pages/", $id, "/reference.tr.md")),
            ],
        }
    };
}

/// Every page with content.
pub const PAGES: &[PageContent] = &[
    page!("getting-started", "getting_started"),
    page!("theme-icons-language", "theme_icons_language"),
    page!("layout", "layout"),
    page!("focus-keys", "focus_keys"),
    page!("text", "text"),
    page!("panel", "panel"),
    page!("button", "button"),
    page!("text-input", "text_input"),
    page!("select", "select"),
    page!("list", "list"),
    page!("scroll-view", "scroll_view"),
    page!("tabs", "tabs"),
    page!("key-hints", "key_hints"),
    page!("markdown", "markdown"),
    page!("code-view", "code_view"),
    page!("motion", "motion"),
    page!("spinner", "spinner"),
    page!("cell-animation", "cell_animation"),
    page!("shimmer-text", "shimmer_text"),
    page!("progress-bar", "progress_bar"),
    page!("checkbox", "checkbox"),
    page!("switch", "switch"),
    page!("segmented", "segmented"),
    page!("radio-group", "radio_group"),
    page!("modal", "modal"),
    page!("confirm", "confirm"),
    page!("hold-to-confirm", "hold_to_confirm"),
    page!("help-layer", "help_layer"),
    page!("command-palette", "command_palette"),
    page!("popover", "popover"),
    page!("tooltip", "tooltip"),
    page!("context-menu", "context_menu"),
    page!("toast", "toast"),
    page!("date-picker", "date_picker"),
    page!("badge", "badge"),
    page!("divider", "divider"),
    page!("empty-state", "empty_state"),
    page!("skeleton", "skeleton"),
    page!("sparkline", "sparkline"),
    page!("gauge", "gauge"),
    page!("bar-chart", "bar_chart"),
    page!("heatmap", "heatmap"),
    page!("timeline", "timeline"),
    page!("big-text", "big_text"),
    page!("example-dashboard", "example_dashboard"),
    page!("scrollbar-styles", "scrollbar_styles"),
    page!("side-panel", "side_panel"),
    page!("splitter", "splitter"),
    page!("widget-dock", "widget_dock"),
    page!("accordion", "accordion"),
    page!("tabs-advanced", "tabs_advanced"),
    page!("tab-rail", "tab_rail"),
    page!("menu", "menu"),
    page!("breadcrumb", "breadcrumb"),
    page!("slider", "slider"),
    page!("number-input", "number_input"),
    page!("time-input", "time_input"),
    page!("duration-input", "duration_input"),
    page!("text-area", "text_area"),
    page!("table", "table"),
    page!("tree", "tree"),
    page!("log-view", "log_view"),
    page!("file-picker", "file_picker"),
    page!("example-file-explorer", "example_file_explorer"),
    page!("terminal", "terminal"),
    page!("form", "form"),
    page!("steps", "steps"),
    page!("settings-list", "settings_list"),
    page!("wizard", "wizard"),
    page!("example-setup-wizard", "example_setup_wizard"),
    page!("clipboard", "clipboard"),
    page!("async-tasks", "async_tasks"),
    page!("handoff", "handoff"),
    page!("storage", "storage"),
    page!("date-time", "date_time"),
    page!("document", "document"),
    page!("page-transitions", "page_transitions"),
    page!("text-selection", "text_selection"),
];

/// The content of page `id`.
#[must_use]
pub fn content(id: &str) -> Option<&'static PageContent> {
    PAGES.iter().find(|page| page.id == id)
}

/// Demo state of every page, kept while browsing other pages.
#[derive(Default)]
pub struct Pages {
    pub getting_started: getting_started::State,
    pub theme_icons_language: theme_icons_language::State,
    pub layout: layout::State,
    pub focus_keys: focus_keys::State,
    pub text: text::State,
    pub panel: panel::State,
    pub button: button::State,
    pub text_input: text_input::State,
    pub select: select::State,
    pub list: list::State,
    pub scroll_view: scroll_view::State,
    pub tabs: tabs::State,
    pub markdown: markdown::State,
    pub code_view: code_view::State,
    pub motion: motion::State,
    pub spinner: spinner::State,
    pub cell_animation: cell_animation::State,
    pub progress_bar: progress_bar::State,
    pub checkbox: checkbox::State,
    pub switch: switch::State,
    pub segmented: segmented::State,
    pub radio_group: radio_group::State,
    pub modal: modal::State,
    pub confirm: confirm::State,
    pub hold_to_confirm: hold_to_confirm::State,
    pub help_layer: help_layer::State,
    pub command_palette: command_palette::State,
    pub popover: popover::State,
    pub tooltip: tooltip::State,
    pub context_menu: context_menu::State,
    pub toast: toast::State,
    pub date_picker: date_picker::State,
    pub badge: badge::State,
    pub divider: divider::State,
    pub empty_state: empty_state::State,
    pub skeleton: skeleton::State,
    pub sparkline: sparkline::State,
    pub gauge: gauge::State,
    pub bar_chart: bar_chart::State,
    pub heatmap: heatmap::State,
    pub timeline: timeline::State,
    pub big_text: big_text::State,
    pub example_dashboard: example_dashboard::State,
    pub scrollbar_styles: scrollbar_styles::State,
    pub side_panel: side_panel::State,
    pub splitter: splitter::State,
    pub widget_dock: widget_dock::State,
    pub accordion: accordion::State,
    pub tabs_advanced: tabs_advanced::State,
    pub tab_rail: tab_rail::State,
    pub menu: menu::State,
    pub breadcrumb: breadcrumb::State,
    pub slider: slider::State,
    pub number_input: number_input::State,
    pub time_input: time_input::State,
    pub duration_input: duration_input::State,
    pub text_area: text_area::State,
    pub table: table::State,
    pub tree: tree::State,
    pub log_view: log_view::State,
    pub file_picker: file_picker::State,
    pub example_file_explorer: example_file_explorer::State,
    pub terminal: terminal::State,
    pub form: form::State,
    pub steps: steps::State,
    pub settings_list: settings_list::State,
    pub wizard: wizard::State,
    pub example_setup_wizard: example_setup_wizard::State,
    pub clipboard: clipboard::State,
    pub async_tasks: async_tasks::State,
    pub handoff: handoff::State,
    pub storage: storage::State,
    pub date_time: date_time::State,
    pub document: document::State,
    pub page_transitions: page_transitions::State,
    pub text_selection: text_selection::State,
}

/// A message from a page's demo.
#[derive(Debug, Clone)]
pub enum PageMsg {
    GettingStarted(getting_started::Msg),
    ThemeIconsLanguage(theme_icons_language::Msg),
    Layout(layout::Msg),
    FocusKeys(focus_keys::Msg),
    Text(text::Msg),
    Panel(panel::Msg),
    Button(button::Msg),
    TextInput(text_input::Msg),
    Select(select::Msg),
    List(list::Msg),
    ScrollView(scroll_view::Msg),
    Tabs(tabs::Msg),
    Markdown(markdown::Msg),
    CodeView(code_view::Msg),
    Motion(motion::Msg),
    Spinner(spinner::Msg),
    CellAnimation(cell_animation::Msg),
    ProgressBar(progress_bar::Msg),
    Checkbox(checkbox::Msg),
    Switch(switch::Msg),
    Segmented(segmented::Msg),
    RadioGroup(radio_group::Msg),
    Modal(modal::Msg),
    Confirm(confirm::Msg),
    HoldToConfirm(hold_to_confirm::Msg),
    HelpLayer(help_layer::Msg),
    CommandPalette(command_palette::Msg),
    Popover(popover::Msg),
    Tooltip(tooltip::Msg),
    ContextMenu(context_menu::Msg),
    Toast(toast::Msg),
    DatePicker(date_picker::Msg),
    Badge(badge::Msg),
    Divider(divider::Msg),
    EmptyState(empty_state::Msg),
    Skeleton(skeleton::Msg),
    Sparkline(sparkline::Msg),
    Gauge(gauge::Msg),
    BarChart(bar_chart::Msg),
    Heatmap(heatmap::Msg),
    Timeline(timeline::Msg),
    BigText(big_text::Msg),
    ExampleDashboard(example_dashboard::Msg),
    ScrollbarStyles(scrollbar_styles::Msg),
    SidePanel(side_panel::Msg),
    Splitter(splitter::Msg),
    WidgetDock(widget_dock::Msg),
    Accordion(accordion::Msg),
    TabsAdvanced(tabs_advanced::Msg),
    TabRail(tab_rail::Msg),
    Menu(menu::Msg),
    Breadcrumb(breadcrumb::Msg),
    Slider(slider::Msg),
    NumberInput(number_input::Msg),
    TimeInput(time_input::Msg),
    DurationInput(duration_input::Msg),
    TextArea(text_area::Msg),
    Table(table::Msg),
    Tree(tree::Msg),
    LogView(log_view::Msg),
    FilePicker(file_picker::Msg),
    ExampleFileExplorer(example_file_explorer::Msg),
    Terminal(terminal::Msg),
    Form(form::Msg),
    Steps(steps::Msg),
    SettingsList(settings_list::Msg),
    Wizard(wizard::Msg),
    ExampleSetupWizard(example_setup_wizard::Msg),
    Clipboard(clipboard::Msg),
    AsyncTasks(async_tasks::Msg),
    Handoff(handoff::Msg),
    Storage(storage::Msg),
    DateTime(date_time::Msg),
    Document(document::Msg),
    PageTransitions(page_transitions::Msg),
    TextSelection(text_selection::Msg),
    /// The slide switch of a row page's playground, with the page that sent it.
    Slide(&'static str, bool),
}

/// Applies a demo message.
pub fn update(pages: &mut Pages, message: PageMsg, log: &mut EventLog) -> Command<Msg> {
    match message {
        PageMsg::GettingStarted(m) => getting_started::update(&mut pages.getting_started, m, log),
        PageMsg::ThemeIconsLanguage(m) => theme_icons_language::update(&mut pages.theme_icons_language, m, log),
        PageMsg::Layout(m) => layout::update(&mut pages.layout, m, log),
        PageMsg::FocusKeys(m) => focus_keys::update(&mut pages.focus_keys, m, log),
        PageMsg::Text(m) => text::update(&mut pages.text, m, log),
        PageMsg::Panel(m) => panel::update(&mut pages.panel, m, log),
        PageMsg::Button(m) => button::update(&mut pages.button, m, log),
        PageMsg::TextInput(m) => text_input::update(&mut pages.text_input, m, log),
        PageMsg::Select(m) => select::update(&mut pages.select, m, log),
        PageMsg::List(m) => list::update(&mut pages.list, m, log),
        PageMsg::ScrollView(m) => scroll_view::update(&mut pages.scroll_view, m, log),
        PageMsg::Tabs(m) => tabs::update(&mut pages.tabs, m, log),
        PageMsg::Markdown(m) => markdown::update(&mut pages.markdown, m, log),
        PageMsg::CodeView(m) => code_view::update(&mut pages.code_view, m, log),
        // The Motion and Page transitions pages remember reduced motion with the other appearance choices.
        PageMsg::Motion(m) => motion::update(&mut pages.motion, &mut pages.storage.settings, m, log),
        PageMsg::Spinner(m) => spinner::update(&mut pages.spinner, m, log),
        // The studio remembers its working animations with the other settings.
        PageMsg::CellAnimation(m) => {
            cell_animation::update(&mut pages.cell_animation, &mut pages.storage.settings, m, log)
        }
        PageMsg::ProgressBar(m) => progress_bar::update(&mut pages.progress_bar, m, log),
        PageMsg::Checkbox(m) => checkbox::update(&mut pages.checkbox, m, log),
        PageMsg::Switch(m) => switch::update(&mut pages.switch, m, log),
        PageMsg::Segmented(m) => segmented::update(&mut pages.segmented, m, log),
        PageMsg::RadioGroup(m) => radio_group::update(&mut pages.radio_group, m, log),
        PageMsg::Modal(m) => modal::update(&mut pages.modal, m, log),
        PageMsg::Confirm(m) => confirm::update(&mut pages.confirm, m, log),
        PageMsg::HoldToConfirm(m) => hold_to_confirm::update(&mut pages.hold_to_confirm, m, log),
        PageMsg::HelpLayer(m) => help_layer::update(&mut pages.help_layer, m, log),
        PageMsg::CommandPalette(m) => command_palette::update(&mut pages.command_palette, m, log),
        PageMsg::Popover(m) => popover::update(&mut pages.popover, m, log),
        PageMsg::Tooltip(m) => tooltip::update(&mut pages.tooltip, m, log),
        PageMsg::ContextMenu(m) => context_menu::update(&mut pages.context_menu, m, log),
        PageMsg::Toast(m) => toast::update(&mut pages.toast, m, log),
        PageMsg::DatePicker(m) => date_picker::update(&mut pages.date_picker, m, log),
        PageMsg::Badge(m) => badge::update(&mut pages.badge, m, log),
        PageMsg::Divider(m) => divider::update(&mut pages.divider, m, log),
        PageMsg::EmptyState(m) => empty_state::update(&mut pages.empty_state, m, log),
        PageMsg::Skeleton(m) => skeleton::update(&mut pages.skeleton, m, log),
        PageMsg::Sparkline(m) => sparkline::update(&mut pages.sparkline, m, log),
        PageMsg::Gauge(m) => gauge::update(&mut pages.gauge, m, log),
        PageMsg::BarChart(m) => bar_chart::update(&mut pages.bar_chart, m, log),
        PageMsg::Heatmap(m) => heatmap::update(&mut pages.heatmap, m, log),
        PageMsg::Timeline(m) => timeline::update(&mut pages.timeline, m, log),
        PageMsg::BigText(m) => big_text::update(&mut pages.big_text, m, log),
        PageMsg::ExampleDashboard(m) => example_dashboard::update(&mut pages.example_dashboard, m, log),
        PageMsg::ScrollbarStyles(m) => scrollbar_styles::update(&mut pages.scrollbar_styles, m, log),
        PageMsg::SidePanel(m) => side_panel::update(&mut pages.side_panel, m, log),
        PageMsg::Splitter(m) => splitter::update(&mut pages.splitter, m, log),
        PageMsg::WidgetDock(m) => widget_dock::update(&mut pages.widget_dock, m, log),
        PageMsg::Accordion(m) => accordion::update(&mut pages.accordion, m, log),
        PageMsg::TabsAdvanced(m) => tabs_advanced::update(&mut pages.tabs_advanced, m, log),
        PageMsg::TabRail(m) => tab_rail::update(&mut pages.tab_rail, m, log),
        PageMsg::Menu(m) => menu::update(&mut pages.menu, m, log),
        PageMsg::Breadcrumb(m) => breadcrumb::update(&mut pages.breadcrumb, m, log),
        PageMsg::Slider(m) => slider::update(&mut pages.slider, m, log),
        PageMsg::NumberInput(m) => number_input::update(&mut pages.number_input, m, log),
        PageMsg::TimeInput(m) => time_input::update(&mut pages.time_input, m, log),
        PageMsg::DurationInput(m) => duration_input::update(&mut pages.duration_input, m, log),
        PageMsg::TextArea(m) => text_area::update(&mut pages.text_area, m, log),
        PageMsg::Table(m) => table::update(&mut pages.table, m, log),
        PageMsg::Tree(m) => tree::update(&mut pages.tree, m, log),
        PageMsg::LogView(m) => log_view::update(&mut pages.log_view, m, log),
        PageMsg::FilePicker(m) => file_picker::update(&mut pages.file_picker, m, log),
        PageMsg::ExampleFileExplorer(m) => example_file_explorer::update(&mut pages.example_file_explorer, m, log),
        PageMsg::Terminal(m) => terminal::update(&mut pages.terminal, m, log),
        PageMsg::Form(m) => form::update(&mut pages.form, m, log),
        PageMsg::Steps(m) => steps::update(&mut pages.steps, m, log),
        PageMsg::SettingsList(m) => settings_list::update(&mut pages.settings_list, m, log),
        PageMsg::Wizard(m) => wizard::update(&mut pages.wizard, m, log),
        PageMsg::ExampleSetupWizard(m) => example_setup_wizard::update(&mut pages.example_setup_wizard, m, log),
        PageMsg::Clipboard(m) => clipboard::update(&mut pages.clipboard, m, log),
        PageMsg::AsyncTasks(m) => async_tasks::update(&mut pages.async_tasks, m, log),
        PageMsg::Handoff(m) => handoff::update(&mut pages.handoff, m, log),
        PageMsg::Storage(m) => storage::update(&mut pages.storage, m, log),
        PageMsg::DateTime(m) => date_time::update(&mut pages.date_time, m, log),
        PageMsg::Document(m) => document::update(&mut pages.document, m, log),
        PageMsg::PageTransitions(m) => {
            page_transitions::update(&mut pages.page_transitions, &mut pages.storage.settings, m, log)
        }
        PageMsg::TextSelection(m) => text_selection::update(&mut pages.text_selection, m, log),
        PageMsg::Slide(page, on) => slide_changed(pages, page, on, log),
    }
}

/// Draws the live demo of page `id`.
pub fn demo(pages: &Pages, id: &str, ui: &mut View<'_, Msg>) {
    match id {
        "getting-started" => getting_started::view(&pages.getting_started, ui),
        "theme-icons-language" => theme_icons_language::view(&pages.theme_icons_language, ui),
        "layout" => layout::view(&pages.layout, ui),
        "focus-keys" => focus_keys::view(&pages.focus_keys, ui),
        "text" => text::view(&pages.text, ui),
        "panel" => panel::view(&pages.panel, ui),
        "button" => button::view(&pages.button, ui),
        "text-input" => text_input::view(&pages.text_input, ui),
        "select" => select::view(&pages.select, ui),
        "list" => list::view(&pages.list, ui),
        "scroll-view" => scroll_view::view(&pages.scroll_view, ui),
        "tabs" => tabs::view(&pages.tabs, ui),
        "key-hints" => key_hints::view(ui),
        "markdown" => markdown::view(&pages.markdown, ui),
        "code-view" => code_view::view(&pages.code_view, ui),
        "motion" => motion::view(&pages.motion, ui),
        "spinner" => spinner::view(&pages.spinner, ui),
        "cell-animation" => cell_animation::view(&pages.cell_animation, ui),
        "shimmer-text" => shimmer_text::view(ui),
        "progress-bar" => progress_bar::view(&pages.progress_bar, ui),
        "checkbox" => checkbox::view(&pages.checkbox, ui),
        "switch" => switch::view(&pages.switch, ui),
        "segmented" => segmented::view(&pages.segmented, ui),
        "radio-group" => radio_group::view(&pages.radio_group, ui),
        "modal" => modal::view(&pages.modal, ui),
        "confirm" => confirm::view(&pages.confirm, ui),
        "hold-to-confirm" => hold_to_confirm::view(&pages.hold_to_confirm, ui),
        "help-layer" => help_layer::view(&pages.help_layer, ui),
        "command-palette" => command_palette::view(&pages.command_palette, ui),
        "popover" => popover::view(&pages.popover, ui),
        "tooltip" => tooltip::view(&pages.tooltip, ui),
        "context-menu" => context_menu::view(&pages.context_menu, ui),
        "toast" => toast::view(&pages.toast, ui),
        "date-picker" => date_picker::view(&pages.date_picker, ui),
        "badge" => badge::view(&pages.badge, ui),
        "divider" => divider::view(&pages.divider, ui),
        "empty-state" => empty_state::view(&pages.empty_state, ui),
        "skeleton" => skeleton::view(&pages.skeleton, ui),
        "sparkline" => sparkline::view(&pages.sparkline, ui),
        "gauge" => gauge::view(&pages.gauge, ui),
        "bar-chart" => bar_chart::view(&pages.bar_chart, ui),
        "heatmap" => heatmap::view(&pages.heatmap, ui),
        "timeline" => timeline::view(&pages.timeline, ui),
        "big-text" => big_text::view(&pages.big_text, ui),
        "example-dashboard" => example_dashboard::view(&pages.example_dashboard, ui),
        "scrollbar-styles" => scrollbar_styles::view(&pages.scrollbar_styles, ui),
        "side-panel" => side_panel::view(&pages.side_panel, ui),
        "splitter" => splitter::view(&pages.splitter, ui),
        "widget-dock" => widget_dock::view(&pages.widget_dock, ui),
        "accordion" => accordion::view(&pages.accordion, ui),
        "tabs-advanced" => tabs_advanced::view(&pages.tabs_advanced, ui),
        "tab-rail" => tab_rail::view(&pages.tab_rail, ui),
        "menu" => menu::view(&pages.menu, ui),
        "breadcrumb" => breadcrumb::view(&pages.breadcrumb, ui),
        "slider" => slider::view(&pages.slider, ui),
        "number-input" => number_input::view(&pages.number_input, ui),
        "time-input" => time_input::view(&pages.time_input, ui),
        "duration-input" => duration_input::view(&pages.duration_input, ui),
        "text-area" => text_area::view(&pages.text_area, ui),
        "table" => table::view(&pages.table, ui),
        "tree" => tree::view(&pages.tree, ui),
        "log-view" => log_view::view(&pages.log_view, ui),
        "file-picker" => file_picker::view(&pages.file_picker, ui),
        "example-file-explorer" => example_file_explorer::view(&pages.example_file_explorer, ui),
        "terminal" => terminal::view(&pages.terminal, ui),
        "form" => form::view(&pages.form, ui),
        "steps" => steps::view(&pages.steps, ui),
        "settings-list" => settings_list::view(&pages.settings_list, ui),
        "wizard" => wizard::view(&pages.wizard, ui),
        "example-setup-wizard" => example_setup_wizard::view(&pages.example_setup_wizard, ui),
        "clipboard" => clipboard::view(&pages.clipboard, ui),
        "async-tasks" => async_tasks::view(&pages.async_tasks, ui),
        "handoff" => handoff::view(&pages.handoff, ui),
        "storage" => storage::view(&pages.storage, ui),
        "date-time" => date_time::view(&pages.date_time, ui),
        "document" => document::view(&pages.document, ui),
        "page-transitions" => page_transitions::view(&pages.page_transitions, ui),
        "text-selection" => text_selection::view(&pages.text_selection, ui),
        _ => {}
    }
}

/// Tells the page on screen what the application heard on the clipboard.
pub fn clipboard_event(pages: &mut Pages, page: &str, event: &ClipboardEvent, log: &mut EventLog) {
    match page {
        "text-selection" => text_selection::heard(&mut pages.text_selection, event, log),
        // Pages with selectable or editable text log what their menus and keys copied.
        "clipboard" => log_clipboard("clipboard", event, log),
        "code-view" => log_clipboard("code-view", event, log),
        "markdown" => log_clipboard("markdown", event, log),
        "terminal" => log_clipboard("terminal", event, log),
        "text-input" => log_clipboard("text-input", event, log),
        "text-area" => log_clipboard("text-area", event, log),
        "number-input" => log_clipboard("number-input", event, log),
        "time-input" => log_clipboard("time-input", event, log),
        "duration-input" => log_clipboard("duration-input", event, log),
        _ => {}
    }
}

/// Logs a copy or an unclaimed paste on `page`, with how many characters it carried: a clean
/// and a raw copy of the same selection differ in length.
fn log_clipboard(page: &'static str, event: &ClipboardEvent, log: &mut EventLog) {
    let (verb, text) = match event {
        ClipboardEvent::Copied(text) => ("copied", text),
        ClipboardEvent::Pasted(text) => ("pasted", text),
    };
    log.push(page, "App::clipboard", format!("{verb} {} characters", text.chars().count()));
}

/// A labelled row of the playground panel: a faint label and a control.
pub fn setting(ui: &mut View<'_, Msg>, label: String, control: impl FnOnce(&mut View<'_, Msg>)) {
    ui.row(|ui| {
        ui.add(Text::new(label).role("secondary").no_wrap()).width(Length::Cells(24));
        control(ui);
    })
    .fill_width();
}

/// An on/off switch for the playground.
pub fn toggle(on: bool, message: impl Fn(bool) -> Msg + 'static) -> Switch<Msg> {
    Switch::new(on).on_toggle(message)
}

/// The playground row that turns the selection slide on and off on a row page (`page`), so the
/// marks and chevrons can be seen to stay put. The slide is one setting of the whole
/// application: this switch and the one in Theme, icons and language change and save the same
/// value.
pub fn slide_setting(ui: &mut View<'_, Msg>, page: &'static str) {
    let on = ui.env().slide();
    setting(ui, t!("rows.slide"), |ui| {
        ui.add(toggle(on, move |on| Msg::Page(PageMsg::Slide(page, on)))).id("slide");
    });
    ui.add(Text::new(t!("rows.slide-hint")).role("faint"));
}

/// Applies the slide switch of a row page, remembers it and writes it to the page's log.
fn slide_changed(pages: &mut Pages, page: &'static str, on: bool, log: &mut EventLog) -> Command<Msg> {
    log.push(page, "Playground", format!("slide = {on}"));
    storage::apply_and_remember(&mut pages.storage.settings, Settings::SLIDE, on, Command::set_slide(on))
}

/// Where `mark` and `label` start, in cells, on the screen row showing `label` after `mark` and
/// some space. Lets the row pages check that marks stay put while labels slide.
#[cfg(test)]
pub fn mark_and_label(h: &qframe::runtime::Harness<crate::app::Showcase>, mark: char, label: &str) -> (usize, usize) {
    let screen = h.screen();
    screen
        .lines()
        .find_map(|line| {
            let chars: Vec<char> = line.chars().collect();
            let label_at = line.find(label).map(|byte| line[..byte].chars().count())?;
            let mark_at = chars[..label_at].iter().rposition(|c| *c == mark)?;
            chars[mark_at + 1..label_at].iter().all(|c| *c == ' ').then_some((mark_at, label_at))
        })
        .unwrap_or_else(|| panic!("no `{mark}` before `{label}`:\n{screen}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn the_slide_switch_of_a_row_page_is_the_one_application_setting() {
        let mut h = showcase_on("list");
        let (x, y) = h.find("Slide on selection").expect("the playground shows the slide switch");
        // The switch sits after the playground's label column.
        h.click(x + 25, y);
        assert!(!h.env().slide());
        assert_eq!(h.app().pages.storage.settings.slide(), Some(false), "the choice is remembered");
        let last = h.app().log.recent("list", 1).first().map(|entry| entry.message.clone());
        assert_eq!(last.as_deref(), Some("slide = false"));
        h.send(Msg::Page(PageMsg::Slide("list", true)));
        assert!(h.env().slide());
    }
}
