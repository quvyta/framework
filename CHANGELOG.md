# Changelog

Notable changes to `quvyta-framework` and `quvyta-framework-showcase`. Both packages share one
version. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
is at 0.1, so a minor release may still change the API.

## 0.1.10 - 2026-09-20

### Added

- Free placement inside a stack (`View::place`) and a `Window` surface: a title strip with the
  application's icon, name and a faint subtitle, three-cell minimize, maximize and close marks,
  a focus pillar, an optional shadow, and moving and resizing reported as `WindowEvent`s
  (`WindowEdge` says which edge). `Ghost` paints a landing tone for a drag or a snap preview.
- The embedded terminal can be started with extra environment variables, a first size, a
  scrollback length and output coalescing (`TerminalSession::builder`), reports the program's
  title, working folder, bell and notifications (`TerminalWatch::next_change`), and ends politely
  (`pid`, `terminate`).
- Shared preferences: the applications of a family share language, theme and icons through one
  file, each key either its own or followed from the family (`Family::preferences`, `set`,
  `Resolved`, `Scope`). `widgets::Appearance` draws the settings rows for them, with the
  "in every application" choice, reduced motion and the pillar.
- `icons::nerd_font` installs Symbols Nerd Font Mono into the user's own font folder, with a
  checksum, progress and an honest word about what a terminal still needs; `icons::GlyphSample`
  shows sample glyphs so the user can judge with their own eyes.
- `storage::state_dir` and `cache_dir` for a family member as well.

### Changed

- Focus landing on a child taller than its scroll view no longer throws away a revealed line.

## 0.1.9 - 2026-09-19

### Added

- `CodeView` colours shell scripts (`Language::Shell`, also chosen from a file name with
  `Language::from_file_name`, such as `PKGBUILD`), goes to a line inside its scroll view
  (`reveal`), tints chosen lines (`highlight_lines`) and marks added and removed lines with a sign
  (`line_marks`).
- `PaintCx::reveal` asks the nearest scroll view to show part of a widget, gliding there unless
  motion is reduced.
- `IconButton`: one glyph in three cells, quiet at rest and lit on hover, for header controls.
- `Tabs::badge` puts a count after a tab's name; a narrow tab shortens its name and keeps the
  count.
- `storage::state_dir` and `storage::cache_dir`, and the same folders for a family member.
- An application's own icon keys are drawn in every theme and follow the glyph mode.
- `Glyph`: a table cell can show an icon key or a literal glyph before its text.
- `qframe::i18n::first_weekday()` reads the active translator's first day of the week in
  `update` and the other `App` methods.
- `qshots::Reel` records a harness frame by frame with a pointer and encodes a GIF and an MP4.
- `CHANGELOG.md`, a contribution guide, issue forms and an English page on the design principles.

### Changed

- A settings list taller than its scroll view scrolls just enough to show the row the keys move
  to, and never on a click.
- A form field whose control is wider than the room beside its label puts the label above, so a
  long placeholder is not cut.
- A table cell's icon without a colour is quiet, and takes the row's text colour when the row is
  selected.
- A missing Nerd Font or Unicode glyph in an icon set is a warning at its place in the file, and a
  plainer glyph stands in.
- When focus lands on a child taller than its scroll view that already fills the view, the view
  stays where it is.

## 0.1.8 - 2026-09-19

### Added

- The framework's own text (buttons, dialogs, hints, dates) in seven more languages: German,
  Spanish, French, Brazilian Portuguese, Russian, Simplified Chinese and Japanese.
- Regional and script locales are chosen from the system language (`pt-BR`, `zh-Hans`), and plural
  forms follow the base language of a regional code.
- `I18n::first_weekday`: calendar weeks start on the region's first day (from CLDR data) when the
  system or the application names a region, and on the language's day otherwise.
- Line wrapping follows Chinese and Japanese rules: breaks between ideographs, no closing
  punctuation at the start of a line, and no-break spaces kept as part of a word.
- `Process::run_with_overwritten` hands over the progress frames a program redraws with a carriage
  return, instead of dropping them.
- `text::truncate_middle` shortens a path in the middle, so both its folder and its file name stay
  visible.
- The showcase tour recorded as `showcase.gif` and `showcase.mp4`, and the dashboard drawn in
  every theme, all from test screens.

### Changed

- Dialog buttons stack one under another when they do not fit side by side, so none is cut.
- A required marker too wide for the label column moves under the control instead of being cut.
- A date field too narrow for the long date shows a shorter one, in every language.
- Long dialog titles and toast messages wrap onto more rows instead of being cut.
- A settings control too wide for half the row gets a line of its own under its label.

### Fixed

- Wide characters stay whole when a layer draws over one half of them.
- The test harness reads a double-width character once, so `screen`, `find`, `click_text` and
  `html` work on Chinese and Japanese text.
- Chinese and Japanese glyphs are drawn in screenshots made from test screens.
- A control placed after a bar that fills its row keeps one line instead of being squeezed onto
  many.

## 0.1.7 - 2026-09-19

### Added

- Tabs can end in a `+` button right after the last tab (`Tabs::on_add`).
- `FolderWatch` reports changes to a folder in batches from inotify events, without polling.
- Tree: select several nodes with `ctrl`, `shift` and `space`, and drop dragged nodes into a folder;
  a closed folder opens when the pointer rests on it.
