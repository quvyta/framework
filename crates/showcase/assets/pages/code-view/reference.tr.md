## Metotlar

- `CodeView::new(kod, dil)` — `Language::Rust`, `Language::Toml`, `Language::Shell` ya da `Language::Plain`.
- `.line_numbers(bool)` — Varsayılan: `true`.
- `.on_copy(msg)` — `c` ile kopyalandıktan sonra gönderilir.
- `.line_marks(işaretler)` — 1. satırdan başlayarak her satıra bir `LineMark`: `Added` (yeşil ton, `+`), `Removed` (kırmızı ton, `−`), `Unchanged`. Varsayılan: yok.
- `.highlight_lines(aralık, ton)` — satırlar 1'den sayılır, `4..=6` ya da `9..` gibi her aralık olur; `LineTone::Accent` (dikey çubuk) ya da `LineTone::Warning` (uyarı ikonu). Fark işaretini geçer; aralıklar çakışınca sonraki çağrı kazanır.
- `.reveal(satır)` — satır değişince çevreleyen `ScrollView` satırı iki satır bağlamla gösterir; kayarak, hareket azaltılmışsa bir anda. Sonu geçerse son satır.

## Tuşlar

- Odaktayken: `c` tüm kodu kopyalar ve parlar.
- Fare: kodun içinde sürüklemek kodu seçer, seçim iç boşluğa taşmaz; satır numarası sütunu süstür, temiz kopyaya girmez.

## Dil

- `Language::from_tag("rust" | "rs" | "toml" | "sh" | "bash" | "shell" | "zsh" | "pkgbuild" | diğer)` — kod bloğu etiketleri için.
- `Language::from_file_name(ad)` — `PKGBUILD`, `.sh`, `.bash`, `.zsh`, `.install` kabuk; `.rs` Rust; `.toml` TOML; gerisi düz metin.

## Tema anahtarları

- `focus` ve `pressed` ile `code` — `bg`, `padding`.
- `code-line-number` — `fg`.
- `code-token.<tür>` — `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `variable`, `plain`.
- `code-line.<görünüm>` — sıra için `bg`, işareti için `fg`; görünüm `added`, `removed`, `accent` ya da `warning`.

## İkonlar

- `line-added`, `line-removed` — fark işaretleri; vurgular için `warning` ve dikey çubuk.
