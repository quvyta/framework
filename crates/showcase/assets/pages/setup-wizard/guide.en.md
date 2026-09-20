## When to use

The first time an application of the family is started. It opens while the application has no settings file of its own, asks the three things every Quvyta application shares, lets the application ask its own, and writes both files in one go at the end.

- **One wizard, not one per application.** The appearance step is the framework's: the same rows, the same boxes, the same words in nine languages everywhere. An application adds only what is its own.
- **Always the first step.** Even when the family already shares a language, a theme and icons, the step is shown, filled in from the shared file and with every box ticked, so Next alone accepts all of it.
- **All or nothing.** Closing the wizard half-way writes nothing at all, not even the shared file, and the wizard comes again next start. "Start with the defaults" is the way out that keeps what is filled in.

## Step by step

1. Hold the state: `Setup::new(Family::QUVYTA, "code", &i18n, Msg::Setup).on_finish(Msg::Ready)`. Migrate an older settings file with `Family::adopt` before it, so a migrated application is not asked.
2. Resolve the preferences the same way at start, without writing: `family.preferences_without_saving("code", &i18n)` gives `Runtime::preferences` its values and leaves the folder alone. `Setup` does this for its own rows already.
3. Draw it while it is wanted: `if self.setup.needed() { SetupWizard::new(&self.setup).step(title, page).show(ui) }`, else the application's own screen.
4. Add the application's own steps with `step(title, |ui| …)`, once per step, in order. Their content and their messages are the application's own; the framework never reads them.
5. Answer every `Msg::Setup(message)` with `self.setup.update(message, &mut self.settings)`. Nothing else is needed: Back, Next, the rows, the font install and Finish are all in there.
6. Write the application's own keys in the `on_finish` message: the wizard has already written the three shared keys and made the file, and `settings` hold them, so `settings.set(..)` and `settings.save()` keep them.
7. Give every step the same height with `page_height(rows)` so the buttons do not move, and `on_cancel(Msg::Quit)` when the wizard should be closable; closing writes nothing.

## How it works

- **Built on `Wizard`.** The steps on top, one page, Back, Next and Finish are the same component every other flow uses; the setup wizard only fills the first page and says what the buttons mean.
- **The appearance step is `Appearance::rows`.** The same three rows a settings page shows, each with its "In every Quvyta application" box, on an `Appearance` that is `without_saving()`: a change is applied at once, so choosing a theme redraws the wizard in that theme, and the file it would go to waits.
- **The eye chooses the icons.** Under the rows the same four icons are drawn in all three glyph modes at once, whatever mode the application draws in. Without a Nerd Font on the machine the step offers to install the symbols, shows how far it is, and afterwards says what to do if the glyphs are still boxes.
- **Finish writes three keys.** Each shared key goes through `Family::set`: to the family's file with the family's id in the application's file while the box is ticked, to the application's file alone when it is cleared. A key kept here alone never reaches the shared file. Making the application's file is what ends the wizard for good.
- **A write that fails keeps the wizard.** The reason is shown on the first step, the application is not told it is over, and nothing is half written.

## Common mistakes

- **Calling `Family::preferences` at start.** It creates the shared file, so a wizard closed half-way would have left something behind. Use `preferences_without_saving` in an application that has a wizard.
- **Writing the application's own keys before `on_finish`.** They would appear in a file that a half-finished wizard must not leave behind.
- **Asking for the language, theme or icons in a step of your own.** They are the family's, and the first step already asks them for every application at once.
- **Pointing a test or a demo at the real folders.** `Setup::new_in(folder, ..)`, `install(..)` and `font_dirs(..)` keep a test away from `~/.config/quvyta` and from the user's fonts.
