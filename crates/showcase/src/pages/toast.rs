//! Toast: notifications that stack in a corner and leave on their own.

use qframe::prelude::*;
use qframe::widgets::{Corner, Segmented, SpinnerStyle, Toast};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "toast";

/// Upload progress and the playground.
#[derive(Debug)]
pub struct State {
    upload: u32,
    corner: usize,
    /// The running upload's icon animation: 0 is none, then one per spinner style.
    motion: usize,
    /// Whether the deploy toast opens its log when pressed.
    pressable: bool,
    /// Whether a press on the deploy toast opened its log.
    log_open: bool,
}

impl Default for State {
    fn default() -> Self {
        let corner = Corner::ALL.iter().position(|corner| *corner == Corner::default()).unwrap_or(0);
        // The upload shows a pulse while it runs.
        let motion = SpinnerStyle::ALL.iter().position(|style| *style == SpinnerStyle::Pulse).map_or(0, |i| i + 1);
        Self { upload: 0, corner, motion, pressable: false, log_open: false }
    }
}

impl State {
    /// The chosen icon animation, if any.
    fn icon_motion(&self) -> Option<SpinnerStyle> {
        self.motion.checked_sub(1).and_then(|index| SpinnerStyle::ALL.get(index).copied())
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Deployed,
    DiskWarning,
    BuildFailed,
    Retry,
    Released,
    UploadStep,
    Corner(usize),
    Motion(usize),
    Pressable(bool),
    OpenDeployLog,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Toast(message))
}

