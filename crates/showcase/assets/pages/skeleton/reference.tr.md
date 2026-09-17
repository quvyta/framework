## Metotlar

- `Skeleton::lines(sayı)` — genişlikleri değişen `sayı` kadar metin satırı; sonuncusu kısa.
- `Skeleton::avatar()` — ikon ya da avatar için iki hücrelik yer tutucu.
- `Skeleton::block()` — alanını dolduran bir blok; düğüme yükseklik verilmezse üç satır.

## Davranış

- Satırlar tüm genişliği ve `sayı` satırı ölçer; bloklar tüm genişliği ve üç satırı; avatar iki hücre ve bir satırı.
- Hareket azaltılmamışsa animasyon karesi ister; azaltılmışsa hiçbir şey kıpırdamaz.
- Süpürmenin konumu ekran sütununa bağlıdır; yan yana iskeletler tek geçişte aydınlanır.
- Odak almaz, mesaj göndermez.

## Tema anahtarları

- `skeleton` — şekiller için `bg`, ışık için `highlight`.
- `[motion]` — süpürmenin bir geçişi için `shimmer`.
