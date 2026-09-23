//! Ready-made widgets. Every one takes its look from the theme and works with keyboard and mouse.

mod accordion;
mod app_shell;
mod appearance;
mod axis;
mod badge;
mod bar_chart;
mod big_text;
mod boundary;
mod breadcrumb;
mod button;
mod card_grid;
mod cells;
mod checkbox;
mod close_mark;
mod code_view;
mod command_palette;
mod context_item;
mod context_menu;
mod copy_value;
mod date_picker;
mod delayed;
mod divider;
mod duration_input;
mod edge_scroll;
pub(crate) mod edit_menu;
mod editor;
mod eighths;
mod empty_state;
mod field;
mod file_browser;
mod file_manager;
mod file_picker;
mod filter;
#[cfg(test)]
mod floating_tests;
mod form;
mod form_errors;
mod gauge;
mod ghost;
mod heatmap;
mod help_layer;
mod highlight;
mod hold_to_confirm;
#[cfg(test)]
mod huge_text_tests;
mod icon_button;
#[cfg(test)]
mod icon_button_tests;
mod key_hints;
mod layer;
mod legend;
mod list;
mod log_buffer;
mod log_view;
mod markdown;
mod menu;
mod modal;
mod number_input;
mod numeric;
mod page_transition;
mod panel;
mod placement;
mod popover;
mod popup_menu;
mod press;
mod progress_bar;
mod radio_group;
mod row;
mod row_menu;
mod rows;
mod scroll_view;
mod scrollbar;
mod sections;
mod segmented;
mod select;
mod settings_list;
mod setup;
mod shimmer_text;
mod side_panel;
#[cfg(test)]
mod sixteen_colours_tests;
mod skeleton;
mod slider;
mod sparkline;
mod spinner;
mod splitter;
mod steps;
mod switch;
mod tab_model;
mod tab_rail;
mod table;
mod tabs;
mod task_list;
#[cfg(feature = "pty")]
mod terminal;
#[cfg(feature = "pty")]
mod terminal_mouse;
#[cfg(feature = "pty")]
mod terminal_notice;
#[cfg(feature = "pty")]
mod terminal_session;
mod text;
mod text_area;
mod text_input;
mod text_rows;
mod time_input;
mod timeline;
mod toast;
mod tooltip;
mod tree;
#[cfg(test)]
mod wide_under_layer_tests;
mod widget_dock;
mod window;
#[cfg(test)]
mod window_tests;
mod wizard;

pub use accordion::Accordion;
pub use app_shell::AppShell;
pub use appearance::{Appearance, AppearanceChange};
pub use axis::Axis;
pub use badge::Badge;
pub use bar_chart::{Bar, BarChart, Series};
pub use big_text::{BigText, Gradient};
pub use breadcrumb::Breadcrumb;
pub use button::Button;
pub use card_grid::CardGrid;
pub use checkbox::{Checkbox, CheckboxStyle};
pub use code_view::{CodeView, LineMark, LineTone};
pub use command_palette::{CommandPalette, PaletteCommand};
pub use context_item::ContextItem;
pub use context_menu::ContextMenu;
pub use copy_value::CopyValue;
pub use date_picker::DatePicker;
pub use divider::Divider;
pub use duration_input::{DurationError, DurationInput, DurationUnit, parse_duration};
pub use empty_state::EmptyState;
pub use field::Field;
pub use file_browser::{FileBrowser, FileEntry, FilePickerMsg, Listing, ListingError, PickMode, read_folder};
pub use file_manager::{
    FileChange, FileDetails, FileError, FileManager, FileManagerMsg, FileManagerState, FileView, FileWork, FolderEntry,
    NameFor, NameProblem, Naming, RowMark, child_key, copy_into, is_inside, is_within, name_of, parent_key,
};
pub use file_picker::FilePicker;
pub use form::{Form, FormFields};
pub use form_errors::FormErrors;
pub use gauge::Gauge;
pub use ghost::Ghost;
pub use heatmap::Heatmap;
pub use help_layer::HelpLayer;
pub use highlight::Language;
pub use hold_to_confirm::HoldToConfirm;
pub use icon_button::IconButton;
pub use key_hints::KeyHints;
pub use legend::Legend;
pub use list::{ItemKind, List, ListItem};
pub use log_buffer::{LogBuffer, LogLevel, LogLine};
pub use log_view::LogView;
pub use markdown::Markdown;
pub use menu::{Menu, MenuGroup, MenuItem};
pub use modal::Modal;
pub use number_input::NumberInput;
pub use page_transition::PageTransition;
pub use panel::Panel;
pub use placement::Placement;
pub use popover::Popover;
pub use progress_bar::ProgressBar;
pub use radio_group::{RadioGroup, RadioStyle};
pub use scroll_view::ScrollView;
pub use scrollbar::{ScrollMetrics, ScrollbarStyle};
pub use sections::Section;
pub use segmented::Segmented;
pub use select::Select;
pub use settings_list::{SettingRow, SettingsList, SettingsRows};
pub use setup::{Setup, SetupMsg, SetupWizard};
pub use shimmer_text::{ShimmerStyle, ShimmerText};
pub use side_panel::{Closed, Side, SidePanel};
pub use skeleton::Skeleton;
pub use slider::Slider;
pub use sparkline::Sparkline;
pub use spinner::{Spinner, SpinnerStyle};
pub use splitter::Splitter;
pub use steps::Steps;
pub use switch::{Switch, SwitchStyle};
pub use tab_model::TabEdit;
pub use tab_rail::{CollapsedMarker, RailTab, TabRail};
pub use table::{Column, ColumnWidth, SortDirection, Table, TableCell, TableRow};
pub use tabs::{Overflow, TabWidth, Tabs};
pub use task_list::TaskList;
#[cfg(feature = "pty")]
pub use terminal::Terminal;
#[cfg(feature = "pty")]
pub use terminal_session::{TerminalBuilder, TerminalChange, TerminalEvent, TerminalSession, TerminalWatch};
pub use text::{Span, Text};
pub use text_area::TextArea;
pub use text_input::TextInput;
pub use time_input::TimeInput;
pub use timeline::{TimeBlock, Timeline};
// `TimeOfDay` lives in `date`, next to `Date`; it is re-exported here because the time field is
// where most applications meet it.
pub use crate::date::TimeOfDay;
pub use toast::{Corner, Toast, ToastKind};
pub(crate) use toast::{ToastPress, ToastStack};
pub use tooltip::Tooltip;
pub use tree::{Tree, TreeDrop, TreeMove, TreeNode};
pub use widget_dock::WidgetDock;
pub use window::{Window, WindowEdge, WindowEvent};
pub use wizard::Wizard;

/// Builds a message from a chosen index.
type IndexMessage<Msg> = Box<dyn Fn(usize) -> Msg>;

/// Builds a message from a new on/off state.
type ToggleMessage<Msg> = Box<dyn Fn(bool) -> Msg>;
