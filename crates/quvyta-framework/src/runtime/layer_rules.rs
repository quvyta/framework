//! The rules of the one layer stack, checked across the widgets that use it: modal dialogs,
//! popovers, toasts, text selection, key listeners and the keymap.

use std::time::Duration;

use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, Modal, NumberInput, Popover, Text, TextArea, TextInput, Toast};

#[derive(Default)]
struct Demo {
    dialog: bool,
    popover: bool,
    log: Vec<String>,
    text: String,
    notes: String,
    number: f64,
}

#[derive(Clone)]
enum Msg {
    Dialog(bool),
    Popover(bool),
    Log(&'static str),
    Text(String),
    Notes(String),
    Number(f64),
    Toast,
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Dialog(open) => self.dialog = open,
            Msg::Popover(open) => self.popover = open,
            Msg::Log(entry) => self.log.push(entry.to_owned()),
            Msg::Text(text) => self.text = text,
            Msg::Notes(text) => self.notes = text,
            Msg::Number(value) => self.number = value,
            Msg::Toast => return Command::toast(Toast::info("Deploy finished").action("Undo", Msg::Log("undo"))),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            ui.add(Text::new("Production deploys run every hour from the main branch.")).selectable(true);
            ui.add(Button::new("Open").on_press(Msg::Dialog(true))).id("open");
            ui.add(TextInput::new(&self.text).on_change(Msg::Text)).id("text");
            ui.add(TextArea::new(&self.notes).on_change(Msg::Notes)).id("notes");
            ui.add(NumberInput::new(self.number).on_change(Msg::Number)).id("number");
            if self.dialog {
                ui.add_with(Modal::new().title("Filters").on_close(Msg::Dialog(false)), |ui| {
                    Popover::new(self.popover)
                        .on_dismiss(Msg::Popover(false))
                        .anchor(|ui| {
                            ui.add(Button::new("Regions").on_press(Msg::Popover(true))).id("regions");
                        })
                        .content(|ui| {
                            ui.add(Text::new("Europe only"));
                        })
                        .show(ui);
                    ui.add(Button::new("Apply").on_press(Msg::Log("apply"))).id("apply");
                });
            }
        });
    }
    fn action(&self, name: &str) -> Option<Msg> {
        match name {
            "help" => Some(Msg::Log("help")),
            "copy" => Some(Msg::Log("copy")),
            "paste" => Some(Msg::Log("paste")),
            "toggle-panel" => Some(Msg::Log("toggle-panel")),
            _ => None,
        }
    }
}

fn open_dialog() -> Harness<Demo> {
    let mut h = Harness::new(Demo::default(), 70, 24);
    h.click_text("Open").advance(Duration::from_millis(200));
    h
}

#[test]
fn a_popover_inside_a_dialog_closes_on_a_press_in_the_dialog_and_the_dialog_stays() {
    let mut h = open_dialog();
    h.click_text("Regions").advance(Duration::from_millis(200));
    assert!(h.screen().contains("Europe only"), "{}", h.screen());
    h.click_text("Filters");
    assert!(!h.app().popover, "a press outside the popover closes it");
    assert!(h.app().dialog, "the dialog beneath it stays");
    assert!(h.app().log.is_empty(), "the closing press reached nothing");
    h.press("esc");
    assert!(!h.app().dialog);
}

#[test]
fn focus_moves_into_the_dialog_and_back_through_focus_requests() {
    let mut h = open_dialog();
    assert!(h.is_focused("regions"), "the first focusable widget inside takes focus");
    h.press("tab").press("tab");
    assert!(h.is_focused("regions"), "tab stays inside the dialog");
    h.press("esc");
    assert!(h.is_focused("open"), "focus returns to the button that opened the dialog");
}

#[test]
fn toasts_stay_clickable_above_a_dialog() {
    let mut h = open_dialog();
    h.send(Msg::Toast).advance(Duration::from_millis(300));
    h.click_text("Undo");
    assert_eq!(h.app().log, ["undo"]);
    assert!(h.app().dialog, "the toast press did not reach the dialog");
}

