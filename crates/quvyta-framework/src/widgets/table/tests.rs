use super::*;
use crate::color::Rgb;
use crate::event::{MouseButton, MouseKind};
use crate::icons::Glyph;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::{Align, Length, View};
use crate::widgets::ContextItem;

struct Demo {
    rows: Arc<[TableRow]>,
    selected: Option<usize>,
    opened: Vec<usize>,
    checked: Option<Vec<bool>>,
    sort: Option<(usize, SortDirection)>,
    wide: bool,
    /// Whether every row carries a menu of its own.
    menu: bool,
    /// The rows a menu entry was chosen on, in order.
    removed: Vec<usize>,
}

#[derive(Clone)]
enum Msg {
    Select(usize),
    Open(usize),
    Toggle(usize),
    Sort(usize, SortDirection),
    Remove(usize),
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(i) => self.selected = Some(i),
            Msg::Open(i) => self.opened.push(i),
            Msg::Toggle(i) => {
                if let Some(checked) = &mut self.checked {
                    checked[i] = !checked[i];
                }
            }
            Msg::Sort(column, direction) => self.sort = Some((column, direction)),
            Msg::Remove(i) => self.removed.push(i),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut columns = vec![
            Column::new("Name").sortable(true),
            Column::new("CPU").width(ColumnWidth::Fit).align(Align::End).sortable(true),
        ];
        if self.wide {
            columns.push(Column::new("Image").width(ColumnWidth::Fixed(12)));
            columns.push(Column::new("Ports").width(ColumnWidth::Fixed(12)));
        }
        let mut table = Table::new(columns, Arc::clone(&self.rows))
            .selected(self.selected)
            .empty_text("No containers")
            .on_select(Msg::Select)
            .on_activate(Msg::Open)
            .on_toggle(Msg::Toggle)
            .on_sort(Msg::Sort);
        if let Some((column, direction)) = self.sort {
            table = table.sort(column, direction);
        }
        if let Some(checked) = &self.checked {
            table = table.checked(checked.clone());
        }
        if self.menu {
            let rows = Arc::clone(&self.rows);
            table = table.context_menu(move |index| {
                let name = rows[index].cells[0].text.clone();
                vec![ContextItem::new(format!("Remove {name}"), Msg::Remove(index))]
            });
        }
        ui.add(table).width(Length::Fill(1)).height(Length::Fill(1)).id("table");
    }
}

fn rows(count: usize) -> Arc<[TableRow]> {
    (0..count)
        .map(|i| TableRow::new([format!("svc-{i}"), format!("{}%", i * 3), "nginx".into(), "80".into()]))
        .collect()
}

fn demo(count: usize) -> Demo {
    Demo {
        rows: rows(count),
        selected: None,
        opened: Vec::new(),
        checked: None,
        sort: None,
        wide: false,
        menu: false,
        removed: Vec::new(),
    }
}

#[test]
fn header_rows_and_right_aligned_numbers() {
    let mut h = Harness::new(demo(3), 30, 4);
    h.set_glyph_mode(GlyphMode::Unicode);
    assert_eq!(
        h.screen(),
        "  Name                     CPU\n  svc-0                     0%\n  svc-1                     3%\n  svc-2                     6%\n"
    );
    let raised = h.env().theme().color("raised");
    assert_eq!(h.bg(10, 0), raised, "the header sits on the raised surface");
    assert_eq!(h.fg(2, 0), h.env().theme().color("muted"), "titles are faint");
}

#[test]
fn selection_slides_only_the_first_cell() {
    let mut h = Harness::new(demo(3), 30, 4);
    h.press("tab").press("down");
    assert_eq!(h.app().selected, Some(0));
    let screen = h.screen();
    assert_eq!(screen.lines().nth(1), Some("▌  svc-0                    0%"), "{screen}");
    h.press("down");
    let active: Option<Rgb> = h.env().theme().color("active");
    assert_eq!(h.bg(20, 2), active);
    h.press("enter");
    assert_eq!(h.app().opened, vec![1]);
    assert_ne!(h.bg(20, 2), active, "activation flashes the row");
}

#[test]
fn sorts_from_titles_and_keys() {
    let mut h = Harness::new(demo(3), 30, 4);
    h.click_text("CPU");
    assert_eq!(h.app().sort, Some((1, SortDirection::Ascending)));
    assert!(h.screen().starts_with("  Name                   ↑ CPU"), "{}", h.screen());
    h.click_text("CPU");
    assert_eq!(h.app().sort, Some((1, SortDirection::Descending)));
    h.press("tab").press("s");
    assert_eq!(h.app().sort, Some((0, SortDirection::Ascending)));
    h.press("shift+s");
    assert_eq!(h.app().sort, Some((0, SortDirection::Descending)));
}

