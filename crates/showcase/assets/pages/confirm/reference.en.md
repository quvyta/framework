## Methods

- `Command::confirm(confirm)` — shows the question and delivers the message of the answer.
- `Confirm::new(title, on_confirm)` — the question and the message for confirming.
- `.message(text)` — explanation below the title.
- `.danger()` — a danger pillar down the left edge and a danger confirm button.
- `.confirm_label(text)` — the confirm button. Default: `quvyta.confirm.confirm`.
- `.cancel_label(text)` — the cancel button. Default: `quvyta.confirm.cancel`.
- `.on_cancel(msg)` — sent on Cancel, Esc and the close mark. Without it cancelling only closes.
- `.dismissable(bool)` — whether Esc and the close mark cancel, together. Default: `true`. With `false` neither works, the mark is hidden and only the buttons answer.
- `.alternative(label, msg)` — a third button between Cancel and the confirm button, a plain button that sends `msg`. Without it the dialog has its two buttons exactly as before.
- `.require_word(word)` — the user types `word` into a field before the confirm button works. The field has focus first; the confirm button is disabled until the text matches, ignoring surrounding spaces and case, with `İ`, `I`, `ı` and `i` the same letter. The alternative is not gated. A blank word asks for nothing.

## Keys

- `enter` / `space` answer with the focused button; Cancel is focused first.
- `tab` / `shift tab` switch between the buttons, left to right: Cancel, the alternative when there is one, the confirm button.
- `esc` cancels while the question is dismissable.
- With a word to type the field is focused first and Tab visits the field, Cancel, the alternative and the confirm button, passing over it while it waits. `enter` in the field confirms once the word matches and does nothing before; the field keeps its editing keys, paste and edit menu.

## Mouse

- Clicking a button answers. Clicking the close mark `×` cancels; its three cells light up under the pointer. Clicks on the dimmed screen are ignored. A confirm button waiting for its word ignores clicks.

## Behaviour

- The runtime draws the dialog after the application's view, so it is above everything the application draws.
- Several questions stack; the newest is shown and answered first, then the next.
- Application shortcuts pause while a question is open; focus returns afterwards.
- `Msg` needs no `Clone`: each message is delivered once.

## Theme keys

- The dialog uses the `Modal` keys: `modal`, `modal.danger`, `modal-title`, `close-mark`, `layer-backdrop`, `layer-hint-key`, `layer-hint-label`.
- Buttons: `button.danger` for destructive questions, `button.primary` otherwise; `button:disabled` while the word is not typed yet.
- The word's line uses the typography roles `secondary`, and `title` for the word itself; its field the `text-input` keys.
- Language — `quvyta.confirm.confirm`, `quvyta.confirm.cancel`, `quvyta.confirm.type-word` (with `{word}`), `quvyta.layer.close`, `quvyta.layer.switch`.
