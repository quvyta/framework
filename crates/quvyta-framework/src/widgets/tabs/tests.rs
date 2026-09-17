use super::*;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{ContextItem, TabEdit};

struct Demo {
    active: usize,
    width: u16,
}

impl App for Demo {
    type Msg = usize;
    fn update(&mut self, index: usize) -> Command<usize> {
        self.active = index;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, usize>) {
        ui.add(Tabs::new(["Demo", "Code", "Guide", "Reference"]).numbered(true).active(self.active).on_select(|i| i))
            .width(crate::widget::Length::Cells(self.width));
    }
}

#[test]
fn opens_tabs_by_keys_numbers_and_clicks() {
    let mut h = Harness::new(Demo { active: 0, width: 60 }, 60, 1);
    assert_eq!(h.screen(), "▌ 1 Demo     2 Code      3 Guide      4 Reference\n");
    assert_eq!(h.bg(2, 0), h.env().theme().color("active"));
    h.press("tab").press("right");
    assert_eq!(h.app().active, 1);
    h.press("4");
    assert_eq!(h.app().active, 3);
    h.click_text("Guide");
    assert_eq!(h.app().active, 2);
}

#[test]
fn turning_slide_off_moves_no_tab_and_changes_no_width() {
    let mut env = crate::env::Env::builtin();
    env.set_slide(false);
    let off = Harness::with_env(Demo { active: 0, width: 60 }, env, 60, 1);
    let on = Harness::new(Demo { active: 0, width: 60 }, 60, 1);
    assert_eq!(on.screen(), "▌ 1 Demo     2 Code      3 Guide      4 Reference\n");
    assert_eq!(off.screen(), "▌ 1 Demo      2 Code      3 Guide      4 Reference\n");
    // The open tab is the same surface either way, and every resting tab starts in the same column:
    // only its label sits one cell further left while slide is on.
    let raised = |h: &Harness<Demo>| (0..60).filter(|x| h.bg(*x, 0) == h.bg(2, 0)).count();
    assert_eq!(raised(&on), raised(&off));
    for label in ["2 Code", "3 Guide", "4 Reference"] {
        let (x_on, _) = on.find(label).expect("label with slide");
        let (x_off, _) = off.find(label).expect("label without slide");
        assert_eq!(x_off, x_on + 1, "{label}");
    }
}

#[test]
fn scrolls_to_keep_the_open_tab_visible() {
    let h = Harness::new(Demo { active: 3, width: 30 }, 30, 1);
    let screen = h.screen();
    assert!(screen.contains("4 Reference"), "{screen}");
    assert!(screen.starts_with(" ◀ "), "{screen}");
}

#[test]
fn plain_tabs_ignore_optional_keys_and_buttons() {
    let mut h = Harness::new(Demo { active: 1, width: 60 }, 60, 1);
    h.press("tab").press("ctrl+w").press("ctrl+shift+right").press("ctrl+pgdn");
    assert_eq!(h.app().active, 1);
    h.mouse(MouseKind::Down(MouseButton::Middle), 3, 0);
    assert_eq!(h.screen(), "▌ 1 Demo    ▌ 2 Code     3 Guide      4 Reference\n");
}

/// Files open in an editor, with every option under test control.
struct Editor {
    files: Vec<&'static str>,
    active: usize,
    width: u16,
    tab_width: TabWidth,
    overflow: Overflow,
    closable: bool,
    reorderable: bool,
    menu: bool,
    /// The first position in view after each step a dragged tab scrolled the strip.
    scrolled: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
enum Msg {
    Open(usize),
    Edit(TabEdit),
    /// Closes every tab but this one.
    KeepOnly(usize),
    Scrolled(usize),
}

impl App for Editor {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open(index) => self.active = index,
            Msg::Edit(edit) => edit.apply(&mut self.files, &mut self.active),
            Msg::KeepOnly(index) => {
                self.files = vec![self.files[index]];
                self.active = 0;
            }
            Msg::Scrolled(first) => self.scrolled.push(first),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut tabs = Tabs::new(self.files.clone())
            .active(self.active)
            .on_select(Msg::Open)
            .tab_width(self.tab_width)
            .overflow(self.overflow)
            .pinned([0])
            .on_drag_scroll(Msg::Scrolled);
        if self.closable {
            tabs = tabs.closable(|i| Msg::Edit(TabEdit::Close(i)));
        }
        if self.menu {
            tabs = tabs.context_menu(|index| {
                vec![
                    ContextItem::new("Close", Msg::Edit(TabEdit::Close(index))),
                    ContextItem::new("Close others", Msg::KeepOnly(index)),
                ]
            });
        }
        if self.reorderable {
            tabs = tabs.reorderable(|from, to| Msg::Edit(TabEdit::Move { from, to }));
        }
        ui.add(tabs).width(crate::widget::Length::Cells(self.width)).id("files");
    }
}

