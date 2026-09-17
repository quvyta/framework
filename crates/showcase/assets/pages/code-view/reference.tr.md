## Metotlar

- `CodeView::new(kod, dil)` — `Language::Rust`, `Language::Toml` ya da `Language::Plain`.
- `.line_numbers(bool)` — Varsayılan: `true`.
- `.on_copy(msg)` — `c` ile kopyalandıktan sonra gönderilir.

## Tuşlar

- Odaktayken: `c` tüm kodu kopyalar ve parlar.
- Fare: kodun içinde sürüklemek kodu seçer, seçim iç boşluğa taşmaz; satır numarası sütunu süstür, temiz kopyaya girmez.

## Dil

- `Language::from_tag("rust" | "rs" | "toml" | diğer)` — kod bloğu etiketleri için.

## Tema anahtarları

- `focus` ve `pressed` ile `code` — `bg`, `padding`.
- `code-line-number` — `fg`.
- `code-token.<tür>` — `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `plain`.
