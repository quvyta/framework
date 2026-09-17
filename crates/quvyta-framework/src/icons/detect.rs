//! Chooses between Nerd Font, Unicode and ASCII glyphs for the current terminal.

use std::fs;
use std::path::{Path, PathBuf};

use super::{GlyphMode, IconMode};

/// Decides which glyphs to draw.
///
/// Order: the `QUVYTA_ICONS` environment variable, then `preference`, then detection:
/// non-UTF-8 or primitive terminals get ASCII; terminals known to ship or commonly use Nerd
/// Fonts, or a Nerd Font file under `font_dirs`, get Nerd glyphs; everything else gets Unicode.
#[must_use]
pub fn detect_glyph_mode(
    preference: IconMode,
    env: impl Fn(&str) -> Option<String>,
    font_dirs: &[PathBuf],
) -> GlyphMode {
    let forced = env("QUVYTA_ICONS").and_then(|value| IconMode::from_name(&value));
    match forced.unwrap_or(preference) {
        IconMode::Nerd => return GlyphMode::Nerd,
        IconMode::Unicode => return GlyphMode::Unicode,
        IconMode::Ascii => return GlyphMode::Ascii,
        IconMode::Auto => {}
    }

    let lower = |name: &str| env(name).map(|v| v.to_lowercase());
    let utf8 = ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .filter_map(|name| lower(name))
        .find(|value| !value.is_empty())
        .is_some_and(|value| value.contains("utf-8") || value.contains("utf8"));
    if !utf8 && !cfg!(windows) {
        return GlyphMode::Ascii;
    }
    if let Some(term) = lower("TERM")
        && (term == "dumb" || term == "linux")
    {
        return GlyphMode::Ascii;
    }
    if let Some(program) = lower("TERM_PROGRAM") {
        if ["ghostty", "wezterm", "kitty", "vscode", "warp", "iterm"].iter().any(|known| program.contains(known)) {
            return GlyphMode::Nerd;
        }
        if program.contains("apple_terminal") {
            return GlyphMode::Unicode;
        }
    }
    if env("KITTY_WINDOW_ID").is_some() || env("GHOSTTY_RESOURCES_DIR").is_some() || env("WT_SESSION").is_some() {
        return GlyphMode::Nerd;
    }
    if font_dirs.iter().any(|dir| contains_nerd_font(dir, 3)) {
        return GlyphMode::Nerd;
    }
    GlyphMode::Unicode
}

/// The font directories worth scanning on this platform.
#[must_use]
pub fn default_font_dirs(env: impl Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(target_os = "macos") {
        if let Some(home) = env("HOME") {
            dirs.push(Path::new(&home).join("Library/Fonts"));
        }
        dirs.push(PathBuf::from("/Library/Fonts"));
    } else if cfg!(windows) {
        if let Some(local) = env("LOCALAPPDATA") {
            dirs.push(Path::new(&local).join("Microsoft/Windows/Fonts"));
        }
        if let Some(windir) = env("WINDIR") {
            dirs.push(Path::new(&windir).join("Fonts"));
        }
    } else {
        if let Some(home) = env("HOME") {
            dirs.push(Path::new(&home).join(".local/share/fonts"));
            dirs.push(Path::new(&home).join(".fonts"));
        }
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        dirs.push(PathBuf::from("/usr/share/fonts"));
    }
    dirs
}

/// Looks for a font file whose name contains "nerd", at most `depth` directories deep.
fn contains_nerd_font(dir: &Path, depth: usize) -> bool {
    if depth == 0 {
        return false;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            contains_nerd_font(&path, depth - 1)
        } else {
            path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.to_lowercase().contains("nerd"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        move |name| map.get(name).cloned()
    }

    const UTF8: (&str, &str) = ("LANG", "tr_TR.UTF-8");

    #[test]
    fn environment_variable_wins() {
        let mode = detect_glyph_mode(IconMode::Nerd, env(&[("QUVYTA_ICONS", "ascii"), UTF8]), &[]);
        assert_eq!(mode, GlyphMode::Ascii);
    }

    #[test]
    fn preference_beats_detection() {
        let mode = detect_glyph_mode(IconMode::Unicode, env(&[UTF8, ("TERM_PROGRAM", "WezTerm")]), &[]);
        assert_eq!(mode, GlyphMode::Unicode);
    }

    #[test]
    fn non_utf8_and_primitive_terminals_get_ascii() {
        if cfg!(windows) {
            return;
        }
        assert_eq!(detect_glyph_mode(IconMode::Auto, env(&[("LANG", "C")]), &[]), GlyphMode::Ascii);
        assert_eq!(detect_glyph_mode(IconMode::Auto, env(&[UTF8, ("TERM", "linux")]), &[]), GlyphMode::Ascii);
    }

    #[test]
    fn known_terminals_get_nerd_glyphs() {
        let mode = detect_glyph_mode(IconMode::Auto, env(&[UTF8, ("TERM_PROGRAM", "ghostty")]), &[]);
        assert_eq!(mode, GlyphMode::Nerd);
        let mode = detect_glyph_mode(IconMode::Auto, env(&[UTF8, ("TERM_PROGRAM", "Apple_Terminal")]), &[]);
        assert_eq!(mode, GlyphMode::Unicode);
    }

    #[test]
    fn installed_nerd_font_is_found() {
        let root = std::env::temp_dir().join(format!("quvyta-fonts-{}", std::process::id()));
        let nested = root.join("JetBrainsMono");
        fs::create_dir_all(&nested).expect("create temp font dir");
        fs::write(nested.join("JetBrainsMonoNerdFont-Regular.ttf"), b"").expect("write font file");
        let found = detect_glyph_mode(IconMode::Auto, env(&[UTF8]), std::slice::from_ref(&root));
        fs::remove_dir_all(&root).expect("clean temp font dir");
        assert_eq!(found, GlyphMode::Nerd);
        assert_eq!(detect_glyph_mode(IconMode::Auto, env(&[UTF8]), &[]), GlyphMode::Unicode);
    }
}
