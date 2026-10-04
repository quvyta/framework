# Changelog

Notable changes to `quvyta-framework` and `quvyta-framework-showcase`. Both packages share one
version. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
is at 0.1, so a minor release may still change the API.

## 0.1.33 - 2026-10-04

### Added

- `Preferences::detected(&i18n)`: the shared preferences this machine starts with, with no file
  behind them. Every key takes the value `Ecosystem::preferences` detects when no file holds one,
  the language the system names, the theme `monochrome`, the icon mode the terminal and the
  installed fonts allow, reduced motion off, and every one of them is `Source::Detected`. An
  application on a machine with no home folder, a desktop session with nowhere to put its settings,
  starts from these instead of resolving from a path that cannot exist. Nothing is read, so there is
  no diagnostic; paired with `Appearance::without_saving`, a change made on the appearance rows is
  applied at once and written nowhere, which cannot fail. The counterpart of `Settings::in_memory`
  for the keys the ecosystem shares.
- `Table::space_activates(false)`: Space is left to the application, for the table that holds the
  focus almost all the time — a music player's songs, where Space is play and pause everywhere
  else. The table does nothing with the key and does not use it, so it travels on to the
  application's keymap actions exactly as an unhandled key does, while Enter and the mouse keep
  selecting, checking and opening rows. A table that checks rows loses the key with it and leaves
  the marks and their click as they are. On by default, so a table behaves as it always has.
- `EventCx::is_reserved(&chord) -> bool` and `EventCx::reserved_action(&chord) ->
  Option<(Scope, String)>`: a widget that takes every key while it has focus asks whether a key
  belongs to the application and returns the ones that do unused, so quitting with `ctrl+q` and the
  application's own shortcuts keep working inside an embedded page, an embedded terminal or a coding
  tool's own screen. The answer comes from the keymap in force when the key arrived, so a rebinding
  in a keymap file moves it and the widget holds no list of its own to keep in step; the runtime
  answers from the same list of the global actions it always handles, which an application no longer
  has to read out of the keymap and copy. `reserved_action` also names the action and the scope it
  is bound in. A global action the runtime only hands to `App::action`, such as `help` and
  `palette`, is not reserved: the runtime does not act on those itself, so a screen that wants
  them keeps them itself.
- `App::cell_pixels(&self, Option<(u16, u16)>) -> Option<Msg>`: a lifecycle hook that hears the
  size of one cell in pixels, before the first frame and again whenever it changes. `Env::cell_pixels`
  answers that only while painting, and `App::resized` is told only when the columns or the rows
  change, so a font size change — the window growing with its cells, by a keystroke or in a tiling
  window manager — reached `update` with nothing to show for it, and a picture decoded for the old
  cell stayed that size on screen. The message it returns goes through `update` like every other,
  so a picture can be decoded again at the size the terminal now shows, before the frame that draws
  it. A cell and a screen size that change in one frame are told in that order, and a terminal that
  stops reporting a cell is heard too. `Harness::set_cell_pixels` reports a change the way the
  runtime does.
- The trash of a `FileManager` that has one is a place of its own, as a desktop's trash icon opens it:
  `FileManagerState::open_trash(wrap)` (or `FileManagerMsg::OpenTrash`) shows what was deleted — every
  entry under the name it has where it came from, the folder it came from and when it went, in the
  columns of the list, where the changed column carries the deletion date — and its rows put an
  entry back where its `.trashinfo` note says, delete one for good, or empty the whole trash. The
  place sorts and narrows the way any folder does, since a note already knows when an entry went.
  `FileManagerMsg::Leave` comes back to the folder. Nothing is ever overwritten: a name that is
  taken again is asked about, with a name that is free there offered, and both destructive
  operations ask first, in the danger colour. An entry whose note is missing or says nothing usable
  is still listed, under the name it has in the trash with its origin unknown, and never stops the
  rest of the list. An entry of the trash is named by what it is called there, so nothing that acts
  on a key below the root can reach one, and a key the trash does not list, `..` or `../x` among
  them, reaches nothing at all. See `Trashed` and `FileManagerMsg::Restore`, `RestoreAs`,
  `Purge`, `PurgeConfirmed`, `EmptyTrash`, `EmptyTrashConfirmed`, `Trashed`.
- `SeekBar`: a progress bar a person can click and drag to a position, the way every music and
  video player offers one. It draws exactly the cells and tones a `ProgressBar` draws, so a player
  can put one where the other stood without the picture changing. `SeekBar::new(fraction).on_seek(
  |fraction| msg)` seeks to the centre of the cell a press landed on and, while the button is held,
  to every cell the pointer moves to — past either end of the track it holds at 0 or 1, and the
  release ends it. Focused, ← and → move a twentieth of the track and Home and End go to its ends.
  Under the pointer the bar steps a tone lighter and the cell under the pointer takes the accent, so
  where a seek would land is the one cell that stands out; `.hover_label(|fraction| text)` writes
  the time there in the library's own tooltip, anchored over the pointer's cell, and `.percent(false)`
  gives the percentage's cells to the bar, and `.variant("success")` colours it as it colours a
  progress bar. Without `on_seek` the bar is a picture, as a progress bar is: no focus, no hover, no
  pointer.
- `BigText` draws every Latin letter, not only A to Z: the Turkish `Ç Ğ İ Ö Ş Ü`, the Western
  European `À Á Â Ä Å È É Ê Ë Ì Í Î Ï Ñ Ò Ó Ô Ö Ø Ù Ú Û Ü Ý Ÿ ß Æ Œ` and the punctuation `'`,
  `!`, `?`, `&`, `(`, `)`, `,`, `/`, beside the digits and `:`, `.`, `%`, `-` it already had. A
  title in Turkish reads whole now, which is what a music player naming the song that is playing
  or a desktop heading the day needs; before, `Ş`, `Ç` and `İ` came out as blanks.
- A lowercase letter in `BigText` is drawn with the Unicode uppercase of its character, with the
  Turkish rule for the two i's: `ı` is always `I`, and `i` is `İ` while the active language is
  Turkish or Azerbaijani and `I` in every other. The language in force while painting decides, so
  the same text follows the language the person is reading.
- `BigText::fits(text)`: whether every character of `text` has a big form, so an application can
  ask which of the two it is about to show, or how much room to leave for it.

### Changed

- An application with background work running no longer wakes fifty times a second to look for its
  result: every message a `Task` or `Command::perform` hands over wakes the loop itself, so the
  result is applied the moment it arrives and an application whose work only waits (a reader of a
  terminal tab, a timer, a watch) sleeps between frames as soundly as an idle one.

### Fixed

- A `BigText` with a character this font has no form for, Cyrillic or Japanese or an emoji among
  them, drew that character as a space and read as a row of holes. It is drawn whole as plain bold
  text now, the way a too narrow area already was, and measures as the one row it paints.
- `Image` with half blocks keeps a picture's shape the way the terminal really shows it. A cell is
  rarely exactly twice as tall as it is wide — 9 × 19 pixels is usual — and `Fit::Contain` and
  `Fit::Cover` counted every half cell as one square pixel, so a picture prepared at an area's
  columns times the cell width by rows times the cell height came out a few columns narrow with a
  strip of ground beside it, and a click aimed at the picture landed a few cells off near its
  edges. Where the terminal reports the pixels of its cells (`Env::cell_pixels()`), a half block is
  now as wide as a cell and half as tall, so such a picture fills its area edge to edge and a
  square picture stands as a square on screen. A terminal that reports no cell size draws exactly as
  before, one square pixel to a half cell, and `Fit::Center` keeps one pixel to a half cell either
  way.

## 0.1.32 - 2026-10-03

### Added

- `Process::collect_watching(keep, cancel, on_tail)`: `collect` that hands the end of the output
  kept so far to `on_tail` while the child runs, once new output has come and at most every 100 ms,
  so a person sees a build or a test run go by. It is cut as `Keep` cuts; a silent child is never
  reported, and the last call has the end of the output.
- `Keep::after_exit(wait)`: `Process::collect` is done with a command `wait` after the command
  itself ended, even when something it left running still holds the output open (`npm run dev &`);
  what it left behind goes with its process group and the command's own exit code comes back. A
  cancel or a `Keep::limit` now asks the child and its group to end with `TERM` first and kills
  them only after that grace (`wait`, or two seconds), so what it writes on its way out is kept.
- A `refresh` icon in the default set, for reading something again: a page, a folder, a list.
  Nerd Font `nf-fa-refresh`, Unicode `↻` and ASCII `R`, one cell in every mode, so every
  application draws the same shape for it.
- A `FileManager`'s list and icons can be narrowed to the entries whose name holds a text,
  whatever the case and the Turkish i: `FileManagerState::set_filter` and `filter`, and
  `FileManagerMsg::Filter` for the application's own key to open it. A field to type in stands
  over the rows while a filter is set; Enter puts the cursor on the first entry left, Esc shows the
  folder whole again, and stepping into another folder lets it go.
