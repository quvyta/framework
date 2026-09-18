## Methods

- `Handoff::new(program, |HandoffOutcome| msg)` — the program and the message delivered once the application has the screen back.
- `.arg(arg)`, `.args(args)` — arguments, in order.
- `.dir(path)` — the working directory of the program; the application's own without it.
- `.env(key, value)` — one variable; the rest of the environment is inherited.
- `.notice(text)` — a line printed on the cleared screen before the program starts.
- `.pause(true)` — waits for one key press after the program ends; off by default.
- `Command::handoff(handoff)` — asks the runtime for it.
- `HandoffOutcome::Finished { code: Option<i32> }`, `HandoffOutcome::Failed(String)`.
- `Harness::handoffs() -> &[HandoffRequest]` — what was asked for, oldest first; `HandoffRequest` carries `program`, `args`, `notice` and `pause`.
- `Harness::set_handoff_outcome(outcome)` — the answer every handoff from then on gets; `Finished { code: Some(0) }` without it.
- `DetachedHandoff::new(program, |DetachedOutcome| msg)` — a program that owns the terminal until its first line; takes `.arg`, `.args`, `.dir`, `.env`, `.notice` and `.pause` like a handoff (`pause` waits only when the program ended without a first line).
- `.on_line(|ChildLine| msg)` — the program's later lines as messages; without it they are read and dropped.
- `Command::handoff_detached(handoff)` — asks the runtime for it; it queues with the other handoffs.
- `DetachedOutcome::Detached { child: LiveChild, first_line: String }`, `DetachedOutcome::Finished { code: Option<i32> }`, `DetachedOutcome::Failed(String)`.
- `ChildLine::Line(String)`, `ChildLine::Ended { code: Option<i32> }`.
- `LiveChild` (cheap to clone, clones share the program): `write_line(&str) -> io::Result<()>`, `close_stdin()`, `kill() -> io::Result<()>`, `try_wait() -> io::Result<Option<Option<i32>>>`, `id() -> Option<u32>`.
- `LiveChild::for_tests() -> (LiveChild, TestChild)`; `TestChild`: `say(line)`, `exit(code)`, `written() -> Vec<String>`, `stdin_open() -> bool`, `killed() -> bool`.
- `Harness::detached_handoffs() -> &[HandoffRequest]`, `Harness::set_detached_outcome(outcome)` — as for handoffs; `Finished { code: Some(0) }` without it.
## Behaviour

- The handoff happens on the drawing thread and the application waits for it. Background tasks keep running; their messages are applied after the handoff.
- Order: leave raw mode and the alternate screen, show the cursor, clear the screen, print the notice, run the program with inherited standard streams, wait for a key when `pause` is on, take raw mode and the alternate screen again, push the keyboard enhancement flags again, clear the terminal and draw the whole screen.
- The program is not put in a session of its own: a `sudo` ticket is kept for the controlling terminal and the session.
- On Unix, when the application is the foreground of its controlling terminal, the program starts in a process group of its own and that group becomes the terminal's foreground until the program ends. The keys' signals (`Ctrl-C`, `Ctrl-\`, `Ctrl-Z`) and resizes reach the program and its children only; the application's signal dispositions are never changed. A program that stops is continued at once. Afterwards the application's group is the foreground again, and a child of the application stopped for reading the terminal meanwhile is continued. Without a controlling terminal the program simply runs in the application's group.
- `code` is `None` when a signal ended the program. A program that cannot be started, and a terminal that cannot be given back or taken again, both end as `Failed(reason)`.
- `pause` waits only when the program actually ran; a program that never started leaves nothing to read.
- The terminal is restored even when the program is killed or the runtime panics: the terminal guard's drop and the panic hook both leave application mode.
- Several handoffs asked for in one update run one after another, oldest first.
- A `Harness` has no terminal: it records the request and answers it with the outcome the test set, delivered in a later update like the work of `Command::perform`.

- A detached handoff runs like a handoff until the program's first line on standard output, with standard input and output piped to the application and standard error on the terminal. At that line the terminal's foreground goes back to the application, the screen is taken back and drawn in full, and the program runs on in its own process group, in the background.
- One thread reads the program's output from its start, so no line after the first is lost; a program that writes faster than the application reads waits on its full pipe. Each line wakes the loop at once.
- A program that ends before its first line ends as `Finished { code }`; one that closes its output but runs on is waited for as a handoff's program, its input closed.
- When the last `LiveChild` clone is dropped, the program's standard input closes; the application's state is dropped when the run ends, so a helper that ends at the end of its input never outlives the application by long.

## Theme and icons

- The handoff draws nothing of its own. The outcome line on this page is a `Badge` and a `Text`; their theme keys apply.

## Language keys

- `quvyta.handoff.pause` — the line printed while the pause waits for a key.
