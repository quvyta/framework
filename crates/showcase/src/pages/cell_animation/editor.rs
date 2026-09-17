//! The studio panel: the animation's settings, its live preview, the frame rows and the TOML.

use std::sync::Arc;

use qframe::animation::{CellAnimation, ColorMode, Playback};
use qframe::prelude::*;
use qframe::widgets::{CodeView, Language, NumberInput, Segmented, Select, TextInput};

use super::draft::{Field, MODES, MOTION_KEYS, Problem, TimeDraft};
use super::{AnimationCell, Msg, State, mode_titles, send};
use crate::app::Msg as AppMsg;
use crate::pages::setting;

/// Widths of the frame row columns, in cells.
const NUMBER: u16 = 3;
const GLYPH: u16 = 9;
const COLOR: u16 = 25;
const DURATION: u16 = 11;

/// A problem as a quiet line with a warning marker.
fn problem_line(ui: &mut View<'_, AppMsg>, indent: u16, text: String) {
    let marker = ui.env().icons().glyph("warning").into_owned();
    ui.row(|ui| {
        ui.spacer().width(Length::Cells(indent));
        ui.add(Text::new(marker).color("warning").no_wrap());
        ui.add(Text::new(text).role("secondary")).fill_width();
    })
    .gap(1)
    .fill_width();
}

/// The problems of `field` on `frame`.
fn problems_of(problems: &[Problem], frame: Option<usize>, field: Field) -> impl Iterator<Item = &Problem> {
    problems.iter().filter(move |problem| problem.frame == frame && problem.field == field)
}

/// A colour naming a token the theme lacks parses, but only the theme can tell it is unknown.
fn unknown_token(ui: &View<'_, AppMsg>, color: &str) -> Option<String> {
    let color = qframe::animation::CellColor::parse(color).ok()?;
    let theme = ui.env().theme();
    color.resolve(theme, qframe::color::Rgb::new(0, 0, 0)).err()
}

/// The studio: settings, preview, frames and TOML of the animation being edited.
pub fn studio(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("cell-animation.studio")).gap(0), |ui| {
        ui.add(Text::new(t!("cell-animation.studio-hint")).role("secondary"));
        for restored in &state.restored {
            problem_line(ui, 0, restored.clone());
        }
        ui.spacer().height(Length::Cells(1));
        settings(state, ui);
        ui.spacer().height(Length::Cells(1));
        preview(state, ui);
        ui.spacer().height(Length::Cells(1));
        frames(state, ui);
        ui.spacer().height(Length::Cells(1));
        actions(state, ui);
    })
    .fill_width();
}