fn editor(width: u16) -> Editor {
    Editor {
        files: vec!["main.rs", "app.rs", "tabs.rs", "theme.toml"],
        active: 0,
        width,
        tab_width: TabWidth::Fit,
        overflow: Overflow::Arrows,
        closable: false,
        reorderable: false,
        menu: false,
        scrolled: Vec::new(),
    }
}

#[test]
fn closable_tabs_close_by_mark_middle_click_and_ctrl_w() {
    let mut app = editor(66);
    app.closable = true;
    let mut h = Harness::new(app, 66, 3);
    assert_eq!(h.screen(), "▌ main.rs     app.rs    ×    tabs.rs    ×    theme.toml    ×\n\n\n");
    let (mark, _) = h.find("×").expect("close mark");
    let (label, _) = h.find("app.rs").expect("second tab");
    let column = u16::try_from(mark).expect("on screen");
    let idle = h.fg(column, 0);
    h.hover(label, 0);
    assert_ne!(h.fg(column, 0), idle, "the mark brightens on a hovered tab");
    let tab_surface = h.bg(column + 3, 0);
    h.hover(mark - 1, 0);
    let lit = h.bg(column, 0);
    assert_ne!(lit, tab_surface, "the mark lights up under the pointer");
    assert_eq!((h.bg(column - 1, 0), h.bg(column + 1, 0)), (lit, lit), "all three cells light up together");
    h.click(mark + 1, 0);
    assert_eq!(h.app().files, vec!["main.rs", "tabs.rs", "theme.toml"], "the cell after the glyph closes too");
    assert_eq!(h.app().files, vec!["main.rs", "tabs.rs", "theme.toml"]);
    let (label, _) = h.find("tabs.rs").expect("next tab");
    h.mouse(MouseKind::Down(MouseButton::Middle), label, 0);
    assert_eq!(h.app().files, vec!["main.rs", "theme.toml"]);
    h.press("tab").press("right").press("ctrl+w");
    assert_eq!(h.app().files, vec!["main.rs"]);
    h.press("ctrl+w");
    assert_eq!(h.app().files, vec!["main.rs"], "pinned tabs stay open");
}

#[test]
fn fixed_and_fill_widths_truncate_and_share_space() {
    let mut app = editor(40);
    app.tab_width = TabWidth::Fixed(9);
    let h = Harness::new(app, 40, 1);
    assert_eq!(h.screen(), "▌ main…    app.…     tabs…     them…\n");
    let mut app = editor(56);
    app.tab_width = TabWidth::Fill;
    let h = Harness::new(app, 56, 1);
    assert_eq!(h.screen(), "▌ main.rs       app.rs        tabs.rs       theme.to…\n");
    assert_eq!(h.bg(9, 0), h.env().theme().color("active"));
}

#[test]
fn menu_lists_hidden_tabs_and_opens_one() {
    let mut app = editor(30);
    app.overflow = Overflow::Menu;
    let mut h = Harness::new(app, 30, 6);
    assert_eq!(h.screen().lines().next(), Some("▌ main.rs     app.rs      ▾ 2"));
    h.click_text("▾").advance(std::time::Duration::from_millis(300));
    let screen = h.screen();
    assert!(screen.contains("tabs.rs") && screen.contains("theme.toml"), "{screen}");
    h.click_text("theme.toml");
    assert_eq!(h.app().active, 3);
    assert!(h.screen().lines().next().is_some_and(|line| line.contains("theme.toml")), "{}", h.screen());
    h.press("tab").press("down").advance(std::time::Duration::from_millis(300));
    h.press("down").press("enter");
    assert_eq!(h.app().active, 1);
}