- `TextInput::on_cancel(msg)`: a message for Esc in the field, such as closing the filter it types.
- A `FileManager`'s list sorts by a press on its name, size or changed title, and a second press
  turns the order round; folders always come first and the cursor stays on its entry.
  `FileManagerState::sorted_by`, `set_sort` and `sort` with `Sort` and `SortBy` (`Name`, `Size`,
  `Changed`, `Kind`) set and read the order, and `FileManagerMsg::Sort` carries a press. An order by
  size or date reads the details of the whole folder in the background; an entry not read yet
  stands last and takes its place when it comes.
- The screenshot fonts carry the Miscellaneous Symbols block as well, such as the `⚙` a coding
  tool marks a setting with, one cell wide like the other symbols.
- `StatusLine`: one line that says how things stand, for the half of the cases a toast does not
  cover. An application that wants "3 of 3 containers healthy" beside a Retry button, or that a
  container is not responding while the screen shows it, has no widget to reach for: a toast is a
  layer the runtime owns, which outlives the view that raised it and leaves on its own, and a badge
  is a pill with no room for a sentence. `StatusLine::new(text).tone(ToastKind)` gives the sentence
  the sign and the colour of one kind, both of them, so a status colour never comes alone and a
  line and a toast of the same kind look like one family. `.action(button)` puts a real button at
  the end of it, which takes focus and answers with its own message. In a narrow area the sentence
  wraps under itself and keeps its column while the sign stands beside it, and the button takes a
  row of its own rather than being cut. The Status line page of the showcase shows all four kinds
  and the wrapping.
- `Field::warning`: a field can now say something worth knowing without refusing anything. A form
  could only say "this is wrong", so a value that is allowed but surprising — a name a stopped
  container still holds, an image that is not in the shared registry — either had to be written as
  a caption beside the control or had to become an error that blocked a submit the user should have
  been allowed to make. `Field::warning(Option<impl Into<String>>)` draws the message where an error
  is drawn, in the warning colour and with the same sign a toast of that kind shows, so the meaning
  is never colour alone. It never reaches `FormErrors`, which is what keeps the submit going, and
  when a field carries both, the error takes the one place under the control and the warning comes
  back when the error is gone. The theme key is `field-warning`.
- `TextInput::suggestions(rows)`, `.on_suggestion(|index| msg)` and `max_suggestions(n)`: a text
  field that offers what the person may want to type, such as the places an address bar has been or
  the names a package search found. The list opens under the field as soon as the text has moved on
  from the text the field had when it gained focus, so focusing a field with a value in it never
  opens anything, and it is exactly as wide as the field, so its edges stand where the field's do and
  a name longer than the field is cut there rather than pushing the list wider. It is drawn like
  every other list of choices: the chosen row is raised in tone with the `▌` pillar beside it, with
  a note on the right and an icon before the label where the application gives one.
  `Suggestion::new(label)` builds a row, `.detail(text)` and `.icon(key)` fill it in, and
  `↑`/`↓` move the chosen row while `Enter` with one chosen takes it and `Enter` without one submits
  as it always did. A click on a row takes it, `Esc` closes the list and keeps the text, and a press
  anywhere else closes the list and still reaches whatever it landed on. Which rows belong in the
  list, and how they are found, stays with the application; a field with no `on_suggestion` to
  send a chosen row to is given no list and behaves exactly as before. The TextInput page of the
  showcase shows a package search built on it.
- `Tabs::max_tab_width(cells)`: a cap on how wide a `TabWidth::Fill` tab grows. The tabs still
  share the strip equally, but none of them takes more than this, so a strip of a few tabs keeps
  its names near each other instead of spreading them over an empty line, and the rest of the
  strip stays empty (or keeps the room the add button needs, which still stands right after the
  last tab). Many tabs are unchanged: their share is already below the cap and they keep the
  readable minimum.
- `ButtonRow`, a row of `Button`s that stays the same row at every width: while they all fit it is
  a plain row of buttons, and where they do not, the last ones move, from the end, into a control
  that opens a menu of them. Choosing an entry sends that button's own message, so a toolbar keeps
  every action when the terminal narrows instead of dropping or cutting the ones at its end. The
  menu is the popup a tab strip and a breadcrumb already share, the control is a button in every
  way a theme can see, and Tab reaches it after the buttons.
- `LevelBars::new(values)`: a row of vertical level columns for a spectrum or for a set of meters,
  one value per column between zero and one, measured in eighths of a cell, for a visualiser, a
  channel meter or anything else the application computes. The columns are as wide as the area
  allows: values that do not fit are merged by averaging their neighbours, so a narrow area still
  shows the whole spectrum, and values with room to spare make the columns wider instead of
  leaving the rest of the row empty. `.gap(cells)` and `.bar_width(cells)` say how they sit,
  `.peaks(values)` puts a thin cap on each column at the level it last reached, `.mirror(true)`
  grows them up and down from the middle row, and `.gradient(true)` blends the colour from the
  theme's base tone to the accent as a column deepens. In a terminal with the sixteen standard
  colours the gradient is left out, and where there are no block glyphs the level is carried by
  tone instead, so the same code reads everywhere.
- `qframe::widgets::IconTile`: a desktop icon as a widget, a glyph over a centred name in a cell of
  ten columns by three rows, with `.selected`, `.cursor`, `.backed`, `.color` and `.faint`, so a
  desktop's floor and a file manager's grid draw their entries the same way and an application
  building either does not write the look twice. `IconTile::WIDTH`, `HEIGHT`, `SIZE` and `PILLAR`
  are the size one tile takes and the column kept for the accent pillar, and
  `IconTile::shown_name` is the name as a tile shows it. The glyph is a key of the icon set, so it
  follows the glyph mode, or a `Glyph::literal` drawn as it is; the name too wide for a tile is cut
  with an ellipsis and reads on whatever ground the tile stands in, so a tile over a picture says
  `backed(true)` and stands on the theme's surface tone. A tile draws and answers nothing: the
  surface that lays tiles out owns the pointer.
- `CardGrid::bare_cards(true)`: cards stand on the ground the grid is given instead of on a
  surface of their own, each taking the whole of its cell, for a grid whose cards draw themselves
  such as a grid of `IconTile`s. The grid's own input is unchanged, the card under the pointer
  lends it the pointer and the card the keys are on lends it its focus, so a card that lights under
  the mouse or breathes its own pillar does so while the grid has them.
- The media and music keys of the default icon set: `media-play`, `media-pause`, `media-stop`,
  `media-next`, `media-previous`, `media-shuffle`, `media-repeat`, `media-repeat-once`,
  `media-volume` and `media-muted` for a player's transport, and `music-note`, `music-album`,
  `music-artist`, `music-playlist` and `music-queue` for what a track is and where it is in. A
  player or a library that carried its own set for these now draws the same sign as every other
  application in the ecosystem, like the marks of the main menu and of a status strip. Each is one
  cell in all three glyph modes and no two of them share a sign, since a transport is read by shape
  alone. The showcase's icon page shows them.
- `Widget::keys()`: a widget says which keys it takes while it has the focus, and the help layer
  lists them at the top of its "This screen" group. A screen needs no hint for the keys a table,
  list, tree or field already takes, and the list cannot fall behind the widget: a table with no
  row to open says nothing about Enter, a list with no check marks says nothing about Space, and a
  tree that cannot expand says nothing about the side arrows. The framework's own widgets declare
  their keys and take their arrows from the icon set, so an ASCII terminal shows `^v` where a
  Unicode one shows `↑↓`. A layer takes the keyboard with it, so the keys listed are those of the
  widget that had the focus when it opened: a help opened while a table has the focus lists the
  table's keys, not the filter inside the layer. `HelpLayer::hint` stays for the keys of a screen
  that no widget takes, and now lists them after the widget's own.
- `KeyHints::action_labelled_first(scope, action, label)`: a keymap action with the application's
  own words, read first and the last to drop when the bar narrows, for the key that says what the
  screen's main action does now, such as `enter` saying "details".
- `Table::lazy(columns, count, row)`: a table whose rows are built only as they are drawn, so a
  frame costs the rows on screen whether the table holds twenty rows or a million. A `Fit` column
  of such a table fits its title, since its rows are not all known.
- `Process::stdout_to(file)`: a child's standard output goes straight into a file of the
  application's, so an archive unpacked with `gzip -dc` or a `tar` writing an artefact costs no
  memory of ours however large it is; the child writes into the file itself and not one byte of it
  passes through us. The error stream is still read, so a failure stays recognisable as a failure:
  `collect` keeps the end of it within `Keep` and `run` hands every line as `Line::Err`. A cancel
  and a `Keep::limit` end the child's process group as before, so nothing more is written to the
  file once the child is gone.
- The screenshot fonts draw the marks real coding tools put on a terminal screen: whole blocks of
  arrows, technical signs, geometric shapes, dingbats and further symbols, among them `⏺`, `⎿`,
  `✻`, `✳`, `✢`, `✽`, `✱`, `⏵`, `⏸`, `↯`, `⬝`, `⎔`, `✘` and `↵`, each one cell wide. A recorded
  terminal tab running such a tool now draws every frame.
