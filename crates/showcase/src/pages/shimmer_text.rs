//! Shimmer text: light passing over a working message, and typing dots.

use qframe::prelude::*;
use qframe::widgets::{ShimmerStyle, ShimmerText, Spinner};

use crate::app::Msg as AppMsg;

/// The live demo.
pub fn view(ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("shimmer-text.sweep")).gap(0), |ui| {
        ui.add(Text::new(t!("shimmer-text.sweep-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: sweep
        ui.add(ShimmerText::new(t!("shimmer-text.processing")));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("shimmer-text.dots")).gap(0), |ui| {
        ui.add(Text::new(t!("shimmer-text.dots-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: dots
        ui.add(ShimmerText::new(t!("shimmer-text.typing")).style(ShimmerStyle::Dots));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("shimmer-text.together")).gap(0), |ui| {
        ui.add(Text::new(t!("shimmer-text.together-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: compose
        ui.row(|ui| {
            ui.add(Spinner::new());
            ui.add(ShimmerText::new(t!("shimmer-text.thinking")));
            ui.add(Text::new(t!("shimmer-text.elapsed")).role("faint").no_wrap());
        })
        .gap(1);
        // endregion
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use crate::tests::showcase_on;

    #[test]
    fn working_messages_are_shown() {
        let h = showcase_on("shimmer-text");
        let screen = h.screen();
        assert!(screen.contains("Processing your request"), "{screen}");
        assert!(screen.contains("Thinking"), "{screen}");
    }
}
