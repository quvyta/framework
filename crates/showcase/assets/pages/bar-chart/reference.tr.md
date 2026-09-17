## Metotlar

- `Bar::new(etiket, değer)` — tek bir çubuk; negatif değerler sıfır sayılır.
- `Bar::value_text(metin)` — değer yerine gösterilen metin.
- `Bar::variant(isim)` — çubuğun tema varyantı; değerinden önce bir işaret ekler.
- `BarChart::new(çubuklar)` — yatay bir grafik.
- `.vertical()` — yan yana çubuklar; değer üstte, etiket altta.
- `.max(değer)` — tam bir çubuğun değeri; varsayılan en büyük değerdir.
- `.gap(hücre)` — yatay çubuklar arasındaki satırlar ya da dikey çubuklar arasındaki sütunlar; varsayılan 1. Dikey çubukların arasında her zaman en az bir sütun kalır, böylece birbirine karışmazlar.

## Davranış

- Yatay grafik çubuk başına bir satır ve boşlukları ölçer; yoğun listeler için `.gap(0)` onları sıkıştırır; dikey grafik yükseklik verilmezse sekiz satır ölçer.
- Yatay etiketler 24 hücre genişliğin altında bırakılır. Dikey çubuklar en fazla altı hücre geniştir.
- Sekizde bir hücre hassasiyeti; ASCII modunda tam hücre. Odak almaz, mesaj göndermez.

## Tema anahtarları

- `bar-chart`, `bar-chart.<varyant>` — `fill`.
- `bar-chart-label` — `fg`.
- `bar-chart-value`, `bar-chart-value.<varyant>` — `fg`, `bold`.
