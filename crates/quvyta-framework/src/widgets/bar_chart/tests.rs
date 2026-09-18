use super::*;
use crate::color::Rgb;
use crate::env::Env;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::theme::ThemeRegistry;
use crate::widget::{Length, View};

/// Every built-in theme, so a chart is checked in all of them.
const THEMES: [&str; 4] = ["monochrome", "nordic", "amber", "iris"];

/// A chart built fresh every frame from the selection the application keeps, the way a real
/// application builds it.
struct Demo {
    build: Box<dyn Fn(Option<usize>) -> BarChart<usize>>,
    height: u16,
    selected: Option<usize>,
    picks: Vec<usize>,
}

impl App for Demo {
    type Msg = usize;

    fn update(&mut self, category: usize) -> Command<usize> {
        self.selected = Some(category);
        self.picks.push(category);
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, usize>) {
        ui.add((self.build)(self.selected)).fill_width().height(Length::Cells(self.height)).id("chart");
    }
}

/// A demo whose chart follows the selection.
fn demo(height: u16, build: impl Fn(Option<usize>) -> BarChart<usize> + 'static) -> Demo {
    Demo { build: Box::new(build), height, selected: None, picks: Vec::new() }
}

/// A demo whose chart never changes.
fn fixed(height: u16, build: impl Fn() -> BarChart<usize> + 'static) -> Demo {
    demo(height, move |_| build())
}

fn services() -> BarChart<usize> {
    BarChart::new([Bar::new("api", 40.0), Bar::new("postgres", 25.0), Bar::new("worker", 90.0).variant("danger")])
}

/// Two days of two series: a shape whose stack and group are easy to read off the screen.
fn week() -> BarChart<usize> {
    BarChart::series(["mon", "tue"], [Series::new("rust", [3.0, 1.0]), Series::new("docs", [1.0, 3.0])])
}

/// The tone of the cell at `(x, y)`. Whole cells of a bar are coloured ground rather than
/// glyphs, so this is how a filled bar shows.
fn tone(h: &Harness<Demo>, x: u16, y: u16) -> Rgb {
    h.bg(x, y).expect("every cell on screen has a background")
}

fn accent_of(h: &Harness<Demo>) -> Rgb {
    h.env().theme().color("accent").expect("every theme has an accent")
}

// ---- The chart that was there before, unchanged ----------------------------------------------

#[test]
fn horizontal_bars_with_labels_and_values() {
    let h = Harness::new(fixed(5, services), 30, 5);
    assert_eq!(
        h.screen(),
        "api             ▏           40\n\npostgres     ▌              25\n\nworker                    ● 90\n"
    );
    let theme = h.env().theme();
    assert_eq!(h.bg(9, 4), theme.color("danger"));
    assert_eq!(h.fg(26, 4), theme.color("danger"));
    assert_ne!(h.bg(9, 3), theme.color("danger"));
}

#[test]
fn narrow_horizontal_drops_labels() {
    let h = Harness::new(fixed(3, || services().gap(0)), 16, 3);
    assert_eq!(h.screen(), "    ▉         40\n              25\n            ● 90\n");
    assert_eq!(h.bg(2, 1), h.env().theme().color("accent"));
}

#[test]
fn vertical_bars_stand_with_value_above_and_label_below() {
    let h = Harness::new(fixed(6, || services().vertical().max(100.0)), 24, 6);
    assert_eq!(
        h.screen(),
        "                 ● 90\n                ▅▅▅▅▅▅\n  40\n▅▅▅▅▅▅    25\n\n  api   postgr… worker\n"
    );
    let theme = h.env().theme();
    assert_eq!(h.bg(0, 4), theme.color("accent"));
    assert_eq!(h.bg(8, 4), theme.color("accent"));
    assert_eq!(h.bg(16, 2), theme.color("danger"));
}