#[test]
fn dragging_moves_a_tab_with_a_ghost_and_keys_move_the_open_tab() {
    let mut app = editor(60);
    app.reorderable = true;
    let mut h = Harness::new(app, 60, 1);
    h.mouse(MouseKind::Down(MouseButton::Left), 3, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 16, 0);
    let theme = h.env().theme();
    let (canvas, active) = (theme.color("canvas"), theme.color("active"));
    assert!(h.bg(15, 0) != canvas && h.bg(15, 0) != active, "the ghost floats at the pointer");
    assert!(h.bg(12, 0) != canvas && h.bg(12, 0) != h.bg(15, 0), "a tinted slot waits where it lands");
    h.mouse(MouseKind::Drag(MouseButton::Left), 25, 0);
    h.mouse(MouseKind::Up(MouseButton::Left), 25, 0);
    assert_eq!(h.app().files, vec!["app.rs", "tabs.rs", "main.rs", "theme.toml"]);
    assert_eq!(h.app().active, 2);
    h.press("tab").press("ctrl+shift+right");
    assert_eq!(h.app().files, vec!["app.rs", "tabs.rs", "theme.toml", "main.rs"]);
    h.press("ctrl+shift+left").press("ctrl+shift+left");
    assert_eq!(h.app().files, vec!["app.rs", "main.rs", "tabs.rs", "theme.toml"]);
}

/// The sum of a colour's channels, to compare how bright two tones are.
fn brightness(color: Option<crate::color::Rgb>) -> u32 {
    color.map_or(0, |c| u32::from(c.r) + u32::from(c.g) + u32::from(c.b))
}

/// The background of three cells from `x`.
fn three(h: &Harness<Editor>, x: u16) -> [Option<crate::color::Rgb>; 3] {
    [h.bg(x, 0), h.bg(x + 1, 0), h.bg(x + 2, 0)]
}

#[test]
fn overflowing_tabs_scroll_with_arrow_buttons() {
    let mut h = Harness::new(editor(30), 30, 1);
    let theme = h.env().theme();
    let (surface, raised, active) = (theme.color("surface"), theme.color("raised"), theme.color("active"));
    assert_eq!(h.screen(), " ◀  ▌ main.rs               ▶\n", "tabs that do not fit get arrows by default");
    assert_eq!(three(&h, 0), [surface; 3], "nothing is hidden before: the back arrow sinks");
    assert_eq!(three(&h, 27), [raised; 3], "a live arrow is three raised cells");

    h.hover(28, 0);
    assert_eq!(h.screen(), " ◀  ▌ main.rs              ▌▶\n", "hover raises the pillar in its first cell");
    assert_eq!(three(&h, 27), [active; 3], "and brightens all three cells");
    assert_ne!(h.fg(27, 0), active, "the pillar shows on the lit cells");

    h.click(29, 0);
    assert_eq!(h.screen(), " ◀   app.rs                ▌▶\n", "a click scrolls one tab");
    assert_eq!(h.app().active, 0, "and opens nothing");
    let pressed = h.bg(28, 0);
    assert!(brightness(pressed) > brightness(active), "a press flashes one tone brighter");
    assert_eq!(three(&h, 27), [pressed; 3]);
    h.advance(Duration::from_millis(200));
    assert_eq!(three(&h, 27), [active; 3], "then settles back to hover");
    assert_eq!(three(&h, 0), [raised; 3], "a tab is hidden before now: the back arrow lives");

    h.hover(1, 0);
    assert_eq!(h.screen(), "▌◀   app.rs                 ▶\n");
    h.click(1, 0);
    assert_eq!(h.screen(), " ◀  ▌ main.rs               ▶\n", "back at the start the arrow under the pointer sinks");
    assert_eq!(three(&h, 0), [surface; 3], "no hover and no flash on a spent arrow");
    h.click(1, 0);
    assert_eq!(h.screen(), " ◀  ▌ main.rs               ▶\n", "and it ignores presses");

    h.click(28, 0).click(28, 0).click(28, 0);
    h.hover(20, 0);
    assert_eq!(h.screen(), " ◀   theme.toml             ▶\n", "one tab per click up to the last");
    assert_eq!(three(&h, 27), [surface; 3], "at the end the forward arrow sinks");
    h.click(28, 0);
    assert!(h.screen().contains("theme.toml"), "{}", h.screen());
    assert_eq!(h.app().active, 0);
}

