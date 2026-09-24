//! Image: a picture drawn with half blocks, in the three fits, at a size the playground chooses.

use qframe::prelude::*;
use qframe::widgets::{Fit, Image, ImageData, Segmented, Slider};

use super::PageMsg;
use super::setting;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "image";

/// The sample picture's size in pixels: wider than the playground at its own size, so
/// `Fit::Center` cuts it, and small enough to work out in no time.
const SAMPLE: (u32, u32) = (160, 100);

/// Rows the playground's picture starts with, and the range its slider covers.
const ROWS: f64 = 10.0;
const MIN_ROWS: f64 = 4.0;
const MAX_ROWS: f64 = 24.0;

/// The fits, in the order the playground offers them.
const FITS: [Fit; 3] = [Fit::Contain, Fit::Cover, Fit::Center];

/// The sample picture and the playground settings.
#[derive(Debug)]
pub struct State {
    picture: ImageData,
    fit: usize,
    rows: f64,
}

impl Default for State {
    fn default() -> Self {
        Self { picture: sample(), fit: 0, rows: ROWS }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Fit(usize),
    Rows(f64),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Image(message))
}

// region: sample
/// A dusk over a lake, worked out pixel by pixel: a sky from indigo down to amber, a low sun, two
/// ridges of hills and the water mirroring the sky. Made here, so the showcase carries no file.
fn sample() -> ImageData {
    let (width, height) = SAMPLE;
    let horizon = height * 3 / 5;
    let mut rgb = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let (fx, fy) = (f64::from(x) / f64::from(width), f64::from(y) / f64::from(height));
            let colour = if y < horizon { land_or_sky(fx, fy) } else { water(fx, fy, horizon, height) };
            rgb.extend(colour);
        }
    }
    ImageData::from_rgb(width, height, &rgb).expect("the sample holds every pixel")
}

/// Above the waterline: the sky, the sun and the hills in front of them.
fn land_or_sky(x: f64, y: f64) -> [u8; 3] {
    let far = 0.42 + 0.06 * (x * 7.0).sin() + 0.03 * (x * 17.0 + 1.0).sin();
    let near = 0.50 + 0.05 * (x * 4.0 + 2.0).cos() + 0.02 * (x * 23.0).sin();
    if y > near {
        return [28, 24, 44];
    }
    if y > far {
        return [58, 46, 82];
    }
    let (dx, dy) = ((x - 0.68) * 1.6, y - 0.40);
    if (dx * dx + dy * dy).sqrt() < 0.07 {
        return [255, 214, 150];
    }
    blend([34, 30, 92], [250, 150, 90], (y / 0.6).powf(1.6))
}

/// Below the waterline: the sky mirrored, darker, broken by ripples.
fn water(x: f64, y: f64, horizon: u32, height: u32) -> [u8; 3] {
    let depth = (y * f64::from(height) - f64::from(horizon)) / f64::from(height - horizon);
    let ripple = 0.5 + 0.5 * (y * 90.0 + (x * 9.0).sin() * 2.0).sin();
    let mirrored = land_or_sky(x, 0.6 - depth * 0.6 * 0.9);
    blend(blend(mirrored, [16, 18, 40], 0.35 + depth * 0.4), [255, 214, 150], ripple * 0.08 * (1.0 - depth))
}

/// `from` mixed towards `to` by `share`, from 0 to 1.
fn blend(from: [u8; 3], to: [u8; 3], share: f64) -> [u8; 3] {
    let share = share.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * share).round().clamp(0.0, 255.0) as u8;
    [mix(from[0], to[0]), mix(from[1], to[1]), mix(from[2], to[2])]
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Fit(index) => {
            state.fit = index;
            log.push(PAGE, "Playground", format!("fit = {:?}", FITS[index]));
        }
        Msg::Rows(rows) => {
            state.rows = rows;
            log.push(PAGE, "Playground", format!("rows = {rows}"));
        }
    }
    Command::none()
}

