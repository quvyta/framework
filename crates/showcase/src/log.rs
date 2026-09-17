//! The event log shown under every demo: what the demo's widgets sent, with a timestamp.

use std::collections::VecDeque;
use std::time::Instant;

/// How many entries are kept.
const CAPACITY: usize = 50;

/// One logged message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub page: &'static str,
    pub time: String,
    pub source: String,
    pub message: String,
}

/// Recent messages from demo widgets.
#[derive(Debug)]
pub struct EventLog {
    started: Instant,
    entries: VecDeque<Entry>,
}

impl EventLog {
    #[must_use]
    pub fn new() -> Self {
        Self { started: Instant::now(), entries: VecDeque::new() }
    }

    /// Records that `source` on `page` sent `message`.
    pub fn push(&mut self, page: &'static str, source: impl Into<String>, message: impl Into<String>) {
        let elapsed = self.started.elapsed();
        let time =
            format!("{:02}:{:02}.{:02}", elapsed.as_secs() / 60, elapsed.as_secs() % 60, elapsed.subsec_millis() / 10);
        self.entries.push_back(Entry { page, time, source: source.into(), message: message.into() });
        while self.entries.len() > CAPACITY {
            self.entries.pop_front();
        }
    }

    /// The last `count` entries of `page`, oldest first.
    #[must_use]
    pub fn recent(&self, page: &str, count: usize) -> Vec<&Entry> {
        let mut recent: Vec<&Entry> = self.entries.iter().rev().filter(|e| e.page == page).take(count).collect();
        recent.reverse();
        recent
    }
}