#[test]
fn ascii_rounds_to_cells() {
    let mut h = Harness::new(fixed(2, || BarChart::new([Bar::new("a", 1.0), Bar::new("b", 0.55)]).gap(0)), 30, 2);
    h.set_glyph_mode(GlyphMode::Ascii);
    assert_eq!(h.screen(), "a                            1\nb                          0.6\n");
    let theme = h.env().theme();
    assert_eq!(h.bg(25, 0), theme.color("accent"));
    assert_eq!(h.bg(14, 1), theme.color("accent"));
    assert_ne!(h.bg(15, 1), theme.color("accent"));
}

#[test]
fn a_huge_gap_leaves_room_for_the_first_bar_only() {
    let h = Harness::new(fixed(5, || services().gap(u16::MAX)), 30, 5);
    assert!(h.screen().starts_with("api "), "{}", h.screen());
    assert!(!h.screen().contains("postgres"), "{}", h.screen());
    let h = Harness::new(fixed(6, || services().vertical().gap(u16::MAX)), 24, 6);
    assert!(h.screen().ends_with("\n…\n"), "one thin bar with its label cut:\n{}", h.screen());
}

#[test]
fn a_chart_without_bars_draws_nothing() {
    let h = Harness::new(fixed(4, || BarChart::new(Vec::new())), 20, 4);
    assert_eq!(h.screen(), "\n\n\n\n");
    let h = Harness::new(fixed(4, || BarChart::series(Vec::<String>::new(), [Series::new("rust", [1.0])])), 20, 4);
    assert_eq!(h.screen(), "\n\n\n\n");
}

// ---- Series: one, stacked, grouped -----------------------------------------------------------

#[test]
fn a_chart_of_one_series_looks_like_a_chart_of_plain_bars() {
    let plain = Harness::new(fixed(3, || BarChart::new([Bar::new("mon", 3.0), Bar::new("tue", 1.0)]).gap(0)), 30, 3);
    let one =
        Harness::new(fixed(3, || BarChart::series(["mon", "tue"], [Series::new("rust", [3.0, 1.0])]).gap(0)), 30, 3);
    assert_eq!(one.screen(), plain.screen());
    assert_eq!(tone(&one, 10, 0), tone(&plain, 10, 0));
    assert_eq!(tone(&one, 10, 0), accent_of(&one), "one series keeps the theme's own tone");
}

#[test]
fn a_stacked_bar_shows_its_shares_and_names_the_ones_with_room() {
    let h = Harness::new(fixed(3, || week().stacked().unit("h").gap(0)), 34, 3);
    assert_eq!(h.screen(), "mon  rust                docs  4 h\ntue  rust   docs               4 h\n\n");
    let accent = accent_of(&h);
    // The first series keeps the theme's own tone, the second is a step down the ramp, and both
    // are grounds: the segments are told apart by tone with no line drawn between them.
    assert_eq!(tone(&h, 5, 0), accent);
    assert_ne!(tone(&h, 24, 0), accent);
    assert_eq!(tone(&h, 24, 0), tone(&h, 13, 1), "the same series has the same tone in every category");
    assert_eq!(tone(&h, 23, 0), accent, "and the boundary falls between two cells");
    assert!(!h.screen().contains('|'), "{}", h.screen());
}

#[test]
fn a_stacked_bar_keeps_its_eighth_cell_tail_and_scales_to_the_largest_total() {
    // Totals 4 and 0.9: the second bar reaches a little under a quarter of the first.
    let chart = || {
        BarChart::series(["mon", "tue"], [Series::new("rust", [3.0, 0.5]), Series::new("docs", [1.0, 0.4])]).stacked()
    };
    let h = Harness::new(fixed(3, chart), 34, 3);
    let screen = h.screen();
    let row = |index: usize| screen.lines().nth(index).unwrap_or_default().to_owned();
    assert!(row(0).contains("rust") && row(0).contains("docs"), "the wide bar names its shares:\n{screen}");
    assert!(row(2).contains('▉'), "the short bar keeps its eighth-cell tail:\n{screen}");
    assert_ne!(h.fg(9, 2), Some(accent_of(&h)), "the tail carries the tone of the last series in the stack");
    assert!(!row(2).contains("rust"), "and a segment too narrow for its name stays blank:\n{screen}");
}

