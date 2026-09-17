## Metotlar

- `EmptyState::new(başlık)` — `başlık` yazan bir boş durum.
- `.icon(anahtar)` — ikon setinden bir ikon; başlığın üstünde soluk çizilir.
- `.message(metin)` — başlığın altındaki açıklama; en fazla 52 hücrede sarılır.
- `.action(buton)` — açıklamanın altında bir Button; kendi varyantını, kısayolunu ve mesajını korur.

## Davranış

- Aldığı tüm genişliği ve parçalarının gerektirdiği satırları ölçer; alanın ortasına çizilir.
- Alan kısaysa sırasıyla şunları bırakır: ikon ve boşluğu, eylemin üstündeki boşluk, açıklama satırları, en son eylem.
- Genişliğe sığmayan satırlar `…` ile kesilir.
- Yalnızca eylem odak alır; gerisi metindir.

## Tema anahtarları

- `empty-state-icon` — `fg`.
- `empty-state-title` — `fg`, `bold`.
- `empty-state-message` — `fg`.
- `inbox` ikonu — boş koleksiyonlar için uygun bir varsayılan ikon.
