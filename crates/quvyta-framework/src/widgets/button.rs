//! Buttons.

use super::cells;
use super::press::{self, Press};
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// A button: the shape is its surface colour, never brackets.
///
/// Sends its message on Enter or Space while focused, or on a click released over it, and
/// flashes one tone brighter to confirm. Hovered and focused buttons show the pillar in their very
/// first cell, before the shortcut segment and the icon (`▌ ⏎ Save`); the cell is reserved at rest,
/// so nothing moves when the pillar appears. Like every widget that is not a list, buttons never
/// slide. A theme padding of zero columns leaves no room for the pillar and draws none. Style keys
/// (`pillar` sets the pillar colour): `button`, `button.<variant>`, states `hover`, `focus`,
/// `pressed`, `disabled`; the shortcut segment uses `button-key` and `button-key.<variant>`.
pub struct Button<Msg> {
    label: String,
    icon: Option<String>,
    shortcut: Option<String>,
    variant: Option<String>,
    disabled: bool,
    loading: bool,
    selected: Option<bool>,
    on_press: Option<Msg>,
}

impl<Msg> Button<Msg> {
    /// A button labelled `label`.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            variant: None,
            disabled: false,
            loading: false,
            selected: None,
            on_press: None,
        }
    }

    /// The message sent when the button is pressed.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Theme variant, e.g. `"primary"` or `"danger"`.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Icon key drawn before the label.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>) -> Self {
        self.icon = Some(key.into());
        self
    }

    /// Key label drawn in a darker segment on the left, e.g. `"⏎"` or `"ctrl s"`. The segment
    /// starts with the button's pillar cell, so the pillar stays the leftmost mark.
    #[must_use]
    pub fn shortcut(mut self, label: impl Into<String>) -> Self {
        self.shortcut = Some(label.into());
        self
    }

    /// Greys the button out; it cannot be focused or pressed.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Shows a spinner in the default spinner style instead of the icon and ignores presses.
    #[must_use]
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Makes this a choice button, such as one of a few view modes: `selected` shows it chosen,
    /// raised with a steady pillar. Choosing is its own feedback, so a choice button does not
    /// flash when pressed.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    fn active(&self) -> bool {
        !self.disabled && !self.loading && self.on_press.is_some()
    }
}

/// Width of the shortcut segment for `key`: the pillar cell when the padding leaves room for a
/// pillar, then the key with a space on each side.
fn key_width(key: &str, horizontal_padding: u16) -> u16 {
    cells::sum([u16::from(horizontal_padding >= 1), text::width(key), 2])
}

