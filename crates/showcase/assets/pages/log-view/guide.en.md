## When to use

Use a log view for output that keeps arriving and is read from the bottom: a deploy, a build, a container's log, a background job. For a finished document use a code view; for rows people compare use a table.

## Step by step

1. Keep a `LogBuffer::new(50_000)` in your state. When it is full the oldest lines fall out.
2. Push lines in `update`: `buffer.push(LogLine::new(LogLevel::Warn, text).time(stamp))`.
3. Show it: `LogView::new(&self.buffer)`. Cloning the buffer is cheap, so this costs nothing per frame.
4. Filter with `.min_level(LogLevel::Warn)` and search with `.search(&self.query)`.
5. Say what an empty log means with `.empty_text(..)` and hear about copies with `.on_copy(|lines| ..)`.
6. Feed it from outside with background commands: a `Command::perform` that waits for the next chunk returns a message, and `update` pushes the lines and asks for the next one.

## How it works

- **It follows until you look away.** At the bottom, new lines scroll into view. Scroll up and the view stays where you are; a quiet note counts the lines below. Scroll back down, press End or click the note to follow again.
- **Levels are words in colour.** Each line carries its level as a word in the level's status colour, so the meaning survives without colour. Timestamps are faint.
- **Search narrows and highlights.** Only lines containing the query stay, with the match lit. A lowercase query ignores case; add a capital letter to match case exactly.
- **Big and fast.** Lines are stored in shared chunks and filtered incrementally: new lines are checked once, not the whole buffer every frame. Only the lines on screen are painted.
- **Copy what you need.** ↑/↓ place a line cursor, shift extends it, a click or a drag selects, and `c` copies time, level and message of the selected lines.
- **Right click to copy.** A right click on the selected lines, or on any line to take just that one, opens Copy and Raw copy. Copy writes `time level message` with single spaces, like `c`; Raw copy keeps the columns lined up as shown. Shift+F10 and the menu key open it for the current selection.

## Common mistakes

- **Rebuilding the log in `view`.** Push lines in `update`; `view` only shows the buffer.
- **Unbounded vectors.** A `Vec<String>` that grows forever eats memory; the buffer's capacity is the limit.
- **Reading a process in `view`.** Read output in a background command and deliver it as messages.
