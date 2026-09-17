## Metotlar

- `Splitter::columns(genişlik)` — yan yana bölmeler; soldaki bölme `genişlik` sütun.
- `Splitter::rows(yükseklik)` — üst üste bölmeler; üstteki bölme `yükseklik` satır.
- `.limits(min, max)` — ilk bölmenin sınırları; `max` bir sayı ya da üst sınır olmaması için `None`; varsayılan 1 ve `None`.
- `.on_resize(|boyut| msg)` — sınırı sürüklenebilir ve odaklanabilir yapar.
- `.first(|ui| ...)`, `.second(|ui| ...)` — bölmeler; `.show(ui)` bölücüyü alanını doldurarak ekler.

## Tuşlar

- Odaktaki sınırda: `left` `right` (sütunlar) ya da `up` `down` (satırlar) bir hücre, `shift` ile beş hücre taşır; `home` `end` sınırlara atlar.

## Fare

- Sınırı sürükle. Üstüne gelince aydınlanır.

## Davranış

- `on_resize` yoksa sınır hücresi yoktur ve hiçbir şey odaklanmaz.
- Sınırlar ne olursa olsun ikinci bölme her zaman en az bir hücre korur; `min`'den küçük bir `max`, `min` sayılır.

## Tema anahtarları

- `split-handle` — `bg`, `fg`; durumlar `hover`, `focus`, `active` (sürüklenirken). Durum yoksa görünmez.
