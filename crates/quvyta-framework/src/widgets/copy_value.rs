//! A value the user copies with one key or click, with quiet confirmation in place.

use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use super::cells;
use super::press::{self, Press};
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::motion::Easing;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// How long the "copied" confirmation stays before it fades back.
const CONFIRMATION: Duration = Duration::from_millis(1400);

/// Cells between the value and the marker.
const MARKER_GAP: u16 = 2;

/// A value on a raised surface that copies itself to the clipboard: an install command, a
/// token, a URL.
///
/// Enter, Space or `c` while focused, or a click, copies the value, flashes the surface and
/// turns the trailing `copy` word into a success mark with `copied` that fades back after a
/// moment, so no toast is needed. The value is truncated with `…` when narrow; the marker always
/// stays visible. The copy goes to the terminal clipboard (OSC 52) and to the application's
/// in-process clipboard.
///
/// Style keys: `copy-value` (`bg`, `fg`, `padding`) with `hover`, `focus`, `pressed`,
/// `disabled`; `copy-value-marker` (`fg`) with variant `copied`. Words come from
/// `quvyta.copy-value.copy` and `quvyta.copy-value.copied`.
pub struct CopyValue<Msg> {
    value: String,
    masked: bool,
    disabled: bool,
    on_copy: Option<Msg>,
}

#[derive(Debug, Default)]
struct CopyMemory {
    copied_at: Option<Duration>,
}

impl<Msg> CopyValue<Msg> {
    /// Shows `value` and copies it.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self { value: value.into(), masked: false, disabled: false, on_copy: None }
    }

    /// Draws the value as mask dots while still copying the real text, for secrets.
    #[must_use]
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Greys the value out; it cannot be focused or copied.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message sent after every copy.
    #[must_use]
    pub fn on_copy(mut self, message: Msg) -> Self {
        self.on_copy = Some(message);
        self
    }

    fn shown(&self, mask: &str) -> String {
        if self.masked { self.value.graphemes(true).map(|_| mask).collect() } else { self.value.clone() }
    }
}

/// The idle and confirmed marker texts.
fn markers(env: &crate::env::Env) -> (String, String) {
    let i18n = env.i18n();
    let check = env.icons().glyph("check");
    (
        i18n.translate("quvyta.copy-value.copy", &[]),
        format!("{check} {}", i18n.translate("quvyta.copy-value.copied", &[])),
    )
}

