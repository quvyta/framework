//! Every widget that shows application text draws, measures and handles input without panicking
//! when that text is wider than any screen. Widths are `u16` cells, and a width saturates at
//! `u16::MAX` rather than wrapping, so every sum over widths must saturate too. The same holds for
//! theme padding: a theme may set any cell count, and twice that must not overflow.

use std::time::Duration;

use crate::event::{MouseButton, MouseKind};
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{
    Accordion, Badge, Bar, BarChart, BigText, Breadcrumb, Button, CardGrid, Checkbox, CodeView, Column, CommandPalette,
    ContextItem, ContextMenu, CopyValue, DatePicker, Divider, EmptyState, Field, Form, Gauge, HelpLayer, HoldToConfirm,
    KeyHints, Language, List, ListItem, LogBuffer, LogLevel, LogLine, LogView, Markdown, Menu, MenuGroup, MenuItem,
    Modal, NumberInput, Overflow, PaletteCommand, Panel, Popover, ProgressBar, RadioGroup, RailTab, Section, Segmented,
    Select, SettingRow, SettingsList, ShimmerText, Slider, Spinner, Steps, Switch, TabRail, TabWidth, Table, TableCell,
    TableRow, Tabs, Text, TextArea, TextInput, Tooltip, Tree, TreeNode, WidgetDock, Wizard,
};

/// Just wider than `u16::MAX` cells: 32 800 double-width characters.
fn huge() -> String {
    "中".repeat(32_800)
}

