## When to use

Read this example when you build a flow that sets something up: a new project, an account, a deployment. It is a complete application screen written only with framework components: a wizard, forms, a radio group, a settings list, checkboxes, a code preview, steps and a progress bar.

## Step by step

1. **State first.** One struct holds every answer, the current step, the errors, and whether creation is running or done.
2. **One validation per step.** `validate(state, step)` returns `FormErrors` named after control ids; nothing else knows the rules.
3. **Next validates, advances and focuses.** On success the step moves on and `Command::focus` puts the cursor on the first control of the new page; on failure `focus_first` goes to the problem.
4. **Each page uses the component that fits.** Names and folders are a `Form`; a single choice among three is a `RadioGroup`; preferences that apply one by one are a `SettingsList`; optional features are `Checkbox`es with a faint explanation.
5. **The summary shows what will happen.** A `CodeView` previews the project file built from the answers.
6. **Finish starts background work.** `Command::perform` runs each creation stage off the drawing thread; each finished stage sends a message that starts the next one, while vertical `Steps` and a `ProgressBar` show the progress.
7. **The end state tells what to do next** and offers to start over.

## How it works

- **Nothing in the example draws by hand.** Every surface, marker and motion comes from components and the theme, so the flow looks right in every theme, language and glyph mode.
- **The wizard keeps its buttons in place** with `page_height`, so short and long pages do not make them jump.
- **Going back keeps answers.** Values live in the state; Back and choosing a finished step only change `step`.
- **Esc and Cancel start over**, because the wizard has `on_cancel`.
- **Errors follow edits after the first try.** Once a step has problems, every change validates it again, so a message disappears as soon as the value is fixed.

## Common mistakes

- **Doing slow work in `update`.** Creating files, calling an engine or cloning a repository belongs in `Command::perform`; drawing must never wait.
- **Asking everything on one page.** Group questions by topic so each page answers one thing.
- **Hiding what Finish does.** Show a summary before the irreversible step.