#[test]
fn grouped_series_get_a_row_each_and_name_their_values() {
    let h = Harness::new(fixed(6, || week().unit("h").gap(1)), 34, 6);
    assert_eq!(
        h.screen(),
        "mon                       rust 3 h\n                          docs 1 h\n\ntue                       rust 1 h\n                          docs 3 h\n\n"
    );
    assert_eq!(tone(&h, 4, 0), accent_of(&h), "the first series keeps the accent");
    assert_ne!(tone(&h, 4, 1), tone(&h, 4, 0), "the second series is a different tone");
    assert_eq!(tone(&h, 4, 3), tone(&h, 4, 0), "and every category shows its series in the same order");
    assert_eq!(tone(&h, 20, 0), tone(&h, 4, 0), "the longest bar of the group fills the room");
    assert_ne!(tone(&h, 20, 1), tone(&h, 4, 1), "the shortest leaves it empty");
}

#[test]
fn a_narrow_grouped_chart_drops_the_labels_and_the_series_names() {
    let h = Harness::new(fixed(4, || week().unit("h").gap(0)), 20, 4);
    assert_eq!(h.screen(), "                 3 h\n     ▍           1 h\n     ▍           1 h\n                 3 h\n");
    assert_eq!(tone(&h, 0, 0), accent_of(&h), "the bars are still there");
}

#[test]
fn vertical_series_stack_and_group() {
    let stacked = Harness::new(fixed(7, || week().stacked().vertical()), 24, 7);
    assert_eq!(stacked.screen(), "     4           4\n\n\n\n\n\n    mon         tue\n");
    let accent = accent_of(&stacked);
    assert_eq!(tone(&stacked, 2, 5), accent, "the first series sits at the foot of the stack");
    assert_ne!(tone(&stacked, 2, 1), accent, "the second series sits on top of it");
    assert_eq!(tone(&stacked, 14, 5), accent, "and the order is the same in every category");
    assert_eq!(tone(&stacked, 14, 1), tone(&stacked, 2, 1));

    let grouped = Harness::new(fixed(7, || week().vertical()), 24, 7);
    assert_eq!(
        grouped.screen(),
        "  3                3\n\n\n       1      1\n     ▅▅▅▅▅  ▅▅▅▅▅\n\n    mon         tue\n"
    );
    assert_eq!(tone(&grouped, 0, 1), accent, "the first series stands on the left of its group");
    assert_eq!(tone(&grouped, 5, 5), tone(&stacked, 2, 1), "the second beside it, in the second tone");
    assert_ne!(tone(&grouped, 5, 1), tone(&grouped, 5, 5), "and reaches only a third as high");
}

#[test]
fn a_vertical_group_too_thin_for_its_bars_stacks_them_instead() {
    // Three cells for two categories leave one cell each: too thin for a bar per series, so the
    // series stack rather than being dropped.
    let h = Harness::new(fixed(6, || week().vertical()), 3, 6);
    let accent = accent_of(&h);
    assert_eq!(tone(&h, 0, 4), accent, "the first series still sits at the foot");
    assert_ne!(tone(&h, 0, 1), accent, "and the second on top: nothing was dropped");
    assert_eq!(tone(&h, 2, 4), accent, "the second category stacks as well");
}

// ---- Every state -----------------------------------------------------------------------------

#[test]
fn values_that_are_all_zero_leave_empty_bars_that_still_read_as_zero() {
    let h = Harness::new(fixed(3, || BarChart::new([Bar::new("api", 0.0), Bar::new("worker", 0.0)]).gap(0)), 24, 3);
    assert_eq!(h.screen(), "api                    0\nworker                 0\n\n");
    assert_ne!(tone(&h, 8, 0), accent_of(&h), "no bar is drawn");
    assert_ne!(tone(&h, 8, 1), accent_of(&h));
}