- `Reel::missing()` and `Recording::missing()`: the characters a recording drew as a box because no
  embedded font has them.
- `Tabs::busy(index, true)`: a thin spinner before a tab's name in the accent colour, turning on
  the framework's own clock, such as a tab whose program is writing; with reduced motion it is one
  dot standing still. `Tabs::status(index, token)` puts a dot in a theme colour in the same cell,
  such as a tab whose work is done and waits for the person, and a busy tab with a status turns in
  that colour. The mark takes the padding cell before the name and the spare cell the name slides
  into, so no other tab moves when it comes or goes, and a narrow tab keeps it while its name
  shortens.
- `qframe::text::highlight(code, language)` and `Token`: the highlighter the code view colours
  with, public so an application that draws code in a widget of its own — a file beside a tree, a
  diff it lays out itself — colours it exactly the same way instead of writing a second one. The
  byte ranges it returns cover the whole text in order, and `Token::style_variant` names the
  theme's `code-token.<kind>` style for each one, so a theme that restyles code reaches an
  application's own drawing too.
- `TextArea::language(Language)`: code is coloured while it is being edited, in the same
  `code-token` colours a `CodeView` gives a file of that language, so an editor and a notes page
  can take a language without gaining a second widget for it. Only the colour changes: the cursor,
  the selection, the undo, the scrolling and the line numbers are the field's own, a selection
  covers the colours under it, a disabled area keeps its muted text and the placeholder stays
  uncoloured. `Language::Plain`, the default, leaves every text area as it was, and
  `Language::from_file_name(name)` picks the language of the file being edited. The text is
  coloured when it or the language changes rather than on every frame, so typing in a file of
  thousands of lines pays for the colouring once.
- `qframe::desktop::set_default(&XdgDirs, mime, app_id)`: make a program the default program of a
  kind of file, which is the "make this the default for every file of this type" a file manager's
  "Open with" dialog and a desktop both offer. It changes one line of the person's own
  `$XDG_CONFIG_HOME/mimeapps.list` and nothing else anywhere: every other line, section and comment
  is put back byte for byte, a kind's line for a program the person does not want is left alone, and
  a file under `$XDG_CONFIG_DIRS`, a running desktop's `<desktop>-mimeapps.list` and every
  `.desktop` entry are never written. The file and its folder are made when the person has none, the
  write goes through a temporary file and a rename, and `Change::Added` or `Change::Replaced` says
  which of the two happened, so an application can say what it did. Reading the file again with
  `Apps::load` gives the new program as that kind's default.
- `qframe::desktop::EntryDetails` and `Apps::details(id)`: what a program's desktop entry says
  besides the command that opens a file with it, read from the same keys in the same pass — its
  `Comment` and `GenericName` in the person's language, its `Keywords` for a search, its
  `Categories` for a menu, its `Path`, and its `NoDisplay`, `Hidden` and `TryExec` keys with
  whether the program `TryExec` names is installed. A launcher lists, searches, groups and pins
  programs with these, so it reads the machine's desktop entries once instead of parsing every
  `.desktop` file a second time.
- `Apps::load_including(dirs, lang, path_var, include)` with `Include::MISSING`, `Include::HIDDEN`,
  `Include::ALL` and `Include::NONE`: a read that keeps the entries a plain `Apps::load` leaves out. A
  program whose own `TryExec` program is not installed is kept with `installed == false`, so a
  launcher can list it, say that it cannot start it yet and offer its package; an entry marked
  `Hidden`, which says the person deleted the program, is kept with `hidden == true` so it can be
  seen. Neither is ever offered as a program that opens a file. The default read is unchanged.
- `DesktopApp::launch_command()`: the command that starts a program with nothing to open, which is
  what a launcher pins, lists and starts on its own. It is the entry's own `Exec` line with the
  codes that take a file or a URL left out and no empty argument in their place, so `foo %U --x`
  runs as `foo --x`; `%c`, `%k` and `%i` are filled in as in `command` and `%%` is a percent.

### Changed

- The media keys of the default icon set draw the transport's own signs in Unicode, `⏸` `⏹` `⏭` `⏮`
  `⇄` `↻` and `♪` for a note, now that the screenshot fonts carry them; ASCII is unchanged.
- A `FileManager` shows its icons view as a desktop does: each entry is an `IconTile`, its glyph
  over its centred name in a cell of ten columns by three rows, the chosen ones raised with the
  pillar down their side, in a `CardGrid` of bare cards. A name too long for a tile is cut with an
  ellipsis. The keys, the mouse, the selection, the box, dragging and the menus are as before.

### Fixed

- `Runtime::harness_in` draws in true colour, as `Harness::new` does. With nothing read from the
  machine it took the terminal for one of 256 colours, so a test read no background colour from
  its cells.
- On a remote terminal the focus pulse and the cursor blink stand still, as with reduced motion.
  Each breath after a key was a stream of frames over the connection: a page of a list cost tens
  of times the bytes of its text.
- The panel and the body of a `SidePanel` are two places to the runtime. They shared one key, so
  an unnamed widget in the body was the same widget as the one in the panel: keys, focus and what
  a widget remembers could reach the wrong one.
- A `FileManager` showing a large folder as a list or as icons no longer does work for every entry
  on every frame. The entries are listed once each time the folder changes and a row's icon, colour
  and details are worked out only when it comes on screen, so a key in a folder of a hundred
  thousand entries costs what it costs in a small one. The size column has a fixed width now, so it
  does not change width while the rows scroll.
- A `Reel` no longer stops at a character the embedded fonts lack. The character is drawn as `▯`,
  the box a terminal shows in its place, in a `Shot` as well, and the recording goes on; a test that
  wants whole frames asserts that `missing()` is empty.
- A problem found while reading a desktop entry or a `mimeapps.list` now names the column its key
  is written at, not only the line, so an application that shows what went wrong can point at the
  key itself: `file:line:column`. A line that is not UTF-8, or a whole database line that makes no
  sense, still points at the start of its line, which is where the fault is.
- A `Command::focus` returned by the update that closes a dialog now keeps its target. The closing
  dialog used to hand focus back to the widget that opened it over the command, so the person's
  next key went to the opener.

## 0.1.31 - 2026-09-29

### Added

- `List::label_first(true)`: a row too narrow for its label and its detail cuts the detail first,
  with `…`, and leaves it out when fewer than four cells of it would remain, so the label stays
  whole as long as it fits on its own. Off by default, since some rows are about their detail.
- `RowMark::plain_sign(icon)`: a file manager row's icon in the row's own colour, so it rises and
  takes the selection with the name; for a row that says nothing in colour, such as a workspace
  root.
- `.wrap(true)` on `List`, `Table`, `Tree`, `Menu`, `RadioGroup` and `Segmented`, and
  `list.wrap(true)` inside `SettingsList::show`: the next-row key on the last row goes on to the
  first and the previous-row key on the first goes to the last, as in a menu, skipping headers,
  gaps and disabled rows. A page and Home/End still stop at the ends, and Shift with an arrow never
  wraps a range. Off by default, so every list keeps stopping at its ends until asked.
- `SettingRow::hint(text)`: an explanation that takes no room of its own. It shows under the row,
  over the rows below, at once while the row is the keyboard's row and after the hover delay while
  the pointer rests on it, wrapped inside the label column so it never covers a control; the row's
  control keeps every key.
- `Appearance::updates_in_background()`: the update notice's switch writes the shared file on a
  thread of its own, so a settings page never waits for the disk to turn it over. The switch shows
  the new value at once, and a file that cannot be written puts the switch back where the person
  left it. What became of the write arrives as a message of the application's own:
  `AppearanceSave::Saved`, or `AppearanceSave::Failed(reason)` to show as a toast. Hand every
  change to `Appearance::update_saving(change, &mut settings, saved)` and the outcome back to
  `Appearance::saved(&save)`. Every other row is saved where it is drawn, as before, and an
  application that never asks for updates is unaffected.
- `qframe::widget::natural_size(&widget, &env, available)` and `Size::MAX`: the cells a widget
  covers when drawn, asked of the widget itself, for a layout decided in `update` where there is no
  `View`, such as where a list splits from its detail; no copy of a widget's padding can drift.
- `Checkbox::description(text)`, a faint explanation under the label in the label's column, and
  `Checkbox::label_column()`.
- `Field::value(text)`: a value to read in place of a control, lined up with the controls, its
  label faint. `CopyValue::labelled(label)`: a faint label above the value, and a value too wide
  for its box written whole above it.
- `TableCell::role(role)`: a typography role for a cell, such as `faint`, which takes the selected
  row's colour on that row. `Text::role("heading")`: the look of a settings list's own headings for
  a heading above anything else.
- `TreeNode::meter(fraction, tone)`: a small block meter at the right of a tree row, dropped
  before the name would be cut.