impl<Msg: Clone + 'static> Widget<Msg> for CopyValue<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let style = cx.env().theme().style("copy-value", None, &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((0, 1));
        let (idle, copied) = markers(cx.env());
        let marker = text::width(&idle).max(text::width(&copied));
        let value = text::width(&self.shown(&cx.env().icons().glyph("mask")));
        Size::new(
            cells::sum([value, MARKER_GAP, marker, horizontal.saturating_mul(2)]),
            vertical.saturating_mul(2).saturating_add(1),
        )
        .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let states = if self.disabled { vec![State::Disabled] } else { cx.states() };
        let style = cx.style("copy-value", None, &states);
        let surface = style.text();
        cx.clear(area, surface.bg.unwrap_or_else(|| cx.color("raised")));
        if !self.disabled {
            cx.register_hit(area);
        }
        let padding = style.padding();
        if let Some(color) = style.color("pillar").filter(|_| padding.left >= 1) {
            cx.pillar(area.x, area.y + i32::from(padding.top), color);
        }
        let inner = area.inset(padding);
        let (idle, copied) = markers(cx.env());

        // The confirmation holds, then its colour blends back to the idle marker over `enter`.
        let copied_at = cx.memory::<CopyMemory>().copied_at;
        let now = cx.now();
        let confirmed = copied_at.filter(|at| now < *at + CONFIRMATION);
        let idle_color = cx.style("copy-value-marker", None, &states).text().fg.unwrap_or_else(|| cx.color("muted"));
        let (marker, color) = match confirmed {
            Some(at) => {
                let success = cx.style("copy-value-marker", Some("copied"), &[]).text().fg;
                let success = success.unwrap_or_else(|| cx.color("success"));
                let enter = cx.env().theme().motion().enter;
                let fade_start = at + CONFIRMATION.saturating_sub(enter);
                let fade = if now < fade_start {
                    cx.request_frame_in(fade_start - now);
                    0.0
                } else {
                    cx.progress_since(fade_start, enter, Easing::EaseIn)
                };
                cx.request_frame_in((at + CONFIRMATION).saturating_sub(now));
                (copied, success.mix(idle_color, fade))
            }
            None => (idle, idle_color),
        };
        let marker_width = text::width(&marker).min(inner.width);
        let marker_x = inner.right() - i32::from(marker_width);
        let mut marker_style = surface;
        marker_style.fg = Some(color);
        marker_style.bold = confirmed.is_some();
        cx.text(marker_x, inner.y, &marker, marker_style, marker_width);

        let budget = inner.width.saturating_sub(marker_width + MARKER_GAP);
        let shown = self.shown(&cx.env().icons().glyph("mask"));
        let value = text::truncate(&shown, budget).into_owned();
        cx.text(inner.x, inner.y, &value, surface, budget);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.disabled {
            return false;
        }
        let copy = match event {
            Event::Key(key) if key.is_plain(Key::Char('c')) => true,
            _ => match press::read(cx, event) {
                Press::Ignored => return false,
                Press::Used => false,
                Press::Key | Press::Click(..) => true,
            },
        };
        if copy {
            cx.copy(self.value.clone());
            cx.flash();
            let now = cx.now();
            cx.memory::<CopyMemory>().copied_at = Some(now);
            if let Some(message) = &self.on_copy {
                cx.emit(message.clone());
            }
        }
        true
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, ClipboardEvent, Command, Harness};
    use crate::widget::View;

    #[derive(Default)]
    struct Demo {
        copies: u32,
        masked: bool,
        disabled: bool,
        heard: Vec<ClipboardEvent>,
        pasted: Option<Option<String>>,
    }

    #[derive(Clone)]
    enum Msg {
        Copied,
        Heard(ClipboardEvent),
        ReadBack,
        Read(Option<String>),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Copied => self.copies += 1,
                Msg::Heard(event) => self.heard.push(event),
                Msg::ReadBack => return Command::read_clipboard(Msg::Read),
                Msg::Read(text) => self.pasted = Some(text),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add(
                CopyValue::new("docker pull ghcr.io/quvyta/api")
                    .masked(self.masked)
                    .disabled(self.disabled)
                    .on_copy(Msg::Copied),
            )
            .fill_width()
            .id("install");
        }
        fn clipboard(&self, event: &ClipboardEvent) -> Option<Msg> {
            Some(Msg::Heard(event.clone()))
        }
    }

    #[test]
    fn copies_with_keys_and_clicks_and_confirms_in_place() {
        let mut h = Harness::new(Demo::default(), 44, 1);
        assert_eq!(h.screen(), "  docker pull ghcr.io/quvyta/api      copy\n");
        h.press("tab").press("c");
        assert_eq!(h.app().copies, 1);
        assert_eq!(h.copied(), &["docker pull ghcr.io/quvyta/api".to_owned()]);
        assert_eq!(h.clipboard(), Some("docker pull ghcr.io/quvyta/api"));
        assert!(h.screen().ends_with("✓ copied\n"), "{}", h.screen());
        let (x, y) = h.find("copied").expect("marker shown");
        let success = h.env().theme().color("success");
        assert_eq!(h.fg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0)), success);
        h.advance(CONFIRMATION + Duration::from_millis(10));
        assert!(h.screen().ends_with(" copy\n"), "{}", h.screen());
        h.click_text("docker");
        h.press("enter");
        assert_eq!(h.app().copies, 3);
        assert_eq!(h.app().heard.len(), 3);
        assert_eq!(h.app().heard[0], ClipboardEvent::Copied("docker pull ghcr.io/quvyta/api".into()));
    }

    #[test]
    fn masks_secrets_but_copies_the_real_value() {
        let mut h = Harness::new(Demo { masked: true, ..Demo::default() }, 44, 1);
        assert!(!h.screen().contains("docker"), "{}", h.screen());
        assert!(h.screen().contains("•••"));
        h.press("tab").press("enter");
        assert_eq!(h.clipboard(), Some("docker pull ghcr.io/quvyta/api"));
        h.set_glyph_mode(GlyphMode::Ascii);
        assert!(h.screen().contains("v copied"), "{}", h.screen());
    }

    #[test]
    fn narrow_keeps_the_marker_and_disabled_ignores_input() {
        let h = Harness::new(Demo::default(), 20, 1);
        assert_eq!(h.screen(), "  docker pu…  copy\n");
        let mut disabled = Harness::new(Demo { disabled: true, ..Demo::default() }, 44, 1);
        disabled.press("tab").press("enter").click_text("docker");
        assert_eq!(disabled.app().copies, 0);
        assert!(disabled.copied().is_empty());
    }

    #[test]
    fn paste_key_and_read_clipboard_use_the_in_process_copy() {
        let mut h = Harness::new(Demo::default(), 44, 1);
        h.send(Msg::ReadBack);
        assert_eq!(h.app().pasted, Some(None));
        h.press("ctrl+v");
        assert!(h.app().heard.is_empty(), "nothing copied yet, so nothing pasted");
        h.press("tab").press("c").press("ctrl+v");
        assert_eq!(h.app().heard.last(), Some(&ClipboardEvent::Pasted("docker pull ghcr.io/quvyta/api".into())));
        h.paste("from the terminal");
        assert_eq!(h.app().heard.last(), Some(&ClipboardEvent::Pasted("from the terminal".into())));
        h.send(Msg::ReadBack);
        assert_eq!(h.app().pasted, Some(Some("docker pull ghcr.io/quvyta/api".into())));
    }

    struct Form {
        value: String,
    }

    impl App for Form {
        type Msg = String;
        fn update(&mut self, value: String) -> Command<String> {
            self.value = value;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, String>) {
            ui.add(CopyValue::new("eu-west-3")).id("region");
            ui.add(crate::widgets::TextInput::new(&self.value).on_change(|v| v)).fill_width().id("field");
        }
    }

    #[test]
    fn paste_key_fills_the_focused_text_field() {
        let mut h = Harness::new(Form { value: String::new() }, 30, 2);
        h.press("tab").press("c");
        h.press("tab").press("ctrl+v");
        assert_eq!(h.app().value, "eu-west-3");
    }
}
