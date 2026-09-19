## Methods

- `Form::new()` — labels above controls, one row between fields, no summary.
- `.label_width(cells)` — labels in a column beside controls while the form is at least `cells + 18` wide. A field whose control asks for more than the room beside the label puts that label above instead.
- `.gap(rows)` — rows between fields; 1 by default.
- `.summary(&errors)` — lists every message of `errors` above the fields; hidden while empty.
- `.show(ui, |form| …)` — adds the form; `form.field(field, |ui| …)` adds a field with its control, `form.ui()` adds anything else.
- `Field::new(label)` — a label, the control added inside, nothing else.
- `.hint(text)` — faint help under the control while there is no error.
- `.error(Option<text>)` — the error under the control in place of the hint.
- `.required(bool)` — the faint required word after the label.
- `.disabled(bool)` — a greyed label; disable the control too.
- `.label_width(cells)` — the label column of this field alone; the form's width applies otherwise.
- `FormErrors::new()`, `.set(name, message)`, `.check(name, valid, message)`, `.remove(name)`, `.clear()`.
- `.get(name)`, `.has(name)`, `.first()`, `.len()`, `.is_empty()`, `.iter()`.
- `.focus_first()` — `Command::focus` on the first problem, or `Command::none()`.

## Behaviour

- Enter that the focused control does not use moves focus to the next focusable widget.
- The label takes the `focus` state while focus is on its control or anything inside the field.
- Beside controls, the required word sits under the label and the hint or error under the control.
- Hints and errors wrap to the field width; the summary cuts long messages with `…`.
- `Command::focus(name)` for a widget that is not on screen yet applies after the next frame if it appears there.

## Theme keys

- `field-label` — `fg`, `bold`; states `focus`, `disabled`.
- `field-required`, `field-hint`, `field-error` — `fg`.
- `form-summary` — `bg`, `padding`.
- `form-summary-title` — `fg`, `bold`; `form-summary-marker`, `form-summary-item` — `fg`.
- `[icons]` — `error`.
- Language — `quvyta.form.required`, `quvyta.form.summary` (plural, `n`).
