## Three files, no code

An application gives quvyta-framework **theme files**, **icon files** and **language files**. The framework ships a complete default of each, so an application runs without any of them; your own files replace or extend the defaults. Because they are plain files, adding a theme or a language is copy, rename, edit.

## Files given as text

A path is not the only way in. `Runtime::theme_source(file, text)`, `icon_source`, `keymap_source` and `locale_source` take the TOML text itself, usually an `include_str!` of a file in your repository. The installed binary then carries its own look and keys and starts with nothing beside it on disk, which a path built from `CARGO_MANIFEST_DIR` cannot do. The file name only labels diagnostics, and for themes and icon sets its stem is the id, exactly as in a directory.

Text and paths live together: text loads last, so it wins, and a path named as well becomes optional. When that path cannot be read the text stands in for it and the reason becomes a diagnostic instead of stopping the program. Without text to stand in for it, an unreadable path is still an error, because then nothing would take the file's place.

## Themes

A theme has two layers.

1. **Colour tokens** in `[colors]`: `canvas`, `surface`, `raised`, `active`, `overlay` for the grounds from lowest to highest, `accent` and `accent-2` for the one emphasis colour and its breathing partner, `text`, `dim`, `muted` for type, `ink` for text on the accent, and `success`, `warning`, `danger`, `info` for meaning.
2. **Style rules** in `[style."widget.variant:state"]`: what each widget looks like in each state, written with tokens.

A theme only writes what differs. `extends = "monochrome"` inherits everything else, so a new theme can be ten lines of colours.

```toml
[meta]
name = "Aurora"
extends = "monochrome"

[colors]
accent = "#7DD3FC"
```

## How style rules are chosen

- The selector grammar is `widget.variant:state:state`. Widget and variant names use lowercase letters, digits and dashes.
- More specific rules win: `button` < `button.primary` < `button:hover` < `button.primary:hover`.
- At equal specificity the later rule wins, and rules of the extended theme count as earlier.
- Values are `$token`, `#RRGGBB`, `mix(a, b, 30%)` or `pulse(a, b)`, which breathes between two colours.

## Readability is checked

When a theme loads, the framework measures contrast between text and grounds and the distance between the accent and the four status colours. A theme where warning looks like the accent, or text is hard to read, produces a warning with the numbers. Broken values never crash: the entry is skipped and the file, line and column are reported. The status line in the demo shows whether the loaded files had problems.

## Terminals with fewer colours

The framework asks the terminal how many colours it shows (`ColorDepth::detect`) and draws the theme's colours in 24-bit, in the 256-colour palette, or in the sixteen standard colours. The theme stays the same file; only the drawing changes.

The sixteen standard colours have two dark greys, black and bright black, while a dark theme builds its shapes from four or five grounds a few steps apart. Rounded to the nearest colour they would all land on black, and a tab strip, a raised panel or a dialog would melt into the screen. So a sixteen-colour frame is painted in full colour and reduced once it is complete, with two rules:

- **A ground the eye tells apart stays apart.** A tone at least 0.05 from `canvas` in OKLab that would still land on the canvas's colour takes the next grey away from it. On a dark theme `canvas` and `surface` stay black while `raised`, `active` and `overlay` become bright black; on a light theme they step down from bright white to white.
- **Text never vanishes into its ground.** Text that would keep less than 1.6:1 against its background takes the quietest grey that keeps 3:1: muted or accent text on a bright black surface turns white, and the page behind an open dialog stays faint, bright black on black, rather than disappearing.

A 256-colour frame is painted in full colour and reduced once complete too, so dimming the page behind a dialog, lifting a menu off a panel and a page transition blend as they do in true colour. Its palette has enough greys for every ground, so backgrounds take the nearest entry. Text keeps its nearest entry unless that falls under 1.6:1 against its background; then it takes the entry closest to its own colour that still keeps 1.6:1, so the faintest labels on the page behind a dialog stay faint instead of vanishing.

`Rgb::to_ansi16_on`, `Rgb::to_ansi16_text`, `Rgb::to_ansi256` and `Rgb::to_ansi256_text` give the same answer the frame does, so a test can check a screen without drawing it in fewer colours; `Harness::set_depth(ColorDepth::Ansi16)` or `ColorDepth::Ansi256` draws it that way.

## Icons

Every icon has three glyphs: `nerd` for Nerd Fonts, `unicode`, and `ascii`. In `auto` mode the framework picks by looking at the terminal and the installed fonts; users can choose a mode, and `QUVYTA_ICONS=ascii` forces one. ASCII glyphs never use brackets to fake shapes.

