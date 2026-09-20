## Setup

The state the application holds; `Msg` is the application's own message type.

- `Setup::new(family, app, &i18n, wrap) -> Setup<Msg>` — the setup of `app`, on its first step, with every message of the first step wrapped as `wrap`. Resolves the shared preferences without writing anything.
- `Setup::new_in(config_dir, family, app, &i18n, wrap)` — the same with another family folder, for a test or a demo.
- `.on_finish(msg)` — what the application is sent once the wizard has written the shared keys and made the file.
- `.install(Install)` — another install than `Install::new()`, e.g. into a temporary folder.
- `.font_dirs(Vec<PathBuf>)` — other folders to look for a Nerd Font in.
- `.needed() -> bool` — whether the wizard is still to be shown: the application has no file of its own and the wizard has not finished.
- `.step() -> usize` — the step it is on; the appearance step is 0.
- `.preferences() -> &Preferences` — the shared values as the first step has them now, before anything is written.
- `.update(SetupMsg, &mut Settings) -> Command<Msg>` — applies a message and returns the command that shows it. On `Finish` it writes.

## SetupMsg

Everything the first step and the buttons send; every one of them goes to `Setup::update`.

- `Appearance(AppearanceChange)` — a row of the first step changed.
- `Install`, `Installing(Progress)` — the font install was asked for, and its steps.
- `Back`, `Next`, `Step(usize)` — the step before, the step after, and a finished step chosen from the steps on top.
- `Finish` — the wizard is over; "Start with the defaults" sends it from the first step.

## SetupWizard

Built in `view` from the `Setup`.

- `SetupWizard::new(&setup)` — the wizard with the appearance step alone.
- `.step(title, |ui| …)` — a step of the application's own, in the order the steps are added.
- `.on_cancel(msg)` — a Cancel button, and Esc inside the wizard; closing writes nothing.
- `.page_height(rows)` — the same height for every step, so the buttons stay put.
- `.show(ui) -> NodeMut` — adds it. The buttons are the `Wizard` ones: `wizard-cancel`, `wizard-back`, `wizard-next`.

## What it uses

- `Appearance::rows(list, message)` — the three rows the family shares, without a heading and without the application's own rows; `Appearance::without_saving()` applies a change without writing a file.
- `Family::preferences_without_saving(app, &i18n)` and `..._in(config_dir, app, &i18n)` — the resolution that leaves a missing shared file missing, for an application with a wizard.
- `Family::set(app, key, value, scope)` — what Finish writes with, once per shared key.
- `GlyphSample::new(GlyphMode)` — the samples beside the glyph mode names.
- `nerd_font::installed_in`, `nerd_font::status_text`, `Install::task`, `Progress`, `nerd_font::after_install_text` — the font install of the first step.

## Texts

In the framework's language files, in all nine languages: `quvyta.setup.sample-hint`, `install`, `defaults`, `not-saved`; the step's own name and its rows are `quvyta.appearance.*`, the buttons `quvyta.wizard.*`.
