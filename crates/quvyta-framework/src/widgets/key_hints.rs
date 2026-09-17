//! The key hint bar.

use crate::geometry::{Rect, Size};
use crate::keymap::Scope;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

use super::cells;

/// A bar of key hints such as `tab next  ctrl q quit`, fed from the keymap and from hints
/// the application adds for the current screen.
///
/// Keys sit on a raised surface and labels are faint; nothing is bracketed. When the bar is
/// too narrow, hints are dropped from the end of the left group first; the right group stays.
/// Style keys: `key-hints` (`bg`, `padding`), `key-hint-key`, `key-hint-label`.
#[derive(Debug, Clone, Default)]
pub struct KeyHints {
    left: Vec<Hint>,
    actions: Vec<(Scope, String, bool)>,
}

impl KeyHints {
    /// An empty bar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a hint on the left: `key` such as `"↑↓"` and its `label`.
    #[must_use]
    pub fn hint(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.left.push((key.into(), label.into()));
        self
    }

    /// Adds a keymap action on the left; its keys and translated label come from the keymap
    /// and the locale.
    #[must_use]
    pub fn action(mut self, scope: Scope, action: impl Into<String>) -> Self {
        self.actions.push((scope, action.into(), false));
        self
    }

    /// Adds a keymap action on the right.
    #[must_use]
    pub fn action_right(mut self, scope: Scope, action: impl Into<String>) -> Self {
        self.actions.push((scope, action.into(), true));
        self
    }

    fn resolved(&self, cx: &PaintCx<'_>) -> (Vec<Hint>, Vec<Hint>) {
        let mut left = self.left.clone();
        let mut right = Vec::new();
        for (scope, action, on_right) in &self.actions {
            let chords = cx.env().keymap().chords_for(*scope, action);
            let Some(chord) = chords.first() else {
                continue;
            };
            let label = cx.env().i18n().translate(&scope.label_key(action), &[]);
            let hint = (chord.label(), label);
            if *on_right {
                right.push(hint);
            } else {
                left.push(hint);
            }
        }
        (left, right)
    }
}

/// A key label and its description.
type Hint = (String, String);

/// Cells between two hints.
const SPACING: u16 = 3;

/// The padded key, a space and the label; saturating, as a hint may be wider than any screen.
fn hint_width(hint: &Hint) -> u16 {
    cells::sum([text::width(&hint.0), 2, 1, text::width(&hint.1)])
}

impl<Msg: 'static> Widget<Msg> for KeyHints {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(available.width, 1.min(available.height))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let bar = cx.style("key-hints", None, &[]);
        let background = bar.text().bg.unwrap_or_else(|| cx.color("surface"));
        cx.clear(area, background);
        let inner = area.inset(bar.padding());
        let key_style = cx.style("key-hint-key", None, &[]).text();
        let label_style = cx.style("key-hint-label", None, &[]).text();
        let (mut left, right) = self.resolved(cx);

        let group_width = |hints: &[Hint]| -> u16 {
            let count = u16::try_from(hints.len()).unwrap_or(u16::MAX);
            let hints = cells::sum(hints.iter().map(hint_width));
            hints.saturating_add(SPACING.saturating_mul(count.saturating_sub(1)))
        };
        let right_width = group_width(&right);
        let separation = if right.is_empty() { 0 } else { SPACING };
        let left_budget = inner.width.saturating_sub(right_width.saturating_add(separation));
        while group_width(&left) > left_budget {
            left.pop();
        }
        let draw = |cx: &mut PaintCx<'_>, mut x: i32, hints: &[Hint]| {
            for (key, label) in hints {
                let padded = format!(" {key} ");
                x += i32::from(cx.text(x, inner.y, &padded, key_style, text::width(&padded))) + 1;
                x += i32::from(cx.text(x, inner.y, label, label_style, text::width(label))) + i32::from(SPACING);
            }
        };
        draw(cx, inner.x, &left);
        draw(cx, inner.right() - i32::from(right_width), &right);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo;

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(
                KeyHints::new()
                    .hint("↑↓", "move")
                    .action(Scope::Global, "focus-next")
                    .action_right(Scope::Global, "quit"),
            )
            .fill_width();
        }
    }

    #[test]
    fn draws_hints_from_keymap_and_drops_what_does_not_fit() {
        let wide = Harness::new(Demo, 50, 1);
        assert_eq!(wide.screen(), "   ↑↓  move    tab  next            ctrl q  quit\n");
        let narrow = Harness::new(Demo, 30, 1);
        assert_eq!(narrow.screen(), "   ↑↓  move     ctrl q  quit\n");
    }

    #[test]
    fn labels_follow_the_language() {
        let mut h = Harness::new(Demo, 50, 1);
        h.set_locale("tr");
        assert!(h.screen().contains("ctrl q  çık"));
    }

    #[test]
    fn hints_wider_than_any_screen_are_dropped_without_overflowing() {
        struct Huge;

        impl App for Huge {
            type Msg = ();
            fn update(&mut self, (): ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                let long = "k".repeat(40_000);
                ui.add(KeyHints::new().hint(long.clone(), long.clone()).hint(long, "move")).fill_width();
            }
        }

        let h = Harness::new(Huge, 30, 1);
        assert_eq!(h.screen(), "\n");
    }
}