/// The three fits side by side, and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("image.fits")).gap(0), |ui| {
        ui.add(Text::new(t!("image.fits-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            for (fit, caption) in
                [(Fit::Contain, "image.contain"), (Fit::Cover, "image.cover"), (Fit::Center, "image.center")]
            {
                ui.column(|ui| {
                    ui.add(Text::new(t!(caption)).role("faint"));
                    // region: fits
                    ui.add(Image::new(&state.picture).fit(fit)).height(Length::Cells(8)).fill_width();
                    // endregion
                })
                .width(Length::Fill(1));
            }
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let rows = state.rows.round().clamp(MIN_ROWS, MAX_ROWS) as u16;
        ui.add(Image::new(&state.picture).fit(FITS[state.fit]))
            .height(Length::Cells(rows))
            .fill_width()
            .id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("image.fit"), |ui| {
            let labels = [t!("image.contain"), t!("image.cover"), t!("image.center")];
            ui.add(Segmented::new(labels).selected(state.fit).on_select(|i| send(Msg::Fit(i)))).id("fit");
        });
        setting(ui, t!("image.rows"), |ui| {
            ui.add(
                Slider::new(state.rows)
                    .range(MIN_ROWS, MAX_ROWS)
                    .step(1.0)
                    .suffix(t!("image.rows-suffix"))
                    .on_change(|rows| send(Msg::Rows(rows))),
            )
            .width(Length::Cells(32))
            .id("rows");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::color::ColorDepth;

    use super::*;
    use crate::tests::{showcase_on, showcase_tall};

    /// Rows of the screen holding half blocks.
    fn picture_rows(screen: &str) -> usize {
        screen.lines().filter(|line| line.contains('▀')).count()
    }

    #[test]
    fn the_three_fits_show_the_sample_in_half_blocks() {
        let h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains('▀'), "{screen}");
        for caption in ["Contain", "Cover", "Center"] {
            assert!(screen.contains(caption), "{caption}:\n{screen}");
        }
    }

    #[test]
    fn the_playground_changes_the_fit_and_the_size() {
        let mut h = showcase_tall(crate::app::Showcase::new(), PAGE, 70);
        // The captions above the three fits read the same words, so a click aims at the row the
        // fit setting sits on.
        let click_fit = |h: &mut qframe::runtime::Harness<crate::app::Showcase>, word: &str| {
            let screen = h.screen();
            let (row, line) = screen
                .lines()
                .enumerate()
                .find(|(_, line)| line.contains(" Fit ") && line.contains("Center"))
                .unwrap_or_else(|| panic!("the fit row:\n{screen}"));
            let byte = line.find(word).unwrap_or_else(|| panic!("{word} on the fit row"));
            let column = line[..byte].chars().count();
            h.click(i32::try_from(column).unwrap_or(0), i32::try_from(row).unwrap_or(0));
        };
        click_fit(&mut h, "Cover");
        assert_eq!(h.app().pages.image.fit, 1, "the fit is chosen by clicking it");
        let before = picture_rows(&h.screen());
        // The height slider stands after its label's column of 24 cells; its keys raise it.
        let (x, y) = h.find("Height").unwrap_or_else(|| panic!("the height row:\n{}", h.screen()));
        h.click(x + 25, y);
        for _ in 0..6 {
            h.press("right");
        }
        assert!(h.app().pages.image.rows > 10.0, "the slider rose: {}", h.app().pages.image.rows);
        assert!(picture_rows(&h.screen()) > before, "a larger picture takes more rows:\n{}", h.screen());
        click_fit(&mut h, "Center");
        assert_eq!(h.app().pages.image.fit, 2);
    }

    #[test]
    fn sixteen_colours_say_the_picture_cannot_be_shown() {
        let mut h = showcase_on(PAGE);
        h.set_depth(ColorDepth::Ansi16);
        let screen = h.screen();
        assert!(!screen.contains('▀'), "{screen}");
        assert!(screen.contains("This terminal cannot"), "{screen}");
    }
}