#[test]
fn a_value_far_below_the_largest_still_tints_a_cell() {
    let h = Harness::new(fixed(3, || BarChart::new([Bar::new("api", 5000.0), Bar::new("worker", 1.0)]).gap(0)), 24, 3);
    let accent = accent_of(&h);
    assert_eq!(h.screen(), "api                 5000\nworker ▏               1\n\n");
    assert_eq!(h.fg(7, 1), Some(accent), "the small bar is there to be seen");
    assert_eq!(tone(&h, 8, 0), accent, "beside a bar that fills the room");
}

#[test]
fn a_short_vertical_area_gives_up_the_value_row_and_then_the_labels() {
    let short = Harness::new(fixed(2, || services().vertical().max(100.0)), 24, 2);
    assert_eq!(short.screen(), "▃▃▃▃▃▃  ▂▂▂▂▂▂  ▇▇▇▇▇▇\n  api   postgr… worker\n");
    let shortest = Harness::new(fixed(1, || services().vertical().max(100.0)), 24, 1);
    assert_eq!(shortest.screen(), "▃▃▃▃▃▃  ▂▂▂▂▂▂  ▇▇▇▇▇▇\n", "one row keeps the bars and gives up the labels");
}

#[test]
fn a_disabled_chart_is_quiet_and_answers_nothing() {
    let mut h = Harness::new(
        demo(3, |selected| services().gap(0).selected(selected).on_select(|index| index).disabled(true)),
        30,
        3,
    );
    let muted = h.env().theme().color("muted").expect("muted");
    assert_eq!(
        h.screen(),
        "api             ▏           40\npostgres     ▌              25\nworker                    ● 90\n"
    );
    assert_eq!(tone(&h, 10, 0), muted, "every bar is muted, whatever its variant");
    assert_eq!(tone(&h, 10, 2), muted);
    assert_eq!(h.fg(0, 0), Some(muted), "and so is every label");
    assert_eq!(h.fg(28, 2), Some(muted), "and every value");
    h.press("tab").press("down");
    assert_eq!(h.app().picks, Vec::<usize>::new(), "no key reaches a disabled chart");
    let resting = h.screen();
    h.hover(4, 0);
    assert_eq!(h.screen(), resting, "and no ground rises under the pointer");
}

#[test]
fn a_chart_without_a_selection_shows_no_ground_and_takes_no_focus() {
    let mut h = Harness::new(fixed(3, || services().gap(0)), 30, 3);
    let resting = h.screen();
    h.hover(4, 0);
    assert_eq!(h.screen(), resting, "a chart nobody listens to does not answer the pointer");
    assert!(resting.starts_with("api "), "and keeps no room for a pillar:\n{resting}");
    h.press("tab").press("down");
    assert!(!h.is_focused("chart"), "it cannot be focused either");
}

// ---- Interaction -----------------------------------------------------------------------------

fn interactive(height: u16) -> Demo {
    demo(height, |selected| services().gap(0).selected(selected).on_select(|category| category))
}

fn standing(height: u16) -> Demo {
    demo(height, |selected| services().vertical().max(100.0).selected(selected).on_select(|category| category))
}

#[test]
fn keys_move_the_selection_along_the_bars_and_send_a_message() {
    let mut h = Harness::new(interactive(3), 30, 3);
    h.press("tab").press("down");
    assert_eq!((h.app().selected, h.app().picks.as_slice()), (Some(0), &[0][..]));
    h.press("down").press("down");
    assert_eq!(h.app().selected, Some(2));
    h.press("down");
    assert_eq!(h.app().picks, vec![0, 1, 2], "the last bar is the end of the way");
    h.press("k");
    assert_eq!(h.app().selected, Some(1));
    h.press("home");
    assert_eq!(h.app().selected, Some(0));
    h.press("end");
    assert_eq!(h.app().selected, Some(2));
    // A horizontal chart runs down the rows, so the sideways keys are not its axis.
    h.press("left").press("right");
    assert_eq!(h.app().selected, Some(2));
}

