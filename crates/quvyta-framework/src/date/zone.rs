//! The machine's offset from UTC, read out of the system time zone file.
//!
//! `std` knows nothing about time zones, and the platform's `localtime` needs a foreign function
//! call, which this framework forbids. What is left is the data the system already keeps: the
//! TZif file (RFC 8536) that `TZ` names or `/etc/localtime` points at. It lists every offset the
//! zone ever had with the instant each one starts, so the offset of any moment is a lookup, and
//! daylight saving comes out right without a rule engine.
//!
//! Limits, all of which end in "the offset is unknown" rather than a wrong number:
//!
//! - `TZ` holding a POSIX rule such as `EST5EDT,M3.2.0,M11.1.0` instead of a zone name is not
//!   parsed. Its offsets have the opposite sign of everything else and getting that wrong is
//!   worse than saying nothing.
//! - Moments after the last offset change the file records — the tables usually run to 2037 —
//!   keep the last recorded offset; the POSIX rule in the file's footer is not evaluated.
//! - Windows keeps no such file, so nothing is found there.
//! - The file is read once per process. A zone changed while an application runs is not noticed;
//!   a daylight-saving change is, because it is a transition inside the file already read.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where the offsets come from.
enum Source {
    /// UTC, with no file to read: `TZ` is set to the empty string.
    Utc,
    /// A TZif file.
    File(PathBuf),
}

/// Where zone files live when `TZDIR` does not say otherwise.
const ZONE_DIR: &str = "/usr/share/zoneinfo";

/// The file the system points at, as POSIX reads `TZ`: unset means the system zone, empty means
/// UTC, a leading colon is ignored, a path is taken as it is and a name is looked up in `TZDIR`.
/// A name that could climb out of the zone directory is refused.
fn source(tz: Option<&str>, tzdir: Option<&str>) -> Option<Source> {
    let Some(tz) = tz else {
        return Some(Source::File(PathBuf::from("/etc/localtime")));
    };
    let name = tz.strip_prefix(':').unwrap_or(tz);
    if name.is_empty() {
        return Some(Source::Utc);
    }
    if name.starts_with('/') {
        return Some(Source::File(PathBuf::from(name)));
    }
    let sane = name.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    sane.then(|| Source::File(PathBuf::from(tzdir.unwrap_or(ZONE_DIR)).join(name)))
}

/// What the system says, read once.
enum Zone {
    /// UTC, with no file to read.
    Utc,
    /// The bytes of a TZif file.
    Tzif(Vec<u8>),
    /// The system does not say: no file, or one that cannot be read.
    Unknown,
}

/// The system zone, read once per process.
fn zone() -> &'static Zone {
    static ZONE: OnceLock<Zone> = OnceLock::new();
    ZONE.get_or_init(|| {
        let tz = std::env::var("TZ").ok();
        let tzdir = std::env::var("TZDIR").ok();
        match source(tz.as_deref(), tzdir.as_deref()) {
            Some(Source::Utc) => Zone::Utc,
            Some(Source::File(path)) => load(&path),
            None => Zone::Unknown,
        }
    })
}

/// The largest zone file believed. Real ones are a few kilobytes; the largest with every
/// transition to 2037 stays well under this.
const MAX_ZONE_FILE: u64 = 256 * 1024;

/// The zone file at `path`, when it is a plain file of a believable size.
///
/// `TZ` can name any path: a directory, a device that never ends such as `/dev/zero`, a pipe
/// nobody writes to. Only a plain file is opened, and no more than [`MAX_ZONE_FILE`] bytes are
/// read even if it grows meanwhile, so none of them can hang the process or fill its memory.
fn load(path: &Path) -> Zone {
    use std::io::Read as _;

    let plain = std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_ZONE_FILE);
    if !plain {
        return Zone::Unknown;
    }
    let mut data = Vec::new();
    let read = std::fs::File::open(path).and_then(|file| file.take(MAX_ZONE_FILE + 1).read_to_end(&mut data));
    match read {
        Ok(_) if u64::try_from(data.len()).is_ok_and(|len| len <= MAX_ZONE_FILE) => Zone::Tzif(data),
        _ => Zone::Unknown,
    }
}

/// Minutes local time is ahead of UTC at `unix`, or `None` when the system does not say.
pub(super) fn offset_minutes(unix: i64) -> Option<i16> {
    let seconds = match zone() {
        Zone::Utc => 0,
        Zone::Tzif(data) => offset_seconds(data, unix)?,
        Zone::Unknown => return None,
    };
    i16::try_from(seconds / 60).ok()
}

/// One block header: the version and the six counts that size the block that follows.
struct Header {
    version: u8,
    isutcnt: usize,
    isstdcnt: usize,
    leapcnt: usize,
    timecnt: usize,
    typecnt: usize,
    charcnt: usize,
}

/// A big-endian `u32` at `at`, as a `usize`.
fn count(data: &[u8], at: usize) -> Option<usize> {
    let bytes: [u8; 4] = data.get(at..at.checked_add(4)?)?.try_into().ok()?;
    usize::try_from(u32::from_be_bytes(bytes)).ok()
}

