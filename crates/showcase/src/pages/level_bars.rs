//! Level bars: a player's spectrum as columns, with peak caps, a mirror and a gradient.

use std::time::Duration;

use qframe::prelude::*;
use qframe::storage::Settings;
use qframe::widgets::{LevelBars, Segmented};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "level-bars";

/// Time one frame of the demo stands, quick enough to read as music.
const FRAME: Duration = Duration::from_millis(120);

/// Frames a cap is held at a level before the band is let down a step.
const HOLD: u32 = 24;

/// Bands the demo's spectrum is measured in.
const BANDS: u32 = 24;

/// Band counts the playground offers.
const WIDTHS: [u32; 3] = [BANDS, 64, 200];

/// Where the demo's clock is and the playground settings.
#[derive(Debug)]
pub struct State {
    /// The frame the spectrum is measured at.
    tick: u32,
    /// Whether the demo's own clock is running.
    playing: bool,
    /// Bumped whenever the clock is started, so a frame from an older one ends it.
    generation: u64,
    mirror: bool,
    gradient: bool,
    peaks: bool,
    height: usize,
    bands: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { tick: 0, playing: false, generation: 0, mirror: false, gradient: true, peaks: true, height: 2, bands: 0 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Play(bool),
    /// The demo's own clock reached the next frame.
    Tick(u64),
    Mirror(bool),
    Gradient(bool),
    Peaks(bool),
    Height(usize),
    Bands(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::LevelBars(message))
}

/// Applies a demo message. The clock rests while motion is reduced, which is why the remembered
/// `settings` are asked about it here and not while the page is drawn.
pub fn update(state: &mut State, settings: &Settings, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let reduced = settings.reduced_motion().unwrap_or(false);
    match message {
        // region: spectrum-play
        Msg::Play(on) => {
            state.playing = on && !reduced;
            state.generation += 1;
            if !state.playing {
                log.push(PAGE, "Switch#play", format!("playing = false{}", rest(reduced && on)));
                return Command::none();
            }
            log.push(PAGE, "Switch#play", "playing = true");
            return next_frame(state.generation);
        }
        Msg::Tick(generation) if state.playing && generation == state.generation => {
            if reduced {
                state.playing = false;
                log.push(PAGE, "Switch#play", "playing = false, the demo rests while motion is reduced");
                return Command::none();
            }
            state.tick += 1;
            return next_frame(generation);
        }
        // endregion
        Msg::Tick(_) => {}
        Msg::Mirror(on) => {
            state.mirror = on;
            log.push(PAGE, "Playground", format!("mirror = {on}"));
        }
        Msg::Gradient(on) => {
            state.gradient = on;
            log.push(PAGE, "Playground", format!("gradient = {on}"));
        }
        Msg::Peaks(on) => {
            state.peaks = on;
            log.push(PAGE, "Playground", format!("peaks = {on}"));
        }
        Msg::Height(index) => {
            state.height = index;
            log.push(PAGE, "Playground", format!("height = {}", index + 1));
        }
        Msg::Bands(index) => {
            state.bands = index;
            log.push(PAGE, "Playground", format!("bands = {}", WIDTHS[index]));
        }
    }
    Command::none()
}

/// What the page's log says about a clock that was asked for while motion is reduced.
fn rest(reduced: bool) -> &'static str {
    if reduced { ", the demo rests while motion is reduced" } else { "" }
}

/// Waits one frame on a background thread, then asks for the next one.
fn next_frame(generation: u64) -> Command<AppMsg> {
    Command::perform(move || {
        std::thread::sleep(FRAME);
        send(Msg::Tick(generation))
    })
}

/// The spectrum of `bands` bands at `tick`, from 0 to 1: loud at the low bands and quieter as the
/// pitch rises, with a voice that swells and fades and a little grain per band, the way a player's
/// analyser answers a track with a strong low end.
fn spectrum(bands: u32, tick: u32) -> Vec<f32> {
    (0..bands)
        .map(|band| {
            let t = tick as f32;
            let at = band as f32 / bands.max(1) as f32;
            let tilt = (1.0 - at * 0.72).clamp(0.05, 1.0);
            let swell = ((t / 7.0) + band as f32 * 0.9).sin() * 0.18 + 0.55;
            let wobble = ((t / 2.1) + band as f32 * 1.7).sin() * 0.12;
            let hash = tick.wrapping_mul(2_654_435_761).wrapping_add(band.wrapping_mul(40_503)) >> 26;
            let grain = hash as f32 / 31.0 * 0.12 - 0.06;
            (tilt * (swell + wobble + grain)).clamp(0.0, 1.0)
        })
        .collect()
}

