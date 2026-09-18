//! The showcase's own language and keymap files, compiled in so an installed binary needs nothing
//! beside it on disk. The guides, references and the catalog are compiled in where they are read.

use qframe::env::AssetDirs;
use qframe::runtime::{App, Runtime};

/// Language files as `(file name, TOML text)`.
pub const LOCALES: [(&str, &str); 2] =
    [("en.toml", include_str!("../assets/locales/en.toml")), ("tr.toml", include_str!("../assets/locales/tr.toml"))];

/// The keymap layered over the framework's built-in one, as `(file name, TOML text)`.
pub const KEYMAP: (&str, &str) = ("keymap.toml", include_str!("../assets/keymap.toml"));

/// The showcase's files for [`qframe::env::Env::load`], given as text only: no path is read.
pub fn dirs() -> AssetDirs {
    AssetDirs {
        locale_sources: LOCALES.iter().map(|(file, text)| ((*file).to_owned(), (*text).to_owned())).collect(),
        keymap_source: Some((KEYMAP.0.to_owned(), KEYMAP.1.to_owned())),
        ..AssetDirs::default()
    }
}

/// Gives `runtime` the same files as [`dirs`].
pub fn install<A: App>(runtime: Runtime<A>) -> Runtime<A> {
    let runtime = LOCALES.iter().fold(runtime, |runtime, (file, text)| runtime.locale_source(*file, *text));
    runtime.keymap_source(KEYMAP.0, KEYMAP.1)
}
