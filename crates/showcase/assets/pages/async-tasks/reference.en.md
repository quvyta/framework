## Methods

- `Task::new(label, |cx: &TaskCx<Msg>| -> Result<Msg, String>)` — background work; `Ok` delivers a message, `Err` fails with a reason.
- `.on_event(|TaskEvent| msg)` — start, progress and outcome as messages.
- `.id()`, `.label()` — known before the task starts.
- `Command::task(task)` — starts it on its own thread.
- `Command::cancel_task(id)` — asks it to stop; does nothing once it has finished.
- `TaskCx::progress(fraction)`, `TaskCx::note(text)` — report; `TaskCx::send(msg)` — any other message.
- `TaskCx::sleep(duration) -> bool` — waits, waking early and returning `false` when cancelled.
- `TaskCx::recv(&receiver) -> Option<T>` — waits for the next item of a channel another thread feeds; `None` when the task is cancelled or every sender is gone. A cancel ends the wait within 50 ms.
- `TaskCx::recv_timeout(&receiver, duration) -> Result<T, RecvWait>` — the same, giving up after `duration`; `RecvWait::Timeout`, `Cancelled` or `Closed` says why no item came.
- `TaskCx::is_cancelled()`, `TaskCx::id()`.
- `Tasks::new()`, `.apply(&event)`, `.entries()`, `.get(id)`, `.running()`, `.clear_finished()`.
- `TaskEntry` — `id`, `label`, `fraction`, `note`, `outcome` (`None` while running).
- `TaskEvent::Started`, `Progress`, `Finished`; `TaskOutcome::Done`, `Failed(reason)`, `Cancelled`.
- `TaskList::new(&tasks).show(ui)` — the rows; `.on_cancel(|id| msg)` adds cancel buttons; `.empty_text(text)`.

## Behaviour

- Order of delivery: `Started` during the update that returned the command, `Progress` as reported, the `Ok` message, then `Finished`.
- A cancelled task always ends as `Cancelled`, even when its work returns `Ok`.
- A panic inside the work ends as `Failed("the task `label` panicked")`.
- `TaskList` rows: spinner or status icon, label, note or outcome word, a 22-cell progress bar when a fraction was reported, and a cancel button when `on_cancel` is set. Finished rows keep the columns aligned.
- In the `Harness`, `cx.sleep` follows the fake clock and every render waits until tasks are asleep or finished; a task that works ten seconds without sleeping fails the test with a clear message. A task waiting in `cx.recv` rests like a sleeper, so no render waits for it; an item the test sends reaches it on its own thread and what it sends then is applied at a later step. `recv_timeout` counts its time on the fake clock.

## Theme and icons

- Built from `Spinner`, `ProgressBar`, `Button` and `Text`; their theme keys apply.
- Icons `success`, `error`, `check-partial`; colours `success` and `danger` with those icons.

## Language keys

- `quvyta.tasks.done`, `quvyta.tasks.cancelled`, `quvyta.tasks.cancel`, `quvyta.tasks.empty`.

## Child process

- `Process::new(program)` — a child with pipes and the application's own environment.
- `.arg(arg)`, `.args(args)`, `.dir(path)` — the command line and the working directory.
- `.env(key, value)` — one variable for the child; the rest of the environment is inherited.
- `.clear_env()` — the child gets nothing of the environment but what `.env(..)` set, `PATH` and `HOME` included.
- `.pty(cols, rows)` — runs the child on a pseudo-terminal of that size instead of pipes.
- `.stdout_to(file)` — sends the child's standard output straight into that file, so nothing of it passes through the application's memory; only the error stream is read. On a pseudo-terminal the terminal is the child's standard output, and wins.
- `.no_stdin()` — the child reads an empty input (`/dev/null`) instead of the terminal, and on Unix runs in a process group of its own.
- `.run(&cancel, &mut on_line) -> io::Result<ProcessOutcome>` — runs it, delivering every line; returns an error when the child cannot be started or the pseudo-terminal cannot be opened.
- `.run_with_overwritten(&cancel, &mut on_line, &mut on_overwritten) -> io::Result<ProcessOutcome>` — runs it like `run` and also hands every frame a `\r` overwrites to `on_overwritten`, as a `Line` tagged with its stream.
- `.collect(keep, &cancel) -> io::Result<Collected>` — runs it and keeps only the end of what it wrote, giving it back as one text.
- `Keep::bytes(n)` — keeps the last `n` bytes; `.lines(n)` keeps the last `n` lines as well, the line being written counting as one; `.limit(duration)` ends the child when the time is over.
- `Collected` — `text`, `outcome`, `trimmed`, `timed_out`, `cancelled`.
- `Line::Out(text)`, `Line::Err(text)` — standard output and standard error, kept apart with pipes.
- `ProcessOutcome::Finished { code }` — `code` is `None` when a signal ended the child; `ProcessOutcome::Cancelled`.

## Behaviour

- Lines arrive one by one, without their newline, and the last line is delivered even without a trailing newline.
- A `\r` overwrites the line being built instead of starting a new one, so a progress bar stays one line; a terminal's `\r\n` still ends a line.
- With `run_with_overwritten` a frame is delivered when the byte after its `\r` is not a `\n`: `\r\n` and `\r\r\n` stay plain line ends, an empty frame is skipped, and output that ends right after a `\r` delivers its last frame. Colour codes and `ESC [K` are left in the text. Frames come through a pipe and a pseudo-terminal alike; with `run` they are dropped and never queued.
- With pipes the two streams are read separately and never merged; with a pseudo-terminal both land on one line, so only `Line::Out` appears.
- `cancel` is asked between lines. When it turns true the child is killed, its pending output is dropped and the outcome is `Cancelled`. With `no_stdin` on Unix the child's whole process group is killed, so the programs it started end too, unless they moved to a group or session of their own; without it only the child itself is killed, because a child that reads the terminal cannot live in a group of its own (the system would stop it at its first read).
- Bytes that are not UTF-8 become the replacement character rather than being dropped.
- With `.stdout_to(file)` the child's standard output is the file as it is, the file is never read, trimmed or finished by us, and only the error stream is read: on `run` every line of it arrives as `Line::Err`, and `collect` keeps the end of it within `Keep`. Ending the child ends the process group as usual, so nothing more is written to the file after a cancel or a limit.
- With `collect` both streams land on one pipe and the text is in the order the child wrote it, with a pseudo-terminal on the terminal as always. The kept bytes are at most what `Keep` asks for plus one read: the oldest fall out while the child writes, `trimmed` says that they did, and the cut falls between characters so no character is half in the text. `Keep::lines` counts the line being written as one, so output that ends without a newline keeps what was written. `cancel` and `Keep::limit` both end the child and, with `no_stdin` on Unix, its whole process group, and both return the output kept so far in bounded time: `cancelled` and `timed_out` say which of them it was, and an outcome of `Finished { code: None }` is how a child ended by the limit reports itself.
- `Keep::after_exit(wait)` — done `wait` after the command itself ended, even when something it left running holds the output open; what it left goes with its group. Also the grace between `TERM` and `KILL` when a cancel or a limit stops the child (two seconds without it).
- `Process::collect_watching(keep, cancel, |tail| …)` — `collect`, handing the kept end of the output to the closure while the child runs: after new output, at most every 100 ms.
- `clear_env` empties the child's environment before the variables given with `.env(..)` are set, so only those reach it.
- Standard input stays the application's own unless `no_stdin` is asked for, and the child is never put in its own session (a process group is not a session), so it keeps the controlling terminal and shares a warm `sudo` ticket. A pseudo-terminal is opened with `rustix`, without `unsafe`, and needs a Unix system.