/// The 44-byte header at `at`, or `None` when the magic is not there.
fn header(data: &[u8], at: usize) -> Option<Header> {
    if data.get(at..at.checked_add(44)?)?.get(..4)? != b"TZif" {
        return None;
    }
    Some(Header {
        version: *data.get(at + 4)?,
        isutcnt: count(data, at + 20)?,
        isstdcnt: count(data, at + 24)?,
        leapcnt: count(data, at + 28)?,
        timecnt: count(data, at + 32)?,
        typecnt: count(data, at + 36)?,
        charcnt: count(data, at + 40)?,
    })
}

/// How many bytes the data block of `header` takes, with `time` bytes per instant.
fn block_size(header: &Header, time: usize) -> Option<usize> {
    let sizes = [
        header.timecnt.checked_mul(time)?,
        header.timecnt,
        header.typecnt.checked_mul(6)?,
        header.charcnt,
        header.leapcnt.checked_mul(time.checked_add(4)?)?,
        header.isstdcnt,
        header.isutcnt,
    ];
    sizes.iter().try_fold(0usize, |total, size| total.checked_add(*size))
}

/// Seconds local time is ahead of UTC at `unix`, read from TZif bytes.
///
/// A version 2 or later file is read from its second block, where instants are 64 bits wide; a
/// version 1 file has only the 32-bit block.
fn offset_seconds(data: &[u8], unix: i64) -> Option<i32> {
    let first = header(data, 0)?;
    let (header, at, time) = if first.version >= b'2' {
        let second = 44usize.checked_add(block_size(&first, 4)?)?;
        (header(data, second)?, second.checked_add(44)?, 8usize)
    } else {
        (first, 44usize, 4usize)
    };
    if header.typecnt == 0 {
        return None;
    }
    let transitions = data.get(at..at.checked_add(header.timecnt.checked_mul(time)?)?)?;
    let indices_at = at.checked_add(transitions.len())?;
    let indices = data.get(indices_at..indices_at.checked_add(header.timecnt)?)?;
    let types_at = indices_at.checked_add(header.timecnt)?;
    let types = data.get(types_at..types_at.checked_add(header.typecnt.checked_mul(6)?)?)?;

    // The last transition that has already happened; the table is in order, so this stops early.
    let mut chosen = None;
    for (index, start) in indices.iter().enumerate() {
        if instant(transitions, index, time)? > unix {
            break;
        }
        chosen = Some(usize::from(*start));
    }
    // Before the first transition, RFC 8536 says to use the first type that is not daylight
    // saving, and the first type when every one of them is.
    let chosen = chosen.unwrap_or_else(|| types.chunks_exact(6).position(|entry| entry[4] == 0).unwrap_or(0));
    let entry = types.get(chosen.checked_mul(6)?..)?.get(..4)?;
    let bytes: [u8; 4] = entry.try_into().ok()?;
    Some(i32::from_be_bytes(bytes))
}

