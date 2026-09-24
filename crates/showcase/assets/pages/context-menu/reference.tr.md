## Metotlar

- `ContextMenu::new(öğeler)` — `ui.add_with(…)` ile eklenen bileşenleri sarar. Mesajlarının `Clone` olması gerekir; `Tabs` ve `TabRail` menülerinde bu gerekmez.
- `.on_left_click(bool)` — menüyü sol tık da imlecin yanında açar; menü açıkken alana sol tıklamak onu kapatır. Etkileşimli bir çocuğun aldığı basışlar yine o çocuğa gider. Varsayılan: `false`.
- `ContextItem::new(etiket, msg)` — seçilince `msg` gönderen eylem.
- `ContextItem::submenu(etiket, öğeler)` — `öğeler`i menünün yanında açan satır.
- `ContextItem::gap()` — gruplar arasında boş satır.
- `.icon(anahtar)` — etiketin önünde ikon.
- `.shortcut(etiket)` — sağ kenarda silik tuş etiketi (yalnızca gösterim).
- `.detail(metin)` — sağda, sönük tonda kısa not; kısayoldan ya da alt menü okundan önce durur, pasif satırda da çizilir; örneğin satırın neden kullanılamadığı. Menü notu sığdıracak kadar genişler; dar alanda etiketten önce kesilir, dört hücrenin altında hiç gösterilmez.
- `.disabled(bool)` — soluk çizilir, klavye atlar. Varsayılan: `false`.
- `.danger(bool)` — tehlike rengiyle çizilir. Varsayılan: `false`.

## Tuşlar

- Kapalı: `shift f10` ya da `menu` (kitty klavye protokolü olan terminallerin bildirdiği menü tuşu) alandaki odaklı bileşenin altında açar. Öğesi olmayan menü açılmaz.
- Açık: `up` `down` gezinir, `home` `end` atlar, bir harf atlar, `right`, `enter` ya da `space` alt menüyü açar, `left` kapatır, `enter` ya da `space` seçer, `esc` bir seviye kapatır, `tab` kapatıp odağı taşır.

## Fare

- Sağ tık imlecin yanında açar. Üstüne gelmek satırı vurgular ve alt menüleri açar; tıklamak seçer, boşluğa ya da pasif satıra tıklamak bir şey yapmaz. Dışarı basmak menüyü kapatır ve basış yine de hedefine ulaşır; alanın içinde sağ tık menüyü orada yeniden açar. `on_left_click(true)` ile sol tık da menüyü aynı biçimde açar; menü açıkken alana sol tıklamak onu kapatır.

## Tema anahtarları

- `context-menu` — `bg` (varsayılan `$overlay`).
- `hover` (`bg`, `fg`, `pillar`) ve `disabled` ile `context-item`; `hover` ile `context-item.danger`.
- Satırın `hover` ve `disabled` durumlarıyla `context-item-shortcut`, `context-item-chevron` ve `context-item-detail`; yerleşik tema üçünü de `$muted` ile çizer ve `hover` durumunda yalnızca oku parlatır.

## İkonlar

- Alt menüleri `chevron-right`, vurgulu satırı `pillar` işaretler.