#[test]
fn presses_on_the_dimmed_screen_never_start_a_text_selection() {
    let mut plain = Harness::new(Demo::default(), 70, 24);
    let (x, y) = plain.find("Production").expect("text");
    plain.drag((x, y), (x + 9, y)).press("ctrl+c");
    assert_eq!(plain.clipboard(), Some("Production"), "without a dialog the text is selectable");
    let mut h = open_dialog();
    let (x, y) = h.find("Production").expect("text beneath the dialog");
    h.drag((x, y), (x + 20, y)).press("ctrl+c");
    assert_eq!(h.clipboard(), None, "nothing was selected beneath the dialog");
    assert!(h.app().log.is_empty(), "copy is not passed to the application");
}

#[test]
fn a_question_mark_types_into_fields_and_opens_help_elsewhere() {
    let mut h = Harness::new(Demo::default(), 70, 24);
    h.press("tab").press("tab").type_text("why?");
    assert_eq!(h.app().text, "why?");
    h.press("tab").type_text("ok?");
    assert_eq!(h.app().notes, "ok?");
    h.press("tab").type_text("?");
    assert!(h.app().log.is_empty(), "a number field keeps the key: {:?}", h.app().log);
    h.press("tab").press("?");
    assert_eq!(h.app().log, ["help"], "unused ? reaches App::action");
}

#[test]
fn runtime_owned_global_keys_do_not_reach_the_application_and_dialogs_pause_the_rest() {
    let mut h = Harness::new(Demo::default(), 70, 24);
    h.press("ctrl+v").press("ctrl+c").press("alt+b");
    assert!(h.app().log.is_empty(), "{:?}", h.app().log);
    let mut h = open_dialog();
    h.press("?");
    assert!(h.app().log.is_empty(), "shortcuts pause while a dialog is open");
}

// Presses that close a non-modal layer still act on what they land on.

mod click_through {
    use std::time::Duration;