fn settings(state: &State, ui: &mut View<'_, AppMsg>) {
    let draft = state.draft();
    setting(ui, t!("cell-animation.start"), |ui| {
        let options: Vec<String> = std::iter::once(t!("cell-animation.new")).chain(state.names()).collect();
        let selected = state.names().iter().position(|name| *name == draft.name).map(|index| index + 1);
        ui.add(
            Select::new(options).selected(selected).placeholder(draft.name.clone()).on_select(|i| send(Msg::Open(i))),
        )
        .width(Length::Cells(32))
        .id("start");
        let status = match state.builtin(&draft.name) {
            None => t!("cell-animation.own"),
            Some(_) if state.changed(draft) => t!("cell-animation.changed"),
            Some(_) => t!("cell-animation.unchanged"),
        };
        ui.add(Text::new(status).role("faint").no_wrap());
    });
    setting(ui, t!("cell-animation.name"), |ui| {
        let invalid = problems_of(&state.problems, None, Field::Name).next().is_some();
        ui.add(TextInput::new(draft.name.clone()).invalid(invalid).on_change(|text| send(Msg::Name(text))))
            .width(Length::Cells(32))
            .id("name");
    });
    for problem in problems_of(&state.problems, None, Field::Name) {
        problem_line(ui, 24, problem.message.clone());
    }
    setting(ui, t!("cell-animation.time"), |ui| {
        let kind = usize::from(matches!(draft.time, TimeDraft::Millis(_)));
        let kinds = [t!("cell-animation.time-theme"), t!("cell-animation.time-fixed")];
        ui.add(Segmented::new(kinds).selected(kind).on_select(|i| send(Msg::TimeKind(i)))).id("time");
        match draft.time {
            TimeDraft::Motion(index) => {
                ui.add(Select::new(MOTION_KEYS).selected(Some(index)).on_select(|i| send(Msg::MotionKey(i))))
                    .width(Length::Cells(18))
                    .id("motion-key");
                let motion = ui.env().theme().motion();
                let lasts = motion.duration(MOTION_KEYS[index]).map_or(0, |d| d.as_millis());
                ui.add(Text::new(t!("cell-animation.lasts", ms = lasts.to_string())).role("faint").no_wrap());
            }
            TimeDraft::Millis(millis) => {
                ui.add(
                    NumberInput::new(f64::from(millis))
                        .range(10.0, 5000.0)
                        .step(10.0)
                        .on_change(|value| send(Msg::Millis(value))),
                )
                .width(Length::Cells(18))
                .id("millis");
                ui.add(Text::new("ms").role("faint").no_wrap());
            }
        }
    });
    setting(ui, t!("cell-animation.playback"), |ui| {
        let index = Playback::ALL.iter().position(|p| *p == draft.playback).unwrap_or(0);
        let names = Playback::ALL.map(Playback::name);
        ui.add(Segmented::new(names).selected(index).on_select(|i| send(Msg::Playback(i)))).id("playback");
    });
    setting(ui, t!("cell-animation.colors"), |ui| {
        let index = ColorMode::ALL.iter().position(|c| *c == draft.colors).unwrap_or(0);
        let names = ColorMode::ALL.map(ColorMode::name);
        ui.add(Segmented::new(names).selected(index).on_select(|i| send(Msg::Colors(i)))).id("colors");
    });
    setting(ui, t!("cell-animation.rest"), |ui| {
        let options: Vec<String> = std::iter::once(t!("cell-animation.rest-auto"))
            .chain((1..=draft.frames.len()).map(|n| t!("cell-animation.frame-number", n = n.to_string())))
            .collect();
        let selected = draft.rest.filter(|rest| *rest < draft.frames.len()).map_or(0, |rest| rest + 1);
        ui.add(Select::new(options).selected(Some(selected)).on_select(|i| send(Msg::Rest(i))))
            .width(Length::Cells(32))
            .id("rest");
    });
}

fn preview(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add(Text::new(t!("cell-animation.preview")).role("faint"));
    if state.built.frames().is_empty() {
        ui.add(Text::new(t!("cell-animation.no-frames")).role("secondary"));
        return;
    }
    ui.row(|ui| {
        for (mode, title) in MODES.into_iter().zip(mode_titles()) {
            ui.column(|ui| {
                let id = format!("large-{}", title.to_lowercase().replace(' ', "-"));
                ui.add(Text::new(title).role("faint").no_wrap());
                ui.add(cell(&state.built, mode, true, None, state.replay)).id(id);
                ui.add(cell(&state.built, mode, false, Some(t!("cell-animation.preview-label")), state.replay));
            })
            .gap(1)
            .width(Length::Cells(24));
        }
    })
    .gap(2);
}

/// A preview cell of `animation` in `mode`.
fn cell(
    animation: &Arc<CellAnimation>,
    mode: qframe::icons::GlyphMode,
    large: bool,
    label: Option<String>,
    replay: u32,
) -> AnimationCell {
    AnimationCell { animation: Arc::clone(animation), mode, large, label, replay, repeat: false }
}

