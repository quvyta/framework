//! Motion: easing, animated values with per-cell colour blending, theme timings and reduced motion.

use std::time::Duration;

use qframe::motion::Easing;
use qframe::prelude::*;
use qframe::storage::Settings;
use qframe::widget::{MeasureCx, PaintCx, Widget};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "motion";

/// Where the knobs of the easing lanes are heading.
#[derive(Debug, Default)]
pub struct State {
    at_end: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Play,
    Reduced(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Motion(message))
}

/// Applies a demo message. Reduced motion is remembered in `settings`, exactly as the switch on the
/// Theme, icons and language page remembers it.
pub fn update(state: &mut State, settings: &mut Settings, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Play => {
            state.at_end = !state.at_end;
            log.push(PAGE, "Button#play", format!("target = {}", u8::from(state.at_end)));
            Command::none()
        }
        Msg::Reduced(on) => {
            log.push(PAGE, "Playground", format!("reduced motion = {on}"));
            // region: reduced
            let save = super::storage::remember(settings, Settings::REDUCED_MOTION, on);
            Command::batch([Command::set_reduced_motion(on), save])
            // endregion
        }
    }
}

// region: lane
/// Width of the moving knob, in cells.
const KNOB: f32 = 2.0;

/// A track with a knob that glides to either end with an easing.
struct Lane {
    easing: Easing,
    at_end: bool,
}

impl<Msg: 'static> Widget<Msg> for Lane {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(available.width, 1)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let target = if self.at_end { 1.0 } else { 0.0 };
        let progress = cx.animate("knob", target, Duration::from_millis(900), self.easing);
        let track = cx.color("raised");
        let knob = cx.color("accent");
        let start = progress * (f32::from(area.width) - KNOB).max(0.0);
        for column in 0..area.width {
            // How much of this cell the knob covers decides how far its colour is blended.
            let left = f32::from(column);
            let covered = ((start + KNOB).min(left + 1.0) - start.max(left)).clamp(0.0, 1.0);
            cx.clear(Rect::new(area.x + i32::from(column), area.y, 1, 1), track.mix(knob, covered));
        }
    }
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("motion.easing")), |ui| {
        ui.add(Text::new(t!("motion.easing-hint")).role("secondary"));
        let label = if state.at_end { t!("motion.back") } else { t!("motion.play") };
        ui.add(Button::new(label).variant("primary").on_press(send(Msg::Play))).id("play");
        for easing in Easing::ALL {
            ui.row(|ui| {
                ui.add(Text::new(easing.name()).role("faint").no_wrap()).width(Length::Cells(14));
                ui.add(Lane { easing, at_end: state.at_end }).width(Length::Fill(1)).id(easing.name());
            })
            .fill_width();
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("motion.timings")).gap(0), |ui| {
        // region: timings
        let motion = ui.env().theme().motion();
        let timings = [
            ("enter", motion.enter),
            ("spinner", motion.spinner),
            ("shimmer", motion.shimmer),
            ("pulse-period", motion.pulse_period),
            ("flash", motion.flash),
            ("step", motion.step),
            ("hover-delay", motion.hover_delay),
        ];
        // endregion
        for (key, duration) in timings {
            ui.row(|ui| {
                ui.add(Text::new(key).role("body").no_wrap()).width(Length::Cells(16));
                ui.add(Text::new(format!("{} ms", duration.as_millis())).role("title").no_wrap())
                    .width(Length::Cells(10));
                ui.add(Text::new(t!(&format!("motion.timing.{key}"))).role("secondary").no_wrap());
            });
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        reduced_motion_setting(ui, t!("motion.reduced"), |on| send(Msg::Reduced(on)));
        ui.add(Text::new(t!("motion.reduced-hint")).role("faint"));
    })
    .fill_width();
}

/// The "Reduce motion" playground row, shared with the Theme, icons and language page. While
/// `QUVYTA_REDUCED_MOTION` decides, the switch shows the value it forces, is disabled and says why,
/// instead of snapping back when pressed.
pub fn reduced_motion_setting(ui: &mut View<'_, AppMsg>, label: String, on_toggle: impl Fn(bool) -> AppMsg + 'static) {
    // region: reduced-forced
    let env = ui.env();
    let (reduced, forced) = (env.reduced_motion(), env.reduced_motion_forced());
    // endregion
    reduced_motion_row(ui, label, reduced, forced, on_toggle);
}

