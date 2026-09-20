//! What an application says about the look of one row of a [`FileManager`](super::FileManager).
//!
//! The manager knows names and folders; what an entry *means* to the application it cannot know.
//! A mark is how the application says it: a sign with a tone, a faint row, or both.

/// How one row of a file manager looks, beyond what the manager itself knows.
///
/// An application marks a row to say something about the entry the manager cannot know: that a
/// backup leaves it out, that it is ignored by a version control system, that it has not been
/// saved. Give one with [`FileManager::row_mark`](super::FileManager::row_mark).
///
/// A tone never comes on its own: [`sign`](Self::sign) takes the icon and the colour together, so
/// a marked row is still told apart where colours are off or cannot be told apart.
///
/// ```
/// use qframe::widgets::RowMark;
///
/// // An entry the backup leaves out: a warning sign, and the row faint.
/// let left_out = RowMark::new().sign("warning", "warning").faint(true);
/// assert_eq!(left_out.icon(), Some("warning"));
/// assert_eq!(left_out.tone(), Some("warning"));
/// assert!(left_out.is_faint());
///
/// // Faintness alone needs no sign: nothing is being said in colour.
/// let quiet = RowMark::new().faint(true);
/// assert_eq!(quiet.icon(), None);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowMark {
    icon: Option<String>,
    tone: Option<String>,
    faint: bool,
}

impl RowMark {
    /// A mark that says nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The row's icon becomes `icon` in the colour `tone`, both of the icon set and the theme:
    /// `"warning"`, `"danger"`, `"success"` and the rest of the theme's own tokens.
    ///
    /// The two come together on purpose: a colour alone says nothing to a person who cannot tell
    /// it from another, and says nothing at all in the sixteen-colour or the ASCII mode.
    #[must_use]
    pub fn sign(mut self, icon: impl Into<String>, tone: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self.tone = Some(tone.into());
        self
    }

    /// Draws the row faint, the way a cut entry is drawn: there, but not what the eye goes to.
    #[must_use]
    pub fn faint(mut self, faint: bool) -> Self {
        self.faint = faint;
        self
    }

    /// The icon of the mark's sign, when it has one.
    #[must_use]
    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    /// The colour of the mark's sign, when it has one.
    #[must_use]
    pub fn tone(&self) -> Option<&str> {
        self.tone.as_deref()
    }

    /// Whether the row is drawn faint.
    #[must_use]
    pub fn is_faint(&self) -> bool {
        self.faint
    }

    /// Whether the mark says nothing at all, so a row that has one looks like a row that has none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.icon.is_none() && self.tone.is_none() && !self.faint
    }
}
