//! Style rule selectors: `widget`, `widget.variant`, `widget:state`, `widget.variant:state:state`.

use std::fmt;

/// An interaction state a style rule can target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum State {
    /// The pointer is over the widget.
    Hover,
    /// The widget has keyboard focus.
    Focus,
    /// The widget is the current choice, e.g. the open tab.
    Active,
    /// The widget is being pressed right now (the confirmation flash).
    Pressed,
    /// The widget cannot be used.
    Disabled,
    /// The row or item is selected.
    Selected,
    /// A checkbox, switch or radio is on.
    Checked,
    /// The value failed validation.
    Invalid,
}

impl State {
    /// Every state, in declaration order.
    pub const ALL: [Self; 8] = [
        Self::Hover,
        Self::Focus,
        Self::Active,
        Self::Pressed,
        Self::Disabled,
        Self::Selected,
        Self::Checked,
        Self::Invalid,
    ];

    /// The name used in theme files.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Hover => "hover",
            Self::Focus => "focus",
            Self::Active => "active",
            Self::Pressed => "pressed",
            Self::Disabled => "disabled",
            Self::Selected => "selected",
            Self::Checked => "checked",
            Self::Invalid => "invalid",
        }
    }

    /// Looks a state up by its theme file name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.name() == name)
    }
}

/// Which widgets, variants and states a style rule applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    widget: String,
    variant: Option<String>,
    states: Vec<State>,
}

impl Selector {
    /// Parses a selector such as `button.primary:hover`.
    ///
    /// # Errors
    ///
    /// Returns a message when a name is empty or uses characters other than `a-z`, `0-9`
    /// and `-`, or when a state is unknown.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut parts = text.split(':');
        let head = parts.next().unwrap_or_default();
        let (widget, variant) = match head.split_once('.') {
            Some((widget, variant)) => (widget, Some(variant)),
            None => (head, None),
        };
        check_name(widget, "widget", text)?;
        if let Some(variant) = variant {
            check_name(variant, "variant", text)?;
        }
        let mut states = parts
            .map(|name| {
                State::from_name(name).ok_or_else(|| {
                    let known: Vec<&str> = State::ALL.iter().map(|s| s.name()).collect();
                    format!("unknown state `:{name}` in `{text}`; known states: {}", known.join(", "))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        states.sort();
        states.dedup();
        Ok(Self { widget: widget.to_owned(), variant: variant.map(str::to_owned), states })
    }

    /// The widget name.
    #[must_use]
    pub fn widget(&self) -> &str {
        &self.widget
    }

    /// The variant name, if the selector names one.
    #[must_use]
    pub fn variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }

    /// The states the selector requires, sorted.
    #[must_use]
    pub fn states(&self) -> &[State] {
        &self.states
    }

    /// Higher wins: states count first, then whether a variant is named.
    #[must_use]
    pub fn specificity(&self) -> (usize, bool) {
        (self.states.len(), self.variant.is_some())
    }

    /// Whether a widget drawn with `variant` in `states` is targeted by this selector.
    #[must_use]
    pub fn matches(&self, widget: &str, variant: Option<&str>, states: &[State]) -> bool {
        self.widget == widget
            && self.variant.as_deref().is_none_or(|v| Some(v) == variant)
            && self.states.iter().all(|state| states.contains(state))
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.widget)?;
        if let Some(variant) = &self.variant {
            write!(f, ".{variant}")?;
        }
        for state in &self.states {
            write!(f, ":{}", state.name())?;
        }
        Ok(())
    }
}

fn check_name(name: &str, what: &str, selector: &str) -> Result<(), String> {
    let starts_with_letter = name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    let valid_chars = name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if starts_with_letter && valid_chars {
        Ok(())
    } else {
        Err(format!(
            "invalid {what} name `{name}` in `{selector}`; use lowercase letters, digits and `-`, starting with a letter"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_parts() {
        let selector = Selector::parse("button.primary:hover:focus").expect("valid");
        assert_eq!(selector.widget(), "button");
        assert_eq!(selector.variant(), Some("primary"));
        assert_eq!(selector.states(), &[State::Hover, State::Focus]);
        assert_eq!(selector.to_string(), "button.primary:hover:focus");
    }

    #[test]
    fn rejects_bad_names_and_states() {
        assert!(Selector::parse("list.item.selected").is_err());
        assert!(Selector::parse("Button").is_err());
        assert!(Selector::parse("button:glow").is_err());
        assert!(Selector::parse("").is_err());
        assert!(Selector::parse("list-item:selected").is_ok());
    }

    #[test]
    fn specificity_orders_base_variant_state_both() {
        let spec = |s: &str| Selector::parse(s).expect("valid").specificity();
        assert!(spec("button") < spec("button.primary"));
        assert!(spec("button.primary") < spec("button:hover"));
        assert!(spec("button:hover") < spec("button.primary:hover"));
    }

    #[test]
    fn matching_requires_all_states() {
        let selector = Selector::parse("button.primary:hover").expect("valid");
        assert!(selector.matches("button", Some("primary"), &[State::Hover, State::Focus]));
        assert!(!selector.matches("button", Some("primary"), &[State::Focus]));
        assert!(!selector.matches("button", None, &[State::Hover]));
        let base = Selector::parse("button").expect("valid");
        assert!(base.matches("button", Some("danger"), &[]));
        assert!(!base.matches("switch", None, &[]));
    }
}
