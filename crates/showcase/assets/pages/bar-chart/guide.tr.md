## Ne zaman kullanılır

Birkaç değeri yan yana karşılaştırmak için çubuk grafik kullan: servis başına bellek, gün başına dağıtım, uç nokta başına hata. Yatay çubuklar uzun isimlere ve sıralı listelere, dikey çubuklar günler ya da saatler gibi bir diziye uyar. Bir kategori birkaç paydan oluşuyorsa grafiğe seri ver: toplam önemliyse yığ, paylar karşılaştırılacaksa grupla. Çok sayıda örnekteki eğilim için sparkline kullan.

## Adım adım

1. Çubukları kur: `Bar::new("postgres-16", 1536.0)`.
2. Değerleri birimleriyle göster: `.value_text("1536 MiB")` ya da tüm grafiğe bir birim ver: `.unit("sa")`.
3. Bir grafiğe ekle: `ui.add(BarChart::new(çubuklar)).width(Length::Cells(72))`.
4. Yalnızca anlamı olan çubukları işaretle: sınırını aşan bir servis için `.variant("danger")`. Gerisini vurgu renginde bırak.
5. Bir dizi için onları dik koy: `.vertical()` ve düğüme bir yükseklik ver.
6. Çubuklar bilinen bir tam değere göre okunacaksa onu belirt: `.max(100.0)`.
7. Kategori başına birkaç değer için kategorileri ve serileri adlandır: `BarChart::series(günler, [Series::new("Rust", saatler)])`. Çubuklar yan yana durur; `.stacked()` onları tek çubukta üst üste koyar.
8. Kişinin bir kategoriyi göstermesini istiyorsan seçimi kendi durumunda tut ve geri ver: `.selected(state.gun).on_select(Msg::Gun)`.
9. Bir kategori haftadan haftaya aynı renkte kalsın diye onu paletteki bir tona bağla: `Series::new("Belge", saatler).tone(1)`. Aynı sayılar `Legend::new(adlar).tones([1, 2])` ile göstergeye de verilir.

## Nasıl çalışır

- **Sekizde bir hücrelik çubuklar.** Yatay çubuklar `▏`…`▉` ile, dikeyler `▁`…`▇` ile biter; birbirine yakın değerler de farklı görünür. Sıfır olmayan her değer en az bir sekizde bire uzanır; böylece çok daha büyük bir payın yanındaki küçük pay yok sayılmaz, görülür.
- **Çubuklar arası boşluk.** Yatay çubukları varsayılan olarak bir boş satır ayırır; böylece her biri ayrı bir şekil olarak okunur; `.gap(0)` onları sıkıştırır. Bir grubun çubukları yan yana, aralıksız durur; böylece grup tek bir şekil olarak okunur.
- **Etiketler ve değerler.** Yatayda etiketler solda (genişliğin en fazla beşte ikisi, `…` ile kesilir), değerler sağa hizalı. Dikeyde değer her çubuğun hemen üstünde, etiket altında durur. Grup her değerin önüne seri adını yazar; yığın toplamı yazar.
- **Seri tonları.** Kararı tema verir: `series-<n>` renk anahtarları. Onlar yoksa ton, vurgudan temanın soluk ucuna giden bir merdiveni yürür; böylece grafik temanın tek vurgu renginin içinde kalır. Merdiven dört seriden sonra başa döner.
- **Yığın kendini adlandırır.** Yatay bir pay, adı iki yanında birer hücre boşlukla sığıyorsa adını kendi içine yazar; böylece yığın hiçbir zaman yalnızca renkle okunmaz. Adların sığmadığı yerde — dar bir çubuk, dik bir çubuk — serileri grafiğin yanındaki bir satır metinle adlandır.
- **İşaretle anlam.** Varyantı olan çubuk değerinden önce bir `●` taşır; tehlike çubuğu renk olmadan da ayrı okunur.
- **Hover ve seçim.** `on_select` verildiğinde imlecin altındaki kategori yükselen zemine, seçili olan etkin zemine oturur; yatay grafik ayrıca satırın kendisine ayrılmış hücresine vurgu çubuğunu diker. O hücreler ilk kareden beri boş tutulur; imleç geldiğinde hiçbir şey yerinden oynamaz: grafik bir liste değildir, asla kaymaz.
- **Klavye.** Oklar çubukların uzandığı eksene uyar: yatay grafikte ↑↓ ya da k ve j, dikeyde ←→ ya da h ve l. Home ve End uçlara atlar.
- **Dar alanlar.** 24 hücrenin altında yatay grafik etiketlerini ve değerlerin önündeki seri adlarını bırakır; dikey çubuklar bir hücreye kadar incelir, etiketler kesilir. Her seriye bir sütun veremeyecek kadar incelen dik bir grup onları üst üste yığar; böylece hiçbir seri düşmez.
- **Kısa alanlar.** Dikey grafik önce değer satırını, sonra etiketleri bırakır; çubukları korur.
- **ASCII modu.** Çubuklar tam hücreye yuvarlanır.

## Sık yapılan hatalar

- **Her çubuğa başka renk.** Renk anlam içindir; servislerden oluşan bir gökkuşağı tehlike çubuğunu görünmez yapar. Tek istisna seri tonlarıdır ve onlar temadan gelir.
- **Sıralanmamış yatay çubuklar.** Sıralı veri, en büyüğü başta olunca en kolay okunur.
- **Rengi değişen kategori.** Bu hafta üç iş türü, geçen hafta iki tür görünüyorsa n'inci seri her hafta başka bir türdür. Her kategoriye kendi `Series::tone(n)` değerini ver, aynı sayıları `.tones(...)` ile `Legend`'e de ver.
- **Çok fazla seri.** Dörtten sonra tonlar başa döner; bir yığının söyleyebileceği en fazla üç dört paydır.
- **Payların toplamdan önemli olduğu yerde yığın.** Onları gruplamak gerekir; yoksa okuyan kişi farklı noktalardan başlayan uzunlukları karşılaştırmak zorunda kalır.
- **Çok fazla çubuk.** Bir düzineyi geçince her satırında küçük bir çubuk olan bir tablo düşün.