    use crate::date::Date;
    use crate::event::{MouseButton, MouseKind};
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};
    use crate::widgets::{
        Button, ContextItem, ContextMenu, DatePicker, List, ListItem, Modal, Popover, Select, Text, Toast, Tooltip,
    };

    #[derive(Default)]
    struct Settings {
        language: Option<usize>,
        theme: Option<usize>,
        row: Option<usize>,
        log: Vec<&'static str>,
        dialog: bool,
        filters: bool,
        date: Option<Date>,
    }

    #[derive(Clone)]
    enum Msg {
        Language(usize),
        Theme(usize),
        Row(usize),
        Log(&'static str),
        Dialog(bool),
        Filters(bool),
        ToggleFilters,
        Date(Date),
        Notify,
    }

    impl App for Settings {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Language(index) => self.language = Some(index),
                Msg::Theme(index) => self.theme = Some(index),
                Msg::Row(index) => self.row = Some(index),
                Msg::Log(entry) => self.log.push(entry),
                Msg::Dialog(open) => self.dialog = open,
                Msg::Filters(open) => self.filters = open,
                Msg::ToggleFilters => self.filters = !self.filters,
                Msg::Date(date) => self.date = Some(date),
                Msg::Notify => return Command::toast(Toast::info("Deploy finished").action("Undo", Msg::Log("undo"))),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.row(|ui| {
                    ui.add(Select::new(["English", "Türkçe"]).selected(self.language).on_select(Msg::Language))
                        .width(Length::Cells(14))
                        .id("language");
                    ui.add(Select::new(["Monochrome", "Iris", "Nordic"]).selected(self.theme).on_select(Msg::Theme))
                        .width(Length::Cells(16))
                        .id("theme");
                    ui.add(Button::new("Deploy").on_press(Msg::Log("deploy"))).id("deploy");
                    ui.add(Button::new("Filters").on_press(Msg::ToggleFilters)).id("toggle");
                })
                .gap(1);
                let today = Date::new(2026, 9, 16).expect("valid");
                ui.row(|ui| {
                    ui.add(DatePicker::new(self.date).today(today).placeholder("Release").on_change(Msg::Date))
                        .width(Length::Cells(20))
                        .id("date");
                    ui.add_with(Tooltip::new("Opens the runbook"), |ui| {
                        ui.add(Button::new("Runbook").on_press(Msg::Log("runbook"))).id("runbook");
                    });
                    ui.add(Button::new("Dialog").on_press(Msg::Dialog(true))).id("dialog");
                })
                .gap(1);
                Popover::new(self.filters)
                    .on_dismiss(Msg::Filters(false))
                    .anchor(|ui| {
                        ui.add(Text::new("filters anchor"));
                    })
                    .content(|ui| {
                        ui.add(Text::new("only running"));
                    })
                    .show(ui);
                let items = [ContextItem::new("Restart", Msg::Log("restart"))];
                ui.add_with(ContextMenu::new(items), |ui| {
                    let rows = ["api-gateway", "billing", "worker"].map(ListItem::new);
                    ui.add(List::new(rows).selected(self.row).on_select(Msg::Row)).height(Length::Cells(3)).id("list");
                })
                .height(Length::Cells(3));
                if self.dialog {
                    ui.add_with(Modal::new().title("Remove web?").width(30).on_close(Msg::Dialog(false)), |ui| {
                        ui.add(Text::new("Its volumes go too."));
                    });
                }
            });
        }
    }

    const UNFOLD: Duration = Duration::from_millis(300);

    fn settings() -> Harness<Settings> {
        Harness::new(Settings { language: Some(0), theme: Some(0), ..Settings::default() }, 90, 24)
    }

    #[test]
    fn one_press_on_another_dropdown_closes_the_open_one_and_opens_it() {
        let mut h = settings();
        h.click_text("English").advance(UNFOLD);
        assert!(h.screen().contains("Türkçe"), "{}", h.screen());
        h.click_text("Monochro").advance(UNFOLD);
        let screen = h.screen();
        assert!(!screen.contains("Türkçe"), "the language list closed:\n{screen}");
        assert!(screen.contains("Nordic"), "the theme list opened with the same press:\n{screen}");
        assert!(h.is_focused("theme"));
        h.press("down").press("enter");
        assert_eq!((h.app().language, h.app().theme), (Some(0), Some(1)), "the keyboard went to the theme list");
    }

    #[test]
    fn a_press_on_a_button_with_a_dropdown_open_fires_the_button() {
        let mut h = settings();
        h.click_text("English").advance(UNFOLD);
        h.click_text("Deploy");
        assert_eq!(h.app().log, ["deploy"]);
        assert!(!h.screen().contains("Türkçe"), "{}", h.screen());
        h.press("enter");
        assert_eq!(h.app().log, ["deploy", "deploy"], "keys went back to normal routing");
    }

    #[test]
    fn a_press_on_a_toast_closes_an_open_dropdown_and_acts_on_the_toast() {
        let mut h = settings();
        h.send(Msg::Notify).advance(UNFOLD);
        h.click_text("English").advance(UNFOLD);
        assert!(h.screen().contains("Türkçe"), "{}", h.screen());
        h.click_text("Undo").advance(UNFOLD);
        assert_eq!(h.app().log, ["undo"]);
        assert!(!h.screen().contains("Türkçe"), "the dropdown closed:\n{}", h.screen());
    }

    #[test]
    fn a_press_on_the_open_dropdowns_own_field_closes_it_without_reopening() {
        let mut h = settings();
        h.click_text("English").advance(UNFOLD);
        h.click_text("English").advance(UNFOLD);
        assert!(!h.screen().contains("Türkçe"), "{}", h.screen());
        h.click_text("English").advance(UNFOLD);
        assert!(h.screen().contains("Türkçe"), "a third press opens it again");
    }

    #[test]
    fn a_press_outside_a_modal_changes_nothing() {
        let mut h = settings();
        h.click_text("Dialog").advance(UNFOLD);
        for target in ["Deploy", "English", "worker"] {
            h.click_text(target).advance(UNFOLD);
        }
        let app = h.app();
        assert!(app.dialog && app.log.is_empty() && app.row.is_none() && app.language == Some(0));
        assert!(!h.screen().contains("Türkçe"), "{}", h.screen());
    }

    #[test]
    fn a_press_on_a_list_row_with_a_context_menu_open_selects_the_row() {
        let mut h = settings();
        h.set_reduced_motion(true);
        let (x, y) = h.find("api-gateway").expect("row");
        h.mouse(MouseKind::Down(MouseButton::Right), x + 2, y);
        h.mouse(MouseKind::Up(MouseButton::Right), x + 2, y);
        assert!(h.screen().contains("Restart"), "{}", h.screen());
        h.click_text("worker");
        assert_eq!(h.app().row, Some(2));
        assert!(!h.screen().contains("Restart"), "{}", h.screen());
        assert!(h.app().log.is_empty());
    }

    #[test]
    fn a_calendar_closes_on_a_press_elsewhere_and_the_press_acts() {
        let mut h = settings();
        h.click_text("Release").advance(UNFOLD);
        assert!(h.screen().contains("September 2026"), "{}", h.screen());
        h.click_text("Deploy");
        assert_eq!(h.app().log, ["deploy"]);
        assert!(!h.screen().contains("September 2026"));
        h.click_text("Release").advance(UNFOLD).click_text("Release");
        assert!(!h.screen().contains("September 2026"), "its own field only closes it");
    }

    #[test]
    fn a_popover_opened_elsewhere_closes_on_its_opener_and_passes_other_presses() {
        let mut h = settings();
        h.click_text("Filters").advance(UNFOLD);
        assert!(h.screen().contains("only running"), "{}", h.screen());
        h.click_text("Filters").advance(UNFOLD);
        assert!(!h.app().filters, "the button that opened it only closes it, it does not reopen");
        h.click_text("Filters").advance(UNFOLD);
        h.click_text("Deploy");
        assert_eq!((h.app().filters, h.app().log.as_slice()), (false, &["deploy"][..]));
    }

    #[test]
    fn a_tooltip_never_stands_in_the_way() {
        let mut h = settings();
        let (x, y) = h.find("Runbook").expect("button");
        h.hover(x, y).advance(Duration::from_secs(1));
        assert!(h.screen().contains("Opens the runbook"), "{}", h.screen());
        h.click(x, y);
        assert_eq!(h.app().log, ["runbook"]);
        h.click_text("Deploy");
        assert_eq!(h.app().log, ["runbook", "deploy"]);
    }
}

