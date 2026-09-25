//! The members of an ecosystem: what every application of it is called, where it keeps its
//! settings and how it is installed and started.

use super::Ecosystem;

/// One application of an ecosystem, as every other member needs to know it: a launcher that
/// lists and opens it, a settings screen that shows whether it follows the shared preferences.
///
/// Only what every member needs is here; how a launcher draws a member (its icon, its words, its
/// release status) stays in the launcher.
///
/// ```
/// use qframe::storage::Ecosystem;
///
/// let ecosystem = Ecosystem::QUVYTA;
/// let desk = ecosystem.members().iter().find(|member| member.id == "desk").expect("qdesk is a member");
/// assert_eq!((desk.settings_id, desk.command), ("desktop", "qdesk"));
/// if let (Some(folder), Some(file)) = (ecosystem.config_dir(), ecosystem.app_file(desk.settings_id)) {
///     assert_eq!(file, folder.join("desktop.conf"));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Member {
    /// The member's lowercase name inside the ecosystem, such as `code` or `desk`: the name its
    /// project goes by. Not always the id of its settings file.
    pub id: &'static str,
    /// The id its settings file goes under in the ecosystem's folder, `<settings_id>.conf`: the
    /// `app` the member itself gives [`Settings::load_member`](super::Settings::load_member) and
    /// [`Ecosystem::preferences`]. qdesk's is `desktop`, the showcase's `showcase` and the
    /// launcher's `launcher`, since the ecosystem's own id names the file every member shares.
    pub settings_id: &'static str,
    /// The member's name as a user reads it, such as `Code`: the name under which its work is
    /// kept, [`Ecosystem::workspace_dir`].
    pub title: &'static str,
    /// The package it is installed as, such as `quvyta-code`.
    pub package: &'static str,
    /// The short command that starts it, such as `qcode`. The package name starts it too.
    pub command: &'static str,
}

/// Every member of [`Ecosystem::QUVYTA`], the applications first and then the framework's
/// showcase and the launcher, which are about the ecosystem rather than applications to work in.
/// The framework itself is a library, so its showcase stands for it.
pub const MEMBERS: [Member; 10] = [
    member("code", "code", "Code", "quvyta-code", "qcode"),
    member("focus", "focus", "Focus", "quvyta-focus", "qfocus"),
    member("tools", "tools", "Tools", "quvyta-tools", "qtools"),
    member("packages", "packages", "Packages", "quvyta-packages", "qpac"),
    member("desk", "desktop", "Desktop", "quvyta-desktop", "qdesk"),
    member("explorer", "explorer", "Explorer", "quvyta-explorer", "qexp"),
    member("cli", "cli", "CLI", "quvyta-cli", "qcli"),
    member("browser", "browser", "Browser", "quvyta-browser", "qbrow"),
    member("framework", "showcase", "Framework", "quvyta-framework-showcase", "qframe"),
    member("quvyta", "launcher", "Quvyta", "quvyta", "quvyta"),
];

const fn member(
    id: &'static str,
    settings_id: &'static str,
    title: &'static str,
    package: &'static str,
    command: &'static str,
) -> Member {
    Member { id, settings_id, title, package, command }
}

impl Ecosystem {
    /// Every member of the ecosystem, in the order a launcher lists them: [`MEMBERS`] for
    /// [`Ecosystem::QUVYTA`], nothing for an ecosystem the framework does not know the members of.
    #[must_use]
    pub fn members(&self) -> &'static [Member] {
        if *self == Self::QUVYTA { &MEMBERS } else { &[] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_member_has_a_settings_file_of_its_own() {
        let members = Ecosystem::QUVYTA.members();
        for (index, member) in members.iter().enumerate() {
            let before = &members[..index];
            assert!(!before.iter().any(|other| other.settings_id == member.settings_id), "{member:?}");
            assert!(!before.iter().any(|other| other.id == member.id), "{member:?}");
            assert!(!before.iter().any(|other| other.command == member.command), "{member:?}");
            assert_ne!(member.settings_id, Ecosystem::QUVYTA.id(), "`quvyta.conf` is the file every member shares");
        }
    }

    #[test]
    fn qdesk_keeps_its_settings_in_desktop_conf() {
        let desk = MEMBERS.iter().find(|member| member.command == "qdesk").expect("qdesk");
        assert_eq!((desk.id, desk.settings_id, desk.package), ("desk", "desktop", "quvyta-desktop"));
        let launcher = MEMBERS.iter().find(|member| member.package == "quvyta").expect("the launcher");
        assert_eq!(launcher.settings_id, "launcher");
    }

    #[test]
    fn only_the_quvyta_ecosystem_has_members() {
        assert_eq!(Ecosystem::QUVYTA.members().len(), MEMBERS.len());
        assert!(Ecosystem::new("tools", "Tools").members().is_empty());
    }
}
