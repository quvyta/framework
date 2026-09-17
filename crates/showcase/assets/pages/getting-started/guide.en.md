## Quvyta and quvyta-framework

quvyta-framework is a Rust framework for building terminal applications. It was started for Quvyta's own applications and is open source. It provides the runtime, the widgets, and theme, icon and language files; in code the library is called `qframe`.

## What an application is

An application written with quvyta-framework is three things: **data**, a function that **draws** the data, and a function that **changes** it. The runtime connects them: it draws the screen, turns keys and clicks into messages, hands each message to your update function and draws again.

You never draw by hand, never track the mouse and never decide when to redraw. You describe the screen and react to messages.

## Step by step

1. Write a struct with everything the screen depends on. Here it is `State` with a counter, an optional file count and a flag that says work is running.
2. Write an enum of everything that can happen: `Msg`. Every button, field and list sends one of these.
3. Implement `update`: match on the message, change the state, and return a `Command` when the runtime should do something for you.
4. Implement `view`: add widgets with `ui.add`, group them with `ui.row` and `ui.column`, and give each widget the message it sends.
5. Start it with `Runtime::new(app).run()`.

## How it works

- **View runs after every change.** It is cheap: widgets are plain values, and the terminal only receives the cells that changed.
- **Nothing is drawn when nothing changes.** Without input or animation the loop waits and paints nothing, so an idle application barely uses the CPU.
- **State you do not care about lives in the runtime.** Hover, focus, cursor positions and scroll offsets are remembered per widget; your state only holds what the application means.
- **Slow work is a command.** `Command::perform` runs a closure on a background thread and delivers its message when it finishes. The count button above reads the disk this way; the spinner keeps turning while it works.

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
