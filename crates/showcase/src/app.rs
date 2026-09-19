//! The showcase application: shell, component menu, sections and settings.

use qframe::icons::{IconMode, PillarStyle};
use qframe::keymap::Scope;
use qframe::prelude::*;
use qframe::runtime::{ClipboardEvent, Termination};
use qframe::storage::Settings;
use qframe::widgets::{CodeView, Language, Markdown, PageTransition, ScrollView, TextInput};

use crate::catalog::{Catalog, GROUPS};
use crate::log::EventLog;
use crate::pages::theme_icons_language::{icon_select, language_select, theme_select};
use crate::pages::{self, FOUNDATIONS, PageMsg, Pages};
use crate::regions;

/// The four sections of a component page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Demo,
    Code,
    Guide,
    Reference,
}

impl Section {
    const ALL: [Self; 4] = [Self::Demo, Self::Code, Self::Guide, Self::Reference];
}

/// Everything that can happen in the showcase.
#[derive(Debug, Clone)]
pub enum Msg {
    Open(String),
    Back,
    Search(String),
    Section(usize),
    Theme(String),
    Locale(String),
    Icons(IconMode),
    Pillar(PillarStyle),
    Slide(bool),
    ReducedMotion(bool),
    ToggleMenu,
    FocusSearch,
    Copied(&'static str, String),
    Clipboard(ClipboardEvent),
    Page(PageMsg),
    Layers(crate::layers::Msg),
}

/// The showcase state.
pub struct Showcase {
    catalog: Catalog,
    router: Router<String>,
    search: String,
    section: Section,
    menu_open: bool,
    pub(crate) log: EventLog,
    pub(crate) pages: Pages,
    layers: crate::layers::State,
}

impl Showcase {
    /// The showcase over a different catalog, for tests of catalog-driven screens.
    #[cfg(test)]
    pub(crate) fn with_catalog(catalog: Catalog) -> Self {
        Self { catalog, ..Self::new() }
    }

    /// A showcase whose theme, language and icon choices live only in memory.
    #[must_use]
    pub fn new() -> Self {
        Self::with_settings(Settings::in_memory())
    }

    /// A showcase that remembers theme, language and icons in `settings`.
    #[must_use]
    pub fn with_settings(settings: Settings) -> Self {
        let mut pages = Pages::default();
        pages.cell_animation = pages::cell_animation::State::restore(&settings);
        pages.storage = pages::storage::State::new(settings);
        Self {
            catalog: Catalog::load(),
            router: Router::new(FOUNDATIONS[0].to_owned()),
            search: String::new(),
            section: Section::Demo,
            menu_open: false,
            log: EventLog::new(),
            pages,
            layers: crate::layers::State::default(),
        }
    }

    /// The page shown now.
    #[must_use]
    pub fn current(&self) -> &str {
        self.router.current()
    }

    /// Page ids of `group` in menu order: foundation pages first, then catalog items that have no
    /// place on a foundation page.
    fn entries(&self, group: &str) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        if group == "foundations" {
            ids.extend(FOUNDATIONS.iter().map(|page| (*page).to_owned()));
        }
        let items = self.catalog.items.iter().filter(|item| item.group == group);
        ids.extend(
            items
                .filter(|item| !item.page.as_deref().is_some_and(|page| FOUNDATIONS.contains(&page)))
                .map(|item| item.id.clone()),
        );
        ids
    }

    /// Where `page` sits in the menu: its group's index in `GROUPS`, the group, the page's index in
    /// the group and the group's length.
    fn locate(&self, page: &str) -> Option<(usize, &'static str, usize, usize)> {
        GROUPS.iter().enumerate().find_map(|(group_index, group)| {
            let entries = self.entries(group);
            let index = entries.iter().position(|id| id == page)?;
            Some((group_index, *group, index, entries.len()))
        })
    }

