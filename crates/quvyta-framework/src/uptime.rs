//! Two monotonic clocks read together, so an application can tell time spent working from time
//! the machine spent asleep.
//!
//! A wall clock cannot answer this: a time-synchronisation step moves it while nobody slept, and
//! on the platforms where the monotonic clock keeps running through a suspend the wall clock
//! moves by exactly the same amount, so their difference is zero. The answer is the difference
//! between two *monotonic* clocks, one that stops while the machine sleeps and one that does not.
//!
//! | Platform | Stops while asleep | Keeps counting |
//! |---|---|---|
//! | Linux, Android | `CLOCK_MONOTONIC` | `CLOCK_BOOTTIME` |
//!
//! Only that pair is read here. On every other platform there is one clock, [`Uptime::elapsed`]
//! equals [`Uptime::awake`], [`Uptime::suspended_since`] is always zero and
//! [`Uptime::detects_suspend`] returns `false`, so an application can say "I cannot tell sleep
//! apart on this system" instead of showing a number that is made up:
//!
//! - **macOS** has `CLOCK_UPTIME_RAW` and `CLOCK_MONOTONIC_RAW`, the pair that would answer this,
//!   but reading them needs a foreign function call, and the framework forbids `unsafe`. The
//!   dependency that provides the clocks safely ([`rustix`](https://docs.rs/rustix)) exposes no
//!   Apple-only clock id, so the pair cannot be read from safe code today.
//! - **Windows** has `QueryUnbiasedInterruptTime` and `QueryInterruptTime`, which again need a
//!   foreign function call, and `rustix` is Unix only. There, `awake` is measured from the first
//!   reading in the process rather than from boot.

use std::time::Duration;

/// A moment read on both monotonic clocks at once.
///
/// The fields are counted from a fixed point in the past — the machine's boot where the platform
/// offers it, otherwise the first reading in this process — so a single `Uptime` is only
/// meaningful next to another one. Keep the reading you started from and compare.
///
/// ```
/// use qframe::uptime::Uptime;
///
/// let started = Uptime::now();
/// // ... the application runs, and the machine may be suspended in between ...
/// let now = Uptime::now();
/// let worked = now.awake.saturating_sub(started.awake);
/// let asleep = now.suspended_since(&started);
/// assert_eq!(asleep, std::time::Duration::ZERO, "nothing slept in a test");
/// assert!(worked < std::time::Duration::from_secs(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uptime {
    /// Time counted by the clock that stops while the machine is asleep.
    pub awake: Duration,
    /// Time counted by the clock that keeps going while the machine is asleep. Equal to `awake`
    /// on a platform with no second clock; see [`Uptime::detects_suspend`].
    pub elapsed: Duration,
}

impl Uptime {
    /// Reads both clocks now.
    #[must_use]
    pub fn now() -> Self {
        let (awake, elapsed) = platform::read();
        Self { awake, elapsed }
    }

    /// How long the machine was suspended between `earlier` and this reading.
    ///
    /// Zero when the readings are in the wrong order, and zero on a platform that cannot tell
    /// sleep apart.
    #[must_use]
    pub fn suspended_since(&self, earlier: &Self) -> Duration {
        let elapsed = self.elapsed.saturating_sub(earlier.elapsed);
        let awake = self.awake.saturating_sub(earlier.awake);
        elapsed.saturating_sub(awake)
    }

    /// Whether this platform has a second clock, so [`Uptime::suspended_since`] can report sleep
    /// at all. `true` on Linux and Android, `false` everywhere else.
    #[must_use]
    pub fn detects_suspend() -> bool {
        platform::DETECTS_SUSPEND
    }
}

