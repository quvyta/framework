## When to use

Use a wizard when a task needs several groups of answers in a fixed order and the later questions depend on the earlier ones: adding a deploy target, setting up a project, connecting an account. If every value fits on one screen, a form is quicker.

## Step by step

1. Keep the current step, every value and a `FormErrors` in your state.
2. Show the wizard: `Wizard::new(labels).current(state.step).on_back(Msg::Back).on_next(Msg::Next).on_finish(Msg::Finish).show(ui, |ui| { … })`.
3. In the page closure, draw the page of `state.step`, usually a `Form`.
4. Write one validation per step. In `update` for `Next`, validate the current step; on problems return `errors.focus_first()` and stay, otherwise move to the next step.
5. On `Finish`, do the work; `.busy(true)` shows Finish working meanwhile.
6. Turn on what the flow needs: `.on_cancel(Msg::Cancel)` for a Cancel button and Esc, `.on_step(Msg::GoTo)` to jump back to finished steps, `.page_height(rows)` so the buttons stay put.

## How it works

- **The wizard is Steps, a page and a button row.** The steps show where the user is; Back appears from the second step; Next becomes Finish on the last step and is the primary button.
- **Validation blocks Next without the wizard knowing.** Next only sends your message. You decide whether to advance, and `focus_first` puts the cursor on the problem.
- **Focus follows the flow.** `Command::focus` works for controls that appear with the same update, so you can focus the first field of a new step.
- **Esc cancels only when asked.** Without `on_cancel` Esc keeps its meaning in the application.
- **Enter moves on.** Inside a form, Enter goes from field to field and from the last field to Next.

## Common mistakes

- **Validating everything at the end.** Check each step on Next, while its fields are on screen.
- **Losing answers on Back.** Values live in your state, so going back keeps them; do not reset them in `Back`.
- **Too many steps.** Three to five steps read well; merge short ones.
- **A Finish that looks done while work runs.** Use `busy` until the work reports back.
