## Quvyta and quvyta-framework

quvyta-framework is a Rust framework for building terminal applications. It was started for Quvyta's own applications and is open source. It provides the runtime, the widgets, and theme, icon and language files; in code the library is called `qframe`.

## What an application is

An application written with quvyta-framework is three things: **data**, a function that **draws** the data, and a function that **changes** it. The runtime connects them: it draws the screen, turns keys and clicks into messages, hands each message to your update function and draws again.

You never draw by hand, never track the mouse and never decide when to redraw. You describe the screen and react to messages.

## Step by step

1. Write a struct with everything the screen depends on. Here it is `State` with a counter and the folder screen below it.
2. Write an enum of everything that can happen: `Msg`. Every button, field and list sends one of these.
3. Implement `update`: match on the message, change the state, and return a `Command` when the runtime should do something for you.
4. Implement `view`: add widgets with `ui.add`, group them with `ui.row` and `ui.column`, and give each widget the message it sends.
5. Start it with `Runtime::new(app).run()`.

## How it works

- **View runs after every change.** It is cheap: widgets are plain values, and the terminal only receives the cells that changed.
- **Nothing is drawn when nothing changes.** Without input or animation the loop waits and paints nothing, so an idle application barely uses the CPU.
- **State you do not care about lives in the runtime.** Hover, focus, cursor positions and scroll offsets are remembered per widget; your state only holds what the application means.
- **Slow work is a command.** `Command::perform` runs a closure on a background thread and delivers its message when it finishes. The count button above reads the disk this way; the spinner keeps turning while it works.

## Screens with their own messages

An application with several screens gives each one its own `Msg`, `update` and `view`, written as if the screen were alone. The folder panel above is such a screen: it sends `folder::Msg::Count`, and its `update` returns a `Command<folder::Msg>`.

The application connects it with one variant and two conversions:

- **The view:** `ui.map(|m| send(Msg::Folder(m)), |ui| state.folder.view(ui))` adds a column built with the screen's messages. Everything inside arrives converted: buttons and fields, nested rows and panels, dialogs the screen opens, focus requests.
- **The commands:** `state.folder.update(m).map(|m| send(Msg::Folder(m)))` converts what the screen asks the runtime to do. A message that background work sends later, like `Counted` here, goes through the same conversion.

The conversion can be a variant such as `Msg::Folder`, or a closure that captures what it needs, such as the tab a screen sits in. A command's conversion runs on background threads, so it must be `Send` and `Sync`.

## The lifecycle: start-up, size and quitting

Four optional `App` methods follow the application through its life. Each has a default, so an application writes only the ones it needs. The Lifecycle panel above shows all four; the showcase itself implements them.

- **`init(&mut self) -> Command<Msg>`** runs once, at the start of the first frame, before its view is built. Return the first work: `Command::focus("menu")` so the first key already reaches the list, a tick to start, a dialog to open. Focus lands as soon as that frame is painted, before any input is read, and the frame is drawn again with the widget focused. The showcase's menu has the keyboard this way: the first ↓ moves in it.
- **`resized(&self, size: Size) -> Option<Msg>`** hears the terminal size when the application starts (just before `init`) and after every resize. Its message goes through `update`, which is where work that needs the size begins, such as `Process::pty(cols, rows)`. It is the same size `ui.size()` reports in `view`, applied before that frame is built, so the two never disagree.
- **`before_quit(&self) -> Option<Msg>`** is asked whenever the runtime is about to quit for the user: the `ctrl q` binding, and the quit action run from the command palette. `None` quits. A message keeps the application running and is delivered instead, e.g. to ask "finish and quit, keep running or cancel". When the application has decided it returns `Command::quit()`, which is its own decision and is not asked about again. Turn on "Ask before quitting" above and press `ctrl q`.
- **`terminating(&self, cause: Termination) -> Option<Msg>`** hears that the system, not the user, is ending the application. It is the one chance to save. `None` quits at once; a message keeps the application running while `update` saves and returns `Command::quit()`. The default answers `Termination::Terminate` with `before_quit` and quits at once on `Termination::Hangup`, so an application that writes neither hook still quits cleanly.

Hooks that only report something (`resized`, `before_quit`, `terminating`, `action`, `clipboard`) read the state and answer with a message; the one that starts work (`init`) returns a command, like `update`. The harness runs all of them where the terminal does: `Harness::new(app, 120, 30)` reports 120 × 30 and runs `init`, `resize(60, 20)` reports 60 × 20, `press("ctrl+q")` asks `before_quit`, and `terminate(Termination::Hangup)` tells `terminating`.

## When the system ends the application

On Unix the runtime catches three signals for as long as it runs, and each becomes a `Termination`:

- **`SIGTERM`** (`kill`, a service manager, a shutdown) and **`SIGINT`** sent from outside become `Terminate`. The terminal is still there, so the application may save and quit, or even ask. Inside the application `ctrl c` is a key, not this signal.
- **`SIGHUP`** becomes `Hangup`: the SSH connection dropped, the window or the `tmux` pane closed. Nobody can answer a question now and nothing is drawn after it: save without asking. The hangup a shell forwards and the one the system sends when the shell ends are the same hangup, told once.

Every way out ends in bounded time. After `Termination::grace` (five seconds after a terminate, three after a hangup) the runtime quits without the application. A second `SIGTERM` or `SIGINT` quits at once. When the loop itself is stuck, in an `update` that never returns, the process is ended a second later all the same, by the signal. The terminal is restored in every case while it exists: raw mode off, the normal screen, the cursor. After a hangup nothing is written to it.

The loop hears a signal the moment it arrives, even while it waits for a key or a deadline. During `Command::handoff` the program owns the terminal; a signal is passed on to it, it ends the way it would as a job of the shell, and then the application takes the terminal back and hears the signal itself.

Start the showcase, turn on "Ask before quitting" and send `kill <pid>` from another terminal: the question comes from `App::terminating`, and the event log shows it.

```rust
fn terminating(&self, _cause: Termination) -> Option<Msg> {
    // A running timer is saved whether a person or the system ends it.
    self.running.then_some(Msg::SaveAndQuit)
}
```

## Testing without a terminal

`Harness` runs the same application against an in-memory screen with a fake clock. Press keys, type, click on text and read the screen back as plain lines:

```rust
let mut app = Harness::new(Counter::default(), 40, 6);
app.press("tab").press("enter");
assert!(app.screen().contains("Count  1"));
```

In the harness `Command::perform` runs inline and tasks follow the fake clock, so tests stay deterministic.

## Common mistakes

- **Doing I/O in `view`.** View runs many times per second while something animates. Read files and call networks through `Command::perform`.
- **Keeping hover or focus in your state.** The runtime already knows; ask widgets to style themselves instead.
- **Forgetting `.id(...)` on widgets that move.** Widgets are recognised by their position; name the ones that appear, disappear or reorder so their state follows them.