    /// The outline number of `page`, such as `1.5`: the group's place in the menu, then the page's
    /// place in the group. Numbers never change with the search, so a page keeps its number while filtering.
    fn number(&self, page: &str) -> Option<String> {
        self.locate(page).map(|(group_index, _, index, _)| format!("{}.{}", group_index + 1, index + 1))
    }

    /// Menu rows for the current search: group headings and entries, each entry with its page id.
    /// Headings and entries carry their outline numbers; the search matches names and numbers.
    fn menu(&self, name: &dyn Fn(&str) -> String) -> Vec<(ListItem, Option<String>)> {
        let query = self.search.trim().to_lowercase();
        let mut rows = Vec::new();
        for (group_index, group) in GROUPS.iter().enumerate() {
            let mut visible: Vec<(ListItem, Option<String>)> = Vec::new();
            let entries = self.entries(group);
            // Numbers are padded to the longest in the group so the names line up.
            let number_width = format!("{}.{}", group_index + 1, entries.len()).len();
            for (index, id) in entries.into_iter().enumerate() {
                let number = format!("{}.{}", group_index + 1, index + 1);
                let label = name(&id);
                if !query.is_empty() && !label.to_lowercase().contains(&query) && !number.starts_with(&query) {
                    continue;
                }
                let mut row = ListItem::new(format!("{number:<number_width$}  {label}"));
                if let Some(item) = self.catalog.get(&id) {
                    // Brightness tells done from planned.
                    row = row.faint(!item.done);
                }
                visible.push((row, Some(id)));
            }
            if visible.is_empty() {
                continue;
            }
            if !rows.is_empty() {
                rows.push((ListItem::gap(), None));
            }
            rows.push((ListItem::header(format!("{}  {}", group_index + 1, t!(&format!("groups.{group}")))), None));
            rows.extend(visible);
        }
        rows
    }

    fn header(&self, ui: &mut View<'_, Msg>) {
        let env = ui.env();
        let (theme, language, icons) = (theme_select(env), language_select(env), icon_select(env));
        ui.row(|ui| {
            ui.add(
                Text::rich([Span::new("quvyta").color("accent").bold(), Span::new("  showcase").role("faint")])
                    .no_wrap(),
            );
            ui.spacer();
            ui.add(Text::new(t!("header.theme")).role("faint").no_wrap());
            ui.add(theme).width(Length::Cells(18)).id("theme");
            ui.add(Text::new(t!("header.language")).role("faint").no_wrap());
            ui.add(language).width(Length::Cells(14)).id("language");
            ui.add(Text::new(t!("header.icons")).role("faint").no_wrap());
            ui.add(icons).width(Length::Cells(15)).id("icons");
        })
        .gap(1)
        .padding(Padding::symmetric(0, 2))
        .fill_width();
    }

    fn sidebar(&self, ui: &mut View<'_, Msg>) {
        let name = |id: &str| t!(&format!("names.{id}"));
        let rows = self.menu(&name);
        let current = self.current();
        let selected = rows.iter().position(|(_, id)| id.as_deref() == Some(current));
        let ids: Vec<Option<String>> = rows.iter().map(|(_, id)| id.clone()).collect();
        let items: Vec<ListItem> = rows.into_iter().map(|(item, _)| item).collect();
        ui.column(|ui| {
            ui.add(TextInput::new(&self.search).placeholder(t!("menu.search")).on_change(Msg::Search))
                .fill_width()
                .id("search");
            ui.add(
                List::new(items)
                    .selected(selected)
                    .empty_text(t!("menu.nothing"))
                    .on_select(move |i| ids[i].clone().map_or(Msg::Back, Msg::Open)),
            )
            .fill()
            .id("menu");
        })
        .gap(1)
        .padding(Padding { top: 1, right: 0, bottom: 0, left: 1 })
        .fill();
    }

    fn body(&self, ui: &mut View<'_, Msg>) {
        let page = self.current().to_owned();
        // Page changes cross-fade and slide in from the side the navigation went.
        let transition = PageTransition::new(page.clone()).slide(true).direction(self.router.direction());
        ui.add_with(transition, |ui| self.page_body(ui, &page)).fill();
    }