#[test]
fn virtualises_huge_tables_and_follows_the_selection() {
    let mut h = Harness::new(demo(200_000), 30, 6);
    h.press("tab").press("end");
    assert_eq!(h.app().selected, Some(199_999));
    let screen = h.screen();
    assert!(screen.contains("svc-199999"), "{screen}");
    assert!(!super::super::scrollbar::column(&h, 29).chars().skip(1).any(|c| c == ' '), "{screen}");
    h.press("pgup");
    assert_eq!(h.app().selected, Some(199_994));
}

#[test]
fn overflowing_columns_scroll_sideways() {
    let mut app = demo(2);
    app.wide = true;
    let mut h = Harness::new(app, 30, 3);
    let screen = h.screen();
    assert!(screen.contains("Name") && !screen.contains("Ports"), "{screen}");
    assert!(screen.lines().next().is_some_and(|l| l.ends_with('▶')), "{screen}");
    h.press("tab").press("right").press("right").press("right");
    let screen = h.screen();
    assert!(screen.contains("Ports") && !screen.contains("Name"), "{screen}");
    assert!(screen.starts_with('◀'), "{screen}");
    h.press("left").press("left").press("left");
    assert!(h.screen().contains("Name"));
}

#[test]
fn the_scroll_arrows_are_buttons() {
    let mut app = demo(2);
    app.wide = true;
    let mut h = Harness::new(app, 30, 3);
    h.set_glyph_mode(GlyphMode::Unicode);
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert!(lines[0].ends_with(" ▶"), "the arrow has its own cell:\n{screen}");
    assert!(lines[1..].iter().all(|line| line.chars().count() < 29), "no value runs under the arrow:\n{screen}");
    let color = |h: &Harness<Demo>, token: &str| h.env().theme().color(token);
    assert_eq!(h.fg(29, 0), color(&h, "dim"), "a clickable arrow is not faint");
    h.hover(29, 0);
    assert_eq!(h.bg(29, 0), color(&h, "active"), "it rises under the pointer");
    assert_eq!(h.fg(29, 0), color(&h, "text"));
    h.click(29, 0).click(29, 0).click(29, 0);
    let screen = h.screen();
    assert!(screen.contains("Ports") && !screen.contains("Name"), "{screen}");
    assert!(screen.starts_with('◀') && !screen.lines().next().is_some_and(|l| l.ends_with('▶')), "{screen}");
    h.click(0, 0);
    assert!(h.screen().lines().next().is_some_and(|l| l.ends_with('▶')), "{}", h.screen());
    h.click(0, 0).click(0, 0);
    assert!(h.screen().starts_with("  Name"), "back at the first column the arrow is gone:\n{}", h.screen());
    h.click(0, 0);
    assert_eq!(h.app().sort, None, "a press where no arrow is drawn does nothing");
}

#[test]
fn a_clipped_last_column_leaves_the_arrow_and_its_air_free() {
    let mut app = demo(2);
    app.wide = true;
    let mut h = Harness::new(app, 36, 3);
    h.set_glyph_mode(GlyphMode::Unicode);
    assert_eq!(h.screen().lines().next(), Some("  Name      CPU  Image         Po… ▶"), "{}", h.screen());
    let mut app = demo(2);
    app.wide = true;
    let mut h = Harness::new(app, 38, 3);
    h.set_glyph_mode(GlyphMode::Unicode);
    assert_eq!(h.screen().lines().next(), Some("  Name      CPU  Image         Ports ▶"), "{}", h.screen());
}

#[test]
fn check_marks_stay_put_while_the_first_cell_slides() {
    let mut app = demo(2);
    app.checked = Some(vec![false, false]);
    let mut h = Harness::new(app, 30, 3);
    h.hover(10, 2);
    assert_eq!(
        h.screen(),
        "    Name                   CPU\n  ☐ svc-0                   0%\n▌ ☐  svc-1                  3%\n"
    );
    let mut env = crate::env::Env::builtin();
    env.set_slide(false);
    let mut app = demo(2);
    app.checked = Some(vec![false, false]);
    let mut h = Harness::with_env(app, env, 30, 3);
    h.hover(10, 2);
    assert_eq!(
        h.screen(),
        "    Name                   CPU\n  ☐ svc-0                   0%\n▌ ☐ svc-1                   3%\n"
    );
}

#[test]
fn multi_select_marks_and_click_toggle() {
    let mut app = demo(2);
    app.checked = Some(vec![false, false]);
    let mut h = Harness::new(app, 30, 3);
    h.hover(10, 2).click(2, 2);
    assert_eq!(h.app().checked.as_deref(), Some(&[false, true][..]));
    assert_eq!((h.app().selected, h.app().opened.as_slice()), (None, &[][..]), "the mark only toggles");
    assert_eq!(h.screen().lines().nth(2), Some("▌ ☑  svc-1                  3%"), "{}", h.screen());
    assert_eq!(h.fg(2, 2), h.env().theme().color("accent"));
    h.click(3, 1);
    assert_eq!(h.app().checked.as_deref(), Some(&[true, true][..]), "the cell after the mark toggles too");
    h.click(4, 1);
    assert_eq!((h.app().selected, h.app().opened.as_slice()), (Some(0), &[0][..]), "the first cell opens the row");
    h.press("down").press("space");
    assert_eq!(h.app().checked.as_deref(), Some(&[true, false][..]));
}

