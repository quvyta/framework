## Ne zaman kullanılır

Bir sayının yanında yakın geçmişin şeklini göstermek için sparkline kullan: son dakikalardaki işlemci, saniyedeki istek, kuyruk derinliği. "Bu yükseliyor mu, düşüyor mu, dalgalı mı?" sorusunu tek satırda yanıtlar. Kesin değerler ya da şeyler arası karşılaştırma için çubuk grafik ya da tablo kullan.

## Adım adım

1. Örneklerin geçmişini durumunda tut, en eskisi önde.
2. Çiz: `ui.add(Sparkline::new(değerler)).width(Length::Fill(1))`.
3. Ölçeği bilinen ölçümlerde sabitle: `.range(0.0, 100.0)`; yoksa sütunlar görünen en düşük ve en yüksek değer arasında uzar ve küçük değişimleri abartır.
4. Güncel değeri yanına metin olarak koy; sparkline sayıyı değil eğilimi gösterir.
5. İşe yaradıkça yetenek ekle: tepe ve dip için `.highlight_extremes()`, bir sınır için `.baseline(80.0)`, daha uzun sütunlar için düğümde `.height(Length::Cells(3))`.
6. Tek bir örneğin okunmasını istiyorsan okunan örneği durumunda tut ve bağla: `.reading(self.reading).on_read(Msg::Read)`. Değeri grafiğin yanına kendin yaz; sparkline sütunu işaretler, anlamı sen söylersin.

## Nasıl çalışır

- **Sekizde bir hücrelik sütunlar.** Her değer `▁▂▃▄▅▆▇█` sütunudur; uzun sparkline'lar tam hücreleri üst üste koyar ve kısmi bir blokla biter, üç satır 24 seviye verir.
- **Her örnek görünür.** En düşük değer sekizde bir hücre tutar; sakin bir an eksik veri gibi görünmez.
- **En yenisi sağda.** Sütundan çok değer varsa en eskiler düşer.
- **Uçlar ve referans.** Sütunlar vurgunun bir kademe altındadır; böylece tepe vurgu renginin kendisiyle parlayabilir; dip soluktur. Referans, kendi seviyesinde satır boyunca sütunların altına çizilen bir ton bandıdır, çizgi değil.
- **Örnek okuma.** Basmak imlecin altındaki sütunu okur, sürüklemek seri boyunca — uçlarını aşsa da — gezinir, böylece bir değer elden kaçmaz. Odakta ← ve → bir örnek ilerler, Home ve End gösterilen en eski ve en yeni örneğe atlar, Esc okumayı bırakır. İki yol da aynı mesajı gönderir; fare ve klavye aynı değerlere ulaşır.
- **İşaret tonla verilir, hareketle değil.** Okunan sütun active zemininde durur ve vurgu renginde çizilir; üzerine gelinen sütunun zemini bir hover adımı aydınlanır. Hiçbir şey kaymaz: sparkline bir satır yapısı değildir, imlecin bıraktığı sütun yerinde kalır.
- **ASCII modu.** Sütunlar yükseltilmiş bir iz üstünde tam hücreleri renkle doldurur; sütunun bittiği hücre, kapladığı pay kadar renk alır. En düşük örnek bile bir hücreyi renklendirir, tek satırlık bir sparkline ton şeridi gibi okunur.

## Sık yapılan hatalar

- **Yüzdelerde otomatik ölçek.** %40–42 arasında düz bir yük tüm yüksekliği doldurur ve korkutucu görünür; yüzdelere `.range(0.0, 100.0)` ver.
- **Sayısı olmayan sparkline.** Kullanıcı önce güncel değeri okur; onu eğilimin yanında göster.
- **Çok az örnek.** Beş sütun eğilim değildir. En az birkaç düzine tut.
- **Okunacak yeri olmayan okuma.** `.on_read` yalnızca hangi örnek olduğunu söyler; grafiğin yanında bir metin satırı yoksa işaret bir şey anlatmaz.
