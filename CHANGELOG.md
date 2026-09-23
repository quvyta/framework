# Changelog

Notable changes to `quvyta-framework` and `quvyta-framework-showcase`. Both packages share one
version. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
is at 0.1, so a minor release may still change the API.

## 0.1.21 - 2026-09-23

### Changed

- The applications that share one settings folder are called an ecosystem, not a family:
  `storage::Family` is now `storage::Ecosystem`, and `Scope::Family` and `Source::Family` are
  `Scope::Ecosystem` and `Source::Ecosystem`. The old names still work as plain aliases, without
  a warning, so an application whose gate denies warnings builds unchanged; a later release marks
  them deprecated and the one after removes them. The two variant aliases are associated
  constants, which still work in a `match`. Nothing on disk changes: the files and keys never
  carried the word. The placeholder of `quvyta.appearance.everywhere` and
  `quvyta.appearance.updates-text` keeps its name, `{family}`, so applications that fill it
  themselves keep working. The icon of Quvyta's own mark is named `ecosystem` instead of
  `family`; the old key still gives the same mark, so an application asking for it keeps its icon. Texts, guides and documentation speak of the Quvyta ecosystem.

## 0.1.20 - 2026-09-23

### Fixed

- The update notice works for an application on a pre-release. A version such as
  `0.1.0-alpha.1` was not read at all, so a person on an alpha never heard of the next one.
  Versions are now ordered as semver orders them; a person on a pre-release hears of the newest
  version after it, pre-release or release, and a person on a release still never hears of a
  pre-release.
- Text cut in ASCII glyph mode ends in `~`, not `…`, which an ASCII terminal cannot show. A table
  cut a long name and a column title with `…` in every mode, and so did lists, menus, badges and
  the rest. The painter now draws the cut mark as `text::ASCII_ELLIPSIS` in ASCII mode, in the
  same single cell, so every widget is covered and nothing moves; Unicode and Nerd Font modes keep
  `…`. `text::truncate` and `text::truncate_middle` are unchanged.

- A sixteen-colour terminal keeps the shape of a screen. Rounded to the nearest of the sixteen,
  every dark ground of a theme landed on black: the page behind an open dialog vanished into the
  screen, and tab strips, raised panels and dialogs melted into the canvas. A sixteen-colour frame
  is now painted in full colour and reduced once it is complete. A ground the eye tells from the
  canvas (0.05 in OKLab) and that would land on the canvas's colour takes the next grey away from
  it, so `raised`, `active` and `overlay` show as bright black on black; text that would keep under
  1.6:1 against its background takes the quietest grey that keeps 3:1, so the dimmed page behind a
  dialog stays faint and muted text on a raised surface stays readable. `Rgb::to_ansi16_on`,
  `Rgb::to_ansi16_text` and `Rgb::from_ansi16` give the same answer outside a frame. True colour
  draws as before.
- In 256 colours the page behind an open dialog stays faint but visible. Each colour was reduced
  to the palette as it was drawn, so the dimming behind a dialog could not blend the palette
  entries it found and gave text and ground the same entry, 1:1. A 256-colour frame is now painted
  in full colour and reduced once it is complete, as a sixteen-colour one is, so the dimming, the
  lift of a menu over a panel of nearly its tone and page transitions blend as in true colour
  before they are reduced. Backgrounds keep the nearest entry; text that would keep under 1.6:1
  against its background takes the entry closest to it that keeps 1.6:1, so the faintest dimmed
  labels stay faint instead of vanishing (`Rgb::to_ansi256_text`). `Rgb::from_ansi256` gives the
  colour of a palette entry. This changes how 256-colour screens draw.
- A faint timeline block on a sixteen-colour terminal recedes towards the ground when no colour
  between its tone and the lifted track shows apart from both, instead of looking like a full
  block.
- The count inside a badge reads in sixteen colours; it kept 1.18:1 against its own fill.

- A time or a duration in a settings list takes two digits. The list forwarded the keys of its
  row to the control but painted the control as unfocused, and a `TimeInput` or `DurationInput`
  forgets the part being typed when it is not focused: typing `12` into an hour gave 02, `05`
  into minutes gave 5 hours, and ← → never reached the minutes. The control of the list's
  keyboard row is now painted focused.

