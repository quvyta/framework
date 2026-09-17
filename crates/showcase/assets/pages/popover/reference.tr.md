## Metotlar

- `Popover::new(açık)` — `açık` doğruyken katmanı gösterir.
- `.anchor(|ui| …)` — katmanın ait olduğu bileşenler; açılır kutunun eklendiği yere yerleşir.
- `.content(|ui| …)` — katmanın içindeki bileşenler.
- `.on_dismiss(msg)` — Esc'te ve çapa ile katmanın dışına basılınca gönderilir.
- `.placement(Placement)` — `Below` (varsayılan), `Above`, `Right` ya da `Left`; yer yoksa karşı tarafa geçer.
- `.focus_inside(bool)` — açılınca odak katmana girer, kapanınca geri döner. Varsayılan: `false`.
- `.show(ui)` — ekler ve düğümü döndürür; böylece `.id(…)` ve boyutlar çapaya uygulanır.

## Placement

- `Placement::ALL`, `.name()` — bütün taraflar ve ayar ekranları için kısa ad.

## Tuşlar

- `esc` — kapatma mesajını gönderir.
- Odak içerideyken `tab` katmanın bileşenlerinde, ardından ekranın geri kalanında gezinir.

## Fare

- Dışarıya basmak kapatma mesajını gönderir ve düştüğü yerdeki şeye de ulaşır. Açılır kutuyu çapanın dışındaki bir bileşen açtıysa ona basmak yalnızca kapatır; çapaya basmak ise çapaya ulaşır, o da genellikle açıp kapatır.
- Katmanın içine ve çapaya basmak her zamanki gibi çalışır.

## Tema anahtarları

- `popover` — `bg` (varsayılan `$overlay`), `padding` (varsayılan `[1, 2]`).
- `[motion] enter` — katmanın açılma süresi.