/// Builds one widget showing `text`.
type Build = fn(&str, &mut View<'_, ()>);

const WIDGETS: &[(&str, Build)] = &[
    ("accordion", |t, ui| {
        ui.add(Accordion::new([Section::new(t).detail(t)]).open([true]).on_toggle(|_, _| ()));
    }),
    ("badge", |t, ui| {
        ui.add(Badge::new(t).count(3));
    }),
    ("bar-chart", |t, ui| {
        ui.add(BarChart::new([Bar::new(t, 3.0).value_text(t)]));
    }),
    ("bar-chart-vertical", |t, ui| {
        ui.add(BarChart::new([Bar::new(t, 3.0).value_text(t)]).vertical());
    }),
    ("big-text", |t, ui| {
        ui.add(BigText::new(t));
    }),
    ("breadcrumb", |t, ui| {
        ui.add(Breadcrumb::new([t, t, t]).on_select(|_| ()));
    }),
    ("button", |t, ui| {
        ui.add(Button::new(t).shortcut(t).on_press(()));
    }),
    ("checkbox", |t, ui| {
        ui.add(Checkbox::new(true).label(t).on_toggle(|_| ()));
    }),
    ("card-grid", |t, ui| {
        let text = t.to_owned();
        let grid = CardGrid::new(40).checked(vec![true; 40]).on_select(|_| ()).on_activate(|_| ()).on_toggle(|_| ());
        ui.add(grid.card(move |ui, _| {
            ui.add(Text::new(text.as_str()).no_wrap());
            ui.add(Text::new(text.as_str()));
        }));
    }),
    ("code-view", |t, ui| {
        ui.add(CodeView::new(t, Language::Rust).line_numbers(true));
    }),
    ("command-palette", |t, ui| {
        let commands = [PaletteCommand::new("deploy", t, ()).chord(t), PaletteCommand::new("stop", t, ())];
        ui.add(CommandPalette::new(commands, ()).placeholder(t));
    }),
    ("context-menu", |t, ui| {
        let items = vec![
            ContextItem::new(t, ()).icon("file").shortcut(t),
            ContextItem::submenu(t, [ContextItem::new(t, ()).shortcut(t)]),
        ];
        ui.add_with(ContextMenu::new(items), |ui| {
            ui.add(Text::new(t));
        });
    }),
    ("copy-value", |t, ui| {
        ui.add(CopyValue::new(t).on_copy(()));
    }),
    ("date-picker", |t, ui| {
        ui.add(DatePicker::new(None).placeholder(t).on_change(|_| ()));
    }),
    ("divider", |t, ui| {
        ui.add(Divider::new().label(t));
    }),
    ("empty-state", |t, ui| {
        ui.add(EmptyState::new(t).message(t).action(Button::new(t).on_press(())));
    }),
    ("form", |t, ui| {
        Form::new().show(ui, |form| {
            form.field(Field::new(t).hint(t).required(true), |ui| {
                ui.add(TextInput::new(t));
            });
        });
    }),
    ("gauge", |t, ui| {
        ui.add(Gauge::new(0.5).label(t).value_text(t));
    }),
    ("help-layer", |t, ui| {
        ui.add(HelpLayer::new(()).hint(t, t));
    }),
    ("hold-to-confirm", |t, ui| {
        ui.add(HoldToConfirm::new(t).on_confirm(()));
    }),
    ("key-hints", |t, ui| {
        ui.add(KeyHints::new().hint(t, t)).fill_width();
    }),
    ("list", |t, ui| {
        ui.add(List::new([ListItem::new(t).detail(t).icon("file", None)]).checked(vec![true]).on_select(|_| ()));
    }),
    ("markdown", |t, ui| {
        ui.add(Markdown::new(&format!("# {t}\n\n- {t}\n\n> {t}\n\n```\n{t}\n```\n")));
    }),
    ("menu", |t, ui| {
        let item = MenuItem::new("key", t).badge(t).icon("file", None);
        ui.add(Menu::new([MenuGroup::new("group", [item]).title(t)]).on_select(|_| ()));
    }),
    ("modal", |t, ui| {
        ui.add_with(Modal::new().title(t).on_close(()).action(Button::new(t).on_press(())), |ui| {
            ui.add(Text::new(t));
        });
    }),
    ("number-input", |t, ui| {
        ui.add(NumberInput::new(1.0).placeholder(t).on_change(|_| ()));
    }),
    ("panel", |t, ui| {
        ui.add_with(Panel::new().title(t), |ui| {
            ui.add(Text::new(t));
        });
    }),
    ("popover", |t, ui| {
        Popover::new(true)
            .anchor(|ui| {
                ui.add(Button::new(t).on_press(()));
            })
            .content(|ui| {
                ui.add(Text::new(t));
            })
            .on_dismiss(())
            .show(ui);
    }),
    ("progress-bar", |_, ui| {
        ui.add(ProgressBar::new(0.5).percent(true));
    }),
    ("radio-group", |t, ui| {
        ui.add(RadioGroup::new([t, t]).horizontal(true).on_select(|_| ()));
    }),
    ("segmented", |t, ui| {
        ui.add(Segmented::new([t, t]).on_select(|_| ()));
    }),
    ("select", |t, ui| {
        ui.add(Select::new([t, t]).placeholder(t).on_select(|_| ()));
    }),
    ("settings-list", |t, ui| {
        SettingsList::show(ui, |list| {
            list.heading(t);
            list.row(SettingRow::new(t).description(t).on_activate(()), |ui| {
                ui.add(Switch::new(true).label(t));
            });
        });
    }),
    ("shimmer-text", |t, ui| {
        ui.add(ShimmerText::new(t));
    }),
    ("slider", |t, ui| {
        let label = t.to_owned();
        ui.add(Slider::new(0.5).suffix(t).format(move |_| label.clone()).on_change(|_| ()));
    }),
    ("spinner", |t, ui| {
        ui.add(Spinner::new().label(t));
    }),
    ("steps", |t, ui| {
        ui.add(Steps::new([t, t]).on_select(|_| ()));
    }),
    ("steps-vertical", |t, ui| {
        ui.add(Steps::new([t, t]).vertical(true).on_select(|_| ()));
    }),
    ("switch", |t, ui| {
        ui.add(Switch::new(true).label(t).on_toggle(|_| ()));
    }),
    ("table", |t, ui| {
        let row = TableRow::new([TableCell::new(t).icon("file", None), TableCell::new(t)]);
        ui.add(Table::new([Column::new(t), Column::new(t)], vec![row]).on_select(|_| ()));
    }),
    ("tab-rail", |t, ui| {
        let tabs = [RailTab::new(t).icon("folder").badge(t).status("success"), RailTab::new(t)];
        let rail = TabRail::new(tabs)
            .on_select(|_| ())
            .closable(|_| ())
            .reorderable(|_, _| ())
            .on_add(|| ())
            .context_menu(|_| vec![ContextItem::new("Close", ())]);
        ui.add(rail).fill_height();
    }),
    ("tab-rail-collapsed", |t, ui| {
        let tabs = [RailTab::new(t).badge(t), RailTab::new(t)];
        ui.add(TabRail::new(tabs).collapsed(true).row_height(3).on_select(|_| ())).fill_height();
    }),
    ("tabs", |t, ui| {
        let tabs = Tabs::new([t, t])
            .numbered(true)
            .on_select(|_| ())
            .closable(|_| ())
            .overflow(Overflow::Arrows)
            .reorderable(|_, _| ())
            .context_menu(|_| vec![ContextItem::new("Close", ())]);
        ui.add(tabs).fill_width();
    }),
    ("tabs-fixed", |t, ui| {
        ui.add(Tabs::new([t, t, t]).tab_width(TabWidth::Fill).overflow(Overflow::Menu).on_select(|_| ())).fill_width();
    }),
    ("text", |t, ui| {
        ui.add(Text::new(t).no_wrap());
    }),
    ("text-area", |t, ui| {
        ui.add(TextArea::new(t).on_change(|_| ()));
    }),
    ("text-input", |t, ui| {
        ui.add(TextInput::new(t).placeholder(t).on_change(|_| ()));
    }),
    ("tooltip", |t, ui| {
        ui.add_with(Tooltip::new(t), |ui| {
            ui.add(Button::new("Restart").on_press(()));
        });
    }),
    ("tree", |t, ui| {
        ui.add(Tree::new([TreeNode::new("root", t).detail(t)]));
    }),
    ("widget-dock", |t, ui| {
        ui.add(WidgetDock::new([Section::new(t).detail(t)]).open([true]).on_toggle(|_, _| ()));
    }),
    ("wizard", |t, ui| {
        Wizard::new([t, t]).on_next(()).on_back(()).on_cancel(()).show(ui, |ui| {
            ui.add(Text::new(t));
        });
    }),
];

struct Demo {
    build: Build,
    text: String,
    log: LogBuffer,
}

impl App for Demo {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.column(|ui| {
            (self.build)(&self.text, ui);
            if self.log.is_empty() {
                return;
            }
            ui.add(LogView::new(&self.log).search(self.text.chars().take(3).collect::<String>()));
        });
    }
}

