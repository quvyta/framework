## When to use

Use the clipboard whenever the user needs to take something out of your application or bring text in: an install command, a container image tag, an API token, a log line.

- **CopyValue** is the ready answer for a value the user copies as a whole. It confirms in place, so you never need a toast for "copied".
- **Selectable text** lets people take part of a log, a document or code with the mouse; see Mouse text selection.
- **Command::copy** copies text your own code decides on, for example the selected row of a table.
- **Paste** needs nothing from you in text fields. Listen to `App::clipboard` only when your screen accepts pasted text without a focused field, such as pasting a URL anywhere on a page.

## Step by step

1. Show a value: `ui.add(CopyValue::new("cargo add quvyta-framework").on_copy(Msg::Copied))`. Enter, Space, `c` or a click copies it.
2. Hide secrets with `.masked(true)`: dots are drawn, the real value is copied.
3. Copy from `update` with `Command::copy(text)` when the text is not on screen as a value.
4. Read the clipboard for a paste button with `Command::read_clipboard(|text| Msg::Pasted(text))`.
5. Implement `fn clipboard(&self, event: &ClipboardEvent) -> Option<Msg>` on your `App` to hear copies made by widgets, menus and the copy key, and pastes no widget took.

## How it works

- **Copying goes two ways.** Every copy goes to the terminal clipboard with OSC 52, which works locally and over SSH, and to an in-process clipboard kept by the runtime.
- **Pasting tries three sources, in order.** `ctrl v`, the Paste entry of a field's menu and `Command::read_clipboard` read:
  1. the system clipboard through its tool: `wl-paste` on Wayland, `xclip` or `xsel` on X11, `pbpaste` on macOS. The tool runs without a shell, on its own thread and for half a second at most, so the screen never waits for it;
  2. the terminal's clipboard through an OSC 52 query, when the tool gave nothing. Many terminals refuse or ignore the query, so the wait is short, and a late answer is dropped instead of arriving as keys;
  3. the text copied last inside the application.
- **Nothing to paste.** When none of the three has text, Paste entries are disabled and `read_clipboard` delivers `None`.
- **The terminal's own paste still works.** Text pasted with the terminal's paste arrives as a paste event and goes to the focused widget.
- **Pastes reach widgets first.** A focused text field takes the paste; only when nothing takes it does `App::clipboard` hear `ClipboardEvent::Pasted`.
- **Selected text is copied clean or raw.** Releasing a mouse selection copies nothing. `ctrl c` and Copy in its right click menu copy it clean: without decoration such as the pillar `▌`, scrollbars and line numbers, and without the spaces that only pad a line to the edge. Raw copy takes every cell exactly as shown. Try it with the heading in the demo: paste both into the note and compare.
- **The confirmation is quiet.** The trailing `copy` word turns into a success check with `copied`, holds for a moment and blends back to the idle colour; the surface flashes once.

## Common mistakes

- **Showing a toast for every copy.** CopyValue already confirms where the user is looking.
- **Answering `ClipboardEvent::Copied` with `Command::copy`.** Copies you ask for are not reported, but copying different text on every notification still confuses the user.
- **Expecting `read_clipboard` to answer at once.** Its message arrives in a later update, after the system tool or the terminal answered.
- **Masking by changing the value.** Use `.masked(true)` so the real value is still copied.
