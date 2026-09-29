## Metotlar

- `KeyHints::new()` — boş bir çubuk.
- `.hint(tuş, etiket)` — tuşu verildiği gibi yazılan, solda bir ipucu.
- `.action(kapsam, isim)` — solda bir kısayol eylemi; tuşlar kısayol haritasından, etiket `quvyta.keys.<isim>` ya da `keys.<isim>` anahtarından.
- `.action_right(kapsam, isim)` — sağda bir kısayol eylemi.
- `.action_first(kapsam, isim)` — bütün `.hint` ipuçlarından önce gelen bir kısayol eylemi; soldakilerin en son düşeni.
- `.action_labelled(kapsam, isim, etiket)` — solda, etiketi uygulamanın kendisinden gelen bir kısayol eylemi; tuş yine kısayol haritasını izler, bağlanmamış eylem hiçbir şey çizmez.
- `.faint(bool)` — bütün çubuk bir adım sessiz; durgunlaşmış bir ekran için.
- `Keymap::label_for(kapsam, isim)` — bir eylemin ilk tuşunun cümlede yazılacak hali; tuşu yoksa `None`.

## Davranış

- Aldığı tüm genişliği ve bir satırı ölçer.
- Sığana kadar soldaki ipuçlarını sondan düşürür; sağdakiler kalır.
- Solda, çağrı sırası ne olursa olsun önce `.action_first` eylemleri, sonra bütün `.hint` ipuçları, sonra bütün `.action` eylemleri gelir: önce sıradan eylemler, en son baştaki eylemler düşer.
- Bir eylemin yalnızca ilk tuşu görünür; tuşu bağlanmamış eylemler gösterilmez.

## Tema anahtarları

- `key-hints` — `bg`, `padding`.
- `key-hint-key` — `bg`, `fg`, `bold`.
- `key-hint-label` — `fg`.
- `key-hint-key.faint`, `key-hint-label.faint` — soluk bir çubuk için aynıları.