/// Draws, focuses, clicks and resizes `build` with `text`.
fn exercise(name: &str, build: Build, text: &str) {
    exercise_in(crate::env::Env::builtin(), name, build, text);
}

/// [`exercise`] under `env`.
fn exercise_in(env: crate::env::Env, name: &str, build: Build, text: &str) {
    let mut log = LogBuffer::new(4);
    if name == "text" {
        log.push(LogLine::new(LogLevel::Info, text));
    }
    let mut h = Harness::with_env(Demo { build, text: text.to_owned(), log }, env, 60, 12);
    h.set_glyph_mode(GlyphMode::Ascii);
    h.press("tab").press("enter").press("end");
    h.hover(3, 0).click(1, 0).mouse(MouseKind::ScrollDown, 2, 1);
    // Context menus open from the menu key and a right click, measuring their items.
    h.press("menu").press("down").press("right").press("esc").press("esc");
    h.mouse(MouseKind::Down(MouseButton::Right), 1, 0).mouse(MouseKind::Up(MouseButton::Right), 1, 0);
    h.hover(4, 2).press("esc");
    h.advance(Duration::from_millis(400));
    for (width, height) in [(1, 1), (300, 3)] {
        h.resize(width, height);
    }
}

#[test]
fn widgets_survive_text_wider_than_any_screen() {
    let text = huge();
    // One thread per widget keeps the sweep quick in debug builds, and a panic names its widget.
    let failed: Vec<&str> = std::thread::scope(|scope| {
        let runs: Vec<_> =
            WIDGETS.iter().map(|(name, build)| (*name, scope.spawn(|| exercise(name, *build, &text)))).collect();
        runs.into_iter().filter_map(|(name, run)| run.join().is_err().then_some(name)).collect()
    });
    assert!(failed.is_empty(), "panicked on text wider than the screen: {failed:?}");
}

#[test]
fn widgets_survive_theme_padding_wider_than_any_screen() {
    // Every style of the built-in theme gets a padding whose double does not fit in `u16`.
    let monochrome = include_str!("../../assets/themes/monochrome.toml");
    let mut theme = String::from("[meta]\nname = \"Padded\"\nextends = \"monochrome\"\n");
    for header in monochrome.lines().filter(|line| line.starts_with("[style.")) {
        theme.push_str(&format!("{header}\npadding = [40000, 40000]\n"));
    }
    let dir = std::env::temp_dir().join(format!("quvyta-huge-padding-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("padded.toml"), theme).expect("theme file");
    let dirs = crate::env::AssetDirs { themes: Some(dir.clone()), ..Default::default() };
    let mut env = crate::env::Env::load(&dirs).expect("loads");
    std::fs::remove_dir_all(&dir).ok();
    env.set_theme("padded");
    assert_eq!(env.theme().style("button", None, &[]).pair("padding"), Some((40000, 40000)));
    let failed: Vec<&str> = std::thread::scope(|scope| {
        let runs: Vec<_> = WIDGETS
            .iter()
            .map(|(name, build)| {
                let env = env.clone();
                (*name, scope.spawn(move || exercise_in(env, name, *build, "Deploy")))
            })
            .collect();
        runs.into_iter().filter_map(|(name, run)| run.join().is_err().then_some(name)).collect()
    });
    assert!(failed.is_empty(), "panicked on theme padding wider than the screen: {failed:?}");
}
