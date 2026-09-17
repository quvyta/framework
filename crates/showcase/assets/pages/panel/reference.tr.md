## Metotlar

- `Panel::new()` — başlıksız panel; içeriği `ui.add_with` ile ekle.
- `.title(metin)` — ilk satırda başlık, ardından bir boş satır.
- `.variant(isim)` — tema varyantı, örneğin `inset`.
- `.gap(satır)` — çocuklar arasındaki satır. Varsayılan: 1.
- `.selected(bool)` — seçili yüzey ve çubuk. Varsayılan: `false`.
- `.on_press(msg)` — paneli basılabilir yapar: hover, odak ve basış görünümü, odak sırası, tuşlar ve tıklama.
- `.disabled(bool)` — tepki vermeyen basılabilir panel: hover, odak, basış yok. `selected` yine görünür. Varsayılan: `false`.

## Davranış

- Hover: yüzey `$text` rengine doğru %8 açılır (yerleşik temalarda duran hale göre en az 1.2:1), başlık `$dim` olur, panelin tüm sol kenarı boyunca yumuşak bir çubuk çıkar.
- Basış (`motion.flash` süren bir parlama): `$text` rengine doğru %16, başlık `$text`, çubuk `$accent`.
- Klavye odağı: hover gibi, çubuk nefes alır. Tıklamayla gelen odak gösterilmez.
- Seçili: `$active` ve boydan boya nefes alan çubuk; seçili panelin üzerine gelince o da %8 açılır.
- `on_press` olmayan ya da pasif panel fare altında hiç değişmez.

## Mesajlar

- Enter, Boşluk ya da panelin üzerinde bırakılan sol tıklamada `on_press` mesajı; fare başka yerde bırakılırsa basma iptal olur.

## Tema anahtarları

- `hover`, `focus`, `pressed`, `selected` ile `panel` — `bg`, `padding`, `pillar`.
- `panel.<varyant>` — örneğin `panel.inset`.
- `hover`, `focus`, `pressed` ile `panel-title` — `fg`, `bold`.