#[test]
fn a_vertical_chart_moves_sideways() {
    let mut h = Harness::new(standing(6), 24, 6);
    h.press("tab").press("right");
    assert_eq!(h.app().selected, Some(0));
    h.press("right");
    assert_eq!(h.app().selected, Some(1));
    h.press("down").press("up");
    assert_eq!(h.app().selected, Some(1), "up and down are not the axis of a vertical chart");
    h.press("l");
    assert_eq!(h.app().selected, Some(2));
}

#[test]
fn the_selected_row_carries_the_pillar_on_a_raised_ground() {
    let mut h = Harness::new(interactive(3), 30, 3);
    h.press("tab").press("down").press("down");
    assert_eq!(
        h.screen(),
        "  api            ▎          40\n▌ postgres    ▉             25\n  worker                  ● 90\n"
    );
    let active = h.env().theme().color("active").expect("active");
    assert_eq!(tone(&h, 3, 1), active, "the selected row rises");
    assert_eq!(h.fg(0, 1), Some(accent_of(&h)), "and the pillar marks it");
    assert_ne!(tone(&h, 3, 0), active, "the resting rows keep the canvas");
}

#[test]
fn hovering_raises_a_row_and_clicking_it_selects_it() {
    let mut h = Harness::new(interactive(3), 30, 3);
    h.hover(20, 2);
    let theme = h.env().theme();
    assert_eq!(tone(&h, 3, 2), theme.color("raised").expect("raised"), "the hovered row rises one step");
    assert_ne!(tone(&h, 3, 2), theme.color("active").expect("active"), "less than the selected one");
    h.click(20, 2);
    assert_eq!((h.app().selected, h.app().picks.as_slice()), (Some(2), &[2][..]));
    // Clicking the same row again changes nothing, so no message is sent twice.
    h.click(20, 2);
    assert_eq!(h.app().picks, vec![2]);
}

#[test]
fn clicking_a_vertical_column_selects_its_category_and_the_gap_selects_nothing() {
    let mut h = Harness::new(standing(6), 24, 6);
    h.click(17, 1);
    assert_eq!(h.app().selected, Some(2));
    h.click(7, 5);
    assert_eq!(h.app().picks, vec![2], "the column between two categories belongs to neither");
    let active = h.env().theme().color("active").expect("active");
    assert_eq!(tone(&h, 16, 0), active, "the whole column rises, label and all");
    assert_eq!(tone(&h, 16, 5), active);
}

#[test]
fn nothing_moves_when_a_category_is_hovered_or_selected() {
    // Every column but the two kept free for the pillar, which is the only cell a marked
    // category adds.
    let columns = |screen: &str| -> Vec<Vec<usize>> {
        screen
            .lines()
            .map(|line| {
                line.chars().enumerate().filter(|(i, c)| *i >= 2 && !c.is_whitespace()).map(|(i, _)| i).collect()
            })
            .collect()
    };
    for vertical in [false, true] {
        let build = move |selected: Option<usize>| {
            let chart = services().gap(0).selected(selected).on_select(|category| category);
            if vertical { chart.vertical().max(100.0) } else { chart }
        };
        let mut h = Harness::new(demo(6, build), 30, 6);
        let resting = h.screen();
        h.hover(20, 1);
        let hovered = h.screen();
        h.press("tab").press(if vertical { "right" } else { "down" });
        let selected = h.screen();
        for (state, screen) in [("hovered", &hovered), ("selected", &selected)] {
            assert_eq!(
                columns(screen),
                columns(&resting),
                "vertical={vertical}: the {state} chart moved a cell:\n{resting}\n{screen}"
            );
        }
    }
}

#[test]
fn the_lead_cells_are_there_before_anything_is_hovered() {
    // A chart that can show a selection keeps room for the pillar from the first frame, so the
    // bars do not jump sideways the moment the pointer arrives.
    let listening = Harness::new(interactive(3), 30, 3);
    let marked = Harness::new(fixed(3, || services().gap(0).selected(Some(1))), 30, 3);
    assert_eq!(listening.screen(), marked.screen().replace('▌', " "));
    assert!(marked.screen().contains('▌'), "a selection given from outside is marked too");
}