#[test]
fn keys_and_the_wheel_scroll_like_the_arrows() {
    let mut h = Harness::new(editor(30), 30, 1);
    let raised = h.env().theme().color("raised");
    h.press("tab").press("ctrl+pgdn");
    assert_eq!(h.screen(), " ◀   app.rs                ▌▶\n", "a scrolling key flashes its arrow");
    assert!(brightness(h.bg(28, 0)) > brightness(raised));
    h.advance(Duration::from_millis(200));
    assert_eq!(h.screen(), " ◀   app.rs                 ▶\n");
    assert_eq!(three(&h, 27), [raised; 3]);
    h.mouse(MouseKind::ScrollDown, 10, 0);
    assert_eq!(h.screen(), " ◀  ▌ tabs.rs               ▶\n", "the wheel scrolls too; the tab under the pointer rises");
    assert_eq!(three(&h, 27), [raised; 3], "without flashing an arrow");
    h.mouse(MouseKind::ScrollUp, 28, 0);
    assert_eq!(h.screen(), " ◀   app.rs                ▌▶\n", "over an arrow as well");
    h.hover(10, 0);
    h.press("ctrl+pgup").press("ctrl+pgup").press("ctrl+pgup");
    assert_eq!(h.screen(), " ◀  ▌ main.rs               ▶\n", "and stops at the start");
    h.press("right");
    assert_eq!(h.app().active, 1);
    assert_eq!(h.screen(), " ◀  ▌ app.rs                ▶\n", "opening a tab brings it into view");
    h.press("end").press("right").press("right");
    assert_eq!(h.screen(), " ◀  ▌ theme.toml            ▶\n");

    let mut fits = Harness::new(editor(60), 60, 1);
    let screen = fits.screen();
    fits.press("tab").press("ctrl+pgdn").mouse(MouseKind::ScrollDown, 10, 0);
    assert_eq!(fits.screen(), screen, "a strip that fits has no arrows and does not scroll");
    assert!(!screen.contains('◀') && !screen.contains('▶'), "{screen}");
}

#[test]
fn arrows_stay_steady_with_reduced_motion() {
    let mut h = Harness::new(editor(30), 30, 1);
    h.set_reduced_motion(true);
    let active = h.env().theme().color("active");
    h.hover(28, 0);
    assert_eq!(h.screen(), " ◀  ▌ main.rs              ▌▶\n");
    h.click(28, 0);
    assert_eq!(h.screen(), " ◀   app.rs                ▌▶\n", "scrolling is a jump, never an animation");
    assert!(brightness(h.bg(28, 0)) > brightness(active), "the press is a tone, not motion: it still shows");
    h.advance(Duration::from_millis(200));
    assert_eq!(three(&h, 27), [active; 3]);
}

#[test]
fn narrow_strips_cut_the_open_tab_and_keep_scrolling() {
    let h = Harness::new(editor(14), 14, 1);
    assert_eq!(h.screen(), " ◀  ▌ m…    ▶\n", "between the arrows the open tab is cut, never gone");
    let mut h = Harness::new(editor(9), 9, 1);
    assert_eq!(h.screen(), "▌ main…\n", "too narrow for arrows: the tab takes the strip");
    h.mouse(MouseKind::ScrollDown, 3, 0);
    assert_eq!(h.screen(), "▌ app.…\n", "the wheel still scrolls");
    h.press("tab").press("ctrl+pgdn");
    assert_eq!(h.screen(), "▌ tabs…\n", "and so do the keys");
    let h = Harness::new(editor(4), 4, 1);
    assert_eq!(h.screen(), "\n", "no room for any tab paints nothing rather than a broken tab");
}

#[test]
fn closing_tabs_never_leaves_room_while_tabs_hide_before() {
    let mut h = Harness::new(editor(30), 30, 1);
    h.click(28, 0).click(28, 0).click(28, 0);
    assert!(h.screen().contains("theme.toml"), "{}", h.screen());
    h.send(Msg::Edit(TabEdit::Close(2))).send(Msg::Edit(TabEdit::Close(1)));
    assert_eq!(h.screen(), "▌ main.rs     theme.toml\n", "the strip scrolls back once the rest fits");
}

#[test]
fn arrows_in_ascii_are_coloured_cells() {
    let mut h = Harness::new(editor(30), 30, 1);
    h.set_glyph_mode(GlyphMode::Ascii);
    let active = h.env().theme().color("active");
    assert_eq!(h.screen(), " <    main.rs               >\n");
    h.hover(28, 0);
    assert_eq!(h.screen(), " <    main.rs               >\n", "the pillar has no glyph in ASCII");
    assert_ne!(h.bg(27, 0), active, "it is a coloured cell");
    assert_eq!((h.bg(28, 0), h.bg(29, 0)), (active, active));
}

