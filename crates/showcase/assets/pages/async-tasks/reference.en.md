## Methods

- `Task::new(label, |cx: &TaskCx<Msg>| -> Result<Msg, String>)` — background work; `Ok` delivers a message, `Err` fails with a reason.
- `.on_event(|TaskEvent| msg)` — start, progress and outcome as messages.
- `.id()`, `.label()` — known before the task starts.
- `Command::task(task)` — starts it on its own thread.
- `Command::cancel_task(id)` — asks it to stop; does nothing once it has finished.
- `TaskCx::progress(fraction)`, `TaskCx::note(text)` — report; `TaskCx::send(msg)` — any other message.
- `TaskCx::sleep(duration) -> bool` — waits, waking early and returning `false` when cancelled.
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
- In the `Harness`, `cx.sleep` follows the fake clock and every render waits until tasks are asleep or finished; a task that works ten seconds without sleeping fails the test with a clear message.

## Theme and icons

- Built from `Spinner`, `ProgressBar`, `Button` and `Text`; their theme keys apply.
- Icons `success`, `error`, `check-partial`; colours `success` and `danger` with those icons.

## Language keys

- `quvyta.tasks.done`, `quvyta.tasks.cancelled`, `quvyta.tasks.cancel`, `quvyta.tasks.empty`.
