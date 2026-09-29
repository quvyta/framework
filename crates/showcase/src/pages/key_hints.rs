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
        ui.add(Text::new(t!("key-hints.first")).role("faint"));
        for faint in [false, true] {
            // region: first-and-faint
            ui.add(
                KeyHints::new()
                    .action_first(Scope::Global, "quit")
                    .hint("↑↓", t!("hints.move"))
                    .action_labelled(Scope::App, "search", t!("key-hints.find"))
                    .faint(faint),
            )
            .width(Length::Cells(60));
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

    #[test]
    fn the_first_action_leads_and_the_labelled_one_speaks_for_the_screen() {
        let h = showcase_on("key-hints");
        let screen = h.screen();
        let line = screen.lines().find(|line| line.contains("find files")).unwrap_or_else(|| panic!("{screen}"));
        let (quit, find) = (line.find("quit"), line.find("find files"));
        assert!(quit < find && quit.is_some(), "quit first: {line}");
    }
}