// Events that arrive together reach the widgets before the next frame.

mod batches {
    use crate::event::{Event, KeyEvent, MouseButton, MouseEvent, MouseKind};
    use crate::keymap::Modifiers;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};
    use crate::widgets::{ContextItem, ContextMenu, Text};

    #[derive(Default)]
    struct Services {
        chosen: Vec<&'static str>,
    }

    impl App for Services {
        type Msg = &'static str;
        fn update(&mut self, msg: &'static str) -> Command<&'static str> {
            self.chosen.push(msg);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, &'static str>) {
            let items = [
                ContextItem::new("Restart", "restart"),
                ContextItem::submenu("Move to", [ContextItem::new("Staging", "staging")]),
            ];
            ui.add_with(ContextMenu::new(items), |ui| {
                ui.add(Text::new("api-gateway"));
            })
            .height(Length::Cells(10))
            .fill_width();
        }
    }

    fn mouse(kind: MouseKind, (x, y): (i32, i32)) -> Event {
        Event::Mouse(MouseEvent { kind, x, y, mods: Modifiers::default() })
    }

    #[test]
    fn a_left_key_and_a_click_on_the_closed_submenu_in_one_batch_do_not_panic() {
        let mut h = Harness::new(Services::default(), 40, 12);
        h.set_reduced_motion(true);
        h.mouse(MouseKind::Down(MouseButton::Right), 3, 0).mouse(MouseKind::Up(MouseButton::Right), 3, 0);
        h.press("m").press("right");
        let staging = h.find("Staging").expect("the submenu is open");
        h.events(&[
            Event::Key(KeyEvent::press("left")),
            mouse(MouseKind::Down(MouseButton::Left), staging),
            mouse(MouseKind::Up(MouseButton::Left), staging),
        ]);
        assert!(!h.screen().contains("Staging"), "{}", h.screen());
        assert!(h.app().chosen.is_empty(), "the press landed where the closed submenu was drawn");
    }
}
