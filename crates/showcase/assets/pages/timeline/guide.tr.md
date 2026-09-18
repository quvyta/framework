## Ne zaman kullanılır

Soru "bu gün içinde ne zaman" olduğunda zaman şeridi kullan: odak saatleri, toplantılar, derlemeler, uyku. Bir şeylerin nereye düştüğünü ve boşlukların nerede kaldığını gösterir; toplam bunu gösteremez. Haftalar boyunca gün başına ne kadar sorusu için ısı haritası, toplamları karşılaştırmak için çubuk grafik kullan.

Bir grafiğin kenarına ad gerektiğinde eksen kullan: şeridin altında saatler, dikey çubukların altında gün ya da ay adları. Hiçbir etiketi kesmez, ikisini üst üste bindirmez; bu yüzden her genişlikte kullanılabilir.

## Adım adım

1. Günün bloklarını durumunda tut: iki `TimeOfDay` ile `TimeBlock::new("Rust", başlangıç, bitiş)`. Bitişi başlangıcından önce olan blok gece yarısını aşar.
2. Her kategoriyi paletteki bir tona bağla: göründüğü her yerde Rust için `.tone(0)`, Belge için `.tone(1)`; adlarını `Legend::new(adlar).tones([0, 1])` ile ver.
3. Günü çiz: `ui.add(Timeline::new(bloklar).axis())`. Şerit yalnızca aynı anda süren bloklar için bir satır büyür.
4. Okuyanın bir bloğu okumasına izin ver: `.readout()`; klavye de blokları gezebilsin diye `.selected(state.secilen).on_select(Msg::Sec)` ile birlikte.
5. Yakınlaştırmaya izin ver: durumunda bir aralık tut, `.range(baslangic, bitis)` ile ver ve `.on_zoom(|b, s| Msg::Yakinlas(b, s))`'ten gelen yenisini sakla. İki uç eşitse bütün gün geri gelmiştir.
6. Bir gece için günü akşam başlat: `.day_starts_at(TimeOfDay::new(18, 0, 0))`.
7. Gece yarısında bölünen bir oturumda sürdüğü yeri işaretle: ilk gündeki parçasına `.open_end()`, ertesi gündekine `.faint().open_start()`. Hâlâ çalışan bir sayaç da `.open_end()` alır.
8. Tek başına bir eksen için: `Axis::weekdays(Weekday::Monday, 7)`, `Axis::months(1, 12)`, `Axis::hours(baslangic, bitis)` ya da `Axis::labels(adlar)`; dikey çubukların altında grafiğin boşluğunu ver: `.gap(1)`.

## Nasıl çalışır

