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

## Icon file

- `[meta] name`, and `[icons]` with `key = { nerd = "…", unicode = "…", ascii = "…" }`.
- Built-in keys: `check check-partial close dot dot-outline select-on select-off radio-mark-small pillar bullet mask prompt enter arrow-left arrow-right arrow-up arrow-down chevron-left chevron-right chevron-down switch-rail switch-knob cap-left cap-right slider-rail slider-knob stepper-minus stepper-plus add search scroll-thumb scroll-track scroll-thin scroll-dot scroll-dot-thumb section-open section-closed tree-collapsed tree-expanded edge-left edge-right crumb-separator path-separator folder file inbox success warning error info`. One-cell animations such as the spinner styles live in `[animations.<name>]` tables of the same files; see the Animation studio page.

## Locale file

- `[meta]` — `name`, `code`, optional `fallback`.
- Sections of keys; values are strings with `{name}` placeholders or plural tables with `zero one two few many other` (`other` required).

## Code

- `t!("key")`, `t!("key", n = 3, name = value)` — translate with the active language.
- `Command::set_theme(id)`, `Command::set_locale(code)`, `Command::set_icon_mode(IconMode::Ascii)`.
- `env.theme()`, `env.themes()`, `env.icons()`, `env.icon_mode()`, `env.glyph_mode()`, `env.i18n()`, `env.keymap()`, `env.diagnostics()`.
- `Runtime::theme_dir`, `icon_dir`, `locale_dir` — load files.
- `Runtime::locale_source` — loads a locale file given as text, such as one compiled in with `include_str!`.
- `Command::set_pillar(PillarStyle::Thin)`, `Command::set_slide(false)`, `Command::set_reduced_motion(true)` — change how every screen feels at runtime; `Env::pillar_style`, `Env::slide`, `Env::reduced_motion` read the current choice; `Settings::PILLAR`, `Settings::SLIDE` and `Settings::REDUCED_MOTION` remember it.
