## Methods

The example uses these framework components; each has its own page with the full reference.

- `Wizard` — `current`, `on_back`, `on_next`, `on_finish`, `on_cancel`, `on_step`, `page_height`.
- `Form` and `Field` — `label_width`, `required`, `hint`, `error`; `FormErrors` with `check` and `focus_first`.
- `TextInput` — `placeholder`, `max_length`, `invalid`, `on_change`.
- `RadioGroup` — the container engine.
- `SettingsList` and `SettingRow` — theme `Select`, icons `Segmented`, animations `Switch`.
- `Checkbox` — optional features.
- `CodeView` — the project file preview, `Language::Toml`.
- `Steps` (`vertical`, `running`) and `ProgressBar` (`percent`) — creation progress.
- `Command::focus`, `Command::perform` — focus on the next page, background work.

## Behaviour

- Step 1 needs a name of at least two lowercase letters, digits or dashes that does not start with a dash, and a location.
- Step 2 needs an engine.
- Next moves focus to the first control of the new step; a blocked Next focuses the first problem.
- Finish runs four stages in the background, one message each, then shows how to run the project.
- Cancel, Esc and "Set up another project" reset every answer.

## Theme keys

- The example has no keys of its own; it looks the way the theme styles its components.
