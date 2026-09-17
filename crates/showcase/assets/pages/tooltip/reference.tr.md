## Metotlar

- `Tooltip::new(metin)` — `ui.add_with(…)` ile eklenen bileşenleri sarar ve `metin` ile açıklar.
- `.placement(Placement)` — `Below` (varsayılan), `Above`, `Right` ya da `Left`; yer yoksa öbür tarafa geçer.
- `.on_focus(bool)` — klavye odağı içerideyken de hemen gösterir. Varsayılan: `false`.

## Davranış

- İmleç `motion.hover-delay` kadar bekleyince görünür; imleç ayrılınca kaybolur.
- Tek satırdır, ekran genişliğine göre `…` ile kısaltılır; imlecin bulunduğu hücreyi asla örtmez.
- Katmanda kendi tıklama alanı yoktur; tıklamalar altındakine ulaşır.

## Tema anahtarları

- `tooltip` — `bg` (varsayılan `$overlay`), `fg` (varsayılan `$text`), `padding` (varsayılan `[0, 1]`).
- `[motion] hover-delay` — ipucu belirmeden önce imlecin bekleme süresi.
- `[motion] enter` — metnin belirme süresi.
