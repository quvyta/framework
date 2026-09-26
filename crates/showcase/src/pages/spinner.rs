//! Spinner: single-cell activity indicators in many styles and tones, finishing with a tick, and
//! one that shows only for work slow enough to notice.

use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::Task;
use qframe::widgets::{Select, Spinner, SpinnerStyle};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "spinner";

/// Tones the playground offers; the first is the default accent.
const TONES: [&str; 4] = ["accent", "success", "warning", "danger"];

/// How long the simulated image build takes.
const BUILD_TIME: Duration = Duration::from_millis(1600);

/// How long the quick and the slow simulated page loads take: one under the spinner's delay, one
/// well over it.
const QUICK_LOAD: Duration = Duration::from_millis(200);
const SLOW_LOAD: Duration = Duration::from_millis(1500);

/// Where the simulated image build is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Build {
    /// Nothing started yet.
    #[default]
    Idle,
    /// The build runs; the spinner turns.
    Running,
    /// The build finished; the spinner rests on its tick.
    Finished,
}

/// Playground settings and the build demo.
#[derive(Debug)]
pub struct State {
    style: usize,
    tone: usize,
    label: bool,
    done: bool,
    build: Build,
    /// Whether a simulated page load runs.
    loading: bool,
    /// The last page load that ended: `Some(true)` for the quick one.
    loaded: Option<bool>,
}