    fn page_body(&self, ui: &mut View<'_, Msg>, page: &str) {
        let item = self.catalog.get(page);
        let done = FOUNDATIONS.contains(&page) || item.is_some_and(|item| item.done);
        let name = t!(&format!("names.{page}"));
        let title = match self.number(page) {
            Some(number) => format!("{number}  {name}"),
            None => name,
        };
        let position = self.position(page);
        ui.page(page, |ui| {
            ui.column(|ui| {
                ui.row(|ui| {
                    ui.add(Text::new(title).role("title").no_wrap());
                    ui.spacer();
                    ui.add(Text::new(position).role("faint").no_wrap());
                })
                .fill_width();
                if !done {
                    Self::planned(ui, item);
                    return;
                }
                let labels = [t!("sections.demo"), t!("sections.code"), t!("sections.guide"), t!("sections.reference")];
                let active = Section::ALL.iter().position(|s| *s == self.section).unwrap_or(0);
                ui.add(Tabs::new(labels).numbered(true).active(active).on_select(Msg::Section)).id("sections");
                match self.section {
                    Section::Demo => {
                        ui.add_with(ScrollView::new(), |ui| {
                            pages::demo(&self.pages, page, ui);
                            self.event_log(ui, page);
                        })
                        .fill()
                        .id("demo");
                    }
                    Section::Code => self.code(ui, page),
                    Section::Guide => self.document(ui, page, "guide"),
                    Section::Reference => self.document(ui, page, "reference"),
                }
            })
            .gap(1)
            .padding(Padding::symmetric(1, 3))
            .fill();
        });
    }

    /// The design notes and the plan of a catalog item that is not built yet.
    fn planned(ui: &mut View<'_, Msg>, item: Option<&crate::catalog::Item>) {
        if let Some(notes) = item.and_then(|item| item.notes.as_ref()) {
            let text = in_active_language(ui, notes).clone();
            ui.add_with(Panel::new().title(t!("planned.notes")), |ui| {
                ui.add(Text::new(text).role("body"));
            })
            .fill_width();
        }
        let priority = item.map(|item| item.priority.to_uppercase()).unwrap_or_default();
        let kind = item.map_or("widget", |item| item.kind.as_str());
        ui.add_with(Panel::new().title(t!("planned.title")), |ui| {
            ui.add(
                Text::new(t!("planned.body", priority = priority, kind = t!(&format!("kinds.{kind}"))))
                    .role("secondary"),
            );
            ui.add(Text::new(t!("planned.hint")).role("faint"));
        })
        .fill_width();
    }

    fn event_log(&self, ui: &mut View<'_, Msg>, page: &str) {
        let entries = self.log.recent(page, 6);
        ui.add_with(Panel::new().title(t!("log.title")).gap(0), |ui| {
            // The entries are content to copy, like the Code section; the panel title is not.
            ui.column(|ui| {
                if entries.is_empty() {
                    ui.add(Text::new(t!("log.empty")).role("faint"));
                }
                for entry in entries {
                    ui.add(
                        Text::rich([
                            Span::new(format!("{}   ", entry.time)).role("faint"),
                            Span::new(format!("{}   ", entry.source)).role("secondary"),
                            Span::new(entry.message.clone()).color("success"),
                        ])
                        .no_wrap(),
                    );
                }
            })
            .fill_width()
            .selectable(true);
        })
        .fill_width();
    }

    fn code(&self, ui: &mut View<'_, Msg>, page: &str) {
        let Some(content) = pages::content(page) else {
            return;
        };
        let id: &'static str = content.id;
        ui.add_with(ScrollView::new(), |ui| {
            for region in regions::extract(content.source) {
                ui.add(Text::new(t!(&format!("regions.{}", region.name))).role("faint"));
                ui.add(
                    CodeView::new(region.code.clone(), Language::Rust).on_copy(Msg::Copied(id, region.name.clone())),
                )
                .fill_width()
                .id(region.name.clone());
            }
            ui.add(Text::new(t!("code.copy_hint")).role("faint"));
        })
        .fill()
        .selectable(true)
        .id("code");
    }