### Added

- `FolderChanges::next_within(bound)`: the wait for folder changes with a limit, `None` when
  nothing changed in time and the watch goes on. `FileManagerState::following_within(bound)`
  follows outside changes with each wait bounded. A screen test runs a wait on the spot, so an
  unbounded one held it for good; now a test can show a folder change arriving.


## 0.1.19 - 2026-09-23

### Fixed

- A line with a control character no longer brings an application down. A `LogView` handed
  Docker's `Sending build context … 2.048kB\r\r` panicked in a debug build and would have sent
  the raw character to the terminal in a release one. `LogLine::new` now keeps text as a
  terminal would show it, and no widget draws a control character into a cell: it keeps the
  cell it is measured at, blank.
- The update notice's row says what it sends as it is: the application's name and its version,
  nothing about the person or the machine. It used to say only the name went out.
- The German texts speak to the person as `du`, as the rest of the family does; seven of them
  still said `Sie`.

### Added

- `text::printable(text)`: one line printed for a terminal, as the terminal would leave it —
  only what the last carriage return left, escape sequences removed, tabs padded to the next
  stop of eight, no other control character.
- `Harness` works out what each frame would send to the terminal, so a cell no terminal can
  take fails the test that drew it.
- `NodeMut::wrap(true)` on a row moves the children that do not fit to the next line instead of
  drawing them past the edge, so a row of buttons that is too wide in some languages or on a narrow
  terminal stays usable. Lines are filled in order; `gap`, `justify` and spacers work on each line;
  a child wider than the row gets a line of its own. The row measures as tall as its lines, so the
  widgets below it move down. A row that fits lays out exactly as before. `NodeMut::line_gap(rows)`
  leaves empty rows between the lines.
- `Table::menu_on_activate(true)`: Enter and a click open the row's context menu instead of
  sending `on_activate`, for a table whose rows are acted on only through a few choices. A row
  whose menu is empty opens nothing.
- `ScrollView::follow_end(true)` keeps the end of growing content in view, for a conversation
  or command output made of any widgets. It opens at the end; while the person is at the end,
  growth glides to the new end (or jumps with reduced motion). Scrolling up with the wheel, the
  keys or the scrollbar holds the view and a faint note counts the lines below; End, scrolling
  back down or a click on the note follows again. A focused widget at the end, such as a reply
  field, keeps following; focusing something higher up holds the view there. Without the option
  nothing changes.

## 0.1.18 - 2026-09-21

### Fixed

- The showcase's search opens the first page it finds when Enter is pressed. Typing a name in the
  sidebar search listed the right page, but neither Enter nor Tab and Enter opened it, so the
  keys alone could not get there.

## 0.1.17 - 2026-09-21

### Added

- An application says when a newer version of itself is out. With the new `updates` feature,
  `Command::check_for_update(UpdateCheck::new(family, app, package, version, Msg::NewVersion))`
  asks crates.io at most once a day, on a thread of its own so the start never waits, and sends
  the message only when a newer version is published; `Update::toast()` is the notice every
  member shows the same way, with how to update. No network or no answer is silence. Only the
  package's name goes out, with a `User-Agent` of the package and its version. The feature
  brings in `ureq`; an application that does not ask carries no network code.
- One switch for the whole family: `Family::update_notice`, `update-notice` in the shared file,
  on unless it is turned off; with it off nothing is asked or written. `Appearance::updates` is
  its row, for an application that asks to put right after the section; the section itself is
  unchanged.
- A test never reaches the network: `Harness::update_checks()` lists the questions and
  `Harness::set_latest_version(Some(..))` answers them.

## 0.1.16 - 2026-09-21

### Fixed

- Keys that arrive together are no longer lost. A fast typist, a terminal multiplexer, a slow
  connection or a paste in a terminal without bracketed paste can hand several keys over in one
  read, and a controlled text field kept only the last of them: `demo` typed at once became `o`.
  The view is now rebuilt off screen between the events of such a burst, so each one meets what
  the one before it did. That frame is never drawn and does not count against the frame limit.