/// The `index`th instant of a transition table whose entries are `time` bytes wide.
fn instant(transitions: &[u8], index: usize, time: usize) -> Option<i64> {
    let at = index.checked_mul(time)?;
    let bytes = transitions.get(at..at.checked_add(time)?)?;
    match time {
        4 => Some(i64::from(i32::from_be_bytes(bytes.try_into().ok()?))),
        8 => Some(i64::from_be_bytes(bytes.try_into().ok()?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A TZif version 2 file with two types and the transitions given as `(instant, type)`.
    /// The first block is empty, as a modern file's is, and the offsets are in seconds.
    fn tzif(offsets: [(i32, u8); 2], transitions: &[(i64, u8)]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut header = |timecnt: usize, typecnt: usize| {
            data.extend_from_slice(b"TZif2");
            data.extend_from_slice(&[0; 15]);
            for value in [0usize, 0, 0, timecnt, typecnt, 0] {
                data.extend_from_slice(&u32::try_from(value).expect("small count").to_be_bytes());
            }
        };
        header(0, 0);
        header(transitions.len(), offsets.len());
        for (instant, _) in transitions {
            data.extend_from_slice(&instant.to_be_bytes());
        }
        for (_, index) in transitions {
            data.push(*index);
        }
        for (offset, daylight) in offsets {
            data.extend_from_slice(&offset.to_be_bytes());
            data.push(daylight);
            data.push(0);
        }
        data
    }

    /// Standard time two hours ahead of UTC, summer time three, changing in 2026.
    fn eastern_europe() -> Vec<u8> {
        tzif([(7_200, 0), (10_800, 1)], &[(1_774_486_800, 1), (1_793_235_600, 0)])
    }

    #[test]
    fn reads_the_offset_of_each_side_of_a_transition() {
        let zone = eastern_europe();
        assert_eq!(offset_seconds(&zone, 1_774_486_799), Some(7_200), "the second before the change");
        assert_eq!(offset_seconds(&zone, 1_774_486_800), Some(10_800), "summer time starts");
        assert_eq!(offset_seconds(&zone, 1_780_000_000), Some(10_800));
        assert_eq!(offset_seconds(&zone, 1_793_235_600), Some(7_200), "and ends");
        assert_eq!(offset_seconds(&zone, 4_000_000_000), Some(7_200), "after the last change it stays");
    }

    #[test]
    fn before_the_first_transition_the_first_standard_type_is_used() {
        // The daylight-saving type comes first, so it must be skipped.
        let zone = tzif([(10_800, 1), (7_200, 0)], &[(1_774_486_800, 0)]);
        assert_eq!(offset_seconds(&zone, 0), Some(7_200));
        assert_eq!(offset_seconds(&zone, 1_774_486_800), Some(10_800));
    }

    #[test]
    fn a_zone_with_no_transitions_still_has_an_offset() {
        let zone = tzif([(-18_000, 0), (0, 0)], &[]);
        assert_eq!(offset_seconds(&zone, 1_774_486_800), Some(-18_000));
    }

    #[test]
    fn broken_files_are_refused_instead_of_guessed() {
        assert_eq!(offset_seconds(b"", 0), None);
        assert_eq!(offset_seconds(b"not a zone file at all, not even close", 0), None);
        let zone = eastern_europe();
        for cut in [0, 10, 44, 60, 100, zone.len() - 1] {
            assert_eq!(offset_seconds(&zone[..cut], 0), None, "truncated to {cut} bytes");
        }
        // Counts far larger than the file, which must not overflow or index out of the slice.
        let mut huge = zone.clone();
        huge[44 + 32..44 + 36].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(offset_seconds(&huge, 0), None);
        // A transition to a type the file does not have is refused rather than read past.
        let lying = tzif([(7_200, 0), (10_800, 1)], &[(1_774_486_800, 2)]);
        assert_eq!(offset_seconds(&lying, 1_774_486_800), None);
        assert_eq!(offset_seconds(&lying, 0), Some(7_200), "before it the file is still readable");
    }

    #[test]
    fn minutes_come_from_the_seconds() {
        let zone = eastern_europe();
        assert_eq!(offset_seconds(&zone, 1_780_000_000).map(|seconds| seconds / 60), Some(180));
    }

    #[test]
    fn the_source_follows_the_tz_variable() {
        let path = |tz, tzdir| match source(tz, tzdir) {
            Some(Source::File(path)) => Some(path.display().to_string()),
            Some(Source::Utc) => Some("UTC".to_owned()),
            None => None,
        };
        assert_eq!(path(None, None).as_deref(), Some("/etc/localtime"));
        assert_eq!(path(Some(""), None).as_deref(), Some("UTC"));
        assert_eq!(path(Some(":"), None).as_deref(), Some("UTC"));
        assert_eq!(path(Some("Europe/Istanbul"), None).as_deref(), Some("/usr/share/zoneinfo/Europe/Istanbul"));
        assert_eq!(path(Some(":Europe/Istanbul"), None).as_deref(), Some("/usr/share/zoneinfo/Europe/Istanbul"));
        assert_eq!(path(Some("UTC"), Some("/opt/zones")).as_deref(), Some("/opt/zones/UTC"));
        assert_eq!(path(Some("/etc/localtime"), None).as_deref(), Some("/etc/localtime"));
        assert_eq!(path(Some("../../etc/shadow"), None), None, "a name cannot leave the zone directory");
        assert_eq!(path(Some("Europe//Istanbul"), None), None);
    }

    #[test]
    fn only_a_plain_file_of_a_believable_size_is_read() {
        let dir = std::env::temp_dir().join(format!("quvyta-zone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test directory");
        let good = dir.join("good");
        std::fs::write(&good, eastern_europe()).expect("write a zone");
        assert!(matches!(load(&good), Zone::Tzif(data) if data == eastern_europe()));
        // `TZ` can name any path. A file far larger than any zone is refused instead of read
        // whole, and so are a directory and a missing file.
        let mut huge = eastern_europe();
        huge.resize(4 * 1024 * 1024, 0);
        let big = dir.join("big");
        std::fs::write(&big, &huge).expect("write a huge file");
        assert!(matches!(load(&big), Zone::Unknown), "a 4 MiB zone file is not believed");
        assert!(matches!(load(&dir), Zone::Unknown));
        assert!(matches!(load(&dir.join("absent")), Zone::Unknown));
        std::fs::remove_dir_all(&dir).expect("clean");
    }

    /// A device that never ends, which `TZ=/dev/zero` names; reading it whole would never stop.
    #[cfg(unix)]
    #[test]
    fn a_device_is_not_read_as_a_zone() {
        assert!(matches!(load(Path::new("/dev/zero")), Zone::Unknown));
    }

    /// What the machine running the tests says, whatever zone it is in: an offset that exists and
    /// is a whole number of minutes inside the range zones use.
    #[test]
    fn this_machine_reports_a_believable_offset() {
        let Some(minutes) = offset_minutes(1_780_000_000) else {
            // A machine with no zone file says nothing, which is the honest answer.
            assert!(matches!(zone(), Zone::Unknown), "a zone was read but gave no offset");
            return;
        };
        assert!((-720..=840).contains(&minutes), "{minutes} minutes is no time zone offset");
    }
}