- Text inputs can open with part or all of their text selected (`select_on_focus`,
  `select_all_on_focus`).
- The node holding focus can answer a keymap action with its own message (`on_action`), so one key
  can leave an embedded terminal and return to it.
- `InstanceLock`: several running copies share one lock, and a service can wait until the last of
  them closes.

## 0.1.6 - 2026-09-18

### Added

- `CardGrid`: cards laid out in columns that follow the width, with two-dimensional movement; only
  the cards on screen are built.
- `DetachedHandoff`: gives the screen back at the program's first line of output and leaves the
  program running with piped input and output.
- The README pictures are drawn from test screens as SVG and PNG, the same on every machine.

## 0.1.5 - 2026-09-18

### Added

- Programs in the embedded terminal that ask for the mouse receive clicks, drags, moves and the
  wheel. Shift-drag still selects text.
- A focused terminal can pass chosen keymap actions to the application (`pass_through`); text keys
  always go to the program.

### Fixed

- Radio groups start in the square style their documentation describes.
- `ctrl+alt+space` keeps its `alt` when a terminal program receives it.

## 0.1.4 - 2026-09-18

### Added

- `Family`: several applications share one settings folder, with a safe move from older locations
  (`Migration`).
- `documents_dir` finds the user's documents folder under the name the desktop gave it.
- Floating surfaces (`PaintCx::floating`) stand apart in tone from the panel they open over.

### Changed

- The showcase command is `qframe` (`quvyta-framework-showcase` still works).
- A settings backup keeps the file's whole name with `.bak` added.

## 0.1.3 - 2026-09-18

### Added

- Confirmation dialogs can ask the user to type a word before an irreversible action
  (`require_word`).

## 0.1.2 - 2026-09-18

### Added

- The showcase is an installable package, `quvyta-framework-showcase`, with every file it shows
  compiled in.
- Themes, icon sets and keymaps can be given as text (`theme_source`, `icon_source`,
  `keymap_source`), so an installed binary needs no files beside it.
- Application lifecycle: `App::init`, `resized`, `before_quit` and `terminating`. A terminal run
  ends gracefully on `SIGTERM`, `SIGINT` and `SIGHUP`, with a bounded grace period.
- Screens with their own message type: `View::map` and `Command::map`.
- `View::size` tells the application the size it is drawing into.
- Date and time: `Date`, `DateTime`, `TimeOfDay`, the local time zone and a suspend-aware clock
  (`Uptime`).
- Storage: application folders (`config_dir`, `data_dir`), `atomic_write`, a one-instance lock
  (`AppLock`), `machine_name`, and a schema-checked loader for an application's own data file
  (`Document`).
- `Handoff` gives the terminal to another program, such as `$EDITOR`, and takes it back.
- `Process` streams a child process line by line, for example into a log view.
- Input idleness: `idle_for` and `on_idle`.
- Charts: bar chart series with stacking, grouping and selection; `Heatmap`, `Legend`, `Timeline`
  and `Axis`; categorical series tones in the theme; reading a sparkline sample with the pointer or
  the keys; big text blended between two theme colours.
- `DurationInput`, which reads lengths such as `1h30m` or `90 min` in the built-in languages.
- Confirmation dialogs can offer a third choice (`alternative`).
- Tree nodes can be reordered among their siblings and carry a context menu.
- `I18n::has` asks whether a language has a key.

### Fixed

- The terminal is restored fully even when one step of restoring it fails.
- A hung-up terminal can no longer trap the event loop.
- Handed-off programs and processes without input run in their own process group.
- A background task ends even when building the message of its outcome panics.
- `atomic_write` keeps the permissions of the file it replaces.
- The date picker marks the local day as today.
- Toasts stay clear of open modal layers.

## 0.1.1 - 2026-09-17

### Added

- Language files can be given as text (`Runtime::locale_source`), for files compiled into an
  installed program.
- The licence text ships inside the crate.

## 0.1.0 - 2026-09-17

First public release.

- An Elm-style runtime (`App`, `Command`, `Runtime`) that redraws only on change, with a `Harness`
  that drives an application without a terminal.
- Theme, icon and language systems loaded from TOML files, with built-in defaults: the Monochrome,
  Iris, Nordic and Amber themes, Nerd Font, Unicode and ASCII glyphs, English and Turkish.
- Layout with rows, columns and layers, a router, an app shell, keymaps with a help layer and a
  command palette, motion with reduced-motion support, the clipboard, mouse text selection and a
  debug layer.
- Widgets: text, panel, button, text input, text area, number input, select, checkbox, switch,
  segmented control, radio group, slider, form and field, wizard, settings list, list, table, tree,
  tabs and tab rail, scroll view, splitter, side panel, widget dock, menu, breadcrumb, accordion,
  steps, modal and confirmation, popover, context menu, toast, tooltip, hold to confirm, badge,
  progress bar, spinner, shimmer text, skeleton, divider, empty state, key hints, Markdown, code
  view, log view, file and folder picker, sparkline, gauge, bar chart, big text, date picker, time
  input, background tasks and an embedded terminal (the `pty` feature).
- Settings storage with an optional self-repair.
- The showcase application, with a page for every item: a live demo, its code, a guide and a
  reference, in English and Turkish.
