## Metotlar

- `BigText::new(metin)` — büyük `metin`.
- `BigText::fits(metin)` — `metin`in her karakterinin büyük biçimi var mı; varsa glif olarak çizilir, yoksa kalın düz metin olarak. Boşluk sayılır, iki sözcük arasında boşluk olarak çizilir.
- `.variant(isim)` — `"accent"` ya da `"dim"` gibi tema varyantı.
- `.gradient(hedef, yön)` — harfleri `hedef` tema rengine doğru karıştırır; yön `Gradient::Columns` (soldan sağa) ya da `Gradient::Rows` (yukarıdan aşağı).

## Davranış

- Gliflerinin genişliğini, aralarında birer sütunla birlikte ve üç satırı (ASCII modunda beş) ölçer.
- `0`–`9`, `:`, `.`, `%`, `-`, `'`, `!`, `?`, `&`, `(`, `)`, `,`, `/` ve bütün Latin harflerini çizer: A–Z harfleri, Türkçe `Ç Ğ İ Ö Ş Ü` ve Batı Avrupa `À Á Â Ä Å È É Ê Ë Ì Í Î Ï Ñ Ò Ó Ô Ö Ø Ù Ú Û Ü Ý Ÿ ß Æ Œ` biçimleriyle.
- Küçük harf, karakterinin büyük harf biçimiyle çizilir: `ı` her dilde `I`, `i` ise etkin dil Türkçe ya da Azerbaycanca olduğunda `İ`, başka her dilde `I`. Hangi dilin geçerli olduğuna boyanırken `Env::i18n` verir.
- Bu yazı tipinin çizemediği bir karakteri olan metin — Kiril, Japon, bir emoji — o karakterin yerinde delik bırakmak yerine baştan sona normal boyda ve kalın çizilir ve çizdiği tek satırı ölçer; glifleri alana sığmayan metin de böyle çizilir, gerekirse `…` ile kesilir. Bu hâl düz rengi korur.
- Gradient yoksa harfler tek renk alır: stilin `fg` değeri. Gradient o renkten adı verilen tema rengine, gittiği yöndeki her hücrede bir adım ilerleyerek karışır.
- Gradient `ColorDepth::Ansi16`'da ve tema adı verilen rengi tanımadığında düz renge düşer; `Ansi256` geçişi korur.
- Durağandır: geçiş hiç oynamaz. Odak almaz, mesaj göndermez.

## Tema anahtarları

- `big-text`, `big-text.<varyant>` — `fg`.
