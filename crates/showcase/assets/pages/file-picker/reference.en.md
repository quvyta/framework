## Methods

- `FilePicker::new(&browser, wrap)` — `wrap` turns a `FilePickerMsg` into your message: a function such as `Msg::Picker`, or a closure that captures what it needs, such as a screen's own conversion (`move |m| Msg::Tab(index, m)`). Any `Fn(FilePickerMsg) -> Msg + 'static`.
- `.open_on(Click)` — `Click::Double`, the default: a click selects and a double click opens a folder or chooses a file. `Click::Single`: one click does it.
- `.show(ui)` — adds the picker as a column and returns its node for sizing.
- `FileBrowser::new(folder, PickMode)`, `.extensions(list)`.
- `.open(folder, wrap)` and `.update(message, wrap)` — both return the command that reads folders. `wrap` runs once on the thread that read the folder: any `FnOnce(FilePickerMsg) -> Msg + Send + 'static`, the same function or closure the picker takes.
- `.folder()` (the folder shown), `.loading()` (the folder being read, if any), `.shows_hidden()`, `.selected_path()`.
- `FilePickerMsg::Open(path) | Loaded(path, listing) | Select(Option<name>) | Filter(text) | ShowHidden(bool) | Refresh | Chosen(path)`.
- `PickMode::Files | Folders`; `read_folder(path) -> Listing`; `FileEntry` (`name`, `is_folder`, `size`, `is_hidden`); `ListingError::PermissionDenied | NotFound | NotAFolder | Other(text)`.

## Behaviour

- The list follows List's keys: `up` `down`, `home` `end`, `pgup` `pgdn`; `enter` or a double click (two presses on one entry within `Click::INTERVAL`, 400 ms) opens a folder or the parent row, and chooses a file in file mode; a click only selects. With `.open_on(Click::Single)` a click opens and chooses. In folder mode a click on the entry the cursor started on counts as pointing at it.
- `tab` moves between the filter, the list, the hidden-files switch and the choose button.
- A click on a path segment opens that folder; the current folder's segment is not a button.
- While a folder is read, the path, filter, list, selection and focus of the folder shown stay as they are and keep working; the new folder replaces them in one frame when its answer arrives. Opening another folder meanwhile makes the earlier answer stale.
- A read longer than 300 ms shows a spinner right after the path (with "Reading folder…" where the line has room); once shown it stays at least 500 ms. An error hides it at once.
- The choose button is disabled before the first folder has been read, after an error, and in file mode until a file is selected.
- Extensions compare without case; folders always show.

## Theme keys

- `path-segment` — `fg`, `bg`, with `hover` and `selected` (the current folder); `path-separator` — `fg`.
- The picker uses the `list-item`, `text-input`, `switch-labeled`, `button.primary`, `spinner` and `spinner-label` styles.
- Icons: `folder`, `file`, `arrow-up`, `path-separator`, `error`.
- Strings: `quvyta.file-picker.*`.