- A folder picker chooses the folder the user opened. Opening a folder puts the cursor on its
  first entry so the keys have somewhere to start, and Choose folder took that entry for the
  choice: opening `myapp` and pressing it chose `myapp/src`. It now chooses a folder inside only
  when the user moved the cursor there.
- A button, a switch, a checkbox and every other control pressed by a click act only when the
  press began on them. A row that opens a page on the press could have that same click's release
  land on a button the new page put in its place, and the button pressed itself: a setup wizard
  finished before anyone looked at it.

## 0.1.15 - 2026-09-21

### Changed

- A large file opens and scrolls at once in a code view. A megabyte of source used to freeze the
  screen for minutes: 0.88 MiB of Rust took 418 seconds to appear, 163 to redraw and 228 to move
  one line. It now appears in about 0.14 seconds, and a redraw or a step down one line takes 0.2 to
  0.4 milliseconds however long the file is. The file is laid out in one pass over its tokens
  rather than reading every token again for every line, a code view remembers how it laid a file
  out by its text and language, and only the rows on screen are drawn. Code blocks in `Markdown`
  draw only their visible rows too.

## 0.1.14 - 2026-09-21

### Added

- A link or a program is opened on the person's own desktop without the screen ever being given
  away (`Command::open`, and `Command::open_with(Open)` when the call needs arguments, a working
  folder, environment or an answer of its own). The program starts on a thread of its own with
  null streams and its own process group, the answer goes out as soon as it has started, and the
  child is waited for only afterwards, so nothing is torn down and redrawn. `OpenOutcome::Opened`
  says the opener was handed the target and no more than that. A test reads `Harness::opens()` and
  decides the answer with `Harness::set_open_outcome()`.
- Numbers, dates and units are written the way the reader's language writes them. Nine locales
  carry `quvyta.number.decimal`, asked for with `I18n::decimal_separator()`,
  `i18n::decimal_separator()` and `i18n::number(value, decimals)`; every number the framework
  draws goes through it, and a number field reads the separator back, still takes a point, and
  rewrites itself when the language changes. A date is composed by the framework rather than by
  hand: `Date::written()`, `written_short()`, `day_and_month()` and `day_and_month_short()`.
  Chinese writes its minute mark beside the number instead of in the middle of a word.
- A diff shows the line numbers of the files its lines came from. Given `line_marks`, a removed
  line carries the old file's number and an added or unchanged line the new file's;
  `CodeView::reveal_number(n)` goes to the line a finding means and prefers the line that stayed
  over the one the old version lost; `CodeView::line_numbers_from(numbers)` gives them outright
  for a hunk that starts part way into a file.
- `qshots::missing_in(text)` asks the fonts whether a piece of text can be drawn at all, without
  drawing it, so an application can check every language it speaks rather than only the ones a
  picture happens to show.
- The icon set answers `help` with a question mark in all three glyph shapes, and `workspace` with
  the ordered place that holds the thing worked on. `project` is unchanged and still means the
  thing itself.
- `FileManagerState::folder_error` answers why any one folder could not be read, the root among
  them; `error()` is now that question asked about the root.

### Changed

- The language, theme and icon choices of the shared appearance rows are as wide as the longest
  name they offer, between a floor and a cap, instead of a fixed eighteen cells. `Português
  (Brasil)` was cut to `Português …` on every screen, however wide, which is exactly where the two
  Portugueses part. The three rows still share one column, so they read as one group.
- A folder in the file manager that could not be read says so on its row and in the foot of a
  flat view. Until now only the root's reason was kept: every other refused folder was drawn as
  an empty one, so a folder a person may not look into told them it held nothing.

## 0.1.13 - 2026-09-20

### Added

- A terminal that only shows (`Terminal::read_only`): it never writes to the program, takes no
  focus and is drawn faint, while keeping the true colours, the cursor the program left, wide
  characters, selection and scrolling back. A window whose program has ended can keep showing its
  last screen without swallowing what is typed into it.
- The file manager shows a folder as a list with its size, date and permissions, or as a grid of
  icons, beside the tree (`FileManager::view`, `FileView`). Stepping into a folder and back is
  part of the state, every operation works in all three shapes, and the details of at most one
  page around the cursor are ever read, so a folder of ten thousand entries is still one screen
  of painting.
