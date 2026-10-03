## When to use

Use a form when the user fills in several values that are checked together before something happens: creating a container, adding a deploy target, signing in. For settings that apply at once, one by one, use a settings list instead.

## Step by step

1. Keep the values and a `FormErrors` in your state: `name: String`, `errors: FormErrors`.
2. Write one function that checks every value in form order and records a message per problem: `errors.check("name", name.len() >= 3, t!("name-short"))`.
3. Lay the fields out: `Form::new().show(ui, |form| { form.field(Field::new(t!("name")).required(true), |ui| { … }); })`.
4. Give each control the error's name as its id, `.id("name")`, pass the message to its field with `.error(errors.get("name"))`, and mark the control itself: `TextInput::invalid(errors.has("name"))`.
5. On submit, validate; when there are problems return `errors.focus_first()`, which moves focus to the first broken control.
6. Say what a valid value will cost, when it is worth saying, with `.warning(…)` on the field. It is drawn where the error is, and it is not a problem: a form whose fields carry only warnings still submits.
7. Add capabilities only where they help: `.hint(…)` on a field, `.label_width(16)` on the form for a label column, `.summary(&errors)` for a list of problems above the fields.

## How it works

- **The application owns everything that matters.** Values, errors and when to validate are yours; the form draws labels, hints, warnings and errors and moves focus.
- **A field is a label, a control and one message line.** Only one of the three messages takes that place: the error first, then the warning, then the hint. An error wins it from a warning, so a field never says two things where one fits.
- **A status colour never comes alone.** The error and the warning are signed, the error with the `error` glyph in the danger colour, the warning with the `warning` glyph in the warning colour — the two signs a toast of those kinds carries, so a field and a toast of the same kind look alike.
- **A warning is not a problem.** It stays out of `FormErrors`, it does not appear in `Form::summary`, it does not move focus and it does not stop the submit. It is the application's own sentence, built from the value beside it: "Not from the shared registry, the pull may be slow".
- **Required is a word.** A faint "required" follows the label; nothing is starred.
- **The label lights up while its control has focus**, so the eye finds the active field without a frame.
- **Enter moves on.** In a text field without its own submit, Enter goes to the next field, and from the last field to the button after the form.
- **Label columns fall back.** With `label_width`, labels sit beside controls while there is room and move above them on narrow screens. A control wider than the room beside its label, such as an input with a long placeholder, puts its own label above and takes the whole row, so nothing is cut; the other fields keep their column.
- **`Command::focus` works across updates**, so a control that appears with the same update can still take focus.

## Common mistakes

- **Showing errors before the user did anything.** Validate on submit first; after that, validating on every edit feels helpful instead of noisy.
- **Error names that differ from control ids.** `focus_first` can only reach a control named like its error.
- **A warning recorded as an error.** If the value is usable, a warning says what it costs; recording it in `FormErrors` makes a form that cannot be sent for a reason the user cannot fix.
- **Forgetting to mark the control invalid.** The field shows the message; the control's tint is its own option.
- **Messages that only say "invalid".** Say what to do: "Choose a port between 1024 and 65535".
