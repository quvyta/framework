## Three files, no code

An application gives quvyta-framework **theme files**, **icon files** and **language files**. The framework ships a complete default of each, so an application runs without any of them; your own files replace or extend the defaults. Because they are plain files, adding a theme or a language is copy, rename, edit.

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

## Icons

Every icon has three glyphs: `nerd` for Nerd Fonts, `unicode`, and `ascii`. In `auto` mode the framework picks by looking at the terminal and the installed fonts; users can choose a mode, and `QUVYTA_ICONS=ascii` forces one. ASCII glyphs never use brackets to fake shapes.

## Languages

Locale files hold flat keys grouped in sections. `t!("files.count", n = 3)` looks the key up in the active language, then in its `fallback`, then in English. Plural forms are tables such as `{ one = "{n} file", other = "{n} files" }`; the language's own rules pick the form, so Turkish, Russian or Arabic each get theirs. A missing key shows as `⟦files.count⟧` so it is noticed, and `missing_keys` lets a test keep every language complete.

## Switching at runtime

`Command::set_theme`, `Command::set_locale` and `Command::set_icon_mode` change everything at once, without restarting. The lists for a settings screen come from `env.themes()` and `env.i18n().list()`. Saving the choice is up to the application.

## Common mistakes

- **Hard-coding colours in widgets.** Use tokens; a hex value in code will not follow the theme.
- **Using the accent for status.** Keep meaning in `success`, `warning`, `danger`, `info`.
- **Colour as the only signal.** Pair status colours with an icon or a word.
