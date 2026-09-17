## When to use

Use a task for anything that takes longer than a frame: building an image, running migrations, calling an API, scanning a directory. The screen keeps drawing and answering keys while the work runs.

- **`Command::perform`** is enough for a single quick call whose only result is a message.
- **`Task`** is for work the user watches: it reports progress and a note about the current step, can be cancelled, and ends with an outcome (done, failed with a reason, cancelled).
- **`TaskList`** shows running and finished tasks with the Spinner and ProgressBar you already know.

## Step by step

1. Write the work: `Task::new("Build image", |cx| { ...; Ok(Msg::Built(digest)) })`. Return `Err(reason)` to fail.
2. Inside, call `cx.note("pushing layers")` and `cx.progress(0.4)` as you go, and `cx.send(msg)` for anything else, such as log lines.
3. Wait with `if !cx.sleep(duration) { return Err("stopped".into()) }`, or check `cx.is_cancelled()` between steps, so cancelling stops the work quickly.
4. Add `.on_event(Msg::Task)`, keep `task.id()` if you want to cancel it, and return `Command::task(task)` from `update`.
5. Keep a `Tasks` model in your state and call `tasks.apply(&event)` for every `TaskEvent`.
6. Show it: `TaskList::new(&self.tasks).on_cancel(Msg::Cancel).show(ui)`, and answer `Msg::Cancel(id)` with `Command::cancel_task(id)`.

## How it works

- **Every task has its own thread.** It talks to the application only through messages, which the runtime applies between frames, so `update` and `view` never run in parallel with your state.
- **Events come in order:** `Started` right away, then `Progress` as reported, then the `Ok` message (when there is one) and finally `Finished` with the outcome.
- **Cancelling is cooperative.** `Command::cancel_task` wakes `cx.sleep` at once and makes `cx.is_cancelled()` true. Whatever the work returns afterwards, the outcome is `Cancelled` and its message is dropped.
- **A panic does not take the application down.** It becomes `Failed` with a short reason.
- **Tests stay fast and exact.** In the `Harness`, `cx.sleep` follows the fake clock: `advance(Duration::from_millis(500))` lets every task run exactly half a second of sleeps, then the harness waits for the tasks to settle before it renders.

## Common mistakes

- **Sleeping with `std::thread::sleep`.** It cannot be cancelled and ignores the harness clock; use `cx.sleep`.
- **Forgetting `on_event`.** Without it you receive only the final message and never see progress or failures.
- **Keeping progress in the task.** The model lives in your state through `Tasks::apply`; the task only reports.
- **Doing file or network work in `view` or `update`.** Move it into a task so drawing never waits.