Some names are shared meanings rather than marks of one widget, so every application in the ecosystem draws the same shape for them: `project`, `profile`, `settings` and `power` for the main menu, `workspace` for the place the work is kept and `help` for the button that answers what the keys do, `window-minimize`, `window-maximize` and `window-restore` for a window's title, `category-system`, `category-development`, `category-network`, `category-office`, `category-media` and `category-files` for the kinds of program a launcher groups its entries by, `ecosystem` for Quvyta's own `❖`, and `cpu`, `memory`, `battery-full`, `battery-half`, `battery-empty`, `battery-charging`, `terminal`, `session` (a terminal multiplexer's) and `network-down` and `network-up` for what a status strip shows of the machine. Their Unicode column stays inside Geometric Shapes, which terminals and monospace fonts draw as one cell, so a row in a dock or a three-cell title mark cannot be pushed apart by a glyph that falls back to another font; `ecosystem` is the one exception, because it is the mark itself. A launcher entry with no icon of its own takes the icon of its category.

The set answers the meanings an application's main menu shows, so every application in the ecosystem draws the same shapes: `project`, `profile`, `settings` and `power`, beside `workspace` and `help`. A project is the thing worked on and a workspace the laid-out place that holds it, so they are two keys rather than two names for one shape; in ASCII both draw as `#` and the row's own words tell them apart. `help` draws a question mark in all three modes, because that is how the meaning is read wherever the ecosystem is read. Nerd Font draws the thing itself; the Unicode column stays inside Geometric Shapes, which terminals and monospace fonts draw as one cell, so no row falls back to another font and overflows.

## Your application's own icons

An application gives its own icons the same way it gives an icon set: `Runtime::icon_source("app.toml", include_str!("../assets/icons/app.toml"))`. Keys the built-in set does not have, such as `category.internet` or `source.aur`, are then drawn by every widget that takes an icon key and by `env.icons().glyph(..)`, whatever set the theme chooses, and switching the glyph mode switches them to their own Nerd, Unicode or ASCII column. The demo's “app icon” row is such a key, drawn in the default theme.

- **Your keys sit under the chosen set.** A theme's set or a theme's single `[icons]` entry can still restyle `category.internet`, the way a user reskins anything else.
- **The framework's keys stay the theme's.** A key the built-in set already has, such as `check`, is a restyling of an icon every widget draws, so it only applies while a theme names your set (`[meta] icon-set`). An application set never changes the icons of a set the user picked. Give your own meanings their own names, ideally with a prefix.
- **Two of your sets giving one key:** the one added later wins, as text given later wins over a directory.
- **A missing column is reported, not fatal.** Without `nerd` the Unicode glyph stands in, since a Nerd Font draws Unicode too; without `unicode` the ASCII glyph does. Both are warnings with the file, line and column. Without `ascii` nothing plainer can stand in, so the icon is skipped with an error, and the screen shows `⟦key⟧` where it would be.

## Languages

Locale files hold flat keys grouped in sections. `t!("files.count", n = 3)` looks the key up in the active language, then in its `fallback`, then in English. Plural forms are tables such as `{ one = "{n} file", other = "{n} files" }`; the language's own rules pick the form, so Turkish, Russian or Arabic each get theirs. A missing key shows as `⟦files.count⟧` so it is noticed, and `missing_keys` lets a test keep every language complete.

## Checking that a key is translated

`i18n.has("tr", "files.count")` asks whether one language carries a key in its own files. It always names the language, so the active one does not change the answer, and it does not follow fallbacks: a key Turkish would borrow from English is `false` for `"tr"` even though the screen shows the English text. A plural key counts as present. A test that lists the keys an application uses can then require each of them in every language:

```rust
for key in ["app.save", "app.files"] {
    assert!(i18n.has("en", key) && i18n.has("tr", key), "{key} is not translated");
}
```

Comparing `translate(key)` with the key does not work for this: a missing key translates to `⟦key⟧`, which is not the key, so such a test passes when the translation is missing.

## The customs of a language

A language is more than its words. Where it puts a number's decimals, which way round it writes a date, how short its unit of time is beside a number: get one of those wrong and the screen reads as a mistake even when every word is right.

- **Decimals.** `quvyta.number.decimal` says what a language writes between a number's whole part and its decimals — a point in English, Japanese and Chinese, a comma in German, Spanish, French, Portuguese, Russian and Turkish. Every number the framework draws goes through it: a chart's labels, a slider's value, a field's number, a file's size. Write your own numbers with `qframe::i18n::number(value, decimals)` and one screen never mixes the two ways.
- **A number field reads what it writes.** `NumberInput` takes the language's separator when it is typed, and a point as well, because a numeric keypad has one whatever the language is. Change the language while the field stands there and it rewrites itself.
- **Dates.** `Date::written()` is the long form a date field shows (`September 18, 2026`, `18. September 2026`, `2026年9月18日`), `Date::written_short()` the one with the short month, `Date::day_and_month()` the day and month without the year, for a heading, and `Date::day_and_month_short()` the narrowest. Use them instead of putting a day and a month together yourself: that is what makes a heading say `September 18` beside a field saying `18 September`.
- **Units beside a number.** A language writes a unit shorter when it stands next to a number than when it stands alone: Chinese says `分` after a number where the word on its own is `分钟`. The `quvyta.duration.*` and `quvyta.time.*` keys carry the short form; the words that are *read* when someone types a length keep both.

## Switching at runtime

`Command::set_theme`, `Command::set_locale` and `Command::set_icon_mode` change everything at once, without restarting. The lists for a settings screen come from `env.themes()` and `env.i18n().list()`. Saving the choice is up to the application.

The view reads the active language with `ui.env().i18n().active()`. In `update`, `init` and the other `App` methods, where there is no `Env`, `qframe::i18n::active_code()` gives the same code (`tr`, `pt-BR`), also right after a `Command::set_locale`: use it to pick text from a source that is not a locale file, such as the comments a desktop database carries in many languages.

## Common mistakes

- **Hard-coding colours in widgets.** Use tokens; a hex value in code will not follow the theme.
- **Using the accent for status.** Keep meaning in `success`, `warning`, `danger`, `info`.
- **Colour as the only signal.** Pair status colours with an icon or a word.
