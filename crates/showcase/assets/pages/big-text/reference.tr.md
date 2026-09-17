## Metotlar

- `BigText::new(metin)` — büyük `metin`.
- `.variant(isim)` — `"accent"` ya da `"dim"` gibi tema varyantı.

## Davranış

- Gliflerinin genişliğini, aralarında birer sütunla birlikte ve üç satırı (ASCII modunda beş) ölçer.
- `0`–`9`, `:`, `.`, `%`, `-`, `A`–`Z` desteklenir (küçük harfler büyük çizilir); diğer karakterler iki sütunluk boşluk olarak çizilir.
- Alan büyük biçimden küçükse metni normal boyda ve kalın çizer, gerekirse `…` ile keser.
- Odak almaz, mesaj göndermez.

## Tema anahtarları

- `big-text`, `big-text.<varyant>` — `fg`.
