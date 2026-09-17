## Tema dosyası

- `[meta]` — `name` (zorunlu), `extends` (tema id), `icon-set` (ikon seti id).
- `[colors]` — on beş değişken `canvas surface raised active overlay accent accent-2 text dim muted ink success warning danger info`; değerler `#hex`, `$değişken` ya da `mix(a, b, N%)`.
- `[motion]` — `"90ms"` ya da `"1.4s"` biçiminde `pulse-period`, `flash`, `cursor-blink`, `step`, `enter`, `spinner`, `shimmer`, `page`, `hover-delay`; `true` ya da `false` olarak `slide`.
- `[typography]` — `fg`, `bold`, `italic`, `underline`, `dim` ile `title`, `body`, `secondary`, `faint` rolleri.
- `[style."bileşen.varyant:durum"]` — `fg`, `bg`, `pillar`, `bold`, `italic`, `underline`, `dim`, `padding = [dikey, yatay]`, `gap` ve bileşenin belgelediği anahtarlar. Bazı anahtarlar sabit bir listeden bir sözcük alır, örneğin `[style.scrollbar] style = "thin"` (`block`, `half`, `thin`, `dots`); listede olmayan sözcük satırıyla birlikte bildirilir.
- `[icons]` — ikon setini ezen ikonlar.
- `[icons] pillar = "thick" | "thin" | "▍"` — her satırın, sekmenin, butonun ve kartın kullandığı tek çubuk: `thick` (`▌`, varsayılan), `thin` (`▎`) ya da tek hücrelik herhangi bir karakter. ASCII terminaller onu renkli bir hücre olarak gösterir.

## Okunabilirlik eşikleri

- Kontrast: canvas ve surface üstünde yazı en az 7; surface üstünde dim 4.5; surface üstünde muted 2.5; surface üstünde her durum rengi 4.5; accent üstünde ink 4.5.
- accent, success, warning, danger ve info arasında en az 0.10 OKLab mesafesi.

## İkon dosyası

- `[meta] name` ve `anahtar = { nerd = "…", unicode = "…", ascii = "…" }` içeren `[icons]`.
- Gömülü anahtarlar: `check check-partial close dot dot-outline select-on select-off radio-mark-small pillar bullet mask prompt enter arrow-left arrow-right arrow-up arrow-down chevron-left chevron-right chevron-down switch-rail switch-knob cap-left cap-right slider-rail slider-knob stepper-minus stepper-plus add search scroll-thumb scroll-track scroll-thin scroll-dot scroll-dot-thumb section-open section-closed tree-collapsed tree-expanded edge-left edge-right crumb-separator path-separator folder file inbox success warning error info`. Spinner stilleri gibi tek hücrelik animasyonlar aynı dosyaların `[animations.<ad>]` tablolarında yaşar; Animasyon stüdyosu sayfasına bak.

## Dil dosyası

- `[meta]` — `name`, `code`, isteğe bağlı `fallback`.
- Anahtar bölümleri; değerler `{isim}` yer tutuculu metinler ya da `zero one two few many other` içeren çoğul tablolarıdır (`other` zorunlu).

## Kod

- `t!("anahtar")`, `t!("anahtar", n = 3, name = değer)` — etkin dille çevirir.
- `Command::set_theme(id)`, `Command::set_locale(kod)`, `Command::set_icon_mode(IconMode::Ascii)`.
- `env.theme()`, `env.themes()`, `env.icons()`, `env.icon_mode()`, `env.glyph_mode()`, `env.i18n()`, `env.keymap()`, `env.diagnostics()`.
- `Runtime::theme_dir`, `icon_dir`, `locale_dir` — dosyaları yükler.
- `Runtime::locale_source` — metin olarak verilen dil dosyasını yükler; `include_str!` ile programa gömülen dosyalar için.
- `Command::set_pillar(PillarStyle::Thin)`, `Command::set_slide(false)`, `Command::set_reduced_motion(true)` — her ekranın hissini çalışırken değiştirir; `Env::pillar_style`, `Env::slide`, `Env::reduced_motion` geçerli seçimi okur; `Settings::PILLAR`, `Settings::SLIDE` ve `Settings::REDUCED_MOTION` onu hatırlar.
