//! Screenshots of a [`Harness`] screen as SVG and PNG, for READMEs.
//!
//! A test builds a scene with the framework's [`Harness`] and hands it to [`Shot::of`]. The
//! picture is the terminal grid on a softly rounded ground that continues the screen's own edge
//! colour, with an optional quiet title; no frame, no border, no shadow.
//!
//! Every glyph is an outline from the embedded JetBrains Mono Nerd Font Mono (Regular and Bold;
//! italics are slanted), so nothing depends on the fonts of the machine: the same scene gives
//! byte-identical files everywhere. The picture shows only what the test drew, so a scene built
//! from fixed sample data can never leak anything personal.
//!
//! ```
//! use qframe::prelude::*;
//!
//! struct Hello;
//!
//! impl App for Hello {
//!     type Msg = ();
//!     fn update(&mut self, (): ()) -> Command<()> {
//!         Command::none()
//!     }
//!     fn view(&self, ui: &mut View<'_, ()>) {
//!         ui.add(Text::new("Hello"));
//!     }
//! }
//!
//! let harness = Harness::new(Hello, 20, 2);
//! let shot = qshots::Shot::of(&harness).title("hello");
//! assert!(shot.to_svg().starts_with("<svg"));
//! assert!(shot.missing().is_empty());
//! // In a README test: shot.save("docs/screenshots/hello")?;
//! ```
//!
//! [`Harness`]: qframe::runtime::Harness

mod font;
mod geometry;
mod png;
mod screen;
mod svg;
#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use qframe::runtime::{App, Harness};

use crate::screen::Screen;

/// One screenshot: a copy of a harness screen, ready to draw as SVG or PNG.
///
/// Built with [`Shot::of`]; [`Shot::title`] adds a title strip. The shot keeps its own copy of
/// the screen, so the harness can go on to the next scene.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    screen: Screen,
    title: Option<String>,
}

impl Shot {
    /// The screen `harness` shows now, with the colours of its current theme.
    ///
    /// Cells without a colour of their own take the theme's `text` and `canvas`. The ground
    /// around the grid takes the background most of the screen's edge shows, so the margin
    /// looks like more of the application.
    #[must_use]
    pub fn of<A: App>(harness: &Harness<A>) -> Self {
        Self { screen: Screen::capture(harness), title: None }
    }

    /// Adds a thin title strip above the grid: `title` centred in the theme's `muted` colour on
    /// a tone a small step lighter (on a light theme, darker) than the ground.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The picture as SVG. It holds glyph outlines instead of text and no external resource, so
    /// it looks the same in every browser, including through `<img>` on GitHub.
    #[must_use]
    pub fn to_svg(&self) -> String {
        svg::draw(&self.screen, self.title.as_deref()).svg
    }

    /// The picture as PNG, at twice the SVG's size.
    ///
    /// # Errors
    ///
    /// Fails only if the rasteriser rejects the picture, which a screen this crate captured does
    /// not cause; the error is kept rather than hidden.
    pub fn to_png(&self) -> io::Result<Vec<u8>> {
        png::render(&self.to_svg())
    }

    /// Characters on the screen the embedded font has no glyph for, such as CJK or emoji. They
    /// are left out of the picture; a README test can assert this is empty.
    #[must_use]
    pub fn missing(&self) -> Vec<char> {
        svg::draw(&self.screen, self.title.as_deref()).missing.into_iter().collect()
    }

    /// Writes `<path>.svg` and `<path>.png`, creating the folder. `path` names the picture
    /// without an extension, e.g. `docs/screenshots/home`.
    ///
    /// # Errors
    ///
    /// Returns the error of creating the folder or writing either file.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let svg = self.to_svg();
        let png = png::render(&svg)?;
        std::fs::write(with_extension(path, "svg"), svg)?;
        std::fs::write(with_extension(path, "png"), png)
    }
}

/// `path` with `.extension` appended, keeping any dot already in the name (`home.v2` stays whole).
fn with_extension(path: &Path, extension: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".");
    name.push(extension);
    PathBuf::from(name)
}