- A row of a `Table` and a card of a `CardGrid` can carry their own context menu
  (`Table::context_menu`, `CardGrid::context_menu`), acting on the row that was right-clicked
  rather than the one the cursor rests on.

### Changed

- Text pasted into a terminal whose program has ended is handed back instead of disappearing, and
  reaches the application as a clipboard event. A wheel step on the alternate screen of a program
  that has ended is handed back too. A mouse report to a program that has ended is dropped on
  purpose: where the pointer is has nothing left to say, and handing it back would let a press
  act on whatever is behind the terminal.

## 0.1.12 - 2026-09-20

### Added

- A shared file manager: a folder as a tree with every file operation on it (`FileManager`,
  `FileManagerState`, `FileManagerMsg`). It reads folders in the background, never while drawing,
  paints only the rows on screen, and leaves what opening a file means to the application
  (`on_open`). `confined()` keeps operations inside the root and refuses a path through a symbolic
  link; `following(true)` follows the folders on screen with a `FolderWatch`. The application adds
  its own items to a row's menu with `menu_items`, and a terminal with `on_open_terminal`.
- Handing a terminal program text of your own: `TerminalSession::paste(text)` sends it as a paste,
  between the bracketed-paste marks when the program turned that mode on and plain when it did
  not, so a message with line breaks arrives in one piece instead of as several half-finished
  lines. Both marks are removed from the text, so text from elsewhere cannot end the paste early
  and have its rest read as keys. The `Terminal` widget sends a person's paste through the same
  call.
- Knowing whether this is a good moment to write: `TerminalSession::last_output()` is when the
  program last wrote anything, marked by the reading thread as the bytes arrive, and
  `last_input()` is when keys were last written to it. An application waits until both the program
  and the person have been quiet. A `paste` is the application writing and does not count as the
  person's input; a person pasting into the widget does.
- The file manager also copies, moves an entry to the desktop trash (and asks plainly when there
  is no trash to take it), shows or hides hidden entries, marks a row with a sign and a tone for
  the application's own meaning (`row_mark`), and runs a long copy in the background with its
  progress and a way to stop it.
- The screenshots crate draws the symbols the framework itself shows: a third embedded font, cut
  to the characters the family uses. What the fonts cover is now fixed by a test, and another
  test asserts that every character the locales, icon sets, themes and keymaps can put on screen
  can be drawn.
- `Family::follow(app, key)` and `Family::follow_in(folder, app, key)` put one application back on
  the family's shared language, theme or icons. Only that application's file is written and the
  shared file is neither read nor written, so a settings page listing every member of the family
  can hand one member back to the shared value without changing what the whole family draws with.

### Changed

- `TerminalSession::write` (and the new `paste`) now fail once the program's end is known, as they
  always documented. A pseudo-terminal keeps taking bytes after the program is gone and nobody
  ever reads them, so the write used to answer `Ok(())` and the text vanished.

## 0.1.11 - 2026-09-20

### Added

- A first-run wizard that asks for the language, theme and icons before an application's own
  steps, writes nothing until it is finished, and can start with the detected defaults
  (`Setup`, `SetupWizard`).
- A frame limit an application chooses (`App::frame_limit`, `FrameLimit`), with fewer frames over
  a remote connection; a key is always drawn at once. `Env::remote` says whether the connection
  is remote.
- The icon set gained a launcher's categories (`category-system`, `-development`, `-network`,
  `-office`, `-media`, `-files`) and the family's own mark (`family`).
- `TerminalWatch::next_change_within` waits for a terminal change with a time limit, so a test
  does not hang on a live but silent program.
- The screenshots crate draws the card a shared link shows (`qshots::Card`): a fixed canvas, the
  application's name, one sentence and a real screen, the same on every machine.
- The recorder can right-click and hold a modifier (`right_click`, `click_with`), advance an
  application's own clock while it records (`hold_with`), and let time pass off camera (`skip`).

### Fixed

- A recording lasts exactly as long as its story, so a looping picture no longer rests on its
  last frame.

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
