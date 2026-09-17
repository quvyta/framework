## Metotlar

- `Select::new(seçenekler)` — metin olarak seçenekler.
- `.selected(Option<usize>)` — seçili seçenek.
- `.on_select(|sıra| msg)` — farklı bir seçenek seçildiğinde gönderilir.
- `.placeholder(metin)` — bir şey seçilmemişken gösterilir.
- `.max_visible(satır)` — liste kaymadan önceki satır. Varsayılan: 8.
- `.disabled(bool)` — odak almaz, açılamaz. Varsayılan: `false`.

## Tuşlar

- Kapalı: `enter`, `space` ya da `down` açar.
- Açık: `up` `down` gezinir, `home` `end` atlar, `pgup` `pgdn` sayfa geçer, bir harf atlar, `enter` ya da `space` seçer, `esc` kapatır, `tab` kapatıp odağı taşır.

## Fare

- Açıp kapatmak için alana tıkla; seçmek için bir seçeneğe tıkla; tekerlek listeyi kaydırır, kaydırma çubuğu basılıp sürüklenebilir.
- Fareyi seçeneklerin üstünde gezdirmek tek vurguyu taşır.
- Dışarı tıklamak kapatır ve tıklama düştüğü yerdeki şeye de ulaşır.

## Tema anahtarları

- `hover`, `focus`, `active` (açık), `disabled` ile `select`.
- `select-placeholder`, `select-chevron`, `select-menu` (`bg`).
- `hover` ve `checked` ile `select-option`; `select-check`.

## İkonlar

- Alanda `chevron-down`, seçili seçeneğin yanında `check`.
