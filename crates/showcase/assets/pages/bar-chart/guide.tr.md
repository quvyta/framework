## Ne zaman kullanılır

Birkaç değeri yan yana karşılaştırmak için çubuk grafik kullan: servis başına bellek, gün başına dağıtım, uç nokta başına hata. Yatay çubuklar uzun isimlere ve sıralı listelere, dikey çubuklar günler ya da saatler gibi bir diziye uyar. Çok sayıda örnekteki eğilim için sparkline kullan.

## Adım adım

1. Çubukları kur: `Bar::new("postgres-16", 1536.0)`.
2. Değerleri birimleriyle göster: `.value_text("1536 MiB")`.
3. Bir grafiğe ekle: `ui.add(BarChart::new(çubuklar)).width(Length::Cells(72))`.
4. Yalnızca anlamı olan çubukları işaretle: sınırını aşan bir servis için `.variant("danger")`. Gerisini vurgu renginde bırak.
5. Bir dizi için onları dik koy: `.vertical()` ve düğüme bir yükseklik ver.
6. Çubuklar bilinen bir tam değere göre okunacaksa onu belirt: `.max(100.0)`.

## Nasıl çalışır

- **Sekizde bir hücrelik çubuklar.** Yatay çubuklar `▏`…`▉` ile, dikeyler `▁`…`▇` ile biter; birbirine yakın değerler de farklı görünür.
- **Çubuklar arası boşluk.** Yatay çubukları varsayılan olarak bir boş satır ayırır; böylece her biri ayrı bir şekil olarak okunur; `.gap(0)` onları sıkıştırır.
- **Etiketler ve değerler.** Yatayda etiketler solda (genişliğin en fazla beşte ikisi, `…` ile kesilir), değerler sağa hizalı. Dikeyde değer her çubuğun hemen üstünde, etiket altında durur.
- **İşaretle anlam.** Varyantı olan çubuk değerinden önce bir `●` taşır; tehlike çubuğu renk olmadan da ayrı okunur.
- **Dar alanlar.** 24 hücrenin altında yatay grafik etiketlerini bırakır; dikey çubuklar bir hücreye kadar incelir, etiketler kesilir.
- **ASCII modu.** Çubuklar tam hücreye yuvarlanır.

## Sık yapılan hatalar

- **Her çubuğa başka renk.** Renk anlam içindir; servislerden oluşan bir gökkuşağı tehlike çubuğunu görünmez yapar.
- **Sıralanmamış yatay çubuklar.** Sıralı veri, en büyüğü başta olunca en kolay okunur.
- **Çok fazla çubuk.** Bir düzineyi geçince her satırında küçük bir çubuk olan bir tablo düşün.
