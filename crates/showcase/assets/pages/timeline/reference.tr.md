## Metotlar

- `TimeBlock::new(ad, başlangıç, bitiş)` — günün bir bloğu; başlangıçtan önceki bitiş gece yarısını aşar, başlangıca eşit bitiş bir andır.
- `TimeBlock::tone(sıra)` — vurgu yerine temanın `sıra`'ncı seri tonu.
- `TimeBlock::open_end()` — blok bitişinden sonra da sürer (ertesi güne ya da hâlâ çalışıyor): son iki hücresi ize doğru solar, okuma satırı "ertesi güne sürüyor" ya da "sürüyor" der.
- `TimeBlock::open_start()` — blok başlangıcından önce başlamıştır: ilk iki hücresi izden doğru belirir, okuma satırı "önceki günden" ya da "önceden" der.
- `TimeBlock::faint()` — bloğun tonuyla iz arasında yarı yolda; tamamı başka yerde gösterilen zaman için.
- `Timeline::new(bloklar)` — gece yarısından başlayan bütün bir gün; art arda bloklar için tek satır.
- `.day_starts_at(saat)` — günü `saat`'te başlatır; gece yarısını aşan bloklar bütün kalır.
- `.range(baslangic, bitis)` — yalnızca o aralığı gösterir; `bitis` `baslangic`'tan sonra değilse ertesi güne uzanır, ikisi eşitse bütün gündür; gün içinde tutulur.
- `.axis()` — şeridin altında bir saat satırı.
- `.readout()` — okunan bloğu yazan satır: ad, saatler, süre.
- `.selected(Some(sıra))` — seçili blok.
- `.on_select(|sıra| mesaj)` — seçimi taşıma mesajı; hover'ı, tıklamayı ve ok tuşlarını açar.
- `.on_zoom(|b, s| mesaj)` — yeni aralık isteyen mesaj; `+`, `-`, `0` ve tekerleği açar.
- `.disabled(pasif)` — soluk, odak almaz, mesaj göndermez.
- `Axis::hours(baslangic, bitis)` — zamana göre yerleşen saatler; zaman şeridiyle aynı hesap.
- `Axis::weekdays(ilk, adet)` — dil dosyalarından gün adları, pazardan sonra başa döner.
- `Axis::months(ilk, adet)` — ay adları, ocak 1'dir, aralıktan sonra başa döner.
- `Axis::labels(adlar)` — kendi adların, her birine bir yuva.
- `.gap(hücre)` — yuvalar arasındaki boş hücreler, grafiğin boşluğuyla eşleşsin diye; saatler bunu kullanmaz.

## Davranış

- Şerit başına bir satır, eksen için bir ve okuma için bir satır ölçer; boş gün tek şerittir.
- Bir hücreden kısa blok bir hücre alır. Aynı tondaki bitişik bloklarda dikiş olur: sonrakinin ilk hücresi ize doğru bir adım.
- Çakışan bloklar üstten ilk boş şeridi alır; kısa alan fazla şeritleri son satırına katlar, seçili blok en üste çizilir.
- Kısa alan önce ekseni, sonra okuma satırını, sonra şeritleri bırakır. Dar okuma satırı önce süreyi, sonra saatleri bırakır, sonra adı `…` ile keser.
- `on_select` (ve seçilecek blok) ya da `on_zoom` ile odak alır. Tuşlar: ←/→ ya da h/l blokları zaman sırasıyla gezer, Home/End uçlara atlar; `+` ya da `=` yakınlaştırır, `-` uzaklaştırır, `0` bütün gün. Tekerlek yalnızca odaktayken imlecin çevresinde yakınlaştırır. Tıklama altındaki bloğu seçer; boşluğa tıklama hiçbir şey seçmez.
- Yakınlaştırma adımları: 24, 12, 6, 3 ve 1 saat; tuşlarla seçili bloğa, tekerlekle imlece bağlı, tam dakikalarda, gün içinde. Yakın bir aralığın dışındaki blok seçilince aralığın ona kayması istenir.
- Hover ve seçim yalnızca tonu değiştirir; zaman şeridi asla kaymaz.
- Açık kenar en çok iki hücrede, ize doğru önce üçte iki sonra üçte bir oranında solar ve bloğun tonunda her zaman bir hücre bırakır; iki ucu açık blok hücrelerini paylaştırır. Yalnızca görünen aralığın içindeki kenar solar: yakınlaştırma kesiği kare kalır. Ad solan hücrelerin dışında durur ya da yazılmaz.
- Okuma satırı açık kenarın anlamını saatlerden sonra yazar; dar satır önce süreyi, sonra saatleri, sonra anlamı bırakır, en son adı keser.
- Soluk blok, solan hücreler ve bunların hover ve seçim adımları 256 ve 16 renkte her zaman izden ayrı tutulur; tonla iz arasında renk bulamayan solan hücre tonda kalır.
- 256 ve 16 renkte ize karışacak blok tonu metin rengini alır; görünmeyecek hover ya da seçim adımı daha ileri itilir.
- Eksen bir satır ölçer, odak almaz ve hiçbir etiket sığmıyorsa hiçbir şey yazmaz; etiketler arasında bir hücre kalır ve hiçbiri kesilmez.

## Tema anahtarları

- `timeline` — `track` (boş gün), `fill` (tonu olmayan blok), `hover`, `selected` (bloğun doğru adım attığı tonlar).
- `timeline:focus` — klavye şeritteyken `selected`.
- `timeline-readout` — `fg` (ad), `detail` (saatler ve süre).
- `axis` — `fg` (etiketler).
- `series-1`'den `series-5`'e renk anahtarları, `Theme::series_color(sıra)` ile.
