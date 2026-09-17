use super::*;
use crate::color::Rgb;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::{Align, Length, View};

struct Demo {
    rows: Arc<[TableRow]>,
    selected: Option<usize>,
    opened: Vec<usize>,
    checked: Option<Vec<bool>>,
    sort: Option<(usize, SortDirection)>,
    wide: bool,
}

#[derive(Clone)]
enum Msg {
    Select(usize),
    Open(usize),
    Toggle(usize),
    Sort(usize, SortDirection),
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
        ui.add(table).width(Length::Fill(1)).height(Length::Fill(1)).id("table");
    }
}

fn rows(count: usize) -> Arc<[TableRow]> {
    (0..count)
        .map(|i| TableRow::new([format!("svc-{i}"), format!("{}%", i * 3), "nginx".into(), "80".into()]))
        .collect()
}

fn demo(count: usize) -> Demo {
    Demo { rows: rows(count), selected: None, opened: Vec::new(), checked: None, sort: None, wide: false }
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
