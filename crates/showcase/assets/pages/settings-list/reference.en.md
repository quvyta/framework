## Methods

- `SettingsList::show(ui, |list| …)` — adds the list, filling the width.
- `list.heading(title)` — a faint group heading; every heading after the first has an empty row above it.
- `list.row(row, |ui| …)` — a row with the one control the closure adds.
- `SettingRow::new(label)` — a one-line row.
- `.description(text)` — a faint second line under the label.
- `.disabled(bool)` — greyed out and skipped by the keyboard.
- `.nested(bool)` — a row that belongs to the row above, such as a choice that qualifies it: its text starts two cells further in; the pillar and the control stay in place.
- `.on_activate(msg)` — sent on Enter or Space the control does not use, or on a click on the label.

## Behaviour

- The label starts two cells in; the control ends two cells before the right edge.
- Label budget: the space up to two cells before the control, minus one spare cell for the slide.
- Keys while focused: ↑/↓ move between enabled rows; Home and End jump when the control does not use them; everything else goes to the selected row's control first. In a `ScrollView` shorter than the list, a key move scrolls just enough to show the new row; a click never scrolls.
- On focus the remembered row, or the first enabled row, is selected.
- The row stays lit while the pointer is over its control.
- A focused list raises one row only: moving the pointer onto an enabled row makes it the keyboard's row, and the arrows continue from there.

## Theme keys

- `setting-row` — `bg`, `pillar`; states `hover`, `selected`, `focus`, `disabled`.
- `setting-label` — `fg`, `bold`; the same states.
- `setting-description` — `fg`; the same states.
- `settings-heading` — `fg`, `bold`.
- `[motion]` — `slide`, `pulse-period`.
- `[icons]` — `pillar`.
