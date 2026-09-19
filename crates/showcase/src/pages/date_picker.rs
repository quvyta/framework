//! Date picker: choosing a day from a calendar layer.

use qframe::date::{Date, Weekday};
use qframe::prelude::*;
use qframe::widgets::{DatePicker, Segmented};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "date-picker";

/// Regions the playground offers besides the language's own week: Sunday, Monday and Saturday.
const REGIONS: [&str; 3] = ["US", "GB", "EG"];

/// Chosen dates and the playground.
#[derive(Debug, Default)]
pub struct State {
    release: Option<Date>,
    maintenance: Option<Date>,
    disabled: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Release(Date),
    Maintenance(Date),
    Clear,
    Disabled(bool),
    Region(Option<&'static str>),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::DatePicker(message))
}

fn iso(date: Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Release(date) => {
            state.release = Some(date);
            log.push(PAGE, "DatePicker#release", format!("changed {}", iso(date)));
        }
        Msg::Maintenance(date) => {
            state.maintenance = Some(date);
            log.push(PAGE, "DatePicker#maintenance", format!("changed {}", iso(date)));
        }
        Msg::Clear => {
            state.release = None;
            state.maintenance = None;
            log.push(PAGE, "Button#clear", "cleared both dates");
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
        Msg::Region(region) => {
            log.push(PAGE, "Playground", format!("region = {}", region.unwrap_or("none")));
            // region: date-region
            return Command::set_region(region);
            // endregion
        }
    }
    Command::none()
}

/// The playground's choices: the language, the offered regions, and the region the system gave
/// when it is none of them, so the selection always tells the truth.
fn region_choices(current: Option<&str>) -> (Vec<Option<&'static str>>, Vec<String>, usize) {
    let mut regions: Vec<Option<&'static str>> = vec![None];
    regions.extend(REGIONS.iter().copied().map(Some));
    let mut names: Vec<String> =
        regions.iter().map(|r| r.map_or_else(|| t!("date-picker.region-language"), str::to_owned)).collect();
    let selected = match regions.iter().position(|r| *r == current) {
        Some(index) => index,
        None => {
            names.push(current.unwrap_or_default().to_owned());
            names.len() - 1
        }
    };
    (regions, names, selected)
}

fn day_name(day: Weekday) -> String {
    t!(&format!("date-picker.day-{}", day.number()))
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        setting(ui, t!("date-picker.release"), |ui| {
            // region: date-basic
            ui.add(
                DatePicker::new(state.release)
                    .placeholder(t!("date-picker.choose"))
                    .disabled(state.disabled)
                    .on_change(|date| send(Msg::Release(date))),
            )
            .width(Length::Cells(24))
            .id("release");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("date-picker.maintenance"), |ui| {
            ui.add(
                DatePicker::new(state.maintenance)
                    .placeholder(t!("date-picker.choose"))
                    .disabled(state.disabled)
                    .on_change(|date| send(Msg::Maintenance(date))),
            )
            .width(Length::Cells(24))
            .id("maintenance");
        });
        ui.spacer().height(Length::Cells(1));
        // region: date-value
        let summary = match (state.release, state.maintenance) {
            (Some(release), Some(maintenance)) if maintenance < release => {
                t!("date-picker.before", n = release.to_days() - maintenance.to_days())
            }
            (Some(release), Some(maintenance)) => {
                t!("date-picker.after", n = maintenance.to_days() - release.to_days())
            }
            _ => t!("date-picker.empty"),
        };
        ui.add(Text::new(summary).role("secondary"));
        // endregion
        // region: date-week
        let i18n = ui.env().i18n();
        let day = day_name(i18n.first_weekday());
        let week = match i18n.region() {
            Some(region) => t!("date-picker.week-region", day = day, region = region),
            None => t!("date-picker.week-language", day = day),
        };
        ui.add(Text::new(week).role("faint"));
        // endregion
        ui.spacer().height(Length::Cells(10));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        let (regions, names, selected) = region_choices(ui.env().i18n().region());
        setting(ui, t!("date-picker.region-label"), |ui| {
            let pick = move |index: usize| send(Msg::Region(regions.get(index).copied().flatten()));
            ui.add(Segmented::new(names).selected(selected).on_select(pick)).id("region");
        });
        setting(ui, t!("date-picker.clear-label"), |ui| {
            ui.add(Button::new(t!("date-picker.clear")).on_press(send(Msg::Clear))).id("clear");
        });
        ui.add(Text::new(t!("date-picker.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn keyboard_chooses_a_day_and_logs_it() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("Choose a date");
        h.press("right").press("enter");
        assert!(h.app().pages.date_picker.release.is_some());
        assert!(h.screen().contains("DatePicker#release"));
    }

    #[test]
    fn a_hovered_day_shows_the_pillar_without_moving_its_number() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("Choose a date");
        let (x, y) = h.find("22").expect("a day");
        let row = |h: &qframe::runtime::Harness<crate::app::Showcase>| {
            h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned()
        };
        let resting = row(&h);
        h.hover(x, y);
        let hovered = row(&h);
        assert_eq!(hovered.chars().nth(usize::try_from(x - 1).unwrap_or(0)), Some('▌'), "{hovered}");
        assert_eq!(hovered.replace('▌', " "), resting.replace('▌', " "), "numbers stay in their columns");
    }

    #[test]
    fn a_region_moves_the_first_day_of_an_english_week() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true).set_region(None);
        assert!(h.screen().contains("Weeks start on Sunday, as the language says."), "{}", h.screen());
        h.click_text("GB");
        assert!(h.screen().contains("Weeks start on Monday, as they do in GB."), "{}", h.screen());
        h.click_text("Choose a date");
        assert!(h.screen().contains("Mo  Tu  We  Th  Fr  Sa  Su"), "{}", h.screen());
        h.press("esc").click_text("EG").click_text("Choose a date");
        assert!(h.screen().contains("Sa  Su  Mo  Tu  We  Th  Fr"), "{}", h.screen());
    }

    #[test]
    fn a_system_region_outside_the_offered_ones_is_shown_selected() {
        let mut h = showcase_on(PAGE);
        h.set_region(Some("JP"));
        assert!(h.screen().contains("as they do in JP"), "{}", h.screen());
        assert!(h.find("JP").is_some());
    }

    #[test]
    fn turkish_calendar_starts_on_monday() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true).set_locale("tr");
        h.click_text("Bir tarih seç");
        assert!(h.screen().contains("Pt  Sa  Ça  Pe  Cu  Ct  Pz"), "{}", h.screen());
    }
}
