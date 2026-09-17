## Metotlar

- `Badge::new(etiket)` — `etiket` yazan nötr bir rozet.
- `.variant(isim)` — ton: `"success"`, `"warning"`, `"danger"`, `"info"`, `"accent"` ya da temanın tanımladığı herhangi bir varyant.
- `.count(n)` — etiketin ardına bir sayı bölümü ekler; 99'un üstü `99+` görünür.

## Davranış

- Tek satır ölçer: boşluk, işaret, bir boşluk, etiket, boşluk, ardından sayı bölümü.
- Daha dar alan verilirse etiket `…` ile kesilir; işaret ve sayı kalır.
- İşaret `dot` ikonudur, yani karakter modunu izler.
- Odak almaz, mesaj göndermez.

## Tema anahtarları

- `badge`, `badge.<varyant>` — `bg`, `fg`, `dot`.
- `badge-count`, `badge-count.<varyant>` — `bg`, `fg`, `bold`.
- `dot` ikonu — işaret.