- **Zamana göre yerleşir.** Blok, saatlerinin düştüğü hücreleri kaplar; bir hücreden kısa blok yine bir hücre alır, böylece beş dakikalık bir inceleme bütün günü gösteren şeritte görünür. Eksenin saatleri aynı hesapla yerleşir; etiket, zamanının çizildiği hücrenin üstünde durur.
- **Boşluk izdir.** Kaydı olmayan zaman boş iz tonudur. Aynı tondaki iki bitişik blok, sonrakinin daha sakin ilk hücresiyle ayrılır: ton farkıyla, asla çizgiyle değil.
- **Çakışan şerit alır.** Her blok üstten, kendisiyle çakışan bir şey olmayan ilk şeridi alır. Başka bir bloğun başladığı anda biten blok onunla aynı şeridi paylaşır. Bütün şeritlere yetmeyen alan kalanları son satırına katlar ve seçili bloğu en üste çizer.
- **Gece yarısı.** Gün, `day_starts_at`'tan başlayan 24 saattir; varsayılanı gece yarısıdır. Gece yarısı başlayan günde 23.00–01.30 bloğu günün sonunda kesilir, kalanı yarının şeridine, 00.00'dan itibaren aittir. 18.00'de başlayan gün onu bütün tutar.
- **Açık kenarlar.** `open_end` ve `open_start`, bloğun gerçekte durmadığı kenarı işaretler. Son ya da ilk iki hücre ize doğru üçte iki ve üçte bir oranında solar; blok kare bitmek yerine güne doğru akar. Glif yoktur; odak anlamına gelen `▌` çubuğu hiç kullanılmaz. Solan hücreler her zaman izden ayrı tutulur, böylece blok olduğundan kısa görünmez. Solmaya yetmeyecek kadar kısa blok kendi tonunda bir hücre tutar; iki ucu açık blok hücrelerini iki uca paylaştırır. Okuma satırı kenarın anlamını yazar: günün sonundaki bitiş için "ertesi güne sürüyor", daha önceki için "sürüyor", günün başındaki başlangıç için "önceki günden", daha sonraki için "önceden".
- **Yalnızca bloğun kendi kenarı açıktır.** Yakın bir aralığın kestiği kenar kare kalır: aralık görünümün bittiği yerdir, eksen bunu zaten söyler; orada solma, bloğun sürdüğünü iddia ederdi. Okuma satırı yine bloğun bütün saatlerini verir.
- **Soluk bloklar.** `faint` bloğu kendi tonuyla iz arasında yarı yola koyar: hâlâ kategorisinin rengidir, açıkça boşluk değildir. Hover ve seçim onu yine adımlar, asla ize değil.
- **Yakınlaştırma senindir.** `+` ve `-` seçili bloğun çevresinde 24, 12, 6, 3 ve 1 saat arasında adım atar, `0` bütün güne döner; tekerlek yalnızca şerit odaktayken imlecin çevresinde yakınlaştırır, böylece yanından kaydırılan sayfa ona takılmaz. Aralığın dışındaki blok seçilince aralık ona kayar.
- **Fare ve klavye eşittir.** İmlecin altındaki blok ve seçili blok metin rengine doğru adım atar; ← ve → (ya da h ve l) blokları zaman sırasıyla gezer, Home ve End uçlara atlar, tıklama seçer. Okuma satırı imlecin altındaki bloğu, yoksa seçili olanı adıyla, saatleriyle ve süresiyle yazar: ikisi de aynı sözleri okur.
- **Hiçbir şey yer değiştirmez.** Hover ve seçim yalnızca tonu değiştirir. Zaman şeridi liste değil, dar bir şerittir; asla kaymaz.
- **Kısa ve dar alan.** Kısa alan önce ekseni, sonra okuma satırını, en son şeritleri bırakır. Dar okuma satırı önce süreyi, sonra saatleri bırakır, adı ancak en sonda keser.
- **Eksenin seyrelmesi.** Gün ve ay adları hepsi sığdıkça tam, sonra hepsi kısa, sonra her ikinci, üçüncü… adın kısası yazılır. Saatler uzun biçimde (`09:30`) sığan en ince yuvarlak adımı alır; kısa biçime (`09`) yalnızca uzun biçim ikiden az etiket bırakacaksa geçer. Kenardan taşacak etiket bırakılır.
- **Her durum.** Boş gün baştan sona izdir ve okuma satırı bunu söyler; pasif şerit soluklaşır ve hiçbir şeye yanıt vermez. Şerit renkten yapılır, her glif kipi onu aynı çizer; 16 renkli terminalde izle ya da kendi hover adımıyla birleşecek ton, ayrışana kadar itilir.

## Sık yapılan hatalar

- **Sıraya göre renk.** Günden güne rengi değişen kategori öğrenilemez. `.tone(n)` ile bağla, göstergeye aynı sayıları ver.
- **Geceyi elle bölmek.** Bunun yerine günü akşam başlat; uyku tek blok kalır.
- **Yeni gibi görünen devam.** Bir oturumun ertesi gündeki parçası aynı iştir; yeni bir başlangıç gibi okunmasın diye `.faint().open_start()` ile çiz.
- **Yakınlaştırma kesiğine açık uç.** Aralık orada bitiyor diye `open_end` ekleme; yalnızca bloğun gerçek sonu açıktır.
- **Okuma satırını unutmak.** Ton bir saat değildir; okuma satırı ya da senin yazdığın bir metin olmadan kimse bloğun ne zaman başladığını söyleyemez.
- **Aralıksız yakınlaştırma.** `on_zoom` yalnızca ister. Gönderdiği aralığı sakla ve `.range()` ile geri ver.