#[test]
fn empty_and_ascii_and_narrow() {
    let h = Harness::new(demo(0), 30, 3);
    assert_eq!(h.screen(), "  Name                     CPU\n  No containers\n\n");
    let mut h = Harness::new(demo(1), 9, 2);
    h.set_glyph_mode(GlyphMode::Ascii);
    let screen = h.screen();
    assert!(!screen.contains('['), "{screen}");
    assert_eq!(screen.lines().count(), 2);
}

#[test]
fn huge_fixed_columns_scroll_instead_of_overflowing() {
    struct Wide;
    impl App for Wide {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let columns = [
                Column::new("Name").width(ColumnWidth::Fixed(40_000)),
                Column::new("Image").width(ColumnWidth::Fixed(40_000)),
            ];
            ui.add(Table::<()>::new(columns, rows(2))).fill();
        }
    }
    let h = Harness::new(Wide, 30, 3);
    assert!(h.screen().starts_with("  Name"), "{}", h.screen());
}

const FIREFOX: char = '\u{e745}';

/// Two rows whose names carry a glyph: a literal one and an icon of the set.
fn icon_demo(name: &str) -> Demo {
    let rows: Arc<[TableRow]> = vec![
        TableRow::new([TableCell::new(name).icon(Glyph::literal(FIREFOX), None), TableCell::new("3%")]),
        TableRow::new([TableCell::new("projects").icon(Glyph::key("folder"), None), TableCell::new("0%")]),
    ]
    .into();
    Demo { rows, ..demo(0) }
}

