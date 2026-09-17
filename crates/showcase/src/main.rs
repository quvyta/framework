//! The Quvyta showcase: every framework component with a live demo, its code, a guide and a
//! reference, in English and Turkish. Internal only; run with `cargo run -p showcase`.

mod app;
mod catalog;
mod layers;
mod log;
mod pages;
mod regions;
#[cfg(test)]
mod tests;

use qframe::env::{AssetDirs, Env};
use qframe::runtime::Runtime;
use qframe::storage::Settings;

fn main() -> std::io::Result<()> {
    let assets = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
    // The showcase remembers theme, language, icons and the appearance switches between runs. Its
    // file is checked against the installed themes and languages and healed while loading.
    let installed =
        Env::load(&AssetDirs { locales: Some(format!("{assets}/locales").into()), ..AssetDirs::default() })?;
    let settings = Settings::load("quvyta-showcase").schema(pages::storage::schema(&installed)).self_heal(true);
    Runtime::new(app::Showcase::with_settings(settings.clone()))
        .settings(&settings)
        .locale_dir(format!("{assets}/locales"))
        .keymap_file(format!("{assets}/keymap.toml"))
        .run()
}
