## When to use

Use a handoff when another program has to talk to the user itself. An application in raw mode on the alternate screen cannot show someone else's prompt: `sudo` writes its password question to the controlling terminal, your screen swallows it, and both programs read the same keys. Stepping aside for a few seconds is the honest answer, and the one lazygit, git and every package manager front end uses.

- **`Command::handoff`** — `sudo -v` for an authorization ticket, `paru` asking "Proceed to review?", `$EDITOR`, a pager.
- **`Task`** — work of your own with progress; the screen keeps drawing.
- **Never imitate the prompt.** A password you draw yourself passes through your memory, and a review question you answer for the user takes away a security decision that is theirs.

## Step by step

1. Build it: `Handoff::new("sudo", Msg::Authorized).arg("-v")`. The message is delivered once the application has the screen back.
2. Say why the screen is leaving: `.notice("Authorizing the installation")`. The line is printed on the cleared screen before the program starts.
3. Turn the pause on for programs whose last lines are worth reading: `.pause(true)` waits for one key press before the application takes the screen. Off by default.
4. Return it from `update`: `Command::handoff(handoff)`.
5. Answer the outcome: `HandoffOutcome::Finished { code }` with `Some(0)` for success, another number for a refusal, `None` when a signal ended it, and `HandoffOutcome::Failed(reason)` when the program could not start at all.
6. In tests, read `harness.handoffs()` and set the answer with `harness.set_handoff_outcome(...)`: no password is ever typed in a test.

## How it works

- **The handoff blocks drawing, on purpose.** It runs on the drawing thread: the user is talking to another program, so there is nothing to draw. Background tasks keep running and their messages arrive afterwards.
- **The program inherits the terminal** — standard input, output and error are the application's own, so the keys go straight from the kernel to it. Nothing is echoed through the application, and no password reaches its memory.
- **`Ctrl-C` reaches the program, never the application.** The program runs in a process group of its own, which is made the terminal's foreground for as long as it runs, the way a shell runs a job. `Ctrl-C` at a `sudo` prompt, `Ctrl-\` in a pager, a resize: the terminal sends them to that group only, so they abort the password prompt without ending the application. Nothing about the application's own signal handling changes, before, during or after.
- **`Ctrl-Z` does not suspend.** The application is not a shell that could offer a way back to a stopped program, so a program that stops is let go on at once; `less` and editors simply draw themselves again.
- **It stays in the application's session.** A process group is not a session: the program keeps the controlling terminal and the session, which is what `sudo` keeps its ticket for, so a warm ticket stays warm. A program put in a session of its own would be asked for the password again.
- **Coming back is complete.** Raw mode and the alternate screen are taken again, the keyboard enhancement flags are pushed again, and the whole screen is drawn from scratch, because the program wrote over what was there.
- **The terminal is restored whichever way it ends.** A program that cannot start, one killed by a signal, a panic of the runtime: the guard and the panic hook put the terminal back either way.
- **Several handoffs run one after another**, in the order they were asked for.

## A program that keeps running

Some programs ask once and then serve the application for the rest of the session. `pkexec /usr/lib/app/helper --serve` asks for the password on the terminal and becomes a privileged helper that answers requests line by line. A handoff waits for its program to end, so its screen would never come back; `Command::handoff_detached` takes it back as soon as the program says it is ready.

1. Build it like a handoff, with a message for each line that comes later: `DetachedHandoff::new("pkexec", Msg::Started).args([helper, "--serve"]).notice(...).on_line(Msg::Helper)`.
2. Return it from `update`: `Command::handoff_detached(handoff)`.
3. The program writes its first line on standard output when it is ready. The terminal's foreground goes back to the application, the screen is drawn again in full, and `DetachedOutcome::Detached { child, first_line }` arrives. Keep the `LiveChild` in your state: `child.write_line(request)` writes to its standard input.
4. Every later line arrives as `ChildLine::Line(text)`, for as long as the program runs; `ChildLine::Ended { code }` follows the last one.
5. A program that ends before its first line — `pkexec` after a refused or cancelled password returns 126 or 127 — gives `DetachedOutcome::Finished { code }`, as a handoff would. `.pause(true)` waits for a key only then, so the reason can be read.
6. To finish, close its input with `child.close_stdin()`; a helper reads the end of its input and ends. When the application's state is dropped at the end of the run, the input closes by itself. `child.kill()` exists too, but a program running as root cannot be signalled by you.
7. In tests, `LiveChild::for_tests()` gives a stand-in child and a `TestChild` that plays the program: `harness.set_detached_outcome(DetachedOutcome::Detached { child, first_line })` answers the handoff, `program.written()` shows what the application wrote, `program.say(line)` and `program.exit(code)` answer it at the next step. `harness.detached_handoffs()` shows what was asked for.

Until the first line everything above holds: `Ctrl-C` reaches the program, `Ctrl-Z` does not suspend, the session is kept. Only standard error stays the terminal, because `pkexec` and `sudo` ask on the controlling terminal itself, not on standard input. Afterwards the program runs in the background of the terminal; it should keep quiet on standard error from then on, since whatever it writes there lands on the application's screen.

## Opening something on the person's own desktop

A link that opens in the browser, a PDF that opens in the viewer, an image editor started on a file: none of that happens in the terminal, so there is nothing to step aside for. `Command::open` starts the program quietly beside the application, with its streams thrown away and a process group of its own, and the screen is left exactly as it is — no blink, nothing drawn again.

1. The short way, when there is nothing to say about it: `Command::open("https://quvyta.com/guide")`. It takes an address, a file or a folder and gives it to this desktop's opener: `xdg-open`, `open` on macOS, `start` on Windows.
2. With an answer: `Command::open_with(Open::new(address).answer(Msg::Opened))`. `OpenOutcome::Opened` says the opener was handed the target; `OpenOutcome::Failed(reason)` says nothing was started at all, because this desktop has no opener installed.
3. A program of your own instead of the opener: `Open::program("gimp").arg(path)`, with `.dir(...)` and `.env(key, value)` as a handoff has them.
4. In tests, `harness.opens()` shows what was asked for and `harness.set_open_outcome(...)` answers it; nothing on the desktop is ever reached from a test. `OpenRequest::target` is what you asked to open, so the test reads the address without knowing which opener the machine running it has; `OpenRequest::dir` and `HandoffRequest::dir` are the working folder the program would have started in.

**`Opened` is not a promise that the person saw anything.** The opener is handed the target and that is the end of what a terminal can know: the browser may already be running, the file type may have no handler, the window may open behind another. Say "opened in your browser" and show the address too, so a person whose desktop did nothing can still read it.

**Every door to the desktop goes through here.** Calling `xdg-open` from your own code means a test run really opens a browser on whoever is running it; an opening is recorded instead, and the test still sees that the call was made.

## Common mistakes

- **Running `sudo` with a pipe.** Then the prompt has nowhere to go. A handoff or a pseudo-terminal, nothing in between.
- **Putting the child in its own session.** `setsid` loses the warm `sudo` ticket; the request file of qpackages measured it.
- **Drawing after the handoff without redrawing everything.** The screen the program left is not yours; the runtime clears it for you, so never keep half a frame.
- **Pausing for a program that prints nothing.** `sudo -v` takes a second and has nothing to read; a key press there is only in the way.
