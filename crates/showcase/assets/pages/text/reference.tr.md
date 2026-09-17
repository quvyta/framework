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
- `qframe::text::{width, grapheme_width, truncate, wrap, wrap_ranges}` özel bileşenler için metni ölçer ve keser.

## Tema anahtarları

- `fg`, `bold`, `italic`, `underline`, `dim` ile `[typography]` rolleri.