// ---- Themes, glyph modes and colour depth ----------------------------------------------------

#[test]
fn every_theme_draws_the_bars_the_values_and_the_selection() {
    for theme in THEMES {
        let mut h = Harness::new(interactive(3), 30, 3);
        h.set_theme(theme);
        h.press("tab").press("down");
        let screen = h.screen();
        assert!(screen.contains("api") && screen.contains("● 90"), "{theme}:\n{screen}");
        assert_eq!(h.fg(0, 0), Some(accent_of(&h)), "{theme} marks the selection with the pillar:\n{screen}");
        let active = h.env().theme().color("active").expect("active");
        assert_eq!(tone(&h, 3, 0), active, "{theme} raises the selected row");

        let mut stacked = Harness::new(fixed(3, || week().stacked().unit("h").gap(0)), 34, 3);
        stacked.set_theme(theme);
        let tones = [tone(&stacked, 5, 0), tone(&stacked, 24, 0)];
        assert_ne!(tones[0], tones[1], "{theme} tells two series apart");
    }
}

#[test]
fn every_glyph_mode_draws_a_stack_and_a_group() {
    for mode in [GlyphMode::Unicode, GlyphMode::Ascii, GlyphMode::Nerd] {
        for stacked in [false, true] {
            let mut h = Harness::new(
                fixed(6, move || {
                    let chart = week().unit("h").gap(0);
                    if stacked { chart.stacked() } else { chart }
                }),
                34,
                6,
            );
            h.set_glyph_mode(mode);
            let screen = h.screen();
            // A stack is read by its total, a group by the value of every series in it.
            let value = if stacked { "4 h" } else { "3 h" };
            assert!(screen.contains("mon") && screen.contains(value), "{mode:?} stacked={stacked}:\n{screen}");
            assert_eq!(tone(&h, 4, 0), accent_of(&h), "{mode:?} stacked={stacked} draws the first series");
            for bad in "[](){}|".chars() {
                assert!(!screen.contains(bad), "{mode:?} drew `{bad}`:\n{screen}");
            }
        }
    }
}

#[test]
fn the_series_ramp_stays_apart_when_the_terminal_has_few_colours() {
    for theme in THEMES {
        let colors = ThemeRegistry::builtin().resolve(theme).theme.expect("built-in themes resolve");
        let tones: Vec<Rgb> =
            (0..crate::theme::SERIES_COLORS).map(|index| series_fill(&colors, index, false)).collect();
        let indexed: Vec<u8> = tones.iter().map(|tone| tone.to_ansi256()).collect();
        for (index, tone) in indexed.iter().enumerate() {
            assert!(
                !indexed[..index].contains(tone),
                "{theme}: series {index} falls on the same 256-colour index as an earlier one: {indexed:?}"
            );
        }
        let count = crate::theme::SERIES_COLORS;
        assert_eq!(series_fill(&colors, count, false), tones[0], "the palette starts over after {count}");
    }
}

#[test]
fn the_theme_decides_the_series_tones_when_it_has_a_palette() {
    let theme = ThemeRegistry::builtin().resolve("monochrome").theme.expect("monochrome resolves");
    assert_eq!(series_fill(&theme, 0, false), theme.color("accent").expect("accent"));
    assert_ne!(series_fill(&theme, 1, false), series_fill(&theme, 0, false));
    // A disabled chart walks a quiet ramp that starts at the faintest tone.
    assert_eq!(series_fill(&theme, 0, true), theme.color("muted").expect("muted"));
}

#[test]
fn tiny_and_narrow_areas_never_panic() {
    for (width, height) in [(1, 1), (2, 2), (3, 4), (7, 3), (12, 6), (19, 9), (40, 2)] {
        for vertical in [false, true] {
            for stacked in [false, true] {
                let build = move |selected: Option<usize>| {
                    let chart = week().unit("h").selected(selected).on_select(|category| category);
                    let chart = if stacked { chart.stacked() } else { chart };
                    if vertical { chart.vertical() } else { chart }
                };
                let mut h = Harness::new(demo(height, build), width, height);
                h.press("tab").press("down").press("right");
                h.hover(0, 0);
                h.click(0, 0);
                assert!(h.screen().lines().count() <= usize::from(height), "{}", h.screen());
            }
        }
    }
}

