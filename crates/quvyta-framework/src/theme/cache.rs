//! Remembering layered styles for the lifetime of a theme.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use super::selector::State;
use super::style::StyleProps;

/// The styles a theme has already layered, by widget, variant and states.
///
/// A theme never changes once resolved, so a layered style stays valid for as long as the theme
/// lives, and clones of a theme share one cache. Widgets ask for the same few styles every
/// frame; layering one means matching every rule of the theme, which a lookup here avoids.
#[derive(Default, Clone)]
pub(super) struct StyleCache {
    entries: Arc<Mutex<HashMap<String, Vec<Entry>>>>,
}

/// One layered style of a widget.
struct Entry {
    variant: Option<String>,
    states: StateSet,
    props: StyleProps,
}

/// A set of states as bits: selectors match on which states are present, so the order and
/// repetition of a state list do not change the style.
type StateSet = u8;

fn state_set(states: &[State]) -> StateSet {
    states.iter().fold(0, |set, state| set | 1 << *state as u8)
}

impl StyleCache {
    /// The style of `widget` with `variant` in `states`, layered by `layer` the first time.
    pub(super) fn get_or_insert(
        &self,
        widget: &str,
        variant: Option<&str>,
        states: &[State],
        layer: impl FnOnce() -> StyleProps,
    ) -> StyleProps {
        let set = state_set(states);
        let find = |entries: &[Entry]| {
            entries
                .iter()
                .find(|entry| entry.states == set && entry.variant.as_deref() == variant)
                .map(|entry| entry.props.clone())
        };
        // Entries are only ever pushed whole, so a cache poisoned by a panic elsewhere is still
        // correct.
        let remembered =
            self.entries.lock().unwrap_or_else(PoisonError::into_inner).get(widget).and_then(|list| find(list));
        if let Some(props) = remembered {
            return props;
        }
        let props = layer();
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let list = entries.entry(widget.to_owned()).or_default();
        if find(list).is_none() {
            list.push(Entry { variant: variant.map(str::to_owned), states: set, props: props.clone() });
        }
        props
    }
}

/// Two themes with the same rules style alike, whatever either has remembered so far.
impl PartialEq for StyleCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl fmt::Debug for StyleCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StyleCache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeRegistry;

    #[test]
    fn remembered_styles_equal_freshly_layered_ones() {
        let registry = ThemeRegistry::builtin();
        for id in ["monochrome", "iris", "nordic", "amber"] {
            let (theme, _) = registry.resolve_or_default(id);
            let asks: [(&str, Option<&str>, &[State]); 5] = [
                ("button", Some("primary"), &[State::Hover, State::Focus]),
                ("button", Some("primary"), &[State::Focus, State::Hover, State::Hover]),
                ("button", None, &[State::Hover]),
                ("list-item", None, &[State::Selected, State::Focus]),
                ("no-such-widget", Some("x"), &[]),
            ];
            for (widget, variant, states) in asks {
                let fresh = theme.layer_rules(widget, variant, states);
                assert_eq!(theme.style(widget, variant, states), fresh, "{id} {widget} first ask");
                assert_eq!(theme.style(widget, variant, states), fresh, "{id} {widget} remembered");
                assert_eq!(theme.clone().style(widget, variant, states), fresh, "{id} {widget} in a clone");
            }
        }
    }

    #[test]
    fn state_order_and_repeats_do_not_matter() {
        assert_eq!(state_set(&[State::Hover, State::Focus]), state_set(&[State::Focus, State::Hover, State::Focus]));
        assert_ne!(state_set(&[State::Hover]), state_set(&[State::Focus]));
        assert_eq!(state_set(&[]), 0);
    }
}
