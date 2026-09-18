## When to use

Use a confirmation before something that is hard to undo: stopping a production container, deleting a project, discarding unsaved changes. Do not ask for things that are easy to reverse; an undo is kinder than a question. When the dialog needs more than a yes or a no, such as a form, build it with `Modal`; a single word to type before an irreversible action is `.require_word`.

## Step by step

1. Give the question its own message, e.g. `Msg::AskStop(id)`, and send it from the button.
2. In `update`, return the question: `Command::confirm(Confirm::new(t!("stop.title"), Msg::Stop(id)))`.
3. Explain what happens: `.message(t!("stop.body"))`.
4. For destructive actions add `.danger()` and name the action on the button: `.confirm_label(t!("stop"))`.
5. If cancelling should do something (write a log line, reset a selection), add `.on_cancel(Msg::Kept(id))`.
6. Handle `Msg::Stop(id)` as usual; it only arrives when the user confirmed.

## How it works

- **No dialog state in your application.** The runtime keeps the pending question and draws the dialog over your view; your `update` only ever sees the answer.
- **The safe answer has focus.** Cancel is focused when the dialog opens, so a hurried Enter keeps things as they are. Tab moves to the confirm button.
- **Esc and × cancel; clicks outside do nothing.** Dismissable means both at once: Esc and the close mark in the top right corner cancel. `.dismissable(false)` removes both, for questions that must be answered with a button. A stray click on the dimmed screen never answers.
- **It looks like every dialog.** The same layer as `Modal`: dimmed screen, pop-in, a pillar down the left edge (danger for destructive questions), a danger confirm button.
- **A third way when two are not enough.** Recovered work asks "Discard · Continue · Save": `.alternative(label, msg)` puts a plain button between Cancel and the confirm button. Cancel still has focus, Tab reaches the third way next and the confirm button last, Esc and × still cancel.
- **Typing to confirm.** For what cannot be undone, such as emptying the trash for good, `.require_word(word)` adds a line "Type web to confirm" and a field below the message. The field has focus when the dialog opens and each question starts with it empty. The confirm button stays disabled, and Tab passes over it, until the typed text matches; then it takes its danger tone and Enter in the field confirms too. Esc and × still cancel at any time, and an alternative is not gated: only the confirm button waits for the word.
- **Matching is forgiving on purpose.** Spaces around the text and case do not matter, and neither do the Turkish dotted and dotless i: `sil`, `SİL`, `SIL` and ` Sil ` all match `SİL`. Typing is a deliberate act, not a password.
- **Questions stack.** Asking again while a question is open puts the new one on top; the newest is answered first.
- **Focus comes back** to the widget that had it before the question.

## Common mistakes

- **Vague buttons.** "Stop" and "Keep running" say what happens; "OK" does not.
- **Asking for everything.** People learn to press Enter without reading. Save questions for real loss.
- **Three ways for a yes-or-no question.** A third button is for a real third outcome, not for "Cancel" said twice.
- **Asking to type for everything.** Keep `.require_word` for the irreversible: a word asked every time is typed without reading, just like an Enter. Pick a word the user recognises, such as the thing's own name.
- **Doing the work before the answer.** Do it in the confirm message's handler, not when asking.