#[test]
fn a_dragged_tab_floats_between_the_arrows() {
    let mut app = editor(30);
    app.reorderable = true;
    let mut h = Harness::new(app, 30, 1);
    let theme = h.env().theme();
    let (canvas, raised, active) = (theme.color("canvas"), theme.color("raised"), theme.color("active"));
    h.mouse(MouseKind::Down(MouseButton::Left), 6, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 29, 0);
    assert!(h.bg(25, 0) != canvas && h.bg(25, 0) != raised, "the ghost reaches the lane's end");
    assert_eq!(h.bg(26, 0), canvas, "and keeps the gap before the arrow");
    assert_eq!(three(&h, 27), [active; 3], "the arrow stays itself under the ghost's path, lit as the tab rests on it");
    h.mouse(MouseKind::Up(MouseButton::Left), 29, 0);
    assert_eq!(h.app().files, vec!["main.rs", "app.rs", "tabs.rs", "theme.toml"]);
    assert_eq!(h.bg(25, 0), canvas, "released over the arrow, the drag ends");
}

#[test]
fn menu_control_is_a_button_and_a_press_on_a_tab_closes_the_menu_and_opens_the_tab() {
    let mut app = editor(30);
    app.overflow = Overflow::Menu;
    let mut h = Harness::new(app, 30, 6);
    let theme = h.env().theme();
    let (raised, active, accent) = (theme.color("raised"), theme.color("active"), theme.color("accent"));
    let first = |h: &Harness<Editor>| h.screen().lines().next().unwrap_or_default().to_owned();
    let enter = Duration::from_millis(300);
    assert_eq!(first(&h), "▌ main.rs     app.rs      ▾ 2");
    assert_eq!([h.bg(25, 0), h.bg(27, 0), h.bg(29, 0)], [raised; 3], "a space, the chevron, the count, a space");
    h.hover(27, 0);
    assert_eq!(first(&h), "▌ main.rs     app.rs     ▌▾ 2", "hover raises the pillar in its first cell");
    assert_eq!([h.bg(25, 0), h.bg(26, 0), h.bg(29, 0)], [active; 3]);

    h.click(27, 0).advance(enter);
    assert!(h.screen().contains("theme.toml"), "{}", h.screen());
    assert_eq!(first(&h), "▌ main.rs     app.rs     ▌▾ 2", "open, it stays lit");
    assert_eq!((h.fg(25, 0), h.fg(26, 0)), (accent, accent), "with a steady accent pillar and chevron");

    h.click(27, 0).advance(enter);
    assert!(!h.screen().contains("theme.toml"), "a press on the control only closes:\n{}", h.screen());
    assert_eq!(h.app().active, 0);

    h.click(27, 0).advance(enter);
    h.click_text("app.rs");
    assert_eq!(h.app().active, 1, "one press on a tab closes the menu and opens the tab");
    assert!(!h.screen().contains("theme.toml"), "{}", h.screen());
}

#[test]
fn a_menu_strip_with_no_room_for_a_tab_lists_every_tab() {
    let mut app = editor(7);
    app.overflow = Overflow::Menu;
    app.active = 2;
    let mut h = Harness::new(app, 24, 6);
    assert_eq!(h.screen().lines().next(), Some("   ▾ 4"), "{}", h.screen());
    h.click(4, 0).advance(Duration::from_millis(300));
    let screen = h.screen();
    for file in ["main.rs", "app.rs", "tabs.rs", "theme.toml"] {
        assert!(screen.contains(file), "{file} is in the menu:\n{screen}");
    }
    let check = h.env().icons().glyph("check").into_owned();
    assert!(screen.lines().any(|line| line.contains("tabs.rs") && line.contains(&check)), "{screen}");
    h.press("up").press("enter");
    assert_eq!(h.app().active, 1, "the menu starts on the open tab");
}