fn frames(state: &State, ui: &mut View<'_, AppMsg>) {
    let draft = state.draft();
    ui.add(Text::new(t!("cell-animation.frames", n = draft.frames.len())).role("faint"));
    ui.row(|ui| {
        ui.spacer().width(Length::Cells(NUMBER + 6));
        for title in mode_titles() {
            ui.add(Text::new(title).role("faint").no_wrap()).width(Length::Cells(GLYPH));
        }
        ui.add(Text::new(t!("cell-animation.color")).role("faint").no_wrap()).width(Length::Cells(COLOR));
        ui.add(Text::new(t!("cell-animation.duration")).role("faint").no_wrap()).width(Length::Cells(DURATION));
    })
    .gap(1);
    let icons = ui.env().icons().clone();
    let default_time = draft.frame_time().to_string();
    for (index, frame) in draft.frames.iter().enumerate() {
        ui.row(|ui| {
            ui.add(Text::new(format!("{}", index + 1)).role("secondary").no_wrap()).width(Length::Cells(NUMBER));
            // The frame alone in each mode, standing still, so fallbacks show at a glance.
            ui.row(|ui| {
                for mode in MODES {
                    match &state.frame_cells[index] {
                        Some(alone) => {
                            ui.add(AnimationCell {
                                animation: Arc::clone(alone),
                                mode,
                                large: false,
                                label: None,
                                replay: 0,
                                repeat: false,
                            });
                        }
                        None => {
                            ui.add(Text::new(" ").no_wrap());
                        }
                    }
                }
            })
            .gap(1)
            .width(Length::Cells(5));
            for (column, _) in MODES.iter().enumerate() {
                let invalid = problems_of(&state.problems, Some(index), Field::Glyph(column)).next().is_some();
                let fallback =
                    frame.glyphs[column + 1..].iter().find(|glyph| !glyph.is_empty()).cloned().unwrap_or_default();
                ui.add(
                    TextInput::new(frame.glyphs[column].clone())
                        .placeholder(fallback)
                        .invalid(invalid)
                        .on_change(move |text| send(Msg::Glyph(index, column, text))),
                )
                .width(Length::Cells(GLYPH))
                .id(format!("frame-{}-{}", index + 1, ["nerd", "unicode", "ascii"][column]));
            }
            let color_invalid = problems_of(&state.problems, Some(index), Field::Color).next().is_some();
            ui.add(
                TextInput::new(frame.color.clone())
                    .placeholder(t!("cell-animation.color-placeholder"))
                    .invalid(color_invalid)
                    .on_change(move |text| send(Msg::Color(index, text))),
            )
            .width(Length::Cells(COLOR))
            .id(format!("frame-{}-color", index + 1));
            let duration_invalid = problems_of(&state.problems, Some(index), Field::Duration).next().is_some();
            ui.add(
                TextInput::new(frame.duration.clone())
                    .placeholder(default_time.clone())
                    .invalid(duration_invalid)
                    .on_change(move |text| send(Msg::Duration(index, text))),
            )
            .width(Length::Cells(DURATION))
            .id(format!("frame-{}-duration", index + 1));
            let last = draft.frames.len() - 1;
            ui.add(
                Button::new(icons.glyph("arrow-up").into_owned()).disabled(index == 0).on_press(send(Msg::Up(index))),
            )
            .id(format!("frame-{}-up", index + 1));
            ui.add(
                Button::new(icons.glyph("arrow-down").into_owned())
                    .disabled(index == last)
                    .on_press(send(Msg::Down(index))),
            )
            .id(format!("frame-{}-down", index + 1));
            ui.add(
                Button::new(icons.glyph("close").into_owned()).disabled(last == 0).on_press(send(Msg::Remove(index))),
            )
            .id(format!("frame-{}-remove", index + 1));
        })
        .gap(1)
        .align(Align::Center);
        let indent = NUMBER + 6;
        for problem in state.problems.iter().filter(|problem| problem.frame == Some(index)) {
            problem_line(ui, indent, problem.message.clone());
        }
        if !color_problem(state, index)
            && let Some(message) = unknown_token(ui, &frame.color)
        {
            problem_line(ui, indent, message);
        }
    }
    ui.add(Button::new(t!("cell-animation.add")).on_press(send(Msg::Add))).id("add-frame");
}

/// Whether frame `index` already reports a broken colour.
fn color_problem(state: &State, index: usize) -> bool {
    problems_of(&state.problems, Some(index), Field::Color).next().is_some()
}

fn actions(state: &State, ui: &mut View<'_, AppMsg>) {
    let draft = state.draft();
    let copyable = !state.built.frames().is_empty() && problems_of(&state.problems, None, Field::Name).next().is_none();
    ui.row(|ui| {
        ui.add(Button::new(t!("cell-animation.copy")).variant("primary").disabled(!copyable).on_press(send(Msg::Copy)))
            .id("copy-toml");
        if draft.playback == Playback::Once {
            ui.add(Button::new(t!("cell-animation.replay")).on_press(send(Msg::Replay))).id("replay");
        }
        let resettable = state.builtin(&draft.name).is_some() && state.changed(draft);
        ui.add(Button::new(t!("cell-animation.reset")).disabled(!resettable).on_press(send(Msg::Reset))).id("reset");
    })
    .gap(2);
    ui.spacer().height(Length::Cells(1));
    let toml = state.built.to_toml(&draft.name);
    ui.add(CodeView::new(toml, Language::Toml).line_numbers(false)).fill_width().id("toml");
}
