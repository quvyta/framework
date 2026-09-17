## Methods

- `LogView::new(&buffer)` — shows a `LogBuffer`.
- `.min_level(LogLevel)` — hides less important lines.
- `.search(query)` — keeps lines containing `query` and highlights it; lowercase ignores case.
- `.empty_text(text)` — shown while the buffer is empty.
- `.on_copy(|lines| msg)` — sent after `c` copied lines.
- `LogBuffer::new(capacity)`, `.push(line)`, `.clear()`, `.len()`, `.is_empty()`, `.capacity()`, `.get(index)`, `.iter()`.
- `LogLine::new(level, text)`, `.time(stamp)`; `.level()`, `.timestamp()`, `.text()`.
- `LogLevel::Trace | Debug | Info | Warn | Error`, `LogLevel::ALL`, `.name()`.

## Behaviour

- Follows the tail while at the bottom; the wheel, the scrollbar or moving the cursor up stop it; reaching the bottom, `end` or a click on the lines-below note resume it.
- Keys while focused: `up` `down` or `k` `j` move the cursor, with `shift` they extend the selection; `pgup` `pgdn` page; `home` goes to the oldest line; `end` follows; `esc` clears the cursor; `c` copies.
- Mouse: a click places the cursor, `shift` click or a drag extends it, the wheel scrolls.
- Copied lines read `time level message`, one per line. Only lines that pass the filters are copied.
- When no line passes the filters the view says `quvyta.log.no-match`.
- A right click on a line keeps a selection that contains it or selects that line, then opens Copy and Raw copy; Shift+F10 and the menu key open it while lines are selected. Both copies send `on_copy` with the line count. Language keys `quvyta.edit.copy`, `quvyta.edit.raw-copy`.

## Theme keys

- `list-item` with `hover`, `selected`, `focus` — rows, shared with List.
- `log-time` — `fg`; `log-level.trace`, `.debug`, `.info`, `.warn`, `.error` — `fg`, `bold`.
- `log-match` — `bg`, `fg` of search matches; `log-more` — `bg`, `fg` of the lines-below note, whose text is `quvyta.log.below`.
- `list-header` — empty text; `scrollbar` — `track`, `thumb`.
