## Methods

- `Wizard::new(labels)` — steps named `labels`, on the first step, no buttons wired.
- `.current(index)` — the current step.
- `.on_back(msg)`, `.on_next(msg)`, `.on_finish(msg)` — the button messages.
- `.on_cancel(msg)` — a Cancel button on the left; Esc inside the wizard sends it too.
- `.on_step(|index| msg)` — finished steps can be chosen.
- `.busy(bool)` — Next or Finish shows a spinner and ignores presses; Back and Cancel are disabled; the current step breathes.
- `.page_height(rows)` — every page gets exactly `rows` rows.
- `.show(ui, |ui| …)` — adds the wizard with the current step's page.

## Behaviour

- Layout: steps, one empty row, the page, one empty row, the buttons (Cancel left; Back and Next right).
- Back is shown from the second step on; on the last step Next reads Finish and sends `on_finish`.
- Named widgets: `wizard-steps`, `wizard-page`, `wizard-cancel`, `wizard-back`, `wizard-next`.
- Esc is handled only while focus is inside the wizard and `on_cancel` is set.

## Theme keys

- The wizard draws with `Steps` and `Button` (`button.primary`) styles; it has no keys of its own.
- Language — `quvyta.wizard.cancel`, `quvyta.wizard.back`, `quvyta.wizard.next`, `quvyta.wizard.finish`.
