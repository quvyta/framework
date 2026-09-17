## Metotlar

- `KeyHints::new()` — boş bir çubuk.
- `.hint(tuş, etiket)` — tuşu verildiği gibi yazılan, solda bir ipucu.
- `.action(kapsam, isim)` — solda bir kısayol eylemi; tuşlar kısayol haritasından, etiket `quvyta.keys.<isim>` ya da `keys.<isim>` anahtarından.
- `.action_right(kapsam, isim)` — sağda bir kısayol eylemi.

## Davranış

- Aldığı tüm genişliği ve bir satırı ölçer.
- Sığana kadar soldaki ipuçlarını sondan düşürür; sağdakiler kalır.
- Solda, çağrı sırası ne olursa olsun bütün `.hint` ipuçları bütün `.action` eylemlerinden önce gelir; bu yüzden önce eylemler düşer.
- Bir eylemin yalnızca ilk tuşu görünür; tuşu bağlanmamış eylemler gösterilmez.

## Tema anahtarları

- `key-hints` — `bg`, `padding`.
- `key-hint-key` — `bg`, `fg`, `bold`.
- `key-hint-label` — `fg`.
