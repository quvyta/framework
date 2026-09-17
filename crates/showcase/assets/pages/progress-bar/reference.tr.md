## Metotlar

- `ProgressBar::new(değer)` — 0..1 aralığına sıkıştırılmış `değer`de, yüzdesi görünen bir çubuk.
- `ProgressBar::indeterminate()` — büyüklüğü bilinmeyen iş için süpüren çubuk.
- `.percent(bool)` — belirli çubuğun ardındaki yüzdeyi gösterir ya da gizler.
- `.variant(isim)` — `"success"`, `"warning"`, `"danger"` gibi tema varyantı.

## Davranış

- Aldığı tüm genişliği ve bir satırı ölçer.
- Yüzde sağda beş hücre kaplar; çubuk kalanını kullanır.
- Belirli çubuklar hücrenin sekizde biri kadar, ASCII modunda en yakın tam hücreye yuvarlanarak dolar.
- Belirsiz çubuklar hareket azaltılmamışsa kare ister; azaltılmışsa düz bir ton gösterir.

## Tema anahtarları

- `progress`, `progress.<varyant>` — `track`, `fill`.
- `progress-label`, `progress-label.<varyant>` — `fg`, `bold`.
- `[motion]` — süpürme için `shimmer`.
