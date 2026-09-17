## Metotlar

- `Accordion::new(bölümler)` — başlıklar `&str`, `String` ya da `Section`; her bölüm için `ui.add_with` ile bir gövde çocuğu ekle.
- `.open(bool dizisi)` — hangi bölümlerin açık olduğu; eksik girdiler kapalıdır.
- `.on_toggle(|sıra, açık| msg)` — bir bölüm açıldı ya da kapandı.
- `.single(bool)` — en fazla bir açık bölüm.
- `Section::new(başlık)`, `.icon(anahtar)`, `.detail(metin)` — başlık satırı.

## Tuşlar

- Odaktayken: `up` `down` başlıklar arasında gezinir, `home` `end` atlar, `enter` ya da `space` açıp kapatır.

## Fare

- Bir başlığın üstünde bırakılan tıklama onu açıp kapatır.

## Davranış

- Açık gövde doğal yüksekliğini ve `section-body` iç boşluğunu alır.
- Açılış gövdeyi iki `motion.enter` süresince satır satır gösterir; kapanış anındadır.
- Satır dar kalınca ayrıntı gizlenir; başlıklar `…` ile kesilir.
- `motion.slide` açıkken hover edilen ya da odaktaki başlıkta yalnızca ikon ve başlık bir hücre sağa kayar; ok ve ayrıntı hiç kıpırdamaz.

## Tema anahtarları

- `section` — bölümler arasındaki `gap` satırı.
- `section-title` — `bg`, `fg`, `bold`, `pillar`; durumlar `hover`, `focus`, `checked` (açık).
- `section-chevron`, `section-detail` — `fg`; aynı durumlar.
- `section-body` — `bg`, `padding`.
- İkonlar — `section-open`, `section-closed`.
