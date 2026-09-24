## Metotlar

- `Window::new(baslik)` — `baslik` adlı pencere; odakta değildir, yalnızca bir yüzeydir.
- `.subtitle(metin)` — adın ardından soluk ikinci başlık: programın kendi başlığı ya da klasörü.
- `.icon(glif)` — addan önceki glif: bir ikon anahtarı ya da uygulamanın kendi bulduğu bir glif için `Glyph::literal`.
- `.focused(acik)` — kullanıcının çalıştığı pencere: bir ton yükselmiş, parlak kalın ad, sol kenarda vurgu çubuğu.
- `.maximized(acik)` — büyütme işareti eski boyuta dönmeyi önerir.
- `.shadow(acik)` — sağda bir sütun, altta bir satır koyulaşır.
- `.on_event(|olay| mesaj)` — pencereyi hareket edebilir yapar ve işaretlerini gösterir; her `WindowEvent` bir mesaja dönüşür.
- `cx.pointer_shape(rect, shape)` — boyarken imlecin `rect` üstünde bir `PointerShape` almasını ister: `Default`, `EwResize`, `NsResize`, `NwseResize`, `NeswResize`. Bir hücre için en son istenen şekil kazanır; yalnızca imlecin üstündeki bileşenden ya da onu saran birinden. İmleci tutan bileşen istediği ilk şekli korur. `Harness::pointer_shape()` testteki imlecin istediği şekli söyler.
- `ui.place(rect, |ui| ..)` — stack'in çocuğunun nereye gideceği, stack'in köşesinden sayılır; `.id(ad)` ona ad verir.
- `Ghost::new()` — bir şeyin nereye oturacağını gösteren ton; `.mix(oran)` zemine ne kadar vurgu karışacağını söyler (varsayılan çeyrek). Hiç glif çizmez, fare almaz.

## Olaylar

- `WindowEvent::Focus` — odakta olmayan pencerenin herhangi bir yerine basılması; tıklamanın yapacağı başka her şeyden önce gelir.
- `Move { dx, dy }` — başlık sürüklendi ya da alt ile sol tuş her yerden sürüklendi.
- `Resize { edge, dx, dy }` — kenar ya da köşe o kadar hücre hareket eder; üst ve alt kenarda `dx`, sol ve sağ kenarda `dy` sıfırdır. Sol ya da üst kenar pencereyi farkla taşır ve boyutunu ters yönde değiştirir; öteki kenar yerinde kalır.
- `Minimize`, `ToggleMaximize` (büyütme işareti ya da başlığa çift tık), `Close` — bir işarete tıklandı.
- `Dropped` — en az bir taşıma ya da boyutlandırmadan sonra tuş kalktı; hiçbir şeyi hareket ettirmeyen tıklama bunu göndermez.
- `WindowEdge` — `Left`, `Right`, `Top`, `Bottom` ve dört köşe; hangi kenarların hareket ettiğini `left()`, `right()`, `top()`, `bottom()` söyler.

## Tuşlar

- Pencerenin kendisi klavye odağı almaz: uygulama kendi tuşlarını bağlar ve aynı mesajlarla hareket eder. Örnek, sıradaki pencereyi odaklamak için `ctrl alt w` bağlar; ok düğmeleri (`tab` ile ulaşılır, `enter` ile basılır) odaktaki pencereyi iki hücre taşır ya da boyutlandırır.

## Fare

- Başlık: sürüklemek taşır, çift tık büyütür ya da eski boyuta döner. İki köşe hücresinin arası taşır; sol üst ve sağ üst köşe boyutlandırır.
- İşaretler: küçült, büyüt ya da eski boyut, kapat; her birinin üç hücresi birlikte aydınlanır. Sağ kenardan bir hücre önce biterler; o hücre sağ üst köşedir.
- Kenarlar: sol sütun, sağ sütun ve alt satır dört köşesiyle birlikte boyutlandırır; gövdenin kalanı içindekine aittir.
- `alt` ile sol tuş: her yerden taşır. `alt` ile sağ tuş: en yakın kenardan ya da köşeden boyutlandırır, üst kenar dahil.
- Sürükleme, tuş bırakılana kadar imleç nereye giderse gitsin pencereye ulaşır.
- İmleç: her tutamağın üstünde boyutlandırma oku (sütunlarda `ew-resize`, alt satırda `ns-resize`, köşelerde `nwse-resize` ve `nesw-resize`), başka yerde olağan imleç; boyutlandırma tuş bırakılana kadar okunu korur. OSC 22 olarak yalnızca foot, kitty ve WezTerm'e, yalnızca değiştiğinde gönderilir; `QUVYTA_POINTER_SHAPES=on|off` her terminal için karar verir. tmux ya da screen içinde hiçbir şey gönderilmez.

## Davranış

- Yerleştirilen çocuklar eklendikleri sırayla çizilir, en son eklenen üstte durur ve üst üste bindikleri yerde fareyi önce o alır. Dikdörtgen stack'in her yanından taşabilir; dışarıda kalan çizilmez ve fare almaz.
- Yerleştirilmiş pencere sağ ve alt kenarının bir hücre ötesine çizebilir; gölgesi oraya düşer. O hücre hiç fare almaz.
- Gövde vurgu çubuğunun sütununu, ondan sonraki bir hücreyi, sağ sütunu ve alt satırı boş tutar.
- Dar başlıkta önce alt başlık kısalır, sonra tamamen çıkar, sonra ad `…` ile kısalır; işaretler her zaman görünür.
- ASCII glif kipinde işaretler ` - `, ` + `, ` x ` (eski boyut için ` o `), vurgu çubuğu ise renkli bir hücredir.
- Pencerede hiçbir şey kendiliğinden değişmez: üst üste binme sırası, odak, boyut sınırları, kenara yapıştırma ve döşeme uygulamada kalır.

## Tema anahtarları

- `window` — `bg`, `pillar`; `focus` durumu.
- `window-title` — `bg`, `fg`, `bold`; `focus` durumu.
- `window-subtitle` — `fg`; `focus` durumu.
- `window-shadow` — `scrim`, yüzde olarak `strength`.
- `close-mark` — üç işaret; pencere odaktayken `active`, imleç altında `hover`.
- `split-handle` — kenar tutamakları; `hover`, sürüklenirken `active`. Bir sütun bütün olarak, alt satır bütün olarak aydınlanır; kendi satırı olmayan üst kenar iki köşe hücresini aydınlatır.
- `ghost` — `bg`, hayaletin zemine karıştırdığı renk; tema susarsa vurgu rengi.
