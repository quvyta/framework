## When to use

Use the picker when the user must point at a file or a folder: a project to open, a config to import, a folder to export into. When the choice is among a few known items, a select or a list is kinder.

## Step by step

1. Keep a `FileBrowser::new(start, PickMode::Files)` in your state; add `.extensions(["toml"])` to show only some files.
2. Start reading with `self.browser.open(start, Msg::Picker)` and return the command it gives you.
3. Show it: `FilePicker::new(&self.browser, Msg::Picker).show(ui)`, sized like any node.
4. In `update`, catch `FilePickerMsg::Chosen(path)` yourself and hand every other picker message to `self.browser.update(message, Msg::Picker)`, returning its command.
5. For folders use `PickMode::Folders`: files turn faint and the button chooses the open folder, or a folder inside it the user clicked or moved the cursor to. The entry the cursor starts on when a folder opens is not a choice, so opening `myapp` and pressing the button chooses `myapp`, never its first subfolder.

## How it works

- **A click selects, two open.** As in a desktop file explorer, a click on an entry only selects it, so a person can look before choosing; a double click, or Enter, opens a folder or chooses a file. `.open_on(Click::Single)` makes one click do it, for a picker where every click is already a choice. The path above the list opens a folder with one click either way.

- **Reading never blocks drawing.** Folders are read in a background command. An answer for a folder the user no longer waits for is ignored.
- **Nothing flashes while a folder is read.** The folder on screen stays exactly as it is, still usable, and the new one replaces it in a single frame when it has been read. Most reads take a few milliseconds, and showing "loading" for them would only blink.
- **The indicator waits, then stays.** Only a read that takes longer than 300 ms shows a small spinner right after the path, and once shown it stays at least 500 ms so it never blinks either. These delays follow common interface guidance: under about 300 ms people do not notice waiting, and anything shown for less than about half a second reads as a glitch. Turn on "Slow disk" in the playground to see both paths.
- **Errors do not wait.** A folder that cannot be read shows its message as soon as the answer arrives, and the spinner goes at once.
- **Errors are states, not crashes.** A folder you may not read, one that vanished or a path that is not a folder shows a danger mark, a sentence and the path.
- **The path is the way back.** Each segment above the current folder is clickable and rises on hover; long paths lose their leading segments to `…`. The first row of the list also leads to the parent folder, so the keyboard can go up.
- **Folders first.** Entries are sorted folders first, then by name ignoring case. Hidden entries stay hidden until the switch shows them.
- **Filtering keeps the selection when it can.** Typing in the filter narrows the list; when the selected entry disappears the first match is selected.

## Common mistakes

- **Calling `read_folder` in `view`.** It touches the disk; use it only inside a command.
- **Forgetting to return the command** from `update`: the folder then never changes (and the path line spins after 300 ms).
- **Clearing your own view while loading.** If you build a similar screen yourself, keep the last answer on screen until the next arrives instead of swapping it for a loading state.
- **Testing with one click.** A test that clicks an entry once now only selects it; click twice, press Enter, or build the picker with `.open_on(Click::Single)`.
- **Treating `Chosen` as another update.** It is the result; close the dialog or open the file there.