// region: toast-show
/// Applies a demo message; most of them answer with a toast.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Deployed => {
            log.push(PAGE, "Command::toast", "success");
            let toast = Toast::success(t!("toast.deployed")).body(t!("toast.deployed-body"));
            // A press on the toast body goes where the news came from; the close mark still closes.
            Command::toast(if state.pressable { toast.on_press(send(Msg::OpenDeployLog)) } else { toast })
        }
        Msg::OpenDeployLog => {
            state.log_open = true;
            log.push(PAGE, "Toast#deployed", "on_press: open the deploy log");
            Command::none()
        }
        Msg::DiskWarning => {
            log.push(PAGE, "Command::toast", "warning");
            Command::toast(Toast::warning(t!("toast.disk")).body(t!("toast.disk-body")))
        }
        Msg::BuildFailed => {
            log.push(PAGE, "Command::toast", "danger with action");
            Command::toast(Toast::danger(t!("toast.failed")).action(t!("toast.retry"), send(Msg::Retry)))
        }
        Msg::Retry => {
            log.push(PAGE, "Toast#build", "retry");
            Command::toast(Toast::info(t!("toast.retrying")))
        }
        Msg::Released => {
            log.push(PAGE, "Command::toast", "info");
            Command::toast(Toast::info(t!("toast.release")))
        }
        Msg::UploadStep => {
            state.upload = if state.upload >= 100 { 25 } else { state.upload + 25 };
            log.push(PAGE, "Toast#upload", format!("{}%", state.upload));
            if state.upload >= 100 {
                // The same key: the running toast settles into a success toast in place.
                return Command::toast(Toast::success(t!("toast.uploaded")).key("upload"));
            }
            let mut toast = Toast::info(t!("toast.uploading", n = state.upload)).key("upload");
            if let Some(style) = state.icon_motion() {
                toast = toast.icon_motion(style);
            }
            Command::toast(toast)
        }
        Msg::Corner(index) => {
            state.corner = index;
            log.push(PAGE, "Playground", format!("corner = {}", Corner::ALL[index].name()));
            Command::toast_corner(Corner::ALL[index])
        }
        Msg::Pressable(on) => {
            state.pressable = on;
            log.push(PAGE, "Playground", format!("on_press = {on}"));
            Command::none()
        }
        Msg::Motion(index) => {
            state.motion = index;
            let name = state.icon_motion().map_or("none", SpinnerStyle::name);
            log.push(PAGE, "Playground", format!("icon motion = {name}"));
            Command::none()
        }
    }
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.row(|ui| {
            ui.add(Button::new(t!("toast.show-success")).on_press(send(Msg::Deployed))).id("success");
            ui.add(Button::new(t!("toast.show-warning")).on_press(send(Msg::DiskWarning))).id("warning");
            ui.add(Button::new(t!("toast.show-danger")).on_press(send(Msg::BuildFailed))).id("danger");
            ui.add(Button::new(t!("toast.show-info")).on_press(send(Msg::Released))).id("info");
        })
        .gap(2);
        ui.row(|ui| {
            ui.add(Button::new(t!("toast.upload")).on_press(send(Msg::UploadStep))).id("upload");
            ui.add(Text::new(t!("toast.upload-hint")).role("faint"));
        })
        .gap(2);
        if state.log_open {
            ui.add(Text::new(t!("toast.deploy-log")).role("secondary")).id("deploy-log");
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("toast.corner"), |ui| {
            let names = Corner::ALL.map(|corner| t!(&format!("toast.{}", corner.name())));
            ui.add(Segmented::new(names).selected(state.corner).on_select(|index| send(Msg::Corner(index))))
                .id("corner");
        });
        setting(ui, t!("toast.motion"), |ui| {
            let mut names = vec![t!("toast.motion-none")];
            names.extend(SpinnerStyle::ALL.map(|style| style.name().to_owned()));
            ui.add(Segmented::new(names).selected(state.motion).on_select(|index| send(Msg::Motion(index))))
                .id("motion");
        });
        setting(ui, t!("toast.pressable"), |ui| {
            ui.add(toggle(state.pressable, |on| send(Msg::Pressable(on)))).id("pressable");
        });
        ui.add(Text::new(t!("toast.hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn buttons_show_toasts_and_the_action_retries() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("Build failed");
        h.advance(Duration::from_millis(10));
        let screen = h.screen();
        assert!(screen.contains("The build of web-frontend failed"), "{screen}");
        h.click_text("Retry");
        assert!(h.screen().contains("Retrying the build"), "{}", h.screen());
    }

    #[test]
    fn only_the_close_mark_dismisses_and_the_playground_makes_the_deploy_toast_open_its_log() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("Deploy finished");
        h.click_text("Deployed api-gateway");
        assert!(h.screen().contains("Deployed api-gateway"), "a press on the body keeps the toast");
        assert!(!h.screen().contains("Deploy log of api-gateway"), "and does nothing by default");
        let (x, y) = h.find("×").expect("close mark");
        h.click(x, y);
        assert!(!h.screen().contains("Deployed api-gateway"), "the close mark dismisses");

        h.send(send(Msg::Pressable(true)));
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "on_press = true");
        h.click_text("Deploy finished");
        h.click_text("Deployed api-gateway");
        assert!(h.screen().contains("Deploy log of api-gateway"), "{}", h.screen());
        assert!(h.screen().contains("Deployed api-gateway"), "the pressed toast stays");
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "on_press: open the deploy log");
    }

    #[test]
    fn upload_updates_one_toast_in_place() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("Upload step").click_text("Upload step");
        let screen = h.screen();
        assert!(screen.contains("Uploading backup 50%"), "{screen}");
        assert_eq!(screen.matches("Uploading backup").count(), 1, "{screen}");
    }

    #[test]
    fn the_upload_pulses_until_it_settles_into_success() {
        let mut h = showcase_on(PAGE);
        h.click_text("Upload step");
        h.advance(Duration::from_millis(300));
        let (x, y) = h.find("Uploading backup 25%").expect("running toast");
        let dot = h
            .screen()
            .lines()
            .nth(usize::try_from(y).unwrap_or(0))
            .and_then(|l| l.chars().nth(usize::try_from(x - 3).unwrap_or(0)));
        assert_eq!(dot, Some('●'), "the pulse plays in the icon cell:\n{}", h.screen());
        h.click_text("Upload step").click_text("Upload step").click_text("Upload step");
        h.advance(Duration::from_millis(300));
        let (x, y) = h.find("Backup uploaded").expect("settled toast");
        let icon = h
            .screen()
            .lines()
            .nth(usize::try_from(y).unwrap_or(0))
            .and_then(|l| l.chars().nth(usize::try_from(x - 3).unwrap_or(0)));
        assert_eq!(icon, Some('✓'), "{}", h.screen());
        assert_eq!(h.screen().matches("Backup uploaded").count(), 1);
        assert!(!h.screen().contains("Uploading backup"), "replaced in place, not stacked");

        h.send(send(Msg::Motion(0)));
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "icon motion = none");
        h.click_text("Upload step");
        h.advance(Duration::from_millis(300));
        let (x, y) = h.find("Uploading backup 25%").expect("running toast");
        let icon = h
            .screen()
            .lines()
            .nth(usize::try_from(y).unwrap_or(0))
            .and_then(|l| l.chars().nth(usize::try_from(x - 3).unwrap_or(0)));
        assert_eq!(icon, Some('ℹ'), "without animation the kind icon shows");
    }
}