#[test]
fn the_close_mark_lights_up_on_the_open_tab_too() {
    let mut app = editor(66);
    app.closable = true;
    let mut h = Harness::new(app, 66, 1);
    h.click_text("app.rs");
    let (mark, _) = h.find("×").expect("close mark of the open tab");
    let column = u16::try_from(mark).expect("on screen");
    let surface = h.bg(column - 4, 0);
    assert_eq!(surface, h.env().theme().color("active"), "the open tab is the active tone");
    h.hover(mark, 0);
    let lit = h.bg(column, 0);
    assert!(brightness(lit) > brightness(surface), "the lit mark climbs above the open tab");
    assert_eq!([h.bg(column - 1, 0), h.bg(column + 1, 0)], [lit, lit], "all three cells light up together");
}

/// An editor 66 cells wide whose tabs have a context menu when `menu` is set.
fn with_menu(menu: bool) -> Harness<Editor> {
    let mut h = Harness::new(Editor { menu, ..editor(66) }, 66, 6);
    h.set_reduced_motion(true);
    h
}

fn right_click(h: &mut Harness<Editor>, x: i32, y: i32) {
    h.mouse(MouseKind::Down(MouseButton::Right), x, y);
    h.mouse(MouseKind::Up(MouseButton::Right), x, y);
}

#[test]
fn a_right_click_does_nothing_without_a_context_menu() {
    let mut h = with_menu(false);
    let screen = h.screen();
    let (x, y) = h.find("tabs.rs").expect("tab");
    right_click(&mut h, x, y);
    h.press("tab").press("menu").press("shift+f10");
    assert_eq!(h.app().active, 0);
    assert_eq!(h.screen().lines().skip(1).collect::<String>(), "", "no menu:\n{}", h.screen());
    h.press("shift+tab").hover(x, 4);
    assert_eq!(h.screen(), screen);
}

#[test]
fn a_right_click_opens_the_menu_of_the_tab_under_the_pointer() {
    let mut h = with_menu(true);
    let (x, y) = h.find("tabs.rs").expect("tab");
    right_click(&mut h, x + 2, y);
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert!(lines[1].contains("Close") && lines[2].contains("Close others"), "at the pointer:\n{screen}");
    let column = |line: &str, text: &str| line.find(text).map(|byte| line[..byte].chars().count());
    assert_eq!(column(lines[1], "Close"), Some(usize::try_from(x).expect("on screen") + 4), "{screen}");
    assert_eq!(h.app().active, 0, "a right click opens no tab");
    h.hover(x + 6, 2);
    let surface = h.bg(u16::try_from(x).expect("on screen"), 0);
    assert!(surface.is_some() && surface != h.bg(0, 5), "its tab stays raised while the pointer is on the menu");
    h.click_text("Close others");
    assert_eq!(h.app().files, vec!["tabs.rs"], "the entry acts on the tab it was opened for");
    assert!(!h.screen().contains("Close"), "and the menu closes:\n{}", h.screen());
}

#[test]
fn a_right_click_on_another_tab_moves_the_menu_and_a_left_click_closes_it() {
    let mut h = with_menu(true);
    let (tabs, _) = h.find("tabs.rs").expect("tab");
    let (app, _) = h.find("app.rs").expect("tab");
    right_click(&mut h, tabs, 0);
    right_click(&mut h, app, 0);
    h.press("enter");
    assert_eq!(h.app().files, vec!["main.rs", "app.rs", "tabs.rs", "theme.toml"], "nothing is highlighted yet");
    h.press("down").press("enter");
    assert_eq!(h.app().files, vec!["main.rs", "tabs.rs", "theme.toml"], "Close closed the second tab");

    right_click(&mut h, tabs, 0);
    let (theme, _) = h.find("theme.toml").expect("tab");
    h.click(theme, 0);
    assert_eq!(h.app().active, 2, "one left click closes the menu and opens the tab");
    assert!(!h.screen().contains("Close"), "{}", h.screen());
}

#[test]
fn the_menu_key_opens_the_menu_of_the_open_tab() {
    let mut h = with_menu(true);
    h.click_text("app.rs");
    h.press("menu");
    let screen = h.screen();
    assert!(screen.lines().nth(1).is_some_and(|line| line.contains("▌") && line.contains("Close")), "{screen}");
    h.press("down").press("enter");
    assert_eq!(h.app().files, vec!["app.rs"], "the keyboard chooses too");

    let mut h = with_menu(true);
    h.press("tab").press("shift+f10").press("esc");
    assert!(!h.screen().contains("Close"), "esc closes:\n{}", h.screen());
    h.press("right");
    assert_eq!(h.app().active, 1, "and the tabs have the keys again");
}