/// The clocks Linux and Android offer: `CLOCK_MONOTONIC` stops while the machine sleeps,
/// `CLOCK_BOOTTIME` does not. Both are read with `clock_gettime`, which the kernel always
/// supports for these two ids.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod platform {
    use std::time::Duration;

    use rustix::time::{ClockId, Timespec, clock_gettime};

    pub const DETECTS_SUSPEND: bool = true;

    pub fn read() -> (Duration, Duration) {
        (duration(clock_gettime(ClockId::Monotonic)), duration(clock_gettime(ClockId::Boottime)))
    }

    /// A clock reading as a `Duration`; a negative reading, which these clocks never give, counts
    /// as zero rather than wrapping around.
    fn duration(time: Timespec) -> Duration {
        let seconds = u64::try_from(time.tv_sec).unwrap_or(0);
        let nanoseconds = u32::try_from(time.tv_nsec).unwrap_or(0).min(999_999_999);
        Duration::new(seconds, nanoseconds)
    }
}

/// Other Unix systems: `CLOCK_MONOTONIC` alone. macOS's `CLOCK_UPTIME_RAW` and the BSD uptime
/// clocks are not reachable from safe code with the dependency in hand, so sleep is not reported.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
mod platform {
    use std::time::Duration;

    use rustix::time::{ClockId, clock_gettime};

    pub const DETECTS_SUSPEND: bool = false;

    pub fn read() -> (Duration, Duration) {
        let time = clock_gettime(ClockId::Monotonic);
        let seconds = u64::try_from(time.tv_sec).unwrap_or(0);
        let nanoseconds = u32::try_from(time.tv_nsec).unwrap_or(0).min(999_999_999);
        let awake = Duration::new(seconds, nanoseconds);
        (awake, awake)
    }
}

/// Everything else, Windows among it: `Instant` measured from the first reading in this process.
/// Its behaviour across a suspend is not specified, so no sleep is reported.
#[cfg(not(unix))]
mod platform {
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};

    pub const DETECTS_SUSPEND: bool = false;

    pub fn read() -> (Duration, Duration) {
        static START: OnceLock<Instant> = OnceLock::new();
        let awake = START.get_or_init(Instant::now).elapsed();
        (awake, awake)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(awake: u64, elapsed: u64) -> Uptime {
        Uptime { awake: Duration::from_secs(awake), elapsed: Duration::from_secs(elapsed) }
    }

    #[test]
    fn suspended_time_is_what_one_clock_counted_and_the_other_did_not() {
        let started = reading(100, 100);
        // Ten seconds of work around an hour of sleep.
        let now = reading(110, 3_710);
        assert_eq!(now.suspended_since(&started), Duration::from_secs(3_600));
        assert_eq!(now.awake - started.awake, Duration::from_secs(10));
    }

    #[test]
    fn a_platform_with_one_clock_reports_no_sleep() {
        let started = reading(100, 100);
        let now = reading(3_700, 3_700);
        assert_eq!(now.suspended_since(&started), Duration::ZERO);
    }

    #[test]
    fn readings_in_the_wrong_order_report_no_sleep() {
        let started = reading(3_700, 3_700);
        assert_eq!(reading(100, 100).suspended_since(&started), Duration::ZERO);
        // A second clock that fell behind the first cannot make the sleep negative either.
        assert_eq!(reading(200, 150).suspended_since(&reading(100, 100)), Duration::ZERO);
    }

    #[test]
    fn the_clocks_move_forward_together_and_report_the_platform_honestly() {
        let started = Uptime::now();
        let mut later = Uptime::now();
        for _ in 0..2_000 {
            later = Uptime::now();
        }
        assert!(later.awake >= started.awake, "{started:?} {later:?}");
        assert!(later.elapsed >= started.elapsed, "{started:?} {later:?}");
        assert!(later.elapsed >= later.awake, "the clock that counts sleep cannot be behind: {later:?}");
        assert_eq!(later.suspended_since(&started), Duration::ZERO, "no test sleeps the machine");
        assert!(later.awake > Duration::ZERO, "the clock is running: {later:?}");
        let linux = cfg!(any(target_os = "linux", target_os = "android"));
        assert_eq!(Uptime::detects_suspend(), linux);
        if !linux {
            assert_eq!(later.elapsed, later.awake, "one clock means the two readings are the same");
        }
    }
}
