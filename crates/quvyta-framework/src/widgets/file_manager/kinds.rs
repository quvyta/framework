//! A file manager's icons by the kind of each entry, and their colours by family.

use crate::color::ColorDepth;
use crate::env::Env;
use crate::icons::{FileKind, GlyphMode, UserFolders, file_kind};

use super::state::ROOT;
use super::{FileManager, parent_key};

/// How the kinds of entries are drawn on one screen, worked out when the manager is shown.
#[derive(Debug, Clone, Default)]
pub(super) struct KindLook<'a> {
    /// Whether icons follow the kinds at all.
    icons: bool,
    /// Whether they take their family's colour, which the terminal must be able to show.
    tones: bool,
    /// The key of the home, when the home is on screen, with its folders.
    home: Option<(String, &'a UserFolders)>,
}

impl<'a, Msg: Clone + 'static> FileManager<'a, Msg> {
    /// How kinds are drawn in `env`: coloured only where tones can be told apart, and the home's
    /// folders known only while the home is inside the root.
    pub(super) fn kind_look(&self, env: &Env) -> KindLook<'a> {
        if !self.kind_icons {
            return KindLook::default();
        }
        let tones = self.kind_tones && env.depth() != ColorDepth::Ansi16 && env.glyph_mode() != GlyphMode::Ascii;
        let root = self.state.root();
        // The person's folders are only read once their home is on screen: a manager elsewhere
        // never needs them.
        let folders = self.user_folders.or_else(|| {
            let home = std::env::var_os("HOME")?;
            std::path::Path::new(&home).starts_with(root).then(UserFolders::current).flatten()
        });
        let home = folders.and_then(|folders| {
            let inside = folders.home().strip_prefix(root).ok()?;
            let parts: Option<Vec<&str>> = inside.components().map(|part| part.as_os_str().to_str()).collect();
            Some((parts?.join("/"), folders))
        });
        KindLook { icons: true, tones, home }
    }

    /// The icon and the colour of the entry `key`, called `name`, when no sign of the application
    /// says otherwise: the plain `folder` or `file`, or its kind's, in the family's colour when
    /// kinds are coloured and in the row's own colour when not.
    pub(super) fn own_icon(&self, key: &str, name: &str, folder: bool, executable: bool) -> (String, Option<String>) {
        let look = &self.kinds;
        if !look.icons {
            return ((if folder { "folder" } else { "file" }).to_owned(), None);
        }
        let kind = self.kind_of(key, name, folder, executable);
        let tone = if look.tones { kind.family().tone().map(str::to_owned) } else { None };
        (kind.icon().to_owned(), tone)
    }

    /// The kind of the entry `key`: the home and its folders by where they are, everything else by
    /// its name.
    fn kind_of(&self, key: &str, name: &str, folder: bool, executable: bool) -> FileKind {
        if folder && let Some((home, folders)) = &self.kinds.home {
            let own = if key == home {
                folders.kind(folders.home())
            } else if key != ROOT && parent_key(key) == home {
                folders.inside(name)
            } else {
                None
            };
            if let Some(kind) = own {
                return kind;
            }
        }
        file_kind(name, folder, executable)
    }
}
