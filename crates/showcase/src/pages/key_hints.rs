//! Key hints: bars built from the keymap and from screen-specific hints.

use qframe::keymap::Scope;
use qframe::prelude::*;

use crate::app::Msg as AppMsg;

/// The live demo.
pub fn view(ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("key-hints.hint")).role("secondary"));
        for width in [96u16, 64, 40] {
            ui.add(Text::new(t!("key-hints.width", n = width)).role("faint"));
            // region: bar
            ui.add(
                KeyHints::new()
                    .hint("↑↓", t!("hints.move"))
                    .hint("enter", t!("key-hints.open"))
                    .action(Scope::App, "search")
                    .action(Scope::Global, "focus-next")
                    .action_right(Scope::Global, "quit"),
            )
            .width(Length::Cells(width));
            // endregion
        }
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use crate::tests::showcase_on;

    #[test]
    fn narrow_bars_drop_hints_but_keep_the_right_side() {
        let h = showcase_on("key-hints");
        let screen = h.screen();
        assert!(screen.matches("ctrl q").count() >= 4, "{screen}");
    }
}
