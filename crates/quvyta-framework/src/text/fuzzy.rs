//! Fuzzy matching: finding text by a few of its letters, in order.

/// A fuzzy match: how good it is and which characters of the text matched. Made by [`fuzzy`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    pub(crate) score: i32,
    /// Character indices of the text, ascending.
    pub(crate) positions: Vec<usize>,
}

impl FuzzyMatch {
    /// How good the match is: higher is better. Only the order of scores for one query means
    /// anything; an empty query scores 0.
    #[must_use]
    pub fn score(&self) -> i32 {
        self.score
    }

    /// The indices of the matched characters of the text, counted in `char`s, ascending: what a
    /// list draws in the match colour.
    #[must_use]
    pub fn positions(&self) -> &[usize] {
        &self.positions
    }
}

/// Matches `query` against `text`, ignoring case and spaces in the query: every query
/// character must appear in order. Consecutive characters and characters at word starts score
/// higher. An empty query matches everything with score 0.
///
/// This is the rule the framework's filter, command palette and pickers find with, so an
/// application's own search that uses it finds what they find.
///
/// ```
/// use qframe::text::fuzzy;
///
/// let found = fuzzy("rst", "Restart container").expect("every letter appears in order");
/// assert_eq!(found.positions(), &[0, 2, 3]);
/// assert!(fuzzy("tsr", "Restart").is_none());
/// let word_start = fuzzy("ol", "Open logs").expect("matches").score();
/// let inside = fuzzy("ol", "Pool").expect("matches").score();
/// assert!(word_start > inside);
/// ```
#[must_use]
pub fn fuzzy(query: &str, text: &str) -> Option<FuzzyMatch> {
    // Both sides keep the first character of a lower case form, so a letter such as `İ`, whose
    // lower case is two characters, compares equal to itself.
    let lower_char = |c: char| c.to_lowercase().next().unwrap_or(c);
    let wanted: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).map(lower_char).collect();
    if wanted.is_empty() {
        return Some(FuzzyMatch { score: 0, positions: Vec::new() });
    }
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars.iter().copied().map(lower_char).collect();
    let word_start = |i: usize| {
        i == 0 || !chars[i - 1].is_alphanumeric() || (chars[i].is_uppercase() && chars[i - 1].is_lowercase())
    };
    // Try every place the first character occurs and keep the best greedy alignment.
    let mut best: Option<FuzzyMatch> = None;
    for first in (0..lower.len()).filter(|i| lower[*i] == wanted[0]) {
        let mut positions = vec![first];
        let mut at = first;
        for c in &wanted[1..] {
            let Some(next) = (at + 1..lower.len()).find(|i| lower[*i] == *c) else {
                break;
            };
            positions.push(next);
            at = next;
        }
        if positions.len() < wanted.len() {
            break;
        }
        let mut score = 0;
        for (index, position) in positions.iter().enumerate() {
            score += 1;
            if word_start(*position) {
                score += 4;
            }
            if index > 0 && positions[index - 1] + 1 == *position {
                score += 3;
            }
        }
        score -= i32::try_from(positions[positions.len() - 1] - positions[0]).unwrap_or(i32::MAX / 2) / 4;
        score -= i32::try_from(first).unwrap_or(0).min(8) / 4;
        if best.as_ref().is_none_or(|b| score > b.score) {
            best = Some(FuzzyMatch { score, positions });
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_in_order_ignoring_case() {
        let found = fuzzy("rst", "Restart container").expect("matches");
        assert_eq!(found.positions, vec![0, 2, 3]);
        assert!(fuzzy("tsr", "Restart").is_none());
        assert_eq!(fuzzy("", "anything").map(|m| m.score), Some(0));
        assert_eq!(fuzzy("o l", "Open logs").map(|m| m.positions), Some(vec![0, 5]));
    }

    #[test]
    fn a_letter_whose_lower_case_is_two_characters_matches_itself() {
        // `İ` lowers to `i` and a combining dot; the text side keeps only the `i`, so the query
        // must do the same or Turkish names never match.
        assert_eq!(fuzzy("İz", "İzmir").map(|m| m.positions), Some(vec![0, 1]));
        assert_eq!(fuzzy("iz", "İzmir").map(|m| m.positions), Some(vec![0, 1]));
    }

    #[test]
    fn word_starts_and_runs_score_higher() {
        let starts = fuzzy("ol", "Open logs").expect("matches").score;
        let middle = fuzzy("ol", "Pool").expect("matches").score;
        assert!(starts > middle, "{starts} {middle}");
        let run = fuzzy("dep", "Deploy").expect("matches").score;
        let spread = fuzzy("dep", "Delete old snapshots and prune").expect("matches").score;
        assert!(run > spread);
        let better = fuzzy("log", "Toggle logs").expect("matches");
        assert_eq!(better.positions, vec![7, 8, 9], "the word start beats the first occurrence");
    }
}
