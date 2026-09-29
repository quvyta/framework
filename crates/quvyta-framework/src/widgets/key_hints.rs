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
/// Style keys: `key-hints` (`bg`, `padding`), `key-hint-key`, `key-hint-label`, and
/// `key-hint-key.faint`, `key-hint-label.faint` for a [faint](Self::faint) bar.
#[derive(Debug, Clone, Default)]
pub struct KeyHints {
    left: Vec<Hint>,
    actions: Vec<Action>,
    faint: bool,
}

/// A keymap action on the bar: where it goes and, when the application names it, its label.
#[derive(Debug, Clone)]
struct Action {
    scope: Scope,
    name: String,
    place: Place,
    label: Option<String>,
}

/// Where an action's hint sits on the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// Before the plain hints, the last to drop.
    First,
    /// After the plain hints.
    Left,
    /// In the right group.
    Right,
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
    pub fn action(self, scope: Scope, action: impl Into<String>) -> Self {
        self.with_action(scope, action.into(), Place::Left, None)
    }

    /// Adds a keymap action before every plain [`hint`](Self::hint), resolved like
    /// [`action`](Self::action). When the bar is too narrow it is the last of the left group to
    /// drop, for the one key a screen cannot do without, such as the key that brings a closed
    /// panel back.
    #[must_use]
    pub fn action_first(self, scope: Scope, action: impl Into<String>) -> Self {
        self.with_action(scope, action.into(), Place::First, None)
    }

    /// Adds a keymap action on the left with the application's own `label`: the key comes from
    /// the keymap, so it follows the person's bindings, and the words from the application, for
    /// a key whose meaning changes with the screen, such as `enter` saying "install" or "open".
    /// Like [`action`](Self::action), an action without a chord draws nothing.
    #[must_use]
    pub fn action_labelled(self, scope: Scope, action: impl Into<String>, label: impl Into<String>) -> Self {
        self.with_action(scope, action.into(), Place::Left, Some(label.into()))
    }

    /// Adds a keymap action on the right.
    #[must_use]
    pub fn action_right(self, scope: Scope, action: impl Into<String>) -> Self {
        self.with_action(scope, action.into(), Place::Right, None)
    }

    /// Draws the whole bar in the faint tone, keys and labels alike, for a screen that has gone
    /// quiet, as [`Breadcrumb::faint`](super::Breadcrumb::faint) does for a path.
    #[must_use]
    pub fn faint(mut self, faint: bool) -> Self {
        self.faint = faint;
        self
    }

    fn with_action(mut self, scope: Scope, name: String, place: Place, label: Option<String>) -> Self {
        self.actions.push(Action { scope, name, place, label });
        self
    }

    fn resolved(&self, cx: &PaintCx<'_>) -> (Vec<Hint>, Vec<Hint>) {
        let mut first = Vec::new();
        let mut left = self.left.clone();
        let mut right = Vec::new();
        for action in &self.actions {
            let Some(key) = cx.env().keymap().label_for(action.scope, &action.name) else {
                continue;
            };
            let label = action
                .label
                .clone()
                .unwrap_or_else(|| cx.env().i18n().translate(&action.scope.label_key(&action.name), &[]));
            match action.place {
                Place::First => first.push((key, label)),
                Place::Left => left.push((key, label)),
                Place::Right => right.push((key, label)),
            }
        }
        first.append(&mut left);
        (first, right)
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
        let variant = self.faint.then_some("faint");
        let key_style = cx.style("key-hint-key", variant, &[]).text();
        let label_style = cx.style("key-hint-label", variant, &[]).text();
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

    /// A bar with a plain hint, an action that must stay and an action named by the application.
    struct Ordered {
        faint: bool,
    }

    impl App for Ordered {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(
                KeyHints::new()
                    .hint("↑↓", "move")
                    .action_labelled(Scope::Global, "focus-next", "install")
                    .action_first(Scope::Global, "quit")
                    .faint(self.faint),
            )
            .fill_width();
        }
    }

    #[test]
    fn a_first_action_comes_before_the_plain_hints_and_is_the_last_to_drop() {
        let wide = Harness::new(Ordered { faint: false }, 60, 1);
        assert_eq!(wide.screen(), "   ctrl q  quit    ↑↓  move    tab  install\n");
        let narrow = Harness::new(Ordered { faint: false }, 20, 1);
        assert_eq!(narrow.screen(), "   ctrl q  quit\n", "the others drop first");
    }

    #[test]
    fn a_labelled_and_a_first_action_follow_the_persons_keymap() {
        let mut env = crate::env::Env::builtin();
        env.keymap_mut().bind(Scope::Global, "quit", &["ctrl+x".parse().expect("chord")]);
        env.keymap_mut().bind(Scope::Global, "focus-next", &["f6".parse().expect("chord")]);
        let h = Harness::with_env(Ordered { faint: false }, env, 60, 1);
        assert_eq!(h.screen(), "   ctrl x  quit    ↑↓  move    f6  install\n");
    }

    #[test]
    fn a_faint_bar_draws_its_keys_and_labels_quieter() {
        let loud = Harness::new(Ordered { faint: false }, 60, 1);
        let quiet = Harness::new(Ordered { faint: true }, 60, 1);
        assert_eq!(loud.screen(), quiet.screen(), "the same hints");
        assert_ne!(loud.fg(4, 0), quiet.fg(4, 0), "the key steps back");
        assert_ne!(loud.bg(4, 0), quiet.bg(4, 0));
        assert_ne!(loud.fg(11, 0), quiet.fg(11, 0), "and so does its label");
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
