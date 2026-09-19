## Text

- `Text::new(metin)` — gövde metni.
- `Text::rich([parçalar])` — parçalardan oluşan metin.
- `.role(isim)` — tipografi rolü: `title`, `body` (varsayılan), `secondary`, `faint`.
- `.color(değişken)` — kendi rengi olmayan her parça için renk değişkeni.
- `.bold()` — her parça kalın.
- `.no_wrap()` — her kaynak satır tek satır, `…` ile kesilir. Varsayılan: sarar.
- `.align(Align)` — `Start` (varsayılan), `Center`, `End`.
- Fareyle seçilemez; düğümünde `NodeMut::selectable(true)` onu bir seçim bölgesi yapar.

## Span

- `Span::new(metin)`, `.role(isim)`, `.color(değişken)`, arka plan için `.on(değişken)`, `.bold()`.

## Ölçüm

- Genişlik en geniş satırdır; yükseklik sarılmış satır sayısıdır. `Length::Fill` ile ya da genişlik verilmeden metin, yerleşimin verdiği genişlikte sarılır.
- Sarma boşluklarda kırar; boşluksuz bir dizi (noktalamasıyla bir kelime, `2026.9.1` gibi bir sürüm) bütün olarak sonraki satıra geçer, satırdan genişse grafemler arasından bölünür.
- Çince ve Japonca ideogram ve kana arasında kırılır; kapanış işaretleri (`。` `、` `，` `！` `）` `」` `ー`, küçük kana ve tam genişlik biçimleri) satır başına, açılış parantezleri (`「` `（` `【`) satır sonuna düşmez. Korece kelimeler bütün kalır.
- Bölünmez boşluklarda (U+00A0, U+202F, U+2007) satır kırılmaz.
- `qframe::text::{width, grapheme_width, truncate, truncate_middle, wrap, wrap_ranges}` özel bileşenler için metni ölçer ve keser.
- `truncate_middle(metin, en_fazla) -> Cow<str>` — en fazla `en_fazla` hücre, ortası `…` olur; sığıyorsa değişmez, tek kalan hücre sona gider, 1'de yalnızca `…`, 0'da hiçbir şey.

## Tema anahtarları

- `fg`, `bold`, `italic`, `underline`, `dim` ile `[typography]` rolleri.
