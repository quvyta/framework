//! Page navigation as a stack.

/// Which way the last navigation went, for transitions that follow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Navigation {
    /// Deeper: [`Router::push`] or [`Router::replace`].
    #[default]
    Forward,
    /// Back: [`Router::back`].
    Back,
}

/// The pages an application has visited, most recent last.
///
/// Draw [`Router::current`] inside [`View::page`](crate::widget::View::page) with a key per page
/// so every page keeps its own focus and scroll position when the user comes back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Router<P> {
    stack: Vec<P>,
    direction: Navigation,
}

impl<P: Clone + PartialEq> Router<P> {
    /// A router starting at `root`.
    #[must_use]
    pub fn new(root: P) -> Self {
        Self { stack: vec![root], direction: Navigation::Forward }
    }

    /// The page on top.
    #[must_use]
    pub fn current(&self) -> &P {
        self.stack.last().expect("the stack always holds the root page")
    }

    /// Opens `page` on top. Opening the current page again does nothing.
    pub fn push(&mut self, page: P) {
        if *self.current() != page {
            self.stack.push(page);
            self.direction = Navigation::Forward;
        }
    }

    /// Goes back one page. Returns `false` at the root.
    pub fn back(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.direction = Navigation::Back;
            true
        } else {
            false
        }
    }

    /// Replaces the current page without adding history.
    pub fn replace(&mut self, page: P) {
        if let Some(top) = self.stack.last_mut() {
            *top = page;
            self.direction = Navigation::Forward;
        }
    }

    /// Whether [`Router::back`] would go anywhere.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    /// Which way the last successful navigation went; `Forward` before any.
    #[must_use]
    pub fn direction(&self) -> Navigation {
        self.direction
    }

    /// Visited pages, root first.
    #[must_use]
    pub fn history(&self) -> &[P] {
        &self.stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_back_and_replace() {
        let mut router = Router::new("home");
        router.push("button");
        router.push("button");
        router.push("list");
        assert_eq!(router.history(), &["home", "button", "list"]);
        assert_eq!(router.direction(), Navigation::Forward);
        assert!(router.back());
        assert_eq!(router.direction(), Navigation::Back);
        assert_eq!(*router.current(), "button");
        router.replace("tabs");
        assert_eq!(router.history(), &["home", "tabs"]);
        assert!(router.back());
        assert!(!router.back());
        assert!(!router.can_go_back());
    }
}