- `Popover::match_anchor_width(true)`: a popover that opens exactly as wide as the widget it is
  anchored to, cut to the screen, instead of as wide as its content. A layer of suggestions under a
  search field then starts at the field's left edge and ends at its right one, whatever its longest
  line, and a line longer than the field is cut there rather than pushing the layer wider; the
  content is measured at that width, so it lays itself out to fit. A popover on its own is
  unchanged. The Popover page of the showcase shows the same layer with the option off and on.
- `Keymap::label_for(scope, action)`: how the first chord of an action is written, for naming
  the key in a sentence that follows the person's own bindings; `None` when it has no chord.
- `KeyHints::action_first(scope, action)`, before every plain hint and the last to drop when the
  bar narrows; `KeyHints::action_labelled(scope, action, label)`, the key from the keymap and the
  words from the application; `KeyHints::faint(bool)` and `Button::faint(bool)`, a step quieter
  for a screen that has gone still, a faint button still pressing and speaking up under the pointer
  or the keyboard.
- `CommandPalette::max_rows(n)`; without it the list now takes up to half the screen's rows, never
  fewer than ten, so a tall screen shows more commands at once.
- `Appearance::label(&i18n, Shared)`, `IconMode::label(&i18n)` and `PillarStyle::label(&i18n)`: the
  words the Appearance box uses, for an application that shows the same preference elsewhere.
- `qframe::text::fuzzy(query, text)` and `FuzzyMatch` (`score()`, `positions()`): the fuzzy
  matching the filter, the command palette and the pickers find with, public so an application's
  own search finds exactly what they find.
- `qframe::desktop::shell_words(line)`: a command line such as `$EDITOR` split into words the way a
  POSIX shell splits it, quotes and backslashes followed and nothing expanded, so a quoted path
  with spaces stays one word.
- `FileManager::menu_for(|target| ..)` and `MenuTarget`: the application's own menu items told the
  row's key, path, whether it is a folder and what an action acts on, so no list of folders has to
  be kept beside the manager; `qframe::widgets::path_of(root, key)` gives the path of any key.
- `Ecosystem::data_dir(app)`: where a member keeps the records a person would miss, such as
  favourites, beside the ecosystem's config, state and cache folders.
- `qframe::storage::display_home(path)` and `display_home_with(path, home)`: a path as a person
  reads it, with the home folder written `~`, and `/home/alice` left alone for the home `/home/ali`.
- `qframe::version::newer(candidate, installed)`: semver's order, pre-releases included, the one the
  update notice uses; a text that is not a version is never newer.
- `qframe::install`: offer to install a missing program's package in front of the person instead
  of telling them to copy a command. `Install::package(name)` finds this machine's package manager
  (`pacman`, `apt-get`, `dnf`, `zypper`, `apk` or `brew`, in that order) and builds the exact
  command, such as `sudo pacman -S --needed libarchive`, with `sudo` only when not root and no
  "yes" flag, so the manager asks its own question. `.name_for(Manager::Apt, "libarchive-tools")`
  gives a package its name on one distribution, `install.confirm(msg)` is the question that shows
  the command line, and `install.handoff(on_finish)` hands the terminal to the package manager,
  where the password is typed, and keeps its last lines on screen until a key is pressed.
  `Install::package_with` takes the program search and the root question, so tests never run a
  package manager. A showcase page shows the question on a pretended machine.
- `Runtime::harness_in(&folder, width, height)`: the application's own `Runtime` opened in a
  `Harness`, started exactly as `run` starts it (theme, icon, locale and keymap files and sources,
  settings, preferences and membership), with the member's ecosystem folder moved to `folder` and
  nothing read from the machine. An application that builds its runtime in one function called by
  both `run` and its tests now has a test that fails when a `.member(..)` or `.locale_source(..)`
  line of that setup is lost.
- `PaintCx::focus_spot(rect)`: a widget taller than a view names where the keyboard is inside it,
  and a `ScrollView` shows that part when the widget takes the focus. `SettingsList` names its
  keyboard row.
