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

## Common mistakes

- **Running `sudo` with a pipe.** Then the prompt has nowhere to go. A handoff or a pseudo-terminal, nothing in between.
- **Putting the child in its own session.** `setsid` loses the warm `sudo` ticket; the request file of qpackages measured it.
- **Drawing after the handoff without redrawing everything.** The screen the program left is not yours; the runtime clears it for you, so never keep half a frame.
- **Pausing for a program that prints nothing.** `sudo -v` takes a second and has nothing to read; a key press there is only in the way.
