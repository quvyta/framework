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

## Icons

Every icon has three glyphs: `nerd` for Nerd Fonts, `unicode`, and `ascii`. In `auto` mode the framework picks by looking at the terminal and the installed fonts; users can choose a mode, and `QUVYTA_ICONS=ascii` forces one. ASCII glyphs never use brackets to fake shapes.

The set answers the meanings an application's main menu shows, so every application in the family draws the same shapes: `project`, `profile`, `settings` and `power`. Nerd Font draws the thing itself; the Unicode column stays inside Geometric Shapes, which terminals and monospace fonts draw as one cell, so no row falls back to another font and overflows.

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

## Switching at runtime

`Command::set_theme`, `Command::set_locale` and `Command::set_icon_mode` change everything at once, without restarting. The lists for a settings screen come from `env.themes()` and `env.i18n().list()`. Saving the choice is up to the application.

## Common mistakes

- **Hard-coding colours in widgets.** Use tokens; a hex value in code will not follow the theme.
- **Using the accent for status.** Keep meaning in `success`, `warning`, `danger`, `info`.
- **Colour as the only signal.** Pair status colours with an icon or a word.
