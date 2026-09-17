//! The line store behind [`LogView`](super::LogView): a ring buffer that is cheap to share
//! with the view every frame.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Lines per shared chunk. Pushing copies at most one chunk that the last frame still shares.
const CHUNK: usize = 256;

/// Source of buffer identities, so views can keep per-buffer caches.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// How important a log line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Very detailed tracing.
    Trace,
    /// Diagnostic detail.
    Debug,
    /// Normal operation.
    Info,
    /// Something unexpected that did not fail.
    Warn,
    /// A failure.
    Error,
}

impl LogLevel {
    /// Every level, least important first.
    pub const ALL: [Self; 5] = [Self::Trace, Self::Debug, Self::Info, Self::Warn, Self::Error];

    /// A short lowercase name: `trace`, `debug`, `info`, `warn`, `error`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One line of a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    level: LogLevel,
    time: Option<String>,
    text: String,
}

impl LogLine {
    /// A line with `level` and `text`.
    #[must_use]
    pub fn new(level: LogLevel, text: impl Into<String>) -> Self {
        Self { level, time: None, text: text.into() }
    }

    /// A timestamp drawn faint before the level, e.g. `"14:02:31.118"`.
    #[must_use]
    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.time = Some(time.into());
        self
    }

    /// The level.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// The timestamp, if any.
    #[must_use]
    pub fn timestamp(&self) -> Option<&str> {
        self.time.as_deref()
    }

    /// The message.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// A bounded store of log lines: when full, pushing drops the oldest line.
///
/// Cloning is cheap (lines live in shared chunks), so an application keeps one buffer in its
/// state, pushes lines in `update` and hands it to a [`LogView`](super::LogView) every frame.
/// Every line gets a number that never changes while it is in the buffer, which lets the view
/// keep its scroll position and selection while old lines fall out.
#[derive(Debug, Clone)]
pub struct LogBuffer {
    id: u64,
    capacity: usize,
    chunks: VecDeque<Arc<Vec<LogLine>>>,
    /// Lines already dropped from the front of the first chunk.
    skip: usize,
    len: usize,
    /// Lines removed from the buffer since it was created; the number of the first line.
    dropped: u64,
}

impl LogBuffer {
    /// An empty buffer keeping at most `capacity` lines (at least one).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            capacity: capacity.max(1),
            chunks: VecDeque::new(),
            skip: 0,
            len: 0,
            dropped: 0,
        }
    }

    /// Adds a line at the end, dropping the oldest line when full.
    pub fn push(&mut self, line: LogLine) {
        match self.chunks.back_mut() {
            Some(last) if last.len() < CHUNK => Arc::make_mut(last).push(line),
            _ => {
                let mut chunk = Vec::with_capacity(CHUNK);
                chunk.push(line);
                self.chunks.push_back(Arc::new(chunk));
            }
        }
        self.len += 1;
        while self.len > self.capacity {
            self.skip += 1;
            self.len -= 1;
            self.dropped += 1;
            if self.chunks.front().is_some_and(|first| self.skip >= first.len()) {
                self.chunks.pop_front();
                self.skip = 0;
            }
        }
    }

    /// Removes every line.
    pub fn clear(&mut self) {
        self.dropped += self.len as u64;
        self.chunks.clear();
        self.skip = 0;
        self.len = 0;
    }

    /// How many lines the buffer holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer holds no lines.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The most lines the buffer keeps.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The line at `index`, counted from the oldest line kept.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&LogLine> {
        if index >= self.len {
            return None;
        }
        let first = self.chunks.front()?.len() - self.skip;
        if index < first {
            return self.chunks[0].get(self.skip + index);
        }
        let rest = index - first;
        self.chunks.get(1 + rest / CHUNK)?.get(rest % CHUNK)
    }

    /// Every line, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &LogLine> {
        self.chunks.iter().enumerate().flat_map(|(i, chunk)| chunk[if i == 0 { self.skip } else { 0 }..].iter())
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// The permanent number of the oldest line kept.
    pub(crate) fn first_number(&self) -> u64 {
        self.dropped
    }

    /// The line with permanent number `number`, if it is still kept.
    pub(crate) fn by_number(&self, number: u64) -> Option<&LogLine> {
        let index = usize::try_from(number.checked_sub(self.dropped)?).ok()?;
        self.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize) -> LogLine {
        LogLine::new(LogLevel::Info, format!("line {n}"))
    }

    #[test]
    fn drops_oldest_lines_when_full_and_keeps_numbers() {
        let mut buffer = LogBuffer::new(600);
        for n in 0..1000 {
            buffer.push(line(n));
        }
        assert_eq!(buffer.len(), 600);
        assert_eq!(buffer.get(0).map(LogLine::text), Some("line 400"));
        assert_eq!(buffer.get(599).map(LogLine::text), Some("line 999"));
        assert_eq!(buffer.get(600), None);
        assert_eq!(buffer.first_number(), 400);
        assert_eq!(buffer.by_number(512).map(LogLine::text), Some("line 512"));
        assert_eq!(buffer.by_number(12), None);
        assert_eq!(buffer.iter().count(), 600);
        assert_eq!(buffer.iter().nth(300).map(LogLine::text), Some("line 700"));
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.first_number(), 1000);
    }

    #[test]
    fn clones_share_lines_until_one_changes() {
        let mut buffer = LogBuffer::new(10_000);
        for n in 0..CHUNK * 3 {
            buffer.push(line(n));
        }
        let shared = buffer.clone();
        buffer.push(line(9999));
        assert_eq!(shared.len(), CHUNK * 3);
        assert_eq!(buffer.len(), CHUNK * 3 + 1);
        assert!(Arc::ptr_eq(&shared.chunks[0], &buffer.chunks[0]), "full chunks stay shared");
        assert_eq!(shared.id(), buffer.id());
    }
}
