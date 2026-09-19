## When to use

Every icon in the framework has a Nerd Font glyph, and those glyphs need a font that carries them. When detection finds no Nerd Font, or a user picks Nerd icons and sees boxes, offer the install: a first-run setup screen next to the icon choice, and a row in the application's settings.

- **Symbols only.** The framework installs Symbols Nerd Font Mono, a font of symbols without letters. The user keeps the font their terminal draws text with; a terminal that takes missing glyphs from other installed fonts finds the icons there. Most terminals on Linux do this through fontconfig.
- **The eye decides.** No program can learn which font a terminal draws with. Show `GlyphSample`s, the same icons in the Nerd and the Unicode column, before and after the install, and let the user say which row reads as shapes.
- **Offer, never force.** Nothing is installed until the user presses the button, and Unicode icons stay a perfectly good choice.

## Step by step

1. Show the state: `nerd_font::installed()` and `nerd_font::status_text(installed)`, and where the font would go, `nerd_font::target_dir()`.
2. Place the samples: `ui.add(GlyphSample::new(GlyphMode::Nerd))` beside `GlyphSample::new(GlyphMode::Unicode)`, each after a label.
3. Start the install in `update`: `nerd_font::install(Msg::Font)` returns the command. For a task you can cancel or show in a `TaskList`, use `Install::new().task(Msg::Font).on_event(Msg::Task)` and keep its `id()`.
4. Show every step: keep the last `Progress` and draw `progress.text()`; `Progress::Downloading { fraction }` fills a `ProgressBar`.
5. On `Progress::Done { path }`, show the samples again and `nerd_font::after_install_text()` below them: if the Nerd row still shows boxes, the text tells the user to install JetBrainsMono Nerd Font and choose it in the terminal's settings.
6. On `Progress::Failed(error)`, `progress.text()` gives the reason, with the program's own message where there is one.
7. In tests, never touch the real font folder: `Install::new().archive(Archive::new("file:///…/fake.tar", sha256)).target(temp).register(false)`.

## How it works

- **A fixed release, a written checksum.** The archive comes from Nerd Fonts v3.5.1 and its SHA-256 is in the source. A download that does not match is deleted and nothing is installed.
- **The system's own programs.** `curl` downloads and reports its progress, `sha256sum` or `shasum -a 256` (on Windows `certutil`) checks, and `tar` unpacks only the font file. They ship with Linux, macOS and Windows 10 and later, so the framework carries no network, archive or hash code. Windows gets the zip archive, which its `tar` opens, and its programs are taken from `System32`.
- **The user's own folder.** `~/.local/share/fonts/QuvytaNerdFont` on Linux (following `XDG_DATA_HOME`), `~/Library/Fonts` on macOS, `%LOCALAPPDATA%\Microsoft\Windows\Fonts` on Windows. No administrator is asked for.
- **The system is told.** On Linux `fc-cache -f` reads the new folder, when fontconfig is installed; on Windows the font is entered under `HKCU\…\Fonts`; macOS needs nothing.
- **Reversible.** `Progress::Done` names the one path to delete to undo the install: the folder of its own on Linux, the font file where the folder is shared.
- **In the background.** The work runs as a `Task`: the screen keeps drawing and the user can go on with other choices. Its label and notes are translated when it is built, in `update`, because its thread has no language.

## Common mistakes

- **Trusting the file on disk.** A font in the folder does not mean the terminal draws with it; always show the samples again after the install.
- **Writing the terminal's settings.** The terminal's configuration belongs to the user. Say what to choose; never edit it.
- **Installing a whole family by default.** A full Nerd Font replaces the user's text font only if they choose it; the symbols-only font changes nothing they chose.
- **Starting the task outside `update`.** Built where no language is active, its label and notes read as keys.
