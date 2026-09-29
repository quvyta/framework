//! Comparing versions the way semver orders them, pre-releases included.

/// Whether `candidate` is a newer version than `installed`, in semver's order: `0.1.10` is newer
/// than `0.1.9`, a release is newer than its own pre-releases (`0.1.0` than `0.1.0-alpha.9`), and
/// between two pre-releases the first part that differs decides, numbers as numbers
/// (`0.1.0-alpha.10` is newer than `0.1.0-alpha.9`). Build metadata after `+` is left out. Either
/// text not being a version makes the answer `false`, so a garbled answer never looks like an
/// update.
///
/// This is the rule the update notice follows; that notice also offers a pre-release only to a
/// person already on one, which is the caller's to decide here.
///
/// ```
/// use qframe::version::newer;
///
/// assert!(newer("0.1.0-alpha.3", "0.1.0-alpha.2"));
/// assert!(newer("0.1.0", "0.1.0-alpha.9"));
/// assert!(!newer("0.1.9", "0.1.10"));
/// assert!(!newer("latest", "0.1.0"));
/// ```
#[must_use]
pub fn newer(candidate: &str, installed: &str) -> bool {
    match (Version::parse(candidate), Version::parse(installed)) {
        (Some(candidate), Some(installed)) => candidate > installed,
        _ => false,
    }
}

/// A version as semver orders it: `major.minor.patch`, then an optional pre-release whose
/// dot-separated parts compare as numbers when numeric and as ASCII text otherwise. Build
/// metadata after `+` is left out, as precedence ignores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Version {
    release: (u64, u64, u64),
    pre: Vec<PrePart>,
}

/// One part of a pre-release. The order of the variants is semver's: a numeric part ranks below a
/// word.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PrePart {
    Number(u64),
    Word(String),
}

impl Version {
    /// `None` for anything that is not a version, so an answer that cannot be read is silence.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = text.split('+').next()?;
        let (release, pre) = match text.split_once('-') {
            Some((release, pre)) => (release, Some(pre)),
            None => (text, None),
        };
        let mut parts = release.split('.').map(|part| part.parse::<u64>().ok());
        let release = (parts.next()??, parts.next()??, parts.next()??);
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(pre) => pre
                .split('.')
                .map(|part| {
                    let valid = !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
                    valid.then(|| part.parse().map_or_else(|_| PrePart::Word(part.to_owned()), PrePart::Number))
                })
                .collect::<Option<Vec<_>>>()?,
        };
        Some(Self { release, pre })
    }

    #[cfg(feature = "updates")]
    fn is_pre_release(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Whether this version may be offered to a person running `running`: a release always, a
    /// pre-release only to someone already on one.
    #[cfg(feature = "updates")]
    pub(crate) fn offered_to(&self, running: &Self) -> bool {
        !self.is_pre_release() || running.is_pre_release()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // A release ranks above its own pre-releases; between two pre-releases the first part
        // that differs decides, and a longer list wins a tie.
        self.release.cmp(&other.release).then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => self.pre.cmp(&other.pre),
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_compare_as_numbers() {
        assert!(newer("0.1.10", "0.1.9"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(!newer("0.1.9", "0.1.10"));
    }

    #[test]
    fn a_release_is_newer_than_its_pre_releases_and_pre_releases_order_among_themselves() {
        assert!(newer("0.1.0", "0.1.0-alpha.9"));
        assert!(!newer("0.1.0-alpha.9", "0.1.0"));
        assert!(newer("0.1.0-alpha.3", "0.1.0-alpha.2"));
        assert!(newer("0.1.0-alpha.10", "0.1.0-alpha.9"));
        assert!(newer("0.1.0-beta", "0.1.0-alpha.9"), "a word after a word compares as text");
        assert!(newer("0.1.0-alpha.1", "0.1.0-alpha"), "a longer list wins a tie");
    }

    #[test]
    fn equal_and_malformed_versions_are_never_newer() {
        assert!(!newer("0.1.30", "0.1.30"));
        assert!(!newer("0.1.30+build.7", "0.1.30"), "build metadata does not count");
        for garbled in ["", "latest", "0.1", "0.1.2.3", "0.1.x", "0.1.0-", "0.1.0-al..pha"] {
            assert!(!newer(garbled, "0.1.0"), "{garbled:?} as the candidate");
            assert!(!newer("0.1.0", garbled), "{garbled:?} as what is installed");
        }
    }
}
