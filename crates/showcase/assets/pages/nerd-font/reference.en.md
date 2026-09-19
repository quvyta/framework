## Functions and constants

- `nerd_font::installed() -> bool` — whether a file with "nerd" in its name lies in this system's font folders, up to three folders deep.
- `nerd_font::installed_in(&[PathBuf]) -> bool` — the same search in the given folders.
- `nerd_font::target_dir() -> Option<PathBuf>` — where the font goes; `None` when the system names no folder. Asking does not create it.
- `nerd_font::install(|Progress| msg) -> Command<Msg>` — installs with `Install::new()` in the background.
- `nerd_font::after_install_text() -> String` — the honest next step, in the active language.
- `nerd_font::status_text(installed) -> String` — "A Nerd Font is installed on this machine" or "No Nerd Font was found on this machine".
- `RELEASE` (`"v3.5.1"`), `FAMILY` (`"Symbols Nerd Font Mono"`), `FONT_FILE` (`"SymbolsNerdFontMono-Regular.ttf"`).

## Install and Archive

- `Install::new()` — the release archive into `target_dir()`, registered with the system.
- `.archive(Archive)`, `.target(dir)`, `.register(bool)` — another archive, another folder, and whether `fc-cache` or `reg` runs.
- `.target_dir() -> Option<&Path>`.
- `.run(&cancel, &mut on_progress) -> Result<PathBuf, InstallError>` — the install on the calling thread.
- `.task(|Progress| msg) -> Task<Msg>` — the install as a background task; build it in `update`.
- `Archive::release()` — `.tar.xz` on Linux and macOS, `.zip` on Windows; `Archive::new(url, sha256)`; `.url()`, `.sha256()`.

## Progress and InstallError

- `Progress::Downloading { fraction: Option<f32> }`, `Verifying`, `Installing`, `Done { path }`, `Failed(InstallError)`; `.text()` in the active language.
- `InstallError::NoFolder`, `MissingTool(name)`, `Download(msg)`, `Verify(msg)`, `Checksum { expected, actual }`, `Extract(msg)`, `Copy(msg)`, `Register(msg)`, `Cancelled`; `.text()`.

## GlyphSample

- `GlyphSample::new(GlyphMode)` — the icons `folder`, `check`, `search` and `settings` in that mode, two cells apart, whatever mode the application draws in.
- `.keys(keys)` — other icon keys; a key the icon set lacks is left out.
- `GlyphSample::KEYS` — the default keys.
- `Icons::glyphs(key) -> Option<&IconGlyphs>` — every glyph of one icon, which the sample reads.

## Behaviour

- Order of steps: `Downloading { fraction: None }`, then `Downloading` with each new share `curl` reports, `Verifying`, `Installing`, and `Done` or `Failed`. A cancelled install ends without `Failed`.
- The download lands in a folder of its own under the system's temporary folder, which is removed whatever the outcome. A download that fails its checksum is deleted before anything else happens.
- Only `FONT_FILE` is taken from the archive. It is copied under a temporary name and renamed into place, so the folder never holds half a font.
- Checksum programs are tried in order: `sha256sum`, then `shasum -a 256` on Linux, the other way round on macOS, `certutil -hashfile … SHA256` on Windows. `MissingTool` names the first when none is there.
- Linux: `fc-cache -f <folder>` after the copy; skipped when fontconfig is missing. Windows: `reg add HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts /v "Symbols Nerd Font Mono Regular (TrueType)" /d <file>`. macOS: nothing.
- `Done { path }` is the folder on Linux and the font file on macOS and Windows; deleting it undoes the install. On Windows the registry entry stays and names a missing file, which Windows ignores.
- The task's label, notes and failure reason are translated when `task` is called; `Progress::text` translates when it is called, in the view.

## Language keys

- `quvyta.nerd-font.task`, `downloading`, `downloading-share` (`{percent}`), `verifying`, `installing`, `done` (`{path}`), `cancelled`.
- `quvyta.nerd-font.error-folder`, `error-tool`, `error-download`, `error-verify`, `error-checksum`, `error-extract`, `error-copy`, `error-register` (`{detail}`).
- `quvyta.nerd-font.after-install`, `found`, `missing`.

## Theme and icons

- `GlyphSample` uses the `text` colour on no background of its own, and the active icon set's glyphs.
- This page's status and outcome are a `Badge` and a `Text`; the progress is a `ProgressBar`.