#[test]
fn the_menu_key_scrolls_the_open_tab_into_view_first() {
    let mut h = Harness::new(Editor { menu: true, ..editor(30) }, 30, 6);
    h.set_reduced_motion(true);
    h.click(28, 0).click(28, 0);
    assert!(!h.screen().lines().next().is_some_and(|line| line.contains("main.rs")), "{}", h.screen());
    h.press("menu");
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    let tab = lines[0].find("main.rs").expect("the open tab is back in view");
    let item = lines[1].find("Close").expect("the menu opens below it");
    assert!(item >= tab.saturating_sub(4), "under the open tab, not at the strip's start:\n{screen}");
}

#[test]
fn the_context_menu_and_the_hidden_tabs_menu_take_turns() {
    let mut h = Harness::new(Editor { menu: true, overflow: Overflow::Menu, ..editor(30) }, 30, 8);
    h.set_reduced_motion(true);
    h.click(27, 0);
    assert!(h.screen().contains("theme.toml"), "the hidden tabs are listed:\n{}", h.screen());
    right_click(&mut h, 3, 0);
    let screen = h.screen();
    assert!(screen.contains("Close others") && !screen.contains("theme.toml"), "a right click swaps menus:\n{screen}");
    h.click(27, 0);
    let screen = h.screen();
    assert!(
        !screen.contains("Close others") && screen.contains("theme.toml"),
        "and a press on the control back:\n{screen}"
    );
}

#[test]
fn a_menu_whose_tab_closed_elsewhere_goes_away() {
    let mut h = with_menu(true);
    let (x, _) = h.find("theme.toml").expect("tab");
    right_click(&mut h, x, 0);
    h.send(Msg::Edit(TabEdit::Close(3)));
    assert!(!h.screen().contains("Close"), "no menu for a tab that is gone:\n{}", h.screen());
    h.press("right");
    assert_eq!(h.app().active, 1, "and the keys are the strip's again");
}

/// A reorderable strip 30 cells wide that shows one of its four tabs at a time.
fn dragging_strip(active: usize) -> Harness<Editor> {
    Harness::new(Editor { reorderable: true, active, ..editor(30) }, 30, 1)
}

const MS: fn(u64) -> Duration = Duration::from_millis;

#[test]
fn a_tab_held_on_an_arrow_scrolls_the_strip_after_a_delay_then_steadily_to_the_end() {
    let mut h = dragging_strip(0);
    let theme = h.env().theme();
    let (surface, active) = (theme.color("surface"), theme.color("active"));
    h.mouse(MouseKind::Down(MouseButton::Left), 6, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 28, 0);
    assert_eq!(three(&h, 27), [active; 3], "the arrow lights up as soon as the tab rests on it");
    h.advance(MS(399));
    assert_eq!(h.app().scrolled, Vec::<usize>::new(), "nothing scrolls before the delay");
    h.advance(MS(1));
    assert_eq!(h.app().scrolled, vec![1], "one tab at the delay");
    assert!(brightness(h.bg(28, 0)) > brightness(active), "the step flashes the arrow like a press");
    h.mouse(MouseKind::Drag(MouseButton::Left), 29, 0);
    h.advance(MS(149));
    assert_eq!(h.app().scrolled, vec![1], "moving on the arrow keeps the rhythm");
    h.advance(MS(1));
    assert_eq!(h.app().scrolled, vec![1, 2], "then one tab every 150 ms");
    h.advance(MS(150));
    assert_eq!(h.app().scrolled, vec![1, 2, 3]);
    h.advance(MS(150)).advance(MS(150)).advance(MS(1000));
    assert_eq!(h.app().scrolled, vec![1, 2, 3], "and it stops at the end");
    assert_eq!(three(&h, 27), [surface; 3], "where the arrow sinks");
    h.mouse(MouseKind::Up(MouseButton::Left), 28, 0);
    assert_eq!(
        h.app().files,
        vec!["app.rs", "tabs.rs", "theme.toml", "main.rs"],
        "dropped on the arrow, it lands on the last tab in view"
    );
    assert_eq!(h.app().active, 3);
    assert_eq!(h.screen(), " ◀  ▌ main.rs               ▶\n", "and stays in view where it landed");
}

