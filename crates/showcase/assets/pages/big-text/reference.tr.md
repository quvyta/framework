## Metotlar

- `BigText::new(metin)` — büyük `metin`.
- `.variant(isim)` — `"accent"` ya da `"dim"` gibi tema varyantı.
- `.gradient(hedef, yön)` — harfleri `hedef` tema rengine doğru karıştırır; yön `Gradient::Columns` (soldan sağa) ya da `Gradient::Rows` (yukarıdan aşağı).

## Davranış

- Gliflerinin genişliğini, aralarında birer sütunla birlikte ve üç satırı (ASCII modunda beş) ölçer.
- `0`–`9`, `:`, `.`, `%`, `-`, `A`–`Z` desteklenir (küçük harfler büyük çizilir); diğer karakterler iki sütunluk boşluk olarak çizilir.
- Alan büyük biçimden küçükse metni normal boyda ve kalın çizer, gerekirse `…` ile keser; bu hâl düz rengi korur.
- Gradient yoksa harfler tek renk alır: stilin `fg` değeri. Gradient o renkten adı verilen tema rengine, gittiği yöndeki her hücrede bir adım ilerleyerek karışır.
- Gradient `ColorDepth::Ansi16`'da ve tema adı verilen rengi tanımadığında düz renge düşer; `Ansi256` geçişi korur.
- Durağandır: geçiş hiç oynamaz. Odak almaz, mesaj göndermez.

## Tema anahtarları

- `big-text`, `big-text.<varyant>` — `fg`.
