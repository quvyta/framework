## Metotlar

- `Bar::new(etiket, değer)` — tek bir çubuk; negatif değerler sıfır sayılır.
- `Bar::value_text(metin)` — değer yerine gösterilen metin.
- `Bar::variant(isim)` — çubuğun tema varyantı; değerinden önce bir işaret ekler.
- `Series::new(ad, değerler)` — tek bir seri; kategori sırasına göre kategori başına bir değer. Daha kısa bir seri o noktadan sonra sıfır sayılır.
- `Series::tone(sıra)` — serinin konumunun tonu yerine temanın `sıra`'ncı seri tonunu alır; böylece bir kategori her grafikte rengini korur. Verilmezse hiçbir şey değişmez.
- `BarChart::new(çubuklar)` — sade çubuklardan oluşan yatay bir grafik.
- `BarChart::series(etiketler, seriler)` — `etiketler` kategorilerinden oluşan, her serisinde kategori başına bir değer bulunan yatay bir grafik; bir kategorinin çubukları yan yana durur.
- `.stacked()` — serileri kategori başına tek çubuğun payları olarak çizer.
- `.vertical()` — yan yana çubuklar; değer üstte, etiket altta.
- `.max(değer)` — tam bir çubuğun değeri; varsayılan en büyük değer, yığılmışsa en büyük kategori toplamıdır.
- `.gap(hücre)` — yatay kategoriler arasındaki satırlar ya da dikey kategoriler arasındaki sütunlar; varsayılan 1. Dikey grafikte kategorilerin arasında her zaman en az bir sütun kalır, böylece birbirine karışmazlar; bir grubun içindeki çubukların arasında boşluk yoktur.
- `.unit(metin)` — grafiğin kendi biçimlendirdiği her değerden sonra yazılır; kendi `value_text`'i olan çubuk o metni korur.
- `.selected(kategori)` — yükselen zemine oturan kategori.
- `.on_select(mesaj)` — seçimi bir kategoriye taşımanın mesajı; fare ve klavye yanıtını açar.
- `.disabled(pasif)` — grafiği soluklaştırır; odak almaz, tuş ve fare yanıtlamaz.

## Davranış

- Yatay grafik bir kategorinin çubuk başına bir satırını ve boşlukları ölçer; yoğun listeler için `.gap(0)` onları sıkıştırır; dikey grafik yükseklik verilmezse sekiz satır ölçer.
- Yatay etiketler ve değerlerin önündeki seri adları 24 hücre genişliğin altında bırakılır. Dikey çubuklar en fazla altı hücre geniştir; seri başına bir sütun veremeyecek kadar ince bir grup yığılır.
- İki satırlık dikey alan değer satırını, bir satırlık alan etiketleri de bırakır.
- Sekizde bir hücre hassasiyeti, ASCII modunda tam hücre; sıfırdan büyük her değer en az bir sekizde bire uzanır. Yığının payları hücre kenarlarında biter, çubuğun sekizde birlik kuyruğu yığının son serisine aittir.
- Yalnızca `on_select` ile odak alır. Tuşlar: yatay grafikte ↑↓ ya da k ve j, dikeyde ←→ ya da h ve l, uçlar için Home ve End. Sol tık imlecin altındaki kategoriyi seçer; iki kategori arasındaki boşluk hiçbirine ait değildir. Zaten seçili olan kategoriyi seçmek mesaj göndermez.
- Hover ve seçim ne genişliği, ne yüksekliği, ne de bir sütunu değiştirir: seçim gösterebilen bir grafiğin baştaki hücreleri ilk kareden beri boş tutulur ve grafik asla kaymaz.

## Tema anahtarları

- `bar-chart`, `bar-chart.<varyant>` — `fill`.
- `bar-chart-bar` ve `hover`, `selected`, `focus` durumları — `bg`, `pillar`. Bunlar yokken imlecin altındaki kategori yükselen zemini, seçili olan etkin zemini, vurgu çubuğu da vurgu rengini alır.
- `bar-chart-label` — `fg`.
- `bar-chart-value`, `bar-chart-value.<varyant>` — `fg`, `bold`.
- `series-<n>` renk anahtarları — `n`. serinin tonu. Bunlar yokken tonlar `accent`'ten `muted`'a giden bir merdiveni yürür ve dört seriden sonra başa döner; pasif grafik `muted`'tan `dim`'e giden bir merdiveni yürür.