#[test]
fn leaving_the_arrow_stops_at_once_and_coming_back_waits_again() {
    let mut h = dragging_strip(3);
    assert_eq!(h.screen(), " ◀  ▌ theme.toml            ▶\n");
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 1, 0);
    h.advance(MS(400));
    assert_eq!(h.app().scrolled, vec![2]);
    h.mouse(MouseKind::Drag(MouseButton::Left), 15, 0);
    h.advance(MS(150)).advance(MS(1000));
    assert_eq!(h.app().scrolled, vec![2], "off the arrow nothing scrolls");
    assert_eq!(three(&h, 0), [h.env().theme().color("raised"); 3], "and the arrow goes back to rest");
    h.mouse(MouseKind::Drag(MouseButton::Left), 0, 0);
    h.advance(MS(399));
    assert_eq!(h.app().scrolled, vec![2], "back on it, the delay starts over");
    h.advance(MS(1));
    assert_eq!(h.app().scrolled, vec![2, 1]);
    h.mouse(MouseKind::Up(MouseButton::Left), 0, 0);
    assert_eq!(
        h.app().files,
        vec!["main.rs", "theme.toml", "app.rs", "tabs.rs"],
        "dropped on the back arrow, it lands on the first tab in view"
    );
}

#[test]
fn pulling_further_past_the_end_scrolls_faster() {
    let mut h = dragging_strip(0);
    h.mouse(MouseKind::Down(MouseButton::Left), 6, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 32, 0);
    h.advance(MS(400));
    assert_eq!(h.app().scrolled, vec![1], "the first step still waits the whole delay");
    h.advance(MS(60));
    assert_eq!(h.app().scrolled, vec![1, 2], "three cells past the edge, a step every 60 ms");
    h.mouse(MouseKind::Drag(MouseButton::Left), 28, 0);
    h.advance(MS(60));
    assert_eq!(h.app().scrolled, vec![1, 2, 3], "the step already due keeps its time");
    h.mouse(MouseKind::Up(MouseButton::Left), 28, 0);
}

#[test]
fn only_a_dragged_tab_scrolls_the_strip() {
    let mut h = dragging_strip(0);
    let screen = h.screen();
    h.hover(28, 0).advance(MS(1000));
    assert_eq!(
        (h.app().scrolled.len(), h.screen()),
        (0, screen.replace(" ▶", "▌▶")),
        "hovering an arrow only lights it"
    );

    h.mouse(MouseKind::Down(MouseButton::Left), 6, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 7, 0);
    h.advance(MS(1000));
    h.mouse(MouseKind::Up(MouseButton::Left), 7, 0);
    assert!(h.app().scrolled.is_empty(), "a press that has not become a drag waits for nothing");

    let mut plain = Harness::new(editor(30), 30, 1);
    plain.mouse(MouseKind::Down(MouseButton::Left), 6, 0);
    plain.mouse(MouseKind::Drag(MouseButton::Left), 28, 0);
    plain.advance(MS(400)).advance(MS(1000));
    plain.mouse(MouseKind::Up(MouseButton::Left), 28, 0);
    assert!(plain.app().scrolled.is_empty(), "a strip that is not reorderable drags nothing");
    assert_eq!(plain.screen(), " ◀  ▌ main.rs              ▌▶\n");

    let mut menu = Harness::new(Editor { reorderable: true, overflow: Overflow::Menu, ..editor(30) }, 30, 1);
    menu.mouse(MouseKind::Down(MouseButton::Left), 3, 0);
    menu.mouse(MouseKind::Drag(MouseButton::Left), 29, 0);
    menu.advance(MS(400)).advance(MS(1000));
    menu.mouse(MouseKind::Up(MouseButton::Left), 29, 0);
    assert!(menu.app().scrolled.is_empty(), "a strip with a menu of hidden tabs has no arrows to hold");
}

#[test]
fn a_dragged_tab_still_scrolls_with_reduced_motion() {
    let mut h = dragging_strip(0);
    h.set_reduced_motion(true);
    h.mouse(MouseKind::Down(MouseButton::Left), 6, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 28, 0);
    h.advance(MS(400)).advance(MS(150));
    assert_eq!(h.app().scrolled, vec![1, 2], "scrolling a drag is function, not decoration");
    h.mouse(MouseKind::Up(MouseButton::Left), 28, 0);
    assert_eq!(h.app().files, vec!["app.rs", "tabs.rs", "main.rs", "theme.toml"]);
}
