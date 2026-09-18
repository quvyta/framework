//! The name of the machine the application runs on, ready to be part of a file name.

/// The name of this machine, safe to use inside a file name, such as `running-<machine>.toml`.
///
/// Two machines that share a synced folder can each keep their own file in it by putting this
/// name in the file's name, so neither overwrites the other's.
///
/// - Unix: the node name `uname -n` prints, read from the kernel without a shell or a file.
/// - Windows: the `COMPUTERNAME` variable, which Windows sets for every process.
/// - Anywhere else: `None`.
///
/// The name is made safe for a file name, so what comes back can differ from what the system
/// calls itself:
///
/// 1. Whitespace around the name is removed.
/// 2. Every character that is not a letter, a digit, `-`, `_` or `.` becomes `-`. Letters and
///    digits of any script are kept (`dizüstü` stays `dizüstü`); a path separator, a space, a
///    colon, a control character and the other characters a file system may refuse do not
///    survive. Bytes that are not UTF-8 count as such a character.
/// 3. Dots at either end are removed: a leading dot hides a file on Unix, and Windows drops a
///    trailing one.
///
/// `None` when the platform gives no name or nothing is left of it after these steps, so the
/// application can fall back to a name of its own instead of writing `running-.toml`. The value
/// is read again on every call; it is not cached, because a machine can be renamed while an
/// application runs.
///
/// ```
/// if let Some(machine) = qframe::storage::machine_name() {
///     let file = format!("running-{machine}.toml");
///     assert!(!file.contains('/') && !file.contains('\\'));
/// }
/// ```
#[must_use]
pub fn machine_name() -> Option<String> {
    file_safe(&platform_name()?)
}

#[cfg(unix)]
fn platform_name() -> Option<String> {
    Some(rustix::system::uname().nodename().to_string_lossy().into_owned())
}

#[cfg(windows)]
fn platform_name() -> Option<String> {
    std::env::var_os("COMPUTERNAME").map(|name| name.to_string_lossy().into_owned())
}

#[cfg(not(any(unix, windows)))]
fn platform_name() -> Option<String> {
    None
}

/// `name` made safe for a file name by the steps [`machine_name`] lists; `None` when nothing is
/// left.
fn file_safe(name: &str) -> Option<String> {
    let safe: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' })
        .collect();
    let safe = safe.trim_matches('.');
    (!safe.is_empty()).then(|| safe.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{file_safe, machine_name};

    #[cfg(unix)]
    #[test]
    fn the_name_is_what_uname_prints() {
        let output = std::process::Command::new("uname").arg("-n").output().expect("uname runs");
        let printed = String::from_utf8_lossy(&output.stdout).into_owned();
        let name = machine_name().expect("this machine has a name");
        assert!(!name.is_empty());
        assert_eq!(Some(name.clone()), file_safe(&printed), "uname -n printed {printed:?}");
        if printed.trim().chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            assert_eq!(name, printed.trim(), "an ordinary host name comes back unchanged");
        }
    }

    #[test]
    fn ordinary_names_pass_unchanged() {
        assert_eq!(file_safe("vm").as_deref(), Some("vm"));
        assert_eq!(file_safe("DESKTOP-AB12CD").as_deref(), Some("DESKTOP-AB12CD"));
        assert_eq!(file_safe("MacBook-Pro.local").as_deref(), Some("MacBook-Pro.local"));
        assert_eq!(file_safe("build_box.2").as_deref(), Some("build_box.2"));
        assert_eq!(file_safe("dizüstü").as_deref(), Some("dizüstü"), "letters of any script stay");
    }

    #[test]
    fn surrounding_whitespace_is_removed() {
        assert_eq!(file_safe("  laptop\n").as_deref(), Some("laptop"));
        assert_eq!(file_safe("\tlaptop \r\n").as_deref(), Some("laptop"));
    }

    #[test]
    fn characters_a_file_name_cannot_hold_become_dashes() {
        assert_eq!(file_safe("a/b").as_deref(), Some("a-b"));
        assert_eq!(file_safe(r"a\b").as_deref(), Some("a-b"));
        assert_eq!(file_safe("my laptop").as_deref(), Some("my-laptop"));
        assert_eq!(file_safe("a:b*c?d\"e<f>g|h").as_deref(), Some("a-b-c-d-e-f-g-h"));
        assert_eq!(file_safe("a\u{0}b\u{7}c").as_deref(), Some("a-b-c"));
        assert_eq!(file_safe("bad\u{FFFD}byte").as_deref(), Some("bad-byte"), "a lossy UTF-8 byte");
    }

    #[test]
    fn dots_at_either_end_are_removed() {
        assert_eq!(file_safe(".hidden").as_deref(), Some("hidden"));
        assert_eq!(file_safe("trailing.").as_deref(), Some("trailing"));
        assert_eq!(file_safe("..").as_deref(), None, "`..` is never a file name");
        assert_eq!(file_safe(".").as_deref(), None);
    }

    #[test]
    fn nothing_left_is_no_name() {
        assert_eq!(file_safe(""), None);
        assert_eq!(file_safe("   \n"), None);
        assert_eq!(file_safe(" . "), None);
    }
}
