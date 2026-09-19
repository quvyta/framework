## When to use

Use a settings list for preferences that apply one by one, at once: a theme, animations on, the container engine. Each row is a label and a control. When several values are checked together before anything happens, use a form.

## Step by step

1. Keep every value in your state: `animations: bool`, `engine: usize`.
2. Build the list: `SettingsList::show(ui, |list| { … })`.
3. Group rows under headings: `list.heading(t!("appearance"))`.
4. Add a row per setting with exactly one control: `list.row(SettingRow::new(t!("animations")), |ui| { ui.add(Switch::new(state.animations).on_toggle(Msg::Animations)); })`.
5. Add capabilities where they help: `.description(…)` for a faint second line, `.disabled(true)` for a locked setting (disable its control too), `.on_activate(msg)` for a row that opens something, such as a value row showing `2.4 GB`.

## How it works

- **Rows are bare until touched.** The row under the pointer raises its surface with a soft pillar; the keyboard's row raises it further with a breathing pillar. A focused list never raises two rows: moving the pointer onto a row makes it the keyboard's row.
- **Only the label slides.** On hover and selection the label and description move one cell right; the pillar and the control stay exactly where they were. The label column keeps one spare cell for this and cuts long labels with `…`.
- **The list takes focus as one control.** Tab reaches the list once, not every switch in it. Up and Down move between enabled rows, skipping disabled ones; every other key goes to the selected row's control: Space toggles a switch, Enter opens a select, Left and Right change a segmented control.
- **One list can be the whole page.** Put a long list in a `ScrollView`: moving with the keys scrolls just enough to keep the selected row in view, and a click leaves the scroll where it is, so the row you pressed stays under the pointer.
- **Unused keys activate the row.** When the control does not use Enter or Space and the row has `on_activate`, the row's message is sent.
- **The pointer goes straight to controls.** Clicking a switch toggles it; clicking a label selects its row and activates it when it can be activated.
- **Values belong to the application; the keyboard's row belongs to the runtime.**

## Common mistakes

- **Settings that wait for Save.** A settings list applies at once; for several values saved together, use a form.
- **Two controls in one row.** Keys reach one control; split the setting instead.
- **Descriptions as documentation.** One short line; longer help belongs in a guide.
- **Forgetting to disable the control of a disabled row.** The row greys out and is skipped, but the control keeps its own state.
