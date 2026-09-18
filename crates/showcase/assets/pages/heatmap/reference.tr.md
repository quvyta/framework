## Metotlar

- `Heatmap::new(değerler)` — `f32` değerlerden bir ızgara, en eskisi önde, sütun sütun dolar, yedi satır yüksekliğinde.
- `.rows(n)` — ızgaranın satırları; en az bir.
- `.max(değer)` — en üst kademenin karşılığı; varsayılan en büyük değer.
- `.starts_at(satır)` — ilk değerden önceki boş hücreler, bir sütun içinde tutulur.
- `.series(indeks)` — tonları vurgu yerine temanın n'inci seri tonundan kurar.
- `.selected(Some(indeks))` — imlecin üzerinde durduğu değer.
- `.on_select(|indeks| msg)` — tıklama ya da Enter ile seçilen hücrenin mesajı; ısı haritasını etkileşimli yapan da budur.
- `.columns(genişlik)` — `genişlik` hücreye kaç sütun sığar, en yeniler kalır.
- `Legend::new(adlar)` — bir grafiğin serilerini çizdiği sırayla adlar; n'inci ad temanın n'inci seri tonunu alır.
- `.tones(sıralar)` — her adın paletteki sırası, adlarla aynı düzende; sonuna yetişemeyen ad kendi konumunun tonunu alır. Grafiğin serilerine verilen sayıları ver.
- `.vertical()` — satır başına bir ad.

## Davranış

- Hafta başına bir sütun ve `rows` satır ölçer; değer yoksa hiç yer ölçmez.
- Sıfır ve altındaki değer boş tonunu, diğer her değer dört kademeden birini alır; ölçeğin üstündeki değerler en üst kademede kalır.
- Dar alan en eski sütunları tam olarak düşürür; kısa alan üstten sığan satırları tutar.
- 16 renkli terminal ayırt edemediği kademeleri düşürür ve seviyeleri kalan tonlara yayar.
- Odak ve tıklama yalnızca `on_select` ile: ←/→ bir sütun, ↑/↓ bir gün gezer, Home/End uçlara gider, Enter ya da tıklama hücreyi bildirir.
- Gösterge odak almaz, mesaj göndermez; uzun adlar `…` ile kesilir.

## Tema anahtarları

- `heatmap` — `empty` (hiçbir şey taşımayan gün), `fill` (tam ton), `cursor` (aydınlanan hücrenin yöneldiği ton).
- `heatmap:focus` — imleci en son klavye oynattıysa `cursor`.
- `legend` — `fg` (adlar).
- `series-1`–`series-5` renk anahtarları, beşten sonra başa dönen `Theme::series_color(indeks)` ile.