    fn document(&self, ui: &mut View<'_, Msg>, page: &str, kind: &str) {
        let Some(content) = pages::content(page) else {
            return;
        };
        let text = *in_active_language(ui, if kind == "guide" { &content.guide } else { &content.reference });
        let types = self.types(page);
        ui.add_with(ScrollView::new(), |ui| {
            if kind == "reference" && !types.is_empty() {
                ui.add(Markdown::new(&format!("{} {types}", t!("reference.types")))).fill_width();
            }
            ui.add(Markdown::new(text)).fill_width();
        })
        .fill()
        .selectable(true)
        .id(kind.to_owned());
    }

    /// The Rust types covered by `page`, as inline code, for the Reference section.
    fn types(&self, page: &str) -> String {
        self.catalog
            .items
            .iter()
            .filter(|item| item.id == page || item.page.as_deref() == Some(page))
            .flat_map(|item| item.types.iter())
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Applies an appearance choice and remembers it with the other settings.
    fn apply_and_remember<T: qframe::storage::Setting>(
        &mut self,
        key: &str,
        value: T,
        apply: Command<Msg>,
    ) -> Command<Msg> {
        pages::storage::apply_and_remember(&mut self.pages.storage.settings, key, value, apply)
    }

    /// "Group · n / m": where the page sits in its menu group.
    fn position(&self, page: &str) -> String {
        self.locate(page)
            .map(|(_, group, index, len)| format!("{} · {} / {}", t!(&format!("groups.{group}")), index + 1, len))
            .unwrap_or_default()
    }
}

/// The English or the Turkish one of `pair`, whichever language is active.
fn in_active_language<'a, T>(ui: &View<'_, Msg>, pair: &'a [T; 2]) -> &'a T {
    if ui.env().i18n().active() == "tr" { &pair[1] } else { &pair[0] }
}

impl Default for Showcase {
    fn default() -> Self {
        Self::new()
    }
}

impl App for Showcase {
    type Msg = Msg;

    fn init(&mut self) -> Command<Msg> {
        pages::getting_started::init()
    }

    fn resized(&self, size: Size) -> Option<Msg> {
        pages::getting_started::resized(size)
    }

    fn before_quit(&self) -> Option<Msg> {
        pages::getting_started::before_quit(&self.pages.getting_started)
    }

    fn terminating(&self, cause: Termination) -> Option<Msg> {
        pages::getting_started::terminating(cause)
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open(page) => {
                self.router.push(page);
                self.menu_open = false;
            }
            Msg::Back => {
                self.router.back();
            }
            Msg::Search(text) => self.search = text,
            Msg::Section(index) => self.section = Section::ALL.get(index).copied().unwrap_or(Section::Demo),
            Msg::Theme(id) => return self.apply_and_remember(Settings::THEME, id.clone(), Command::set_theme(id)),
            Msg::Locale(code) => {
                return self.apply_and_remember(Settings::LANGUAGE, code.clone(), Command::set_locale(code));
            }
            Msg::Icons(mode) => {
                return self.apply_and_remember(Settings::ICONS, mode.name().to_owned(), Command::set_icon_mode(mode));
            }
            Msg::Pillar(style) => {
                return self.apply_and_remember(Settings::PILLAR, style.name().to_owned(), Command::set_pillar(style));
            }
            Msg::Slide(on) => return self.apply_and_remember(Settings::SLIDE, on, Command::set_slide(on)),
            Msg::ReducedMotion(on) => {
                return self.apply_and_remember(Settings::REDUCED_MOTION, on, Command::set_reduced_motion(on));
            }
            Msg::ToggleMenu => self.menu_open = !self.menu_open,
            Msg::FocusSearch => {
                self.menu_open = true;
                return Command::focus("search");
            }
            Msg::Copied(page, region) => self.log.push(page, format!("CodeView#{region}"), "copied"),
            Msg::Clipboard(event) => {
                pages::clipboard_event(&mut self.pages, self.router.current(), &event, &mut self.log);
            }
            Msg::Page(message) => return pages::update(&mut self.pages, message, &mut self.log),
            Msg::Layers(message) => crate::layers::update(&mut self.layers, message),
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        AppShell::new()
            .sidebar_width(30)
            .collapse_below(100)
            .sidebar_open(self.menu_open)
            .header(|ui| self.header(ui))
            .sidebar(|ui| self.sidebar(ui))
            .body(|ui| self.body(ui))
            .footer(|ui| {
                ui.add(
                    KeyHints::new()
                        .hint("↑↓", t!("hints.move"))
                        .action(Scope::App, "search")
                        .action(Scope::App, "back")
                        .action(Scope::Global, "focus-next")
                        .action(Scope::App, "menu")
                        .action_right(Scope::Global, "debug")
                        .action_right(Scope::Global, "quit"),
                )
                .fill_width();
            })
            .show(ui);
        crate::layers::view(&self.layers, &self.catalog, ui);
    }

