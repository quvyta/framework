//! Validation results an application keeps for its form.

use crate::runtime::Command;

/// The problems of a form, in the order the application found them, each under the name of the
/// control it belongs to.
///
/// Validation stays in the application: it checks its values and records a message for every
/// problem. Use the same name for the error and the control's [`NodeMut::id`](crate::widget::NodeMut::id),
/// so [`FormErrors::focus_first`] can take the user to the first problem.
///
/// ```
/// use qframe::widgets::FormErrors;
///
/// let name = "ab";
/// let mut errors = FormErrors::new();
/// errors.check("name", name.chars().count() >= 3, "Use at least 3 characters");
/// assert_eq!(errors.get("name"), Some("Use at least 3 characters"));
/// assert_eq!(errors.first(), Some("name"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormErrors {
    entries: Vec<(String, String)>,
}

impl FormErrors {
    /// No problems.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `message` for control `name`, replacing an earlier message for it.
    pub fn set(&mut self, name: impl Into<String>, message: impl Into<String>) {
        let (name, message) = (name.into(), message.into());
        match self.entries.iter_mut().find(|(existing, _)| *existing == name) {
            Some(entry) => entry.1 = message,
            None => self.entries.push((name, message)),
        }
    }

    /// Records `message` for `name` when `valid` is false and forgets any problem of `name`
    /// when it is true.
    pub fn check(&mut self, name: impl Into<String>, valid: bool, message: impl Into<String>) {
        let name = name.into();
        if valid {
            self.remove(&name);
        } else {
            self.set(name, message);
        }
    }

    /// Forgets the problem of `name`.
    pub fn remove(&mut self, name: &str) {
        self.entries.retain(|(existing, _)| existing != name);
    }

    /// Forgets every problem.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The message for `name`, if it has a problem.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries.iter().find(|(existing, _)| existing == name).map(|(_, message)| message.as_str())
    }

    /// Whether `name` has a problem.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Whether there are no problems.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many problems there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The name of the first problem.
    #[must_use]
    pub fn first(&self) -> Option<&str> {
        self.entries.first().map(|(name, _)| name.as_str())
    }

    /// Every problem as `(name, message)`, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(name, message)| (name.as_str(), message.as_str()))
    }

    /// Moves focus to the control of the first problem; nothing when there are none.
    #[must_use]
    pub fn focus_first<Msg: Send + 'static>(&self) -> Command<Msg> {
        self.first().map_or_else(Command::none, Command::focus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_order_replaces_and_forgets() {
        let mut errors = FormErrors::new();
        errors.set("name", "too short");
        errors.set("engine", "choose one");
        errors.set("name", "taken");
        assert_eq!(errors.iter().collect::<Vec<_>>(), vec![("name", "taken"), ("engine", "choose one")]);
        errors.check("name", true, "unused");
        assert_eq!(errors.first(), Some("engine"));
        assert_eq!(errors.len(), 1);
        errors.clear();
        assert!(errors.is_empty() && !errors.has("engine"));
    }
}
