## Methods

- `Command::confirm(confirm)` — shows the question and delivers the message of the answer.
- `Confirm::new(title, on_confirm)` — the question and the message for confirming.
- `.message(text)` — explanation below the title.
- `.danger()` — a danger pillar down the left edge and a danger confirm button.
- `.confirm_label(text)` — the confirm button. Default: `quvyta.confirm.confirm`.
- `.cancel_label(text)` — the cancel button. Default: `quvyta.confirm.cancel`.
- `.on_cancel(msg)` — sent on Cancel, Esc and the close mark. Without it cancelling only closes.
- `.dismissable(bool)` — whether Esc and the close mark cancel, together. Default: `true`. With `false` neither works, the mark is hidden and only the buttons answer.

## Keys

- `enter` / `space` answer with the focused button; Cancel is focused first.
- `tab` / `shift tab` switch between the two buttons.
- `esc` cancels while the question is dismissable.

## Mouse

- Clicking a button answers. Clicking the close mark `×` cancels; its three cells light up under the pointer. Clicks on the dimmed screen are ignored.

## Behaviour

- The runtime draws the dialog after the application's view, so it is above everything the application draws.
- Several questions stack; the newest is shown and answered first, then the next.
- Application shortcuts pause while a question is open; focus returns afterwards.
- `Msg` needs no `Clone`: each message is delivered once.

## Theme keys

- The dialog uses the `Modal` keys: `modal`, `modal.danger`, `modal-title`, `close-mark`, `layer-backdrop`, `layer-hint-key`, `layer-hint-label`.
- Buttons: `button.danger` for destructive questions, `button.primary` otherwise.
- Language — `quvyta.confirm.confirm`, `quvyta.confirm.cancel`, `quvyta.layer.close`, `quvyta.layer.switch`.
