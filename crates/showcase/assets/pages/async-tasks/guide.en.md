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

## Streaming a child process

`Process` runs a child inside a task and hands its output to the application line by line, which is what a package manager, a build or a deploy script needs.

1. Build it: `Process::new("sh").arg("-c").arg(script).env("LC_ALL", "C")`, plus `.dir(path)` when it must run elsewhere.
2. Run it inside a task: `process.run(&|| cx.is_cancelled(), &mut |line| cx.send(Msg::Streamed(line)))`. The call blocks its own thread, never the screen.
3. Turn each `Line` into a log line: `Line::Out` is ordinary output, `Line::Err` is the child's standard error, kept apart so a failure stays recognisable.
4. Answer the cancel button with `Command::cancel_task(id)`; the task's flag makes `run` kill the child and return `ProcessOutcome::Cancelled`.
5. Add `.no_stdin()` unless the child really has to read the keyboard. The child then reads an empty input instead of the terminal, so a program that asks a question cannot take the user's keys from the application, and it runs in a process group of its own, so cancelling ends the programs it started too (`podman`, then `buildah`, then the build's steps).
6. Fix the language of anything you are going to parse with **both** `LC_ALL` and `LANG`; some programs read only one of them.

Add `.pty(cols, rows)` when the child should keep its progress bar and its colour: programs check for a terminal and go plain when they write to a pipe. The child then sees a terminal of the size you gave, not the real one's, so its progress bar fits the space you are going to draw it in. Standard input stays the application's own and the child keeps the controlling terminal, which is what keeps a warm `sudo` ticket shared; with a pseudo-terminal both streams land on the same line, so every line arrives as `Line::Out`.

When you need what a progress bar says, not only where it ends (`Building [=>  ] 12/46` from `cargo`, a download's percentage from `pacman` or `curl`), run it with `.run_with_overwritten(&cancel, &mut on_line, &mut on_frame)` instead of `.run(..)`. Every frame a `\r` is about to overwrite then reaches `on_frame`, tagged `Line::Out` or `Line::Err` like a line, in the order the child wrote it; `on_line` still gets exactly the lines `run` would give. Colour codes and `ESC [K` stay in the text, so strip them before you parse.

## Common mistakes

- **Merging the two streams with `2>&1`.** The diagnosis is then lost: many programs keep their standard error empty until something is wrong.
- **Expecting a progress bar from a pipe.** Without `.pty(..)` the child sees no terminal and prints plain lines; that is the child's choice, not ours.
- **Waiting for a newline to show progress.** A `\r` overwrites the line being built, so with `run` the finished line is what you get, once; ask for the frames with `run_with_overwritten`.
- **Killing the child and expecting its children to go too.** Only a child started with `.no_stdin()` takes its own children with it; without it only the child itself is killed, and a program that starts its own children can leave them running.
- **Letting a child share the terminal's input.** Without `.no_stdin()` a child that asks something reads the keys meant for your application, and cancelling it leaves its own children running.
- **Letting a child without input ask for a `sudo` password.** Its group does not own the terminal, so the system stops it when it reads the password; it waits until cancelled. Warm the ticket first with a handoff of `sudo -v`, or run `sudo -n`.
