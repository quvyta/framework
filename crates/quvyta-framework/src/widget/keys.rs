//! A key a widget takes, as the help layer lists it.

/// One row of the "This screen" group of a [`HelpLayer`](crate::widgets::HelpLayer): the keys a
/// widget takes and what they do, both ready to be drawn.
///
/// A widget builds these in [`Widget::keys`] from its own labels in the language files and the
/// glyphs of the icon set in use, so an arrow is an arrow in Unicode and a caret on a terminal
/// that can show nothing else.
///
/// [`Widget::keys`]: crate::widget::Widget::keys
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetKey {
    /// The keys, as they are drawn: `"↑↓"`, `"pgup pgdn"`, `"enter"`.
    pub keys: String,
    /// What they do, in the active language.
    pub label: String,
}

impl WidgetKey {
    /// A row for `keys` and what they do.
    #[must_use]
    pub fn new(keys: impl Into<String>, label: impl Into<String>) -> Self {
        Self { keys: keys.into(), label: label.into() }
    }
}