    fn clipboard(&self, event: &ClipboardEvent) -> Option<Msg> {
        Some(Msg::Clipboard(event.clone()))
    }

    fn action(&self, name: &str) -> Option<Msg> {
        match name {
            "search" => Some(Msg::FocusSearch),
            "back" => Some(Msg::Back),
            "menu" => Some(Msg::ToggleMenu),
            "demo" => Some(Msg::Section(0)),
            "code" => Some(Msg::Section(1)),
            "guide" => Some(Msg::Section(2)),
            "reference" => Some(Msg::Section(3)),
            "help" => Some(Msg::Layers(crate::layers::Msg::Help(true))),
            "palette" => Some(Msg::Layers(crate::layers::Msg::Palette(true))),
            "terminal-focus" if self.router.current() == "terminal" => {
                pages::terminal::action(&self.pages.terminal, name)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_tall;

    #[test]
    fn the_event_log_copies_clean_and_its_title_stays_out() {
        let mut harness = showcase_tall(Showcase::new(), "splitter", 80);
        harness.set_reduced_motion(true);
        harness.send(Msg::Page(PageMsg::Splitter(pages::splitter::Msg::Resizable(false))));
        harness.send(Msg::Page(PageMsg::Splitter(pages::splitter::Msg::Resizable(true))));
        let lines: Vec<String> = harness
            .app()
            .log
            .recent("splitter", 2)
            .iter()
            .map(|entry| format!("{}   Playground   {}", entry.time, entry.message))
            .collect();
        let (first, second) = (lines[0].clone(), lines[1].clone());

        // A triple press selects the whole row; the clean copy drops the panel padding after it.
        let (x, y) = harness.find(&first).unwrap_or_else(|| panic!("`{first}` on screen:\n{}", harness.screen()));
        harness.click(x + 2, y).click(x + 2, y).click(x + 2, y).press("ctrl+c");
        assert_eq!(harness.clipboard(), Some(first.as_str()));

        // A drag across both entries keeps the spaces between the columns and the line break.
        let (end_x, end_y) = harness.find(&second).expect("second entry");
        let end = end_x + i32::try_from(second.chars().count()).unwrap_or(1) - 1;
        harness.drag((x, y), (end, end_y)).press("ctrl+c");
        assert_eq!(harness.clipboard().map(str::to_owned), Some(format!("{first}\n{second}")));

        // The title is not part of the log's text.
        let copies = harness.copied().len();
        let (title_x, title_y) = harness.find("EVENT LOG").expect("log title");
        harness.drag((title_x, title_y), (title_x + 4, title_y)).press("ctrl+c");
        assert_eq!(harness.copied().len(), copies, "the title never starts a selection");
    }
}