/// The caps of `levels` at `tick`: the level each band reached and has not fallen away from since,
/// the high mark a meter holds. The frames behind it are measured again, which is what lets a cap
/// fall a step at a time instead of dropping with the band.
fn caps(levels: &[f32], tick: u32) -> Vec<f32> {
    let bands = u32::try_from(levels.len()).unwrap_or(0);
    let recent: Vec<Vec<f32>> = (tick.saturating_sub(HOLD)..=tick).map(|t| spectrum(bands, t)).collect();
    levels
        .iter()
        .enumerate()
        .map(|(band, level)| recent.iter().filter_map(|frame| frame.get(band).copied()).fold(*level, f32::max))
        .collect()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let levels = spectrum(BANDS, state.tick);
    ui.add_with(Panel::new().title(t!("level-bars.spectrum")).gap(0), |ui| {
        ui.add(Text::new(t!("level-bars.spectrum-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: spectrum-bars
        ui.add(LevelBars::new(levels.clone()).peaks(caps(&levels, state.tick)))
            .width(Length::Fill(1))
            .height(Length::Cells(8))
            .id("spectrum");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("level-bars.play"), |ui| {
            ui.add(toggle(state.playing, |on| send(Msg::Play(on)))).id("play");
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: spectrum-playground
        let values = spectrum(WIDTHS[state.bands], state.tick);
        let mut bars = LevelBars::new(values.clone()).mirror(state.mirror).gradient(state.gradient);
        if state.peaks {
            bars = bars.peaks(caps(&values, state.tick));
        }
        ui.add(bars)
            .width(Length::Fill(1))
            .height(Length::Cells(u16::try_from(state.height + 1).unwrap_or(1)))
            .id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("level-bars.bands"), |ui| {
            ui.add(Segmented::new(["24", "64", "200"]).selected(state.bands).on_select(|i| send(Msg::Bands(i))))
                .id("bands");
        });
        setting(ui, t!("level-bars.height"), |ui| {
            ui.add(Segmented::new(["1", "2", "3", "4"]).selected(state.height).on_select(|i| send(Msg::Height(i))))
                .id("height");
        });
        setting(ui, t!("level-bars.peaks"), |ui| {
            ui.add(toggle(state.peaks, |on| send(Msg::Peaks(on)))).id("peaks");
        });
        setting(ui, t!("level-bars.mirror"), |ui| {
            ui.add(toggle(state.mirror, |on| send(Msg::Mirror(on)))).id("mirror");
        });
        setting(ui, t!("level-bars.gradient"), |ui| {
            ui.add(toggle(state.gradient, |on| send(Msg::Gradient(on)))).id("gradient");
        });
        ui.add(Text::new(t!("level-bars.playground-hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::color::Rgb;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_tall;

    /// The page as a person reaches it, tall enough for the playground below the demo.
    fn page() -> Harness<Showcase> {
        showcase_tall(Showcase::new(), PAGE, 64)
    }

    /// The rows between the playground's title and its first control, so a change in the controls
    /// below is never mistaken for a change in the columns.
    fn columns(h: &Harness<Showcase>) -> String {
        let (_, title) = h.find("PLAYGROUND").expect("the playground is on screen");
        let (_, bands) = h.find("Bands").expect("its rows are on screen");
        let screen = h.screen();
        let rows: Vec<String> = (title + 1..bands)
            .filter_map(|row| screen.lines().nth(usize::try_from(row).unwrap_or(0)))
            .map(str::to_owned)
            .collect();
        rows.join("\n")
    }

    /// Plays the demo for a few frames and stops it again, so the spectrum has a shape to look at
    /// and the columns hold still while the playground is read.
    fn played(h: &mut Harness<Showcase>) {
        let (x, y) = h.find("Play the demo").expect("the demo's switch is on screen");
        h.click(x + 25, y);
        h.render().render().render();
        h.click(x + 25, y);
        assert!(!h.app().pages.level_bars.playing, "the demo is at rest again");
    }

    #[test]
    fn the_demo_runs_on_its_own_clock_and_rests_when_asked() {
        let mut h = page();
        let before = h.screen();
        assert!(before.contains('█') || before.contains('▆'), "{before}");
        // The switch stands after its label's column of 24 cells.
        let (x, y) = h.find("Play the demo").expect("the demo's switch is on screen");
        h.click(x + 25, y);
        assert!(h.app().pages.level_bars.playing, "the click starts the clock");
        let resting = h.app().pages.level_bars.tick;
        // Each step runs the page's own wait for one frame, as one pass of the terminal loop would,
        // so the clock moves the spectrum on without anything else being asked of it.
        h.render().render();
        let tick = h.app().pages.level_bars.tick;
        assert!(tick > resting, "the page's own clock moved the spectrum on\n{}", h.screen());
        h.click(x + 25, y);
        assert!(!h.app().pages.level_bars.playing, "the second click stops it");
        let stopped = h.app().pages.level_bars.tick;
        h.render().render();
        assert_eq!(h.app().pages.level_bars.tick, stopped, "a stopped clock does not move again");
    }

    #[test]
    fn reduced_motion_stops_the_clock_and_keeps_it_stopped() {
        let mut h = page();
        let (x, y) = h.find("Play the demo").expect("the demo's switch is on screen");
        h.click(x + 25, y);
        assert!(h.app().pages.level_bars.playing);
        h.send(AppMsg::ReducedMotion(true));
        h.send(send(Msg::Tick(h.app().pages.level_bars.generation)));
        assert!(!h.app().pages.level_bars.playing, "reduced motion stops the demo at once");
        h.click(x + 25, y);
        assert!(!h.app().pages.level_bars.playing, "and it does not start while motion is reduced");
        let said = h.app().log.recent(PAGE, 1)[0].message.clone();
        assert!(said.starts_with("playing = false"), "{said}");
    }

    #[test]
    fn the_playground_turns_the_capabilities_on_one_at_a_time() {
        let mut h = page();
        played(&mut h);
        let plain = columns(&h);
        assert!(plain.contains('█'), "the playground starts with a plain row:\n{plain}");
        h.send(send(Msg::Bands(2)));
        let merged = columns(&h);
        assert_ne!(merged, plain, "two hundred bands are merged into the same columns:\n{merged}");
        h.send(send(Msg::Bands(0)));
        h.send(send(Msg::Peaks(false)));
        assert_ne!(columns(&h), plain, "a cap is a mark of its own, not more of the column");
        h.send(send(Msg::Peaks(true)));
        assert_eq!(columns(&h), plain, "and the caps come back");
        h.send(send(Msg::Mirror(true)));
        let mirrored = columns(&h);
        assert_ne!(mirrored, plain, "a mirrored row grows from the middle:\n{mirrored}");
        h.send(send(Msg::Height(3)));
        assert_ne!(columns(&h), mirrored, "and four rows stand for more of the spectrum");
    }

    #[test]
    fn a_frame_from_a_clock_that_has_stopped_is_not_heard() {
        let mut state = State::default();
        let mut log = EventLog::new();
        let settings = Settings::in_memory();
        state.playing = true;
        let generation = state.generation;
        let _ = update(&mut state, &settings, Msg::Tick(generation + 1), &mut log);
        assert_eq!(state.tick, 0, "a frame from another clock moves nothing");
        let _ = update(&mut state, &settings, Msg::Tick(generation), &mut log);
        assert_eq!(state.tick, 1, "and the frame of the clock that is running is heard");
    }

    #[test]
    fn a_gradient_deepens_the_playground_columns() {
        let mut h = page();
        played(&mut h);
        // With the caps off, only the gradient speaks for the colour of a column.
        h.send(send(Msg::Peaks(false)));
        assert!(tones_down_a_column(&h) > 1, "the column deepens from its base to its top");
        h.send(send(Msg::Gradient(false)));
        assert_eq!(tones_down_a_column(&h), 1, "and without the gradient it is one tone");
    }

    /// How many tones a column of the playground is drawn in, its own cells read from the row its
    /// controls start at.
    fn tones_down_a_column(h: &Harness<Showcase>) -> usize {
        let (_, bands) = h.find("Bands").expect("the playground's rows are on screen");
        let tones: Vec<Option<Rgb>> =
            (bands - 4..bands - 1).map(|row| h.fg(36, u16::try_from(row).unwrap_or(0))).collect();
        tones.iter().enumerate().filter(|(index, tone)| !tones[..*index].contains(tone)).count()
    }
}
