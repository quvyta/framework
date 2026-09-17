## Metotlar

- `ContextMenu::new(öğeler)` — `ui.add_with(…)` ile eklenen bileşenleri sarar. Mesajlarının `Clone` olması gerekir; `Tabs` ve `TabRail` menülerinde bu gerekmez.
- `ContextItem::new(etiket, msg)` — seçilince `msg` gönderen eylem.
- `ContextItem::submenu(etiket, öğeler)` — `öğeler`i menünün yanında açan satır.
- `ContextItem::gap()` — gruplar arasında boş satır.
- `.icon(anahtar)` — etiketin önünde ikon.
- `.shortcut(etiket)` — sağ kenarda silik tuş etiketi (yalnızca gösterim).
- `.disabled(bool)` — soluk çizilir, klavye atlar. Varsayılan: `false`.
- `.danger(bool)` — tehlike rengiyle çizilir. Varsayılan: `false`.

## Tuşlar

- Kapalı: `shift f10` ya da `menu` (kitty klavye protokolü olan terminallerin bildirdiği menü tuşu) alandaki odaklı bileşenin altında açar. Öğesi olmayan menü açılmaz.
- Açık: `up` `down` gezinir, `home` `end` atlar, bir harf atlar, `right`, `enter` ya da `space` alt menüyü açar, `left` kapatır, `enter` ya da `space` seçer, `esc` bir seviye kapatır, `tab` kapatıp odağı taşır.

## Fare

- Sağ tık imlecin yanında açar. Üstüne gelmek satırı vurgular ve alt menüleri açar; tıklamak seçer, boşluğa ya da pasif satıra tıklamak bir şey yapmaz. Dışarı basmak menüyü kapatır ve basış yine de hedefine ulaşır; alanın içinde sağ tık menüyü orada yeniden açar.

## Tema anahtarları

- `context-menu` — `bg` (varsayılan `$overlay`).
- `hover` (`bg`, `fg`, `pillar`) ve `disabled` ile `context-item`; `hover` ile `context-item.danger`.
- Satırın `hover` ve `disabled` durumlarıyla `context-item-shortcut` ve `context-item-chevron`; yerleşik tema `hover` durumunda yalnızca oku parlatır.

## İkonlar

- Alt menüleri `chevron-right`, vurgulu satırı `pillar` işaretler.