impl Default for State {
    fn default() -> Self {
        Self { style: 0, tone: 0, label: true, done: false, build: Build::Idle, loading: false, loaded: None }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Style(usize),
    Tone(usize),
    Label(bool),
    Done(bool),
    Build,
    Built,
    /// Starts a page load, the quick one when `true`.
    Load(bool),
    /// The page load started with `Load(quick)` ended.
    Loaded(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Spinner(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Style(index) => {
            state.style = index;
            log.push(PAGE, "Playground", format!("style = {}", SpinnerStyle::ALL[index].name()));
        }
        Msg::Tone(index) => {
            state.tone = index;
            log.push(PAGE, "Playground", format!("tone = {}", TONES[index]));
        }
        Msg::Label(on) => {
            state.label = on;
            log.push(PAGE, "Playground", format!("label = {on}"));
        }
        Msg::Done(on) => {
            state.done = on;
            log.push(PAGE, "Playground", format!("done = {on}"));
        }
        // region: finish-build
        Msg::Build => {
            state.build = Build::Running;
            log.push(PAGE, "Button#build", "build started");
            return Command::perform(|| {
                std::thread::sleep(BUILD_TIME);
                send(Msg::Built)
            });
        }
        Msg::Built => {
            state.build = Build::Finished;
            log.push(PAGE, "Spinner#build-status", "done = true");
        } // endregion
        // region: delayed-load
        Msg::Load(quick) => {
            state.loading = true;
            let (button, time) = if quick { ("quick-load", QUICK_LOAD) } else { ("slow-load", SLOW_LOAD) };
            log.push(PAGE, format!("Button#{button}"), "load started");
            return Command::task(Task::new("page load", move |cx| {
                if !cx.sleep(time) {
                    return Err("stopped".into());
                }
                Ok(send(Msg::Loaded(quick)))
            }));
        }
        Msg::Loaded(quick) => {
            state.loading = false;
            state.loaded = Some(quick);
            log.push(PAGE, "Spinner#page-load", "busy = false");
        } // endregion
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("spinner.styles")), |ui| {
        ui.add(Text::new(t!("spinner.studio-hint")).role("secondary"));
        // Two rows of four keep the styles in aligned columns on narrower screens.
        for row in SpinnerStyle::ALL.chunks(4) {
            ui.row(|ui| {
                for &style in row {
                    // region: styles
                    ui.add(Spinner::new().style(style).label(style.name())).width(Length::Cells(12));
                    // endregion
                }
            })
            .gap(2);
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("spinner.tones")).gap(0), |ui| {
        // region: tones
        ui.add(Spinner::new().label(t!("spinner.pulling")));
        ui.add(Spinner::new().style(SpinnerStyle::Pulse).variant("success").label(t!("spinner.healthy")));
        ui.add(Spinner::new().style(SpinnerStyle::Dots).variant("warning").label(t!("spinner.retrying")));
        ui.add(Spinner::new().style(SpinnerStyle::Pop).variant("danger").label(t!("spinner.lost")));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("spinner.finish")), |ui| {
        ui.add(Text::new(t!("spinner.finish-hint")).role("secondary"));
        ui.row(|ui| {
            let running = state.build == Build::Running;
            ui.add(Button::new(t!("spinner.build")).disabled(running).on_press(send(Msg::Build))).id("build");
            // region: finish
            match state.build {
                Build::Idle => {
                    ui.add(Text::new(t!("spinner.idle")).role("faint").no_wrap());
                }
                Build::Running | Build::Finished => {
                    let finished = state.build == Build::Finished;
                    let label = if finished { t!("spinner.built") } else { t!("spinner.building") };
                    ui.add(Spinner::new().label(label).done(finished)).id("build-status");
                }
            }
            // endregion
        })
        .gap(2)
        .align(Align::Center);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("spinner.delayed")), |ui| {
        ui.add(Text::new(t!("spinner.delayed-hint")).role("secondary"));
        ui.row(|ui| {
            let busy = state.loading;
            ui.add(Button::new(t!("spinner.quick-load")).disabled(busy).on_press(send(Msg::Load(true))))
                .id("quick-load");
            ui.add(Button::new(t!("spinner.slow-load")).disabled(busy).on_press(send(Msg::Load(false))))
                .id("slow-load");
            // region: delayed
            ui.add(Spinner::new().label(t!("spinner.loading")).delayed(state.loading)).id("page-load");
            // endregion
            let result = match state.loaded {
                None => t!("spinner.not-loaded"),
                Some(true) => t!("spinner.loaded-quick"),
                Some(false) => t!("spinner.loaded-slow"),
            };
            ui.add(Text::new(result).role("faint").no_wrap());
        })
        .gap(2)
        .align(Align::Center);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.row(|ui| {
            // region: configured
            let mut spinner = Spinner::new().style(SpinnerStyle::ALL[state.style]).done(state.done);
            if TONES[state.tone] != "accent" {
                spinner = spinner.variant(TONES[state.tone]);
            }
            if state.label {
                spinner = spinner.label(t!("spinner.pulling"));
            }
            ui.add(spinner).id("configured");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("spinner.style"), |ui| {
            let names = SpinnerStyle::ALL.map(SpinnerStyle::name);
            ui.add(Select::new(names).selected(Some(state.style)).on_select(|i| send(Msg::Style(i))))
                .width(Length::Cells(16))
                .id("style");
        });
        setting(ui, t!("spinner.tone"), |ui| {
            ui.add(Select::new(TONES).selected(Some(state.tone)).on_select(|i| send(Msg::Tone(i))))
                .width(Length::Cells(16))
                .id("tone");
        });
        setting(ui, t!("spinner.label"), |ui| {
            ui.add(toggle(state.label, |on| send(Msg::Label(on)))).id("label");
        });
        setting(ui, t!("spinner.completed"), |ui| {
            ui.add(toggle(state.done, |on| send(Msg::Done(on)))).id("completed");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_tall;

    /// The page tall enough to show the event log under the playground.
    fn page() -> Harness<Showcase> {
        showcase_tall(Showcase::new(), PAGE, 70)
    }

    #[test]
    fn styles_and_tones_are_shown_and_playground_configures() {
        let mut h = page();
        let screen = h.screen();
        let order: Vec<usize> = SpinnerStyle::ALL.iter().filter_map(|style| screen.find(style.name())).collect();
        assert_eq!(order.len(), 7, "{screen}");
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "styles appear alphabetically:\n{screen}");
        assert_eq!(SpinnerStyle::ALL[h.app().pages.spinner.style], SpinnerStyle::Arc, "the playground starts at arc");
        assert!(h.screen().contains("Connection lost"));
        h.send(send(Msg::Label(false)));
        h.send(send(Msg::Style(5)));
        assert_eq!(h.app().pages.spinner.style, 5);
        assert!(h.screen().contains("style = quarters"));
        assert!(!h.app().pages.spinner.label);
        h.send(send(Msg::Done(true)));
        assert!(h.app().pages.spinner.done);
        assert!(h.screen().contains("done = true"));
    }

    #[test]
    fn the_build_demo_runs_and_finishes_with_a_tick() {
        let mut h = page();
        h.set_glyph_mode(qframe::icons::GlyphMode::Unicode);
        assert!(h.screen().contains("No build yet"), "{}", h.screen());
        h.send(send(Msg::Build));
        assert_eq!(h.app().pages.spinner.build, Build::Finished);
        let screen = h.screen();
        assert!(screen.contains("✔ Image deploy-api built"), "{screen}");
        assert!(screen.contains("build started") && screen.contains("Spinner#build-status"), "{screen}");
    }

    #[test]
    fn the_slow_load_button_shows_the_delayed_spinner_only_after_300_ms() {
        let mut h = page();
        assert!(h.screen().contains("No page loaded yet"), "{}", h.screen());
        let beside = h.find("No page loaded yet");
        h.click_text("Slow load");
        assert!(h.app().pages.spinner.loading);
        assert!(!h.screen().contains("Loading page"), "nothing shows at once:\n{}", h.screen());
        assert_eq!(h.find("No page loaded yet"), beside, "the hidden spinner keeps its cells");
        h.advance(Duration::from_millis(299));
        assert!(!h.screen().contains("Loading page"));
        h.advance(Duration::from_millis(1));
        assert!(h.screen().contains("Loading page"), "{}", h.screen());
        assert_eq!(h.find("No page loaded yet"), beside, "showing it moves nothing");
        h.advance(Duration::from_millis(1200));
        let screen = h.screen();
        assert!(!screen.contains("Loading page") && screen.contains("Loaded in 1.5 s"), "{screen}");
        assert!(screen.contains("Button#slow-load") && screen.contains("busy = false"), "{screen}");
    }

    #[test]
    fn the_quick_load_button_never_shows_the_spinner() {
        let mut h = page();
        h.click_text("Quick load");
        assert!(h.app().pages.spinner.loading);
        for _ in 0..4 {
            assert!(!h.screen().contains("Loading page"), "{}", h.screen());
            h.advance(Duration::from_millis(50));
        }
        assert!(!h.app().pages.spinner.loading);
        h.advance(Duration::from_millis(1000));
        let screen = h.screen();
        assert!(!screen.contains("Loading page") && screen.contains("Loaded in 0.2 s"), "{screen}");
    }
}