/// The row itself, for a `reduced` value and whether the environment `forced` it.
fn reduced_motion_row(
    ui: &mut View<'_, AppMsg>,
    label: String,
    reduced: bool,
    forced: bool,
    on_toggle: impl Fn(bool) -> AppMsg + 'static,
) {
    setting(ui, label, |ui| {
        // A disabled switch sends nothing, so a press neither changes nor saves the setting.
        ui.add(toggle(reduced, on_toggle).disabled(forced)).id("reduced");
    });
    if forced {
        ui.row(|ui| {
            // Under the switch, past the label column of `setting`.
            ui.spacer().width(Length::Cells(24));
            ui.add(Text::new(t!("motion.forced")).role("faint")).id("reduced-forced");
        })
        .fill_width();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn knobs_glide_and_reduced_motion_can_be_switched() {
        let mut h = showcase_on(PAGE);
        h.click_text("Play");
        assert!(h.app().pages.motion.at_end);
        h.advance(Duration::from_millis(1000));
        assert!(h.screen().contains("Back"));
        h.send(send(Msg::Reduced(true)));
        assert!(h.env().reduced_motion());
    }

    /// Draws reduced-motion rows as the environment variable would leave them, without touching
    /// the process environment, and records what the switches send.
    #[derive(Default)]
    struct Forced {
        sent: Vec<bool>,
    }

    impl App for Forced {
        type Msg = AppMsg;
        fn update(&mut self, message: AppMsg) -> Command<AppMsg> {
            if let AppMsg::Page(PageMsg::Motion(Msg::Reduced(on))) = message {
                self.sent.push(on);
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, AppMsg>) {
            ui.column(|ui| {
                reduced_motion_row(ui, "Forced on".to_owned(), true, true, |on| send(Msg::Reduced(on)));
                reduced_motion_row(ui, "Forced off".to_owned(), false, true, |on| send(Msg::Reduced(on)));
                reduced_motion_row(ui, "Free off".to_owned(), false, false, |on| send(Msg::Reduced(on)));
            });
        }
    }

    #[test]
    fn a_switch_the_environment_decides_is_disabled_shows_the_value_and_says_why() {
        let mut h = Harness::with_env(Forced::default(), crate::tests::env(), 100, 8);
        h.set_locale("en");
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let hint = "Set by the QUVYTA_REDUCED_MOTION environment variable.";
        assert_eq!((lines[1].trim(), lines[3].trim()), (hint, hint), "{screen}");
        assert!(lines[1].starts_with(&" ".repeat(24)), "the hint sits under the switch: {screen}");
        assert_eq!(screen.matches("Set by").count(), 2, "the free row says nothing: {screen}");

        let switch_bg = |h: &Harness<Forced>, row: u16| (24..30).map(|x| h.bg(x, row)).collect::<Vec<_>>();
        assert_ne!(switch_bg(&h, 0), switch_bg(&h, 2), "a forced switch still shows the value in force");

        for (label, row) in [("Forced on", 0), ("Forced off", 2)] {
            let (x, y) = h.find(label).expect("forced row");
            assert_eq!(y, row);
            h.click(x + 24, y).click(x + 25, y).click(x + 27, y);
        }
        assert!(h.app().sent.is_empty(), "a forced switch sends nothing, so nothing changes or is saved");
        let (x, y) = h.find("Free off").expect("free row");
        h.click(x + 25, y);
        assert_eq!(h.app().sent, [true], "the free switch works as before");

        h.set_locale("tr");
        assert!(h.screen().contains("QUVYTA_REDUCED_MOTION ortam değişkeni belirliyor."), "{}", h.screen());
    }

    #[test]
    fn reduced_motion_is_remembered_like_the_one_on_the_appearance_page() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("Reduce motion").expect("playground row");
        // The switch sits after the 24-cell label column.
        h.click(x + 25, y);
        assert!(h.env().reduced_motion());
        assert_eq!(h.app().pages.storage.settings.reduced_motion(), Some(true), "stored by the Motion page");
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "reduced motion = true");

        h.send(AppMsg::Open("theme-icons-language".into()));
        h.advance(Duration::from_secs(1));
        let (x, y) = h.find("Reduce motion").expect("appearance row");
        h.click(x + 25, y);
        assert!(!h.env().reduced_motion(), "the other switch showed it on and turns it off");
        assert_eq!(h.app().pages.storage.settings.reduced_motion(), Some(false));

        h.send(AppMsg::Open(PAGE.into()));
        h.advance(Duration::from_secs(1));
        h.send(send(Msg::Reduced(true)));
        assert_eq!(h.app().pages.storage.settings.to_toml(), "reduced-motion = true\n", "one key, one value");
    }
}
