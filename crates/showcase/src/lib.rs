//! The Quvyta framework showcase: every framework component with a live demo, its code, a guide
//! and a reference, in English and Turkish.
//!
//! The package installs one program under two names, `qframe` and
//! `quvyta-framework-showcase`; both start [`run`]. Everything the showcase shows is compiled
//! into the binary, so it runs from any folder.

mod app;
mod assets;
mod catalog;
mod layers;
mod log;
mod pages;
mod regions;
#[cfg(test)]
mod tests;

use qframe::env::Env;
use qframe::runtime::Runtime;
use qframe::storage::{Family, Settings, config_dir};

/// Runs the showcase in the terminal until the user quits.
///
/// # Errors
///
/// Returns I/O errors from the terminal.
pub fn run() -> std::io::Result<()> {
    // The showcase remembers theme, language, icons and the appearance switches between runs, in
    // the Quvyta family's folder; settings from the folder it used on its own are moved in first.
    // The file is checked against the installed themes and languages and healed while loading.
    let installed = Env::load(&assets::dirs())?;
    let adopted = config_dir("quvyta-showcase").map(|old| Family::QUVYTA.adopt("showcase", &old)).unwrap_or_default();
    let settings = Settings::load_member(&Family::QUVYTA, "showcase")
        .with_diagnostics(adopted.diagnostics().iter().cloned())
        .schema(pages::storage::schema(&installed))
        .self_heal(true);
    assets::install(Runtime::new(app::Showcase::with_settings(settings.clone()))).settings(&settings).run()
}