#[test]
fn a_chart_stands_the_pillar_only_where_the_row_model_applies() {
    let mut env = Env::builtin();
    env.set_slide(false);
    let mut h = Harness::with_env(interactive(3), env, 30, 3);
    h.press("tab").press("down");
    assert_eq!(h.fg(0, 0), Some(accent_of(&h)), "the pillar is not a slide, so it stays:\n{}", h.screen());
    let mut vertical = Harness::new(standing(6), 24, 6);
    vertical.press("tab").press("right");
    assert!(!vertical.screen().contains('▌'), "a standing bar has no row to lead:\n{}", vertical.screen());
}

// ---- Fixed tones -----------------------------------------------------------------------------

/// Three kinds of work over two days, each series pinned to `tones` when given.
fn pinned(tones: Option<[usize; 3]>) -> BarChart<usize> {
    let series = [("rust", [3.0, 1.0]), ("docs", [1.0, 2.0]), ("review", [2.0, 1.0])];
    let series = series.iter().enumerate().map(move |(position, (name, values))| {
        let series = Series::new(*name, *values);
        match tones {
            Some(tones) => series.tone(tones[position]),
            None => series,
        }
    });
    BarChart::series(["mon", "tue"], series)
}

#[test]
fn tones_that_match_the_positions_draw_the_same_chart() {
    for (width, height, vertical, stacked, disabled) in [
        (40, 8, false, false, false),
        (40, 3, false, true, false),
        (30, 9, true, false, false),
        (30, 9, true, true, false),
        (40, 8, false, false, true),
    ] {
        let build = move |tones: Option<[usize; 3]>| {
            move || {
                let mut chart = pinned(tones).disabled(disabled);
                if vertical {
                    chart = chart.vertical();
                }
                if stacked { chart.stacked() } else { chart }
            }
        };
        let plain = Harness::new(fixed(height, build(None)), width, height);
        let same = Harness::new(fixed(height, build(Some([0, 1, 2]))), width, height);
        assert_eq!(plain.buffer(), same.buffer(), "{width}×{height} vertical={vertical} stacked={stacked}");
    }
}

#[test]
fn a_pinned_tone_follows_its_category_whichever_others_are_shown() {
    // This week shows all three kinds; last week only docs and review. Docs keeps tone 1.
    let this_week = Harness::new(fixed(3, || pinned(Some([0, 1, 2])).stacked().gap(0)), 40, 3);
    let last_week = Harness::new(
        fixed(3, || {
            let docs = Series::new("docs", [2.0, 2.0]).tone(1);
            let review = Series::new("review", [2.0, 2.0]).tone(2);
            BarChart::series(["mon", "tue"], [docs, review]).stacked().gap(0)
        }),
        40,
        3,
    );
    let theme = last_week.env().theme();
    let docs = theme.series_color(1);
    assert_eq!(tone(&last_week, 5, 0), docs, "docs leads last week's stack in its own tone, not the first tone");
    assert_ne!(tone(&last_week, 5, 0), theme.series_color(0));
    assert!(
        (0..40).any(|x| this_week.bg(x, 0) == Some(docs)),
        "and it is the same tone this week, beside rust:\n{}",
        this_week.screen()
    );
}

#[test]
fn a_disabled_chart_keeps_pinned_categories_apart() {
    let h = Harness::new(
        fixed(3, || {
            let docs = Series::new("docs", [1.0]).tone(1);
            let review = Series::new("review", [1.0]).tone(2);
            BarChart::series(["mon"], [docs, review]).stacked().disabled(true).gap(0)
        }),
        30,
        3,
    );
    assert_eq!(tone(&h, 5, 0), series_fill(h.env().theme(), 1, true), "the quiet ramp follows the pinned tone too");
}