/// Where `text` is on screen, as the cell coordinates the colour checks take.
fn spot(h: &Harness<Demo>, text: &str) -> (u16, u16) {
    let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` is drawn:\n{}", h.screen()));
    (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0))
}

#[test]
fn a_cell_glyph_is_muted_and_takes_the_row_text_colour_when_the_row_is_selected() {
    let mut h = Harness::new(icon_demo("firefox"), 30, 3);
    h.set_glyph_mode(GlyphMode::Unicode);
    let muted = h.env().theme().color("muted");
    let (x, y) = spot(&h, &FIREFOX.to_string());
    let screen = h.screen();
    let row = screen.lines().nth(1).unwrap_or_default();
    assert!(row.starts_with(&format!("  {FIREFOX} firefox ")) && row.ends_with(" 3%"), "{row:?}");
    assert_eq!(h.fg(x, y), muted, "a glyph is quieter than the name");
    assert_ne!(h.fg(x + 2, y), muted);
    let folder = h.env().icons().glyph("folder").into_owned();
    let (fx, fy) = spot(&h, &folder);
    assert_eq!((fx, h.fg(fx, fy)), (2, muted));
    h.press("tab").press("down");
    assert_eq!(h.app().selected, Some(0));
    let (x, y) = spot(&h, &FIREFOX.to_string());
    assert_eq!(h.fg(x, y), h.fg(x + 2, y), "on the selected row the glyph takes the name's colour");
    assert_ne!(h.fg(x, y), muted);
    assert_eq!(h.fg(fx, fy), muted, "the other row keeps its quiet glyph");
}

#[test]
fn a_narrow_column_cuts_the_text_and_keeps_the_glyph_and_its_space() {
    for mode in [GlyphMode::Unicode, GlyphMode::Ascii] {
        let mut h = Harness::new(icon_demo("firefox-developer-edition"), 16, 3);
        h.set_glyph_mode(mode);
        let screen = h.screen();
        let row = screen.lines().nth(1).unwrap_or_default();
        assert!(row.starts_with(&format!("  {FIREFOX} fir")), "{mode:?}: {row:?}");
        assert!(row.contains('…'), "{mode:?}: {row:?}");
        let folder = h.env().icons().glyph("folder").into_owned();
        let row = screen.lines().nth(2).unwrap_or_default();
        assert!(row.starts_with(&format!("  {folder} pro")), "{mode:?}: {row:?}");
    }
    let mut h = Harness::new(icon_demo("firefox"), 9, 3);
    h.set_glyph_mode(GlyphMode::Unicode);
    let row = h.screen().lines().nth(1).unwrap_or_default().to_owned();
    assert!(row.starts_with(&format!("  {FIREFOX} ")), "even with no room for a letter: {row:?}");
}

#[test]
fn a_one_cell_glyph_takes_one_cell_and_one_space_in_a_fitting_column() {
    /// A fitting column of kinds, whose cell is a glyph and `pkg`, or the same written as text.
    struct Kinds(bool);
    impl App for Kinds {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let columns = [Column::new("Kind").width(ColumnWidth::Fit), Column::new("Name")];
            let kind = if self.0 { TableCell::new("pkg").icon(Glyph::literal('▲'), None) } else { "▲ pkg".into() };
            let rows = vec![TableRow::new([kind, TableCell::new("firefox")])];
            ui.add(Table::<()>::new(columns, rows)).fill();
        }
    }
    let mut glyph = Harness::new(Kinds(true), 24, 2);
    glyph.set_glyph_mode(GlyphMode::Unicode);
    let mut text = Harness::new(Kinds(false), 24, 2);
    text.set_glyph_mode(GlyphMode::Unicode);
    assert_eq!(glyph.screen(), text.screen(), "the glyph and its space are two cells, like `▲ `");
}

/// A table whose rows each carry a menu, with the motion off so a menu is there at once.
fn menu_demo(count: usize) -> Harness<Demo> {
    let mut demo = demo(count);
    demo.menu = true;
    let mut h = Harness::new(demo, 30, 8);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h
}

/// Right-clicks the cell at `(x, y)`.
fn right_click(h: &mut Harness<Demo>, x: i32, y: i32) {
    h.mouse(MouseKind::Down(MouseButton::Right), x, y);
    h.mouse(MouseKind::Up(MouseButton::Right), x, y);
    h.render();
}

#[test]
fn a_rows_menu_acts_on_the_row_that_was_right_clicked_and_not_on_the_selected_one() {
    let mut h = menu_demo(5);
    h.send(Msg::Select(0));
    let (x, y) = h.find("svc-3").expect("the row is on screen");
    right_click(&mut h, x, y);
    assert!(h.screen().contains("Remove svc-3"), "the menu is the row's own:\n{}", h.screen());
    assert_eq!(h.app().selected, Some(3), "the row the menu belongs to became the selection");
    h.click_text("Remove svc-3").render();
    assert_eq!(h.app().removed, vec![3], "the entry acts on the row that was clicked");
}

#[test]
fn the_menu_key_opens_the_menu_of_the_selected_row() {
    let mut h = menu_demo(5);
    h.press("tab").press("down").press("down");
    assert_eq!(h.app().selected, Some(1));
    h.press("menu").render();
    assert!(h.screen().contains("Remove svc-1"), "{}", h.screen());
    h.press("enter").render();
    assert_eq!(h.app().removed, vec![1]);
}

#[test]
fn the_menu_key_scrolls_the_selected_row_into_view_first() {
    let mut h = menu_demo(200);
    h.press("tab").press("end");
    assert_eq!(h.app().selected, Some(199));
    h.press("menu").render();
    assert!(h.screen().contains("Remove svc-199"), "{}", h.screen());
}

#[test]
fn a_right_press_beside_the_menu_closes_it_without_choosing() {
    let mut h = menu_demo(5);
    let (x, y) = h.find("svc-1").expect("the row is on screen");
    right_click(&mut h, x, y);
    assert!(h.screen().contains("Remove svc-1"), "{}", h.screen());
    let (other_x, other_y) = h.find("svc-4").expect("another row");
    right_click(&mut h, other_x, other_y);
    assert!(h.screen().contains("Remove svc-4"), "the other row's menu took its place:\n{}", h.screen());
    assert!(h.app().removed.is_empty(), "nothing was chosen");
}

#[test]
fn a_right_press_on_the_header_or_beside_the_rows_opens_nothing() {
    let mut h = menu_demo(3);
    right_click(&mut h, 4, 0);
    assert!(!h.screen().contains("Remove"), "the header has no row menu:\n{}", h.screen());
    right_click(&mut h, 4, 6);
    assert!(!h.screen().contains("Remove"), "below the last row there is no row:\n{}", h.screen());
}

#[test]
fn a_menu_row_keeps_its_surface_raised_while_its_menu_is_open() {
    let mut h = menu_demo(5);
    let (x, y) = h.find("svc-2").expect("the row is on screen");
    let row = u16::try_from(y).expect("a row on screen");
    let resting = h.bg(1, row);
    right_click(&mut h, x, y);
    assert_ne!(h.bg(1, row), resting, "the row the menu acts on stays lit:\n{}", h.screen());
}

#[test]
fn a_table_without_a_menu_answers_no_right_press() {
    let mut h = Harness::new(demo(3), 30, 6);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    right_click(&mut h, 4, 1);
    assert!(!h.screen().contains("Remove"), "nothing opens:\n{}", h.screen());
    assert!(h.app().removed.is_empty() && h.app().opened.is_empty(), "and the right button does nothing else");
}