- The `Terminal` widget answers the questions a program asks its terminal, as every real terminal
  does: where the cursor is (`CSI 6 n`), that it is well (`CSI 5 n`), what kind of terminal it is
  (`CSI c`), which colours it draws text, ground and cursor in (`OSC 10`, `11`, `12` with `?`, the
  theme's colours) and whether a mode is on (`CSI ? mode $ p`). An editor that asks for the ground
  colour and then for the cursor now picks its light or dark colours at once instead of waiting
  and printing a warning. The answers go back in the order the questions came.
- `TerminalChange::Copied(text)`: a program inside a `Terminal` offers text to be copied, and the
  offer is heard like a title or a notification. A program that copies a selection, a yanked line
  or a path sends OSC 52 (`52;targets;base64`); the text arrives decoded, and an application
  answers it with `Command::copy(text)`, which puts it on the system clipboard with OSC 52 — so it
  works over SSH — and on the application's own, so pasting still works where the terminal has no
  clipboard of its own. This is what a shell's own copy commands, a pager and an editor's yank need
  to reach the person's clipboard from inside an embedded terminal. A read request, a program asking
  what the person copied, is never reported and never answered: the clipboard belongs to the person.
  An offer that is not base64, or not UTF-8 once decoded, is dropped, and what arrives is bounded
  like every OSC string.
- Focus reports in the `Terminal` widget: a program that turns xterm's mode 1004 on
  (`CSI ? 1004 h`, off with `l`) is sent `ESC [ I` when the terminal takes the focus and `ESC [ O`
  when it loses it. Programs that draw differently depending on whether the window has the focus —
  editors, pagers, anything with a status line — need this to know, and a program that turns the
  mode on while the terminal already has the focus is told at once, since it cannot see the state it
  missed. Only a change is reported, never a frame's worth of the same answer, and only for a live
  terminal that is not `read_only()`. The reports are the terminal's own rather than the person's
  typing, so they move neither `TerminalSession::last_input` nor `TerminalSession::line_pending`, and
  an application waiting for a quiet terminal is not held off by them.
- `Harness::member_in_with_env(app, env, ecosystem, &folder, "code", width, height)`: starts a
  member test with the application's own `Env`, so its locale files and keymap are available to
  the test while the shared language, theme, icons and reduced-motion preferences still apply.
  `Harness::member_in` remains the built-in-environment form.
- `EmptyState` can offer several equal action buttons in one centred row, or stack them on narrow
  screens without losing their individual messages. `EmptyState::tone(ToastKind)` gives an empty
  state the matching success, warning, danger or info colour and a visible status sign, including in
  sixteen-colour and ASCII terminals.
- `EventCx::clicks()`: how many presses in a row the press or release being handled is, counted
  once by the runtime for every widget: the same button on the same cell within 400 ms counts on
  (2 for a double click, 3 for a triple click), anything else starts at 1, and other events are 0.
  A widget tells a double click from two clicks without keeping time itself, and in a `Harness`
  two clicks on one cell with no time advanced between them are a double click.
- `TaskCx::recv(&receiver)` and `TaskCx::recv_timeout(&receiver, duration)` with `RecvWait`
  (`Timeout`, `Cancelled`, `Closed`): a task waits for the next item of a channel another thread
  feeds, such as a connection handing over what it read, and still stops within 50 ms when it is
  cancelled. In a `Harness` a waiting task rests like a sleeping one, so no test hangs on it.
- `IconButton::selected(bool)`: the glyph in the accent colour for a control that is on, such as a
  filter or a bookmark, with hover, focus and presses as usual; theme key `icon-button:selected`.
- `Button::on_middle_press(msg)`: a message of its own for a middle click, as a browser closes a
  tab with it; a button without it ignores the middle button as before.
### Fixed

- In the ASCII glyph mode the hint lines of the command palette and the help layer write the
  arrows and Enter from the icon set (`^v`, `>`) instead of characters such a terminal may lack.
- A screen nobody touches no longer draws frames: the breathing focus pillar and every other
  pulsing theme colour come to rest at the end of a breath five seconds after the last key, click
  or paste, and the text cursor stops blinking and stays drawn; the next input wakes both. Before,
  a focused list over SSH kept sending about 1.6 KB a second while the person was away.
- `ScrollView` no longer jumps to the top of a long list that takes the focus: it shows the list's
  keyboard row, a widget taller than the view that is already partly on screen stays where it is,
  and a widget that was clicked into never scrolls the view. A warning above a settings list stays
  in view when the list takes the focus, and clicking from a field back into the list chooses what
  was clicked instead of moving the page under the pointer.
- `TerminalSession`: a program that asks the terminal about the keyboard protocol again and again
  without reading its input no longer stops the session for good. The answer is now written in as
  much as the program's input will take, so what it cannot have been waiting for is left out, and
  the session closes again even when the program's input is full.
- `Process::clear_env()`: a child that gets nothing of the environment but what it was given. A
  child normally inherits everything, which is right for a build or a package manager and wrong
  for anything that should not be steered by the shell the application happened to be started
  from: `PATH`, `HOME` and `LANG` all change how a program behaves. With the environment cleared
  the child sees exactly what the application said it would see, so the variables it needs are
  given to it, `PATH` first since that is what it looks other programs up with.
- `Process::collect(keep, cancel)`: runs a child and keeps only the end of its output, within
  `Keep`, and gives it back as one piece of text. Both of the child's streams land on one pipe, so
  the text is in the order the program wrote it, and nothing is held beyond what `Keep` asks for
  while it runs: a build that writes megabytes, a program that never stops printing and a log far
  too long to show each cost the size or the line count asked for. `Keep::bytes`,
  `Keep::lines` and `Keep::limit` are the three knobs, and the limit ends the child and its
  process group the way cancelling does. `Collected` says in `trimmed`, `timed_out` and
  `cancelled` how the run ended, and its `text` is lossy UTF-8 cut between characters.
- `i18n::check`: the checks a multi-language application runs on its own locale files in a test.
  `locales(files)` asks for every key of the reference file in every other file and no key of its
  own, the same `{name}` placeholders wherever both files have a message, and the plural forms each
  language's own plural rule chooses from — Russian needs `one`, `few` and `many` where English
  needs two. `same_keys`, `same_placeholders` and `plural_forms` are those three one at a time, and
  `own_words(files, at_least)` asks how much of a file must be words of its own: a file whose
  values are all copied from English passes every other check, so a share is the only thing that
  can see it. Left out of both the count and the share are the values that are the same in every
  language by nature, a pure placeholder, a number, a shape such as `{hour}:{minute}` and a key
  name like `ctrl+s`, and plural tables, whose forms are a language's wording as a whole. Every
  problem is a `Problem` with the file, the line, the column, the key and what is wrong, and its
  `Display` is what an editor opens. The files are read with the parser and the plural rules the
  runtime reads them with, so a file these calls call complete is one the runtime can draw, and a
  malformed file is a problem rather than a panic.

### Changed

- An open `Select` list and a popup menu (a breadcrumb's hidden segments) go round their ends
  with ↑ and ↓, as a context menu always has; Home/End and a page still stop at the ends.

### Fixed

- A copy pasted into a closed folder of a `FileManager` opens the folder, as a move does, so the
  selected copy is on screen and the keys go on from it.
- A switch wrapped in a `Tooltip` inside a settings row takes Space and Enter: the list offers a
  key to the innermost control first and then to each wrapper around it, as focus would. A
  `Tooltip::on_focus(true)` there shows while its row is the keyboard's row.
- In the ASCII glyph mode the hint lines of the command palette and the help layer write the
  arrows and Enter from the icon set (`^v`, `>`) instead of characters such a terminal may lack.
- A screen nobody touches no longer draws frames: the breathing focus pillar and every other
  pulsing theme colour come to rest at the end of a breath five seconds after the last key, click
  or paste, and the text cursor stops blinking and stays drawn; the next input wakes both. Before,
  a focused list over SSH kept sending about 1.6 KB a second while the person was away.
- `ScrollView` no longer jumps to the top of a long list that takes the focus: it shows the list's
  keyboard row, a widget taller than the view that is already partly on screen stays where it is,
  and a widget that was clicked into never scrolls the view. A warning above a settings list stays
  in view when the list takes the focus, and clicking from a field back into the list chooses what
  was clicked instead of moving the page under the pointer.
- `TerminalSession`: a program that asks the terminal about the keyboard protocol again and again
  without reading its input no longer stops the session for good. The answer is now written in as
  much as the program's input will take, so what it cannot have been waiting for is left out, and
  the session closes again even when the program's input is full.

## 0.1.30 - 2026-09-27

### Added

- `Env::cell_pixels()`: the size of one terminal cell in pixels, or `None` where the terminal
  reports no pixels. An application preparing a picture for an area of the screen (a page
  rendered off screen, a preview, a wallpaper) asks for the area's columns and rows times this
  size, so the terminal shows it pixel for pixel instead of guessing ten by twenty. The runtime
  reads it at start and at every resize, a font size change included, and draws a new frame when
  it changes. Sixel pictures are shrunk to the same value, and now follow a font size change
  that leaves the columns and rows as they were. `Harness::set_cell_pixels` sets it in tests.
  The showcase's terminal panel shows it.
- `Spinner::delayed(busy)`: a spinner that shows only for work slow enough to notice. Nothing is
  drawn until the work has run 300 ms, and once shown it stays at least 500 ms, so quick work never
  blinks an indicator and slow work never flickers one away. Its cells stay blank while hidden, so
  what sits beside it never moves; it keeps its timing itself and asks for a frame exactly when it
  is due to appear or disappear. The Spinner page of the showcase demonstrates it with a quick and
  a slow load.

### Fixed

- `Terminal`: Shift+Enter and Ctrl+Enter no longer reach the program as a plain Enter, which sent
  the message a chat program was holding instead of starting a new line in it. A program that
  turned on the kitty keyboard protocol gets `CSI 13;2u` and `CSI 13;5u`; any other program gets
  `ESC CR`, the bytes of Alt+Enter, which such programs read as a line break. The session keeps
  the protocol's flag stack for each screen and answers `CSI ? u`, so a program asking whether
  the protocol is there hears back at once. Only the first flag is followed, and only Enter
  changes under it.

## 0.1.29 - 2026-09-24

### Added

- `Runtime::member(ecosystem, app)`: one call starts an application as a member of the
  ecosystem. It loads the application's settings (`Settings::load_member`) and the shared
  preferences (`Ecosystem::preferences`, in the language files the runtime loads), applies both
  before the first frame, and follows them while the application runs. `member_in` does the same
  in a folder of the application's choosing. Settings or preferences also given with `.settings`
  or `.preferences` are used as given and not read twice.
- Live follow of the shared preferences: a member watches the ecosystem's folder with the
  system's own events, on a thread that sleeps until something changes. When `quvyta.conf` or the
  application's own file changes, the preferences are resolved again without writing anything and
  only what differs is applied: language, theme and icons, and reduced motion and the pillar when
  the application's own file changed them. A value the application chose for itself stays. The
  same values written again change nothing, so an application saving its own change never loops.
  Where the folder cannot be watched, the application keeps what it started with.
- `App::preferences(&self, &Preferences) -> Option<Msg>`: hears a member's shared preferences
  before the first frame (after `graphics`, before `init`) and whenever another application
  changes them. The default ignores them.
- `Appearance::refresh(preferences)`: an open appearance section takes preferences resolved again,
  so its rows and boxes show them and the next change is saved where the box now says.
- `Harness::member_in(app, ecosystem, folder, app_id, width, height)` and
  `Harness::poll_preferences()`: a member started in a test's own folder, and its files read again
  the way the runtime does when they change.
- `storage::Member`, `storage::MEMBERS` and `Ecosystem::members()`: every member of Quvyta with its
  id, the id of its settings file, its title, package and command, in one place. qdesk's settings
  file is `desktop.conf`, the showcase's `showcase.conf` and the launcher's `launcher.conf`; a
  screen that lists the members reads them from here instead of guessing.
- `Ecosystem::settle(app)` and `settle_in(folder, app)`: one look at an application's own file for
  shared keys written there as fixed values. A value equal to the one in `quvyta.conf` goes back to
  following the ecosystem, a different one stays as the person's choice, and the file keeps
  `shared-checked = true` (`Settings::SHARED_CHECKED`) so a later choice is never undone. For the
  members that used to write "in every Quvyta application" into their own file alone.
- `Setup::appearance_only()`: an application without steps of its own needs no wizard when the
  shared file already holds a language, a theme and icons; its file is written following them.
- `Setup::asks_appearance()`: whether the wizard shows the appearance step.
- `Shared::ReducedMotion` and `Preferences::reduced_motion()`: reduced motion is shared like the
  language. `quvyta.conf` may hold `reduced-motion = true`, an application's own file a boolean of
  its own or `"quvyta"`; `Ecosystem::set` takes `true` or `false` for it and writes a boolean.

### Changed

- The setup wizard leaves out the appearance step when `quvyta.conf` already holds a language, a
  theme and icons, and opens on the application's own first step; Finish makes the application
  follow the shared values. Without a whole shared file it asks as before. `Setup::step()` still
  counts the appearance step as 0.
- A settings file holding nothing but the `shared-checked` mark does not count as a setup, so the
  wizard still opens over it; an empty file still counts, as before. Every member's settings know the mark, and self-healing keeps it.
- A finish that cannot be saved says why on the step Finish was pressed on, not only on the
  appearance step.
- Reduced motion is a shared preference. The `Appearance` section gives it the "In every Quvyta
  application" box the language has, and a switch turned with the box checked saves
  `reduced-motion = true` in `quvyta.conf` and `reduced-motion = "quvyta"` in the application's
  file, where it used to save `reduced-motion = true` there alone. The pillar stays the
  application's own. `Preferences::apply` switches reduced motion too, `Runtime::preferences` and
  `Runtime::member` start with it and a member follows it live; `QUVYTA_REDUCED_MOTION` still
  decides over all of them. A new shared file leaves the key out, and a missing key reads as motion,
  as before. `Ecosystem::settle` turns an application's `reduced-motion = false` back into
  following when the shared file does not hold the key; a `true` stays.
- `Shared` is `#[non_exhaustive]`, so a later shared preference is not a breaking change; a `match`
  on it needs a `_` arm. `Shared::ALL` has four entries.

## 0.1.28 - 2026-09-24

### Added

- `TerminalSession::line_pending()`: whether the person has typed something since the last Enter
  (or Ctrl+C or Ctrl+U, which throw the line away). The widget's keys, `write` and a person's
  paste into the widget count; the application's own `paste` and mouse reports do not. An
  application that writes into a terminal somebody uses waits for it to be false, so its text
  never joins a half-typed line that stopped for a pause.
- `Window::on_drag` and `WindowDrag`: a window's moves and resizes can arrive with how far the
  pointer has gone since the button went down (`total_dx`, `total_dy`) beside the step. An
  application that holds a window at the screen's edge or at a smallest size places it from
  where it started plus the totals, and the window waits until the pointer is back over it
  instead of turning the moment the pointer does. `WindowEvent` is unchanged; the showcase's
  window demo uses the totals.
- Icon keys for a status strip, in the Nerd Font, Unicode and ASCII columns of the built-in set:
  `cpu`, `memory`, `battery-full`, `battery-half`, `battery-empty`, `battery-charging`,
  `terminal`, `session` (a terminal multiplexer's), `network-down` and `network-up`. The Unicode
  glyphs stay inside Geometric Shapes, so a strip of them keeps one cell each.

### Changed

- A `Tree` where no row can open keeps no column for chevrons, so a flat list of leaves (a
  favourites bar) starts in the same column as a `Menu` beside it instead of two cells to its
  right. A tree with any openable row is drawn as before.
- The frame limit merges the pointer's motion too. A drag, the pointer moving with no button held
  and the wheel wait for the limit like the application's own work: every event still reaches
  the widgets and the application, and the screen shows the latest state at most one gap later.
  A key, a paste, a button going down and a button coming up are still drawn at once, and so is
  the first motion after a rest of one gap. A two-second drag at 5 frames a second now writes 12
  frames instead of one for every motion (202 in the measured drag, 25 415 bytes against 1 839);
  at the local default of 60 the pointer waits 16 ms at most.
- Sixel pictures on a local terminal are cut into pieces around whatever stands on them, as kitty
  pictures are: icons, menus and windows over a wallpaper stay text and the rest stays pixels,
  where before the whole picture turned to half blocks. The screen remembers which cells show
  which pixels and sends pixels only for the cells that lost them: selecting an icon sends
  nothing, or the two cells its bar gave back, and a window dragged over a wallpaper sends the
  strip it uncovered, 14 KB a step on a grainy full-screen photo at 100 × 30 against 188 KB when
  every piece was sent again. Over a remote connection a sixel is still shown only while nothing
  covers it, since there pieces sent again cost more than half blocks.

### Fixed

- The last row of a sixel picture whose height is not a whole number of six-pixel bands no
  longer shows a thin line of canvas under the picture: those cells take the colour of the
  missing pixels as their ground. The pixels are worked out at the cells' full height and cut,
  instead of squeezed into fewer rows, so pieces of one picture meet without stretching.

## 0.1.27 - 2026-09-24

### Added

- `PaintCx::takes_text()`: a widget that takes typed text says so while it paints, and then
  gets every key however fast keys come; see Fixed. `TextInput`, `TextArea`, `Terminal` and the
  filter of the command palette and the help layer call it. A widget painted with its
  container's focus lent to it marks that container too.
- The file manager selects from the keyboard the way a desktop file explorer does, in all three
  views: Shift with the arrows, PgUp/PgDn, Home or End stretches the selection from where it
  started, Ctrl+A selects every entry shown, and Esc leaves only the entry under the cursor
  selected. Ctrl+A never selects the root or the shown folder's own row.
- `Table::multi_select` and `CardGrid::multi_select` take the same keys: Shift with the steps,
  Ctrl+A and Esc, as `Tree::multi_select` already did with Shift and Esc. `Tree::multi_select`
  gains Ctrl+A for every row shown.
- In the file manager's list and icons, the shown folder's own row takes a drop for the folder
  above it, lit while the drag is over it, as a desktop explorer's path takes one for a parent;
  with Ctrl held at the release it copies. At the root it takes nothing.

### Fixed

- Fast keys are no longer lost. A second Enter or Space within 100 ms of the first was taken for
  a held key and given to no widget, so text that arrives in one read (tmux `send-keys`, a slow
  SSH link, dictation) lost its spaces and Enters: `alpha beta gamma` arrived as
  `alpha betagamma`. The rule is now:
  - Where the keys go to a widget that takes text, nothing is guessed: every Enter and Space
    reaches it, repeats the terminal reports included, since holding Space in a field or a
    terminal types spaces as holding a letter types letters.
  - Anywhere else (a button, a list) a key the terminal reports as a repeat, or an Enter or
    Space within 100 ms of the same key, still counts as the press before, so a held key on a
    terminal without the kitty keyboard protocol presses a button once.
  - Only an uninterrupted run is guessed held. A held key repeats alone, so a second Space after
    another key is a new press however soon it follows; this also keeps typed words apart in
    an application's own widget that does not call `takes_text`.
  Whether keys arrived in one read is not used to decide: repeats of a held key that queue up
  while a slow frame is drawn arrive in one read too, so it cannot tell typing from holding on a
  button, and where text is typed no guess is made at all.
- `Terminal`: programs that move the cursor with HVP (`CSI row;col f`), such as btop, no longer
  draw their whole screen on one line; HVP is drawn as the CUP it means. A sequence with a
  private marker, a sub-parameter or an intermediate byte is left alone.
- `Terminal`: the DEC line drawing set (`ESC ( 0`, in G0 or G1 with Shift Out and Shift In) is
  drawn as lines, so programs such as ncdu draw boxes instead of `lqqk`. `ESC ( B` and a full
  reset return to ASCII. Both rewrites follow a sequence split across reads.
  Autowrap mode (`CSI ?7 l`) is still not followed: a line reaching the last column wraps.

### Changed

- A box or Ctrl+A over the file manager's tree no longer puts the root in the selection: the
  root is where the person is, not an entry to cut, copy or drag.
- Esc on a `Table` or `CardGrid` with several rows selected reduces the selection to the cursor's
  row instead of going on to the parents; with one row or none selected it goes on as before.

## 0.1.26 - 2026-09-24

### Added

- `Harness::set_remote(bool)`: a screen test draws what a remote connection gets, as
  `Env::remote()` answers it in every view that follows; a harness stays local until told.
- `Env::remote_session()`: whether this process runs over SSH, by the rule `Env::remote` uses,
  read from `SSH_CONNECTION` and `SSH_TTY` alone, so an application can ask before the runtime
  starts without loading a second environment.
- Sixel pictures: where `Env::graphics()` is `Graphics::Sixel`, `Image` is painted by the
  terminal in real pixels while nothing at all is painted over it, shrunk with the box filter to
  the pixels of its cells on screen (the cell size the terminal reports for its window, or 10 × 20)
  and reduced to a fixed 6 × 7 × 6 palette, so it suits SSH too. Its height is cut to whole
  six-pixel bands so it never reaches below its last row. A menu, a dialog's backdrop or anything
  else over any of it draws it with half blocks for that frame, and it is painted again once
  uncovered, moved, when a cell under it changes and after a handoff; cells a vanished sixel
  covered are written again. An idle screen writes nothing, and the encoding is kept while the
  picture is painted. Switching between kitty and sixel at runtime leaves nothing of the other
  behind. The showcase's image page names sixel as the way pictures are drawn.
- `App::graphics(&self, Graphics) -> Option<Msg>`, a lifecycle hook like `App::resized`: it hears
  `Env::graphics()` before the first frame (after the first size, before `init`) and again
  whenever it changes, such as when the glyph mode is switched to ASCII and back, so an
  application decodes a picture at the size the terminal shows and decodes again when that
  changes. A value already reported is not reported again; the harness's `set_graphics`,
  `set_depth` and `set_glyph_mode` report a change as the runtime does. The method has a default,
  so existing applications compile unchanged.
- `Graphics::can_draw()`: whether a picture is drawn at all, true for everything but
  `Graphics::None`. It is the question `Image` asks itself, so an application that would rather
  show nothing than the image's "cannot show" state (a wallpaper) asks it first instead of
  keeping its own copy of the rule.
- `ImageData::decode_bytes(&bytes, max)`: a picture held in memory, such as one built in with
  `include_bytes!`, decoded and shrunk exactly as `decode_file` does; it has no name.
- `ImageData::EXTENSIONS` (`png`, `jpg`, `jpeg`, `gif`, `webp`) and `ImageData::reads(path)`: the
  file types the decoder reads, for `FileBrowser::extensions`. A test holds the list to the
  formats compiled in. The showcase's image page lists them.
- `List::activate_on(Click)`, as `Table`, `Tree` and `CardGrid` have it: with `Click::Double` a
  click only selects a row and a double click within `Click::INTERVAL` activates it; Enter
  activates either way.
- `FilePicker::open_on(Click)`: `Click::Single` keeps the picker opening and choosing with one
  click. The showcase's file picker page has a switch for it.

### Fixed

- `Image` follows `Env::graphics()` alone. With `QUVYTA_GRAPHICS=none` it drew half blocks on a
  terminal that could, and with `QUVYTA_GRAPHICS=halfblock` it refused to draw at 16 colours,
  although the variable is meant to win over everything; now `none` shows what the picture is and
  a forced `halfblock` draws.
- `FilePicker` no longer chooses a file, or opens a folder, the moment it is clicked. A click now
  selects the entry, the way the file manager and a desktop file explorer do; a double click or
  Enter opens or chooses. In folder mode a click on the entry a folder selected by itself when it
  opened counts as pointing at it, so Choose folder takes that entry. An application whose tests
  click an entry once to choose it clicks twice, presses Enter, or asks for
  `open_on(Click::Single)`.
- A kitty picture no longer shows through windows, launchers and help panels in half blocks.
  Whether a cell under a picture was dimmed by a backdrop was guessed from its colours, and any
  grey space on a grey ground (the gaps between words in a window) passed for a dimmed picture.
  `PaintCx::tint`, which paints dialog backdrops and window shadows, now records where it blended
  and by how much; a cell counts as dimmed only when those records turn the picture's mark and
  ground into exactly its colours, and anything else painted there covers the picture.

### Changed

- A kitty picture with something in its middle (an icon, a window, a desktop gadget) stays in
  real pixels around it. The cells left showing are split into rectangles, each row's runs merged
  downward while they line up, and the picture is placed once in each with its own crop and
  placement number; neighbouring crops meet at the same pixel, so no seam shows. Cells under a
  backdrop or a shadow are drawn with half blocks in the same frame, dimmed, while the rest stays
  pixels. Only a picture cut into more than 64 rectangles falls back to half blocks for that
  frame. Moving a window over a picture writes only placement commands, and an idle frame still
  writes nothing.
- Over a slow SSH link a kitty terminal is no longer taken for half blocks for good. The graphics
  probe still waits 150 ms at most, so starting never waits on the network; a kitty `OK` that
  arrives later is no longer only swallowed: it switches `Env::graphics()` to `Graphics::Kitty`
  from the next frame, and `App::graphics` hears it.
- The image guide advises decoding smaller over a remote connection (`Env::remote()`): about
  twice the half-block density, since every pixel crosses the link and a grainy photo hardly
  compresses.

## 0.1.25 - 2026-09-24

### Added

- `Image` on a terminal that speaks the kitty graphics protocol (`Env::graphics()` is
  `Graphics::Kitty`): the terminal draws the picture itself, in real pixels, and the widget paints
  no half blocks, only the plain `canvas` ground under it. The pixels are sent once, zlib
  compressed, at the size `ImageData` keeps; every later frame only places them, so a screen where
  nothing moved still writes nothing. A place no longer used is deleted and a picture shown nowhere
  is freed from the terminal's memory; after a handoff everything is sent and placed again, and
  leaving the application frees them all. Every command carries `q=2`, so no answer can arrive as
  keys. The picture sits under text: what is painted over one side of it (a menu, a panel) cuts it
  to the part left showing, and something in its middle, or a dialog's dimmed backdrop, draws it
  with half blocks for that frame. The showcase's image page says which way it draws.

### Fixed

- The parts of an `AppShell` (header, sidebar, body, footer) and the two panes of a `Splitter`
  are separate widgets again. All of them took the same place, so they and the first widget in
  each shared one identity: a key the body claimed, such as a file list's Ctrl+C, was looked for
  in the header instead, and state kept for one part could land on another.

## 0.1.24 - 2026-09-24

### Added

- `Env::graphics()` and `Graphics::{Kitty, Sixel, HalfBlock, None}`: how a picture can be drawn
  in this terminal. On Unix the runtime asks the terminal once as it starts, a kitty graphics
  query and a device attributes request, waiting 150 ms at most; the answers are read from the
  terminal before the input parser starts and never arrive as keys. 16 colours or ASCII glyphs
  give `None`, and inside tmux or GNU screen (`TMUX`, `STY`) kitty and sixel become half blocks.
  `QUVYTA_GRAPHICS=kitty|sixel|halfblock|none` decides over all of it. Tests ask nothing:
  `Env::builtin` gives half blocks and `Harness::set_graphics` answers as another terminal would.
  The showcase's getting-started page shows what was detected.
- `FileManager::id(name)`: names the rows, the tree, the list or the icons, whichever is drawn,
  so `Command::focus(name)` gives them the keyboard after an application takes the person to
  another folder. The rows are one widget in all three views, so rows that have the keyboard keep
  it when the view changes. `show` now answers with the column holding the rows in the tree too,
  as it already did in the list and the icons; a name given there takes no focus, so an
  application that named the tree through `show(ui).id(name)` moves the name to `id`.
- `Breadcrumb::faint(true)`: the path drawn a step quieter, the current place too, for a place
  the person cannot open, such as a folder that cannot be read. The levels still rise on hover
  and still open. Themes style it with `crumb.faint` and `crumb.faint-current`.
- `FileManager`: Ctrl+X, Ctrl+C and Ctrl+V cut, copy and paste entries, as in a desktop file
  explorer, in the tree, the list and the icons. Cut and copy act on the selection, or the entry
  under the cursor when nothing is selected; paste goes into the folder the list and the icons
  show, and in the tree into the folder under the cursor. A name already taken is refused and
  said, as from the menu. The keys are the rows' only while they have focus and no text is
  selected with the mouse, so text fields and mouse selections copy and paste text as before.
- `NodeMut::on_clipboard(ClipboardKey, msg)`: a node claims Ctrl+X and the keys of `copy` and
  `paste` while focus is inside it. The focused widget and text selected with the mouse keep them
  first.
- `Env::load_with(&dirs, lookup)`: the application's files loaded as `Env::load` loads them, with
  every variable it reads answered by `lookup` and the operating system's own language never
  asked. A test that loaded the real files took the machine's language and region with them, so
  in English the week began on Monday on a Turkish machine and on Sunday elsewhere.
- `TextArea::variant("plain")`: the area is paper, not a raised field. It keeps the tone of
  whatever it sits on in every state; the pillar shows hover and focus, a danger pillar marks
  invalid text, and the cursor and the selection look as before. Themes draw it with the new
  `see-through` flag on `text-area`, which leaves the ground unpainted.
- `ContextMenu::on_left_click(true)`: a left click opens the menu too, at the pointer, where a
  right click would, and a left click on the area while it is open closes it, so an area such
  as a status bar item works as a button for its menu. Keys, choosing and the right click stay
  as they are; a child that takes presses itself keeps its left click. Off by default.
- `TerminalBuilder::env_remove(name)`: the program starts without the variable at all, not with
  an empty value, whether it came from the application's own environment or an earlier `env`.
  The last `env` or `env_remove` of a name wins. For a variable whose mere presence changes what
  the program does, such as `TMUX` when it starts tmux.
- Pictures, behind the new `image` feature (off by default). `ImageData::decode_file(path, max)`
  reads PNG, JPEG, GIF (its first frame) and WebP, recognised by their first bytes, and keeps
  them shrunk to fit within `max` pixels, so a 4000 by 3000 photo shown in a 200 by 60 area keeps
  about 57 KB rather than 36 MB. `ImageError` says in a plain sentence why a file could not be
  shown: missing, unreadable, not a picture, damaged. `ImageData::from_rgb` takes pixels an
  application already has. `Image::new(&data).fit(Fit::Contain | Fit::Cover | Fit::Center)`
  draws it with half blocks, two nearly square pixels a cell, in every terminal with 256 colours
  or more and over SSH; shrinking averages the pixels covered, and the cells are worked out once
  and again only when the area, the fit or the picture changes. Cells the picture does not reach
  keep what is under it, so a picture can be a wallpaper other widgets are drawn on. With sixteen
  colours or ASCII glyphs an empty state says what the picture is and that the terminal cannot
  show it. The showcase has an Image page.
- `PaintCx::pointer_shape(rect, shape)` and `PointerShape`: a widget asks, while it paints, for
  the mouse pointer to take a shape over part of itself, and the runtime tells the terminal with
  OSC 22 only when the shape under the pointer changes, inside the frame's synchronized update;
  a frame that changes no cell and no shape still writes nothing. Only foot, kitty and WezTerm
  are sent it, never inside tmux or screen; `QUVYTA_POINTER_SHAPES=on` or `off` decides for any
  terminal. The pointer gets its usual shape back before a handoff and when the application
  ends. `Harness::pointer_shape()` answers what a test's pointer asks for. A `Window` uses it:
  the pointer is a resize arrow over each edge and corner, and keeps it for a whole resize.

### Changed

- Every side of a `Window` resizes with a plain drag. The left column and the top left, top right
  and bottom left corners are handles now, like the right column and the bottom row: before, the
  left and top sides took alt and the right button. The top side is the title strip, which still
  moves the window, so only its two end cells resize there. The marks moved one cell left to
  leave the top right corner to the right edge, and the handles light under the pointer as
  before. A drag on the left or top side sends `WindowEvent::Resize` with `WindowEdge::Left`,
  `TopLeft`, `TopRight` or `BottomLeft`, which an application already applies by moving the
  window and changing its size by the opposite amount.
- `Command::focus(name)` on a widget that takes no focus itself focuses the first widget inside
  it that does. A name put on what `FileManager::show` returns keeps reaching the rows, in the
  tree too, where `show` now returns the column the rows are drawn in.

### Fixed

- In 256 colours a half block (`▀`, `▄`) takes the nearest palette entry for both its halves. It
  was reduced like text, so an upper half close in colour to the lower one was pushed to a far
  entry to stay readable, which would streak a smooth picture.
- A frame equal to the one on screen writes nothing to the terminal. Each message drew a frame,
  and even when no cell changed about forty bytes went out (the synchronized update markers, the
  cursor, the colour reset): over SSH a clock that shows minutes but ticks every second sent
  bytes all the time. The frame is now compared with the one shown and skipped when they match;
  after a program had the screen, the next frame is still written whole.
- A menu that opens with a left click closes with a second left click on the same place, also
  when what it covers takes the pointer itself, as a tooltip does. The first click closed the
  menu and the same click, reaching the area next, opened it again.
- The showcase's folder watch page no longer hangs a screen test that starts it. Its wait had no
  end, and a screen test runs background work in place; it now waits with a bound and waits
  again when nothing changed, as its guide now recommends.

## 0.1.23 - 2026-09-24

### Changed

- `FileManager`'s mouse works the way a desktop file explorer's does, in the tree, the list and
  the icons. A click only selects, so it can start a drag or a selection instead of opening
  something; a double click (two presses on one entry within `Click::INTERVAL`, 400 ms) or Enter
  opens: a file through `on_open`, a folder by stepping into it or, in the tree, by opening or
  closing it, where the chevron and ← → still do that with one click. Ctrl+click and Shift+click
  select several in every view, a drag from the free space draws a box in the `text-selection`
  tone that selects what it covers (adding to the selection with Ctrl), and a drag from the
  selection onto a folder moves it there, or copies it when Ctrl is held at the release
  (`FileManagerMsg::DropCopy`). A terminal that does not report Ctrl with the pointer moves; a
  release anywhere but a folder does nothing; nothing is overwritten. The flat views show the
  selection in the selection tone instead of a check beside each row. `open_on(Click::Single)`
  keeps a click that opens.

### Added

- `Click::Single | Click::Double` and three layers for rows a pointer picks, each off until
  asked for: `activate_on(Click)` on `Tree`, `Table` and `CardGrid`; `box_select(bool)` on the
  three, a box drawn from the free space that selects the rows it covers; and `on_copy_drop` on
  the three for a drop released with Ctrl held. `Table` and `CardGrid` gain `multi_select` (Ctrl
  and Shift clicks, Space) and `droppable`, which answers with a `RowDrop { rows, into }`.
- `qframe::i18n::active_code()`: the code of the active language (`tr`, `pt-BR`) where there is
  no `Env`, in `update`, `init` and the other `App` methods. It is the code the view reads from
  `env.i18n().active()`, also right after `Command::set_locale`, so an application can pick text
  in the person's language from a source that is not a locale file.
- `ContextItem::detail(text)`: a faint note on the right of a menu row, in the muted tone, that
  says why an entry cannot be used (`bsdtar needed`). It sits before the shortcut or the submenu
  arrow, is drawn on disabled rows too and has its own theme key, `context-item-detail`. The menu
  widens to fit it; in a narrow space the note is cut before the label, and left out when fewer
  than four cells remain.
- `DesktopApp::launch` starts the program in the file's folder, for terminal and graphical
  programs alike, as desktop file managers do: relative paths and "Save as" begin beside the file.
  The harness records the folder: `HandoffRequest::dir` and `OpenRequest::dir` hold what
  `Handoff::dir` and `Open::dir` gave, `None` without it.
- `MimeDb::comment(mime, lang)`: what a kind of file is called in words ("Rust source code" for
  `text/x-rust`), from the `<comment>` lines of shared-mime-info's `mime/<kind>.xml`, the
  person's data folder first. An alias is followed to the kind's own name; the comment in `lang`
  wins, then its base language (`pt` for `pt-BR`), then the one with no language.
  `MimeDb::comment_with_diagnostics` also reports a broken file, which is skipped. The Open with
  page of the showcase shows it above the type.

### Fixed

- A list the application reads again on its own word, after an archive is unpacked or a program
  it handed the screen to returns, shows its rows' size, date and permissions again without
  waiting for a key. The list asked for them only when the person was quiet, which such a read
  does not end.

## 0.1.22 - 2026-09-24

### Added

- The person's own folders by their real names: `storage::user_dir(UserDir::Desktop)` reads the
  desktop's line of `user-dirs.dirs`, so a Turkish desktop gives `~/Masaüstü` and a German one
  `~/Schreibtisch`, and falls back to the English name in the home. Every XDG user folder is
  there (`UserDir::ALL`), `user_dir_in(which, home, config)` reads only the folders it is given,
  and `documents_dir()` is now the Documents case of it. The file-kind icons read the same file
  by the same rules.
- Icons by kind of file, so a person knows what a file is before reading its name.
  `icons::file_kind(name, folder, executable)` answers with a `FileKind`: its icon key and its
  `KindFamily`. The name alone decides, in order: the whole name (`Cargo.toml`, `Dockerfile`,
  `PKGBUILD`, a `README` or `LICENSE` however it goes on), an ending of several parts
  (`.tar.gz`, `.pkg.tar.zst`), the extension, a folder's telling name (`.git`, `node_modules`),
  and only then the executable bit. Letter case never matters and nothing is read from disk.
  More than a hundred whole names, nearly five hundred extensions and nearly forty folder
  names; a hidden folder that says nothing more has an icon of its own.
- The built-in icon set has a `file-*` or `folder-*` key for each of 140 kinds. In a Nerd Font
  each draws its own glyph, named in a comment; in Unicode and ASCII each draws its family's shape
  (`◇` `&` code, `◩` `%` pictures, `▣` `@` archives and so on), one cell and never bracketed.
- `icons::UserFolders` knows the folders of a home (Desktop, Downloads, Music…) by the names the
  person's language gives them, from `user-dirs.dirs`, and only where they are: in the home.
- `FileManager::kind_icons(true)` draws each row's icon by its kind, in the tree, the list and the
  icons. The icon has no colour of its own, as the plain ones have none, and a `RowMark` sign
  still wins over it. `FileManager::kind_tones(true)` colours the icons by family, folders in the
  accent; it is not drawn in sixteen colours or in ASCII. `FileManager::user_folders` gives the
  home whose folders it knows.
- `FolderEntry::executable`: whether a file whose name says nothing of its kind may be run. It is
  the one thing read with a folder beyond the names, and only for such files.
- `qframe::desktop`: which program opens a file, read from the desktop's own databases as the
  freedesktop specifications describe them, so every Quvyta application gives the answer the
  person's file manager gives. `MimeDb` names a file's kind from shared-mime-info's `globs2`
  (weight, then case-sensitive, then the longest pattern) and from its first 4 KiB when no pattern
  knows the name, and follows `subclasses` and `aliases`, so Rust source is plain text. `Apps`
  reads the installed programs' desktop entries and every `mimeapps.list`; `for_mime` and
  `default_for` list a kind's programs and the one to use, and `Openers::for_file` answers both for
  one file. `DesktopApp::command` fills the `Exec` line in without a shell, so a quote or a space in
  a file name stays in its one argument. `DesktopApp::launch` starts a terminal program through a
  `Handoff` and a graphical one through `Open::program`, and says there is no graphical session
  instead of trying when `DISPLAY` and `WAYLAND_DISPLAY` are empty. `XdgDirs` is built by hand in
  tests, so they never read the system's folders. Broken files and lines are skipped with a
  diagnostic. The showcase has an Open with page.

### Fixed

- A child given a width in cells is measured at that width, the one it is drawn at. A column
  `.width(Length::Cells(80))` was measured with the whole row's room, so a paragraph in it seemed
  to take fewer lines than it was drawn in, and a centred page cut its last rows: on a wide screen
  the button under the text was not drawn at all.
- A legend's names keep their colour behind a dialog. The style key `legend` was defined in no
  theme, so the names were drawn with no colour of their own and a dialog dimmed them into the
  ground, in true colour as well. The built-in themes now give them the dim text colour, and a
  theme without the key falls back to it.

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
