## Theme file

- `[meta]` — `name` (required), `extends` (theme id), `icon-set` (icon set id).
- `[colors]` — the fifteen tokens `canvas surface raised active overlay accent accent-2 text dim muted ink success warning danger info`; values `#hex`, `$token` or `mix(a, b, N%)`.
- `[motion]` — `pulse-period`, `flash`, `cursor-blink`, `step`, `enter`, `spinner`, `shimmer`, `page`, `hover-delay` as durations like `"90ms"` or `"1.4s"`; `slide` as `true` or `false`.
- `[typography]` — roles `title`, `body`, `secondary`, `faint` with `fg`, `bold`, `italic`, `underline`, `dim`.
- `[style."widget.variant:state"]` — `fg`, `bg`, `pillar`, `bold`, `italic`, `underline`, `dim`, `padding = [v, h]`, `gap`, and keys a widget documents. A few keys take a word from a fixed list, such as `[style.scrollbar] style = "thin"` (`block`, `half`, `thin`, `dots`); any other word is reported with its line.
- `[icons]` — icons overriding the icon set.
- `[icons] pillar = "thick" | "thin" | "▍"` — the one pillar every row, tab, button and card uses: `thick` (`▌`, default), `thin` (`▎`) or any one-cell character. ASCII terminals show it as a coloured cell.

## Readability thresholds

- Contrast: text on canvas and surface at least 7; dim on surface 4.5; muted on surface 2.5; each status colour on surface 4.5; ink on accent 4.5.
- OKLab distance of at least 0.10 between accent, success, warning, danger and info.

## Colour depth

- `ColorDepth::detect(env)` — `TrueColor`, `Ansi256` or `Ansi16` from `COLORTERM`, `TERM` and the terminal's name; `Harness::set_depth` draws a test at one of them.
- Sixteen colours: a frame is painted in full colour and reduced when complete. Backgrounds take `Rgb::to_ansi16_on(canvas)`: the nearest of the sixteen, except that a tone at least 0.05 from the canvas in OKLab that would land on the canvas's colour moves one grey away along black, bright black, white, bright white. Text takes `Rgb::to_ansi16_text(bg, canvas)`: the same, and under 1.6:1 against its background the quietest grey that keeps 3:1. Text in the exact colour of its background is left alone.
- `Rgb::to_ansi16()` is the plain nearest entry and `Rgb::from_ansi16(index)` the xterm default colour of an entry.
- 256 colours: a frame is painted in full colour and reduced when complete. Backgrounds take `Rgb::to_ansi256()`, the nearest palette entry. Text takes `Rgb::to_ansi256_text(bg)`: the nearest entry, and under 1.6:1 against its background's entry the entry closest to it in OKLab that keeps 1.6:1. Text in the exact colour of its background is left alone. `Rgb::from_ansi256(index)` is the xterm default colour of an entry.

## Icon file

- `[meta] name`, and `[icons]` with `key = { nerd = "…", unicode = "…", ascii = "…" }`.
- A missing `nerd` glyph is a warning and the `unicode` glyph stands in; a missing `unicode` glyph is a warning and the `ascii` glyph stands in; a missing `ascii` glyph is an error and the icon is skipped. Each names the file, line and column of the icon.
- An application's sets (from `icon_dir` and `icon_source`): keys the built-in set lacks are drawn in every set, under the chosen set and the theme's `[icons]`, a later set winning a key two of them give; keys the built-in set has apply only while a theme names the set.
- Built-in keys: `check check-partial close dot dot-outline select-on select-off radio-mark-small pillar bullet mask prompt enter arrow-left arrow-right arrow-up arrow-down chevron-left chevron-right chevron-down switch-rail switch-knob cap-left cap-right slider-rail slider-knob stepper-minus stepper-plus add search scroll-thumb scroll-track scroll-thin scroll-dot scroll-dot-thumb section-open section-closed tree-collapsed tree-expanded edge-left edge-right crumb-separator path-separator folder file inbox success warning error info help project workspace profile settings power window-minimize window-maximize window-restore category-system category-development category-network category-office category-media category-files ecosystem cpu memory battery-full battery-half battery-empty battery-charging terminal session network-down network-up`. One-cell animations such as the spinner styles live in `[animations.<name>]` tables of the same files; see the Animation studio page.

## Locale file

- `[meta]` — `name`, `code`, optional `fallback`.
- Sections of keys; values are strings with `{name}` placeholders or plural tables with `zero one two few many other` (`other` required).

- `quvyta.number.decimal` — what this language writes between a number's whole part and its decimals. `I18n::decimal_separator()` and `qframe::i18n::decimal_separator()` give it; `qframe::i18n::number(value, decimals)` writes a number with it. A point for a language that does not give it.
- `quvyta.date.format`, `format-short`, `format-day-month-long`, `format-day-month` — the four forms of a date, behind `Date::written()`, `written_short()`, `day_and_month()` and `day_and_month_short()`. A language whose month names change inside a date gives them as `month-in-date-1` to `month-in-date-12`.
- `quvyta.duration.*` and `quvyta.time.*` — units as they are written beside a number, which is shorter than the word on its own in Chinese. `*-words` keeps every word that is read when a length is typed, long forms included.

## Code

- `t!("key")`, `t!("key", n = 3, name = value)` — translate with the active language.
- `i18n.has(code, key)` — whether the files of language `code` carry `key` themselves; it ignores the active language, does not follow fallbacks, and counts a plural key. `i18n.missing_keys(code, reference)` — the keys `reference` has and `code` lacks.
- `qframe::i18n::active_code() -> String` — the code of the active language (`tr`, `pt-BR`), the same as `env.i18n().active()`, for `update`, `init` and the other `App` methods where there is no `Env`; follows `Command::set_locale`. `en` outside the runtime.
- `Command::set_theme(id)`, `Command::set_locale(code)`, `Command::set_icon_mode(IconMode::Ascii)`.
- `env.theme()`, `env.themes()`, `env.icons()`, `env.icon_sets()`, `env.icon_mode()`, `env.glyph_mode()`, `env.i18n()`, `env.keymap()`, `env.diagnostics()`.
- `Runtime::theme_dir`, `icon_dir`, `locale_dir`, `keymap_file` — load files from paths.
- `Runtime::theme_source(file, text)`, `icon_source(file, text)`, `keymap_source(file, text)`, `locale_source(file, text)` — load the TOML text itself, such as an `include_str!`, so an installed binary needs no files beside it. Text loads after the paths, so it wins; for themes and icon sets the file stem is the id. A path named as well is optional: when it cannot be read the text stands in for it and the reason is a diagnostic instead of an error.
- `Command::set_pillar(PillarStyle::Thin)`, `Command::set_slide(false)`, `Command::set_reduced_motion(true)` — change how every screen feels at runtime; `Env::pillar_style`, `Env::slide`, `Env::reduced_motion` read the current choice; `Settings::PILLAR`, `Settings::SLIDE` and `Settings::REDUCED_MOTION` remember it.
