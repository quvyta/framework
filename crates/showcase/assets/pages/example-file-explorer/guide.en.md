## When to use

Read this example when you build a tool that browses something nested and shows details of what is selected: a file manager, a container browser, a database explorer. It is a whole small application made only of framework widgets.

## Step by step

1. Keep the data you read in the state: a map from folder to its listing (loading, ready or failed), the open folders, the current folder, its sorted entries and the preview.
2. Draw a `Tree` of folders on the left. Folders are `expandable` before they are read; opening one returns a command that reads it and marks it `.loading(true)` meanwhile; the tree shows its spinner only when the read is slow.
3. Draw a `Table` of the current folder in the middle, with a sortable name and a right-aligned size. Sort the entries in `update` when the sort message arrives.
4. When a row is selected, read the start of the file in a background command and show it in a `CodeView` or `Markdown` inside a `ScrollView`.
5. Put the path above and a status line below as `Text` spans: faint where secondary, bold where current.

## How it works

- **Nothing blocks.** Apart from the root folder, read once at start-up so the example opens ready, every folder and file is read by `Command::perform`; the view only draws what the state already holds, so the interface keeps answering while a large folder is read.
- **Three panes, no lines.** The tree, the table and the preview are separated by two cells of space and their own row surfaces; nothing is drawn between them.
- **What is shown stays until the answer comes.** Going to a folder or selecting a file does not clear the table or the preview; the new content replaces the old in one frame. A folder read that takes longer than 300 ms spins on its tree row, for at least 500 ms, so quick reads never flash.
- **Every state has a face.** A slow folder spins, an unreadable folder says so in the danger colour, an empty folder has a sentence, and a binary file is named as such instead of printed.
- **The table opens folders.** Opening a folder row moves there and opens its ancestors in the tree, so both panes agree.

## Common mistakes

- **Reading files in `view`.** Even a small `read_to_string` in `view` runs on every frame.
- **Previewing whole files.** Read a bounded prefix; logs and builds can be gigabytes.
- **Losing late answers.** Check that a preview answer still belongs to the selected file before showing it.