impl<Msg: Clone + 'static> Button<Msg> {
    fn press(&self, cx: &mut EventCx<'_, Msg>) {
        if let Some(message) = &self.on_press {
            if self.selected.is_none() {
                cx.flash();
            }
            cx.emit(message.clone());
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Button<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let theme = cx.env().theme();
        let style = theme.style("button", self.variant.as_deref(), &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((0, 2));
        let icon_width = self
            .icon
            .as_deref()
            .map_or(0, |key| text::width(&cx.env().icons().glyph(key)) + 1)
            .max(if self.loading { 2 } else { 0 });
        let shortcut_width = self.shortcut.as_deref().map_or(0, |key| key_width(key, horizontal));
        let width = cells::sum([shortcut_width, horizontal.saturating_mul(2), icon_width, text::width(&self.label)]);
        Size::new(width, vertical.saturating_mul(2).saturating_add(1)).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        // A loading button ignores presses but keeps its hover and focus look, so a short job does
        // not make it blink.
        let interactive = !self.disabled && self.on_press.is_some();
        // A clicked button stays calm under the pointer; its focus breathes only when it was
        // reached with the keyboard.
        let mut states = if interactive { cx.pressable_states() } else { Vec::new() };
        if self.selected == Some(true) {
            states.push(State::Selected);
        }
        if self.disabled {
            states.push(State::Disabled);
        }
        let variant = self.variant.as_deref();
        let style = cx.style("button", variant, &states);
        let surface = style.text();
        let background = surface.bg.unwrap_or_else(|| cx.color("raised"));
        cx.clear(area, background);
        if interactive {
            cx.register_hit(area);
        }

        let padding = style.padding();
        let y = area.y + i32::from(padding.top);
        let mut x = area.x;
        let shortcut_width = self.shortcut.as_deref().map_or(0, |key| key_width(key, padding.left));
        if let Some(shortcut) = &self.shortcut {
            let key_style = cx.style("button-key", variant, &states).text();
            cx.clear(Rect::new(x, area.y, shortcut_width, area.height), key_style.bg.unwrap_or(background));
            let label = format!(" {shortcut} ");
            let label_width = text::width(&label);
            cx.text(x + i32::from(shortcut_width - label_width), y, &label, key_style, label_width);
        }
        // Hover and focus raise the pillar in the button's first cell: the leftmost mark of the
        // raised thing, before the shortcut segment. Buttons never slide: only list structures
        // do, so every button behaves the same whatever its width.
        let pillar = style.color("pillar").filter(|_| padding.left >= 1);
        if let Some(color) = pillar {
            cx.pillar(x, y, color);
        }
        x += i32::from(shortcut_width);
        x += i32::from(padding.left);
        let right = area.right() - i32::from(padding.right);
        let budget = |x: i32| crate::geometry::clamp_u16(right - x);

        if self.loading {
            // The same turning arc as a plain `Spinner`, so busy states look alike everywhere.
            let spinner = super::SpinnerStyle::default().animation();
            let cell = cx.animation(spinner, surface, Some(std::time::Duration::ZERO));
            x += i32::from(cx.text(x, y, &cell.glyph, cell.style, budget(x).min(1)));
            x += 1;
        } else if let Some(icon) = &self.icon {
            let glyph = cx.env().icons().glyph(icon).into_owned();
            x += i32::from(cx.text(x, y, &glyph, surface, budget(x)));
            x += 1;
        }
        let label = text::truncate(&self.label, budget(x)).into_owned();
        cx.text(x, y, &label, surface, budget(x));
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        match press::read(cx, event) {
            Press::Ignored => false,
            Press::Used => true,
            Press::Key | Press::Click(..) => {
                self.press(cx);
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{MouseButton, MouseKind};
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use std::time::Duration;

    #[derive(Default)]
    struct Demo {
        presses: u32,
        disabled: bool,
        loading: bool,
    }

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            self.presses += 1;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.row(|ui| {
                ui.add(Button::new("Save").variant("primary").shortcut("⏎").on_press(()).disabled(self.disabled))
                    .id("save");
                ui.add(Button::new("Wait").loading(self.loading).on_press(()));
            })
            .gap(1);
        }
    }

    #[test]
    fn draws_without_brackets() {
        let h = Harness::new(Demo::default(), 24, 1);
        assert_eq!(h.screen(), "  ⏎   Save     Wait\n");
        let theme = h.env().theme();
        assert_ne!(h.bg(6, 0), theme.color("accent"), "a resting primary button is a tint, not the full fill");
        assert_eq!(h.fg(6, 0), theme.color("accent"));
    }

    /// The sum of a colour's channels, to compare how bright two tones are.
    fn brightness(color: Option<crate::color::Rgb>) -> u32 {
        color.map_or(0, |c| u32::from(c.r) + u32::from(c.g) + u32::from(c.b))
    }

    #[test]
    fn tones_climb_from_rest_to_hover_to_press_and_never_invert() {
        let mut h = Harness::new(Demo::default(), 24, 1);
        let rest = h.bg(6, 0);
        h.hover(6, 0);
        let hover = h.bg(6, 0);
        h.mouse(MouseKind::Down(MouseButton::Left), 6, 0).mouse(MouseKind::Up(MouseButton::Left), 6, 0);
        let pressed = h.bg(6, 0);
        assert!(brightness(rest) < brightness(hover) && brightness(hover) < brightness(pressed));
        assert_eq!(h.fg(6, 0), h.env().theme().color("accent"), "the label keeps its colour while pressed");
        h.advance(Duration::from_millis(200));
        assert_eq!(h.bg(6, 0), hover, "a clicked button settles back to its hover tone, calm under the pointer");
    }

    #[test]
    fn choice_buttons_show_selection_and_do_not_flash() {
        struct Modes(usize);
        impl App for Modes {
            type Msg = usize;
            fn update(&mut self, mode: usize) -> Command<usize> {
                self.0 = mode;
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, usize>) {
                ui.row(|ui| {
                    ui.add(Button::new("List").selected(self.0 == 0).on_press(0));
                    ui.add(Button::new("Grid").selected(self.0 == 1).on_press(1));
                });
            }
        }
        let mut h = Harness::new(Modes(0), 20, 1);
        assert!(h.screen().starts_with("▌ List"), "the chosen one carries a steady pillar: {}", h.screen());
        h.click_text("Grid");
        assert_eq!(h.app().0, 1);
        let chosen = h.bg(12, 0);
        h.advance(Duration::from_millis(200));
        assert_eq!(h.bg(12, 0), chosen, "no flash: the selection itself is the feedback");
    }

    #[test]
    fn presses_by_keyboard_and_click_and_flashes() {
        let mut h = Harness::new(Demo::default(), 24, 1);
        h.press("tab");
        assert!(h.is_focused("save"));
        let focused = h.bg(6, 0);
        h.press("enter");
        assert_eq!(h.app().presses, 1);
        let flash = h.bg(6, 0);
        assert!(brightness(flash) > brightness(focused), "a press flashes one tone brighter");
        h.advance(Duration::from_millis(200));
        assert_eq!(h.bg(6, 0), focused);
        h.click_text("Save");
        assert_eq!(h.app().presses, 2);
    }

    #[test]
    fn hover_and_focus_raise_the_pillar_without_sliding() {
        let mut h = Harness::new(Demo::default(), 24, 1);
        assert_eq!(h.screen(), "  ⏎   Save     Wait\n");
        h.hover(15, 0);
        assert_eq!(h.screen(), "  ⏎   Save   ▌ Wait\n", "the pillar rises; the label stays where it is");
        h.hover(23, 0);
        h.press("tab");
        assert_eq!(h.screen(), "▌ ⏎   Save     Wait\n", "the pillar comes before the shortcut segment");
        let theme = h.env().theme();
        assert_ne!(h.fg(0, 0), theme.color("ink"), "the pillar stays bright on the tint");
        assert_eq!(h.bg(0, 0), h.bg(2, 0), "the pillar cell belongs to the shortcut segment");
        h.set_reduced_motion(true);
        assert_eq!(h.screen(), "▌ ⏎   Save     Wait\n");
    }

    /// Buttons with a shortcut, an icon or both, side by side.
    struct Marked;

    impl App for Marked {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.column(|ui| {
                ui.add(Button::new("Open palette").shortcut("ctrl p").on_press(()));
                ui.add(Button::new("Deploy").icon("check").on_press(()));
                ui.add(Button::new("Save").icon("check").shortcut("⏎").variant("primary").on_press(()));
            });
        }
    }

    #[test]
    fn the_pillar_is_the_leftmost_cell_with_a_shortcut_an_icon_or_both() {
        let mut h = Harness::new(Marked, 30, 3);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        let rest = h.screen();
        assert_eq!(rest, "  ctrl p   Open palette\n  ✓ Deploy\n  ⏎   ✓ Save\n");
        for (row, hovered) in ["▌ ctrl p   Open palette", "▌ ✓ Deploy", "▌ ⏎   ✓ Save"].into_iter().enumerate()
        {
            let y = i32::try_from(row).unwrap_or(0);
            h.hover(3, y);
            let line = h.screen().lines().nth(row).unwrap_or_default().to_owned();
            assert_eq!(line, hovered, "hover draws the pillar first");
            for (x, (lit, calm)) in line.chars().zip(rest.lines().nth(row).unwrap_or_default().chars()).enumerate() {
                assert!(x == 0 || lit == calm, "only the first cell changes: {line:?}");
            }
        }
        h.hover(29, 2).press("tab");
        assert!(h.screen().starts_with("▌ ctrl p   Open palette"), "keyboard focus too: {}", h.screen());
    }

    #[test]
    fn a_theme_without_padding_draws_no_pillar_and_no_pillar_cell() {
        let dir = std::env::temp_dir().join(format!("quvyta-button-flat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let theme = "[meta]\nname = \"Flat\"\nextends = \"monochrome\"\n[style.button]\npadding = [0, 0]\n";
        std::fs::write(dir.join("flat.toml"), theme).expect("theme file");
        let dirs = crate::env::AssetDirs { themes: Some(dir.clone()), ..Default::default() };
        let env = crate::env::Env::load(&dirs).expect("loads");
        let mut h = Harness::with_env(Marked, env, 24, 3);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode).set_theme("flat");
        h.hover(1, 0);
        assert_eq!(h.screen(), " ctrl p Open palette\n✓ Deploy\n ⏎ ✓ Save\n");
        std::fs::remove_dir_all(dir).ok();
    }

    /// A button with an icon that may be loading.
    struct Job(bool);

    impl App for Job {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Button::new("Count").icon("folder").loading(self.0).on_press(()));
        }
    }

    #[test]
    fn a_short_loading_state_keeps_the_raised_look() {
        let mut idle = Harness::new(Job(false), 20, 1);
        let mut busy = Harness::new(Job(true), 20, 1);
        idle.hover(4, 0);
        busy.hover(4, 0);
        busy.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        assert!(busy.screen().contains("◜ Count"), "the default spinner turns in place of the icon: {}", busy.screen());
        let label = |h: &Harness<Job>| h.find("Count");
        assert_eq!(label(&idle), label(&busy), "the label does not jump while a quick job runs");
        assert_eq!(idle.screen().starts_with('▌'), busy.screen().starts_with('▌'));
        assert_eq!(idle.bg(10, 0), busy.bg(10, 0), "the surface keeps its hover tone");
        busy.press("enter");
    }

    #[test]
    fn held_enter_does_not_repeat() {
        let mut h = Harness::new(Demo::default(), 24, 1);
        h.press("tab").press("enter");
        let event = crate::event::Event::Key(crate::event::KeyEvent::press("enter"));
        let now = Duration::from_millis(330);
        h.inject(event.clone(), now);
        h.inject(event, now + Duration::from_millis(30));
        assert_eq!(h.app().presses, 1);
    }

    #[test]
    fn disabled_and_loading_ignore_presses() {
        let mut h = Harness::new(Demo { disabled: true, loading: true, ..Demo::default() }, 24, 1);
        h.press("tab").press("enter");
        assert_eq!(h.app().presses, 0);
        let theme = h.env().theme();
        assert_eq!(h.fg(6, 0), theme.color("muted"));
    }
}
