## Metotlar

- `Markdown::new(kaynak)` — belgeyi ayrıştırır.
- Belgenin tamamı bir metin seçim bölgesidir; başlık ve alıntı çubukları (boşluklarıyla), kod bloklarının iç boşluğu ve satır numaraları temiz kopya için süstür.

## Bloklar

- Çubuklu `#` ve `##` başlıklar; `###` ve daha derini sessiz başlık.
- Paragraflar, yumuşak ve sert satır sonları.
- İç içe `-` ve `1.` listeleri.
- `>` alıntılar.
- `rust`, `toml` ya da dilsiz kod blokları.
- `---` boşluğa dönüşür.

## Satır içi

- `**kalın**`, `*vurgu*`, `` `kod` ``, `[bağlantı](url)` (yalnızca metin).

## Tema anahtarları

- `h1`, `h2`, `h3` varyantlarıyla `markdown-heading` — `fg`, `bold`, `pillar`.
- `markdown-text`, `markdown-strong`, `markdown-emphasis`, `markdown-code`, `markdown-link`, `markdown-bullet`, `markdown-quote` (`fg`, `italic`, `pillar`).
- Kod blokları `code`, `code-line-number` ve `code-token.<tür>` kullanır.
