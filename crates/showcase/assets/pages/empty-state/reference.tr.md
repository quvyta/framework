## Metotlar

- `EmptyState::new(başlık)` — `başlık` yazan bir boş durum.
- `.icon(anahtar)` — durum tonu yokken başlığın üstünde çizilen ikon setinden bir ikon.
- `.message(metin)` — başlığın altındaki açıklama; en fazla 52 hücrede sarılır.
- `.action(buton)` — açıklamanın altına bir Button ekler. Birkaç eşit seçenek için birden çok kez çağır; ilk eklenen birincildir ve okuma ile odak sırasını korur.
- `.tone(ToastKind)` — ikonu ve başlığı `success`, `warning`, `danger` ya da `info` tema rengine boyar ve uygun durum işaretini kullanır. Ton verilmezse mevcut ikon ve başlık renkleri değişmez.

## Davranış

- Aldığı tüm genişliği ve parçalarının gerektirdiği satırları ölçer; alanın ortasına çizilir.
- Birden çok eylem butonu, diyalog eylemleriyle aynı iki hücrelik boşlukla ortalanmış bir satırda yan yana durur. Sığmazlarsa eklenme sırasına göre her biri kendi ortalanmış satırında alt alta durur.
- Alan kısaysa sırasıyla şunları bırakır: ikon ve boşluğu, eylemlerin üstündeki boşluk, açıklama satırları, en son eylemler. Yer olduğu sürece başlık ve eylemler kalır.
- Genişliğe sığmayan satırlar `…` ile kesilir.
- Her eylem odak alabilir. Tab ve ok tuşları butonları okuma sırasında gezdirir; her buton kendi varyantını, kısayolunu ve mesajını korur.
- Ton, `Toast` ile aynı durum rengini ve işaretini kullanır; işaret on altı renkte ve ASCII ikon kipinde de görünür hücre olarak kalır.

## Tema anahtarları

- `empty-state-icon` — `fg`.
- `empty-state-title` — `fg`, `bold`.
- `empty-state-message` — `fg`.
- `success`, `warning`, `danger` ve `info` — `.tone(..)` için kullanılan durum renkleri.
- `inbox` — ton işareti sağlamadığında boş koleksiyonlar için uygun bir varsayılan ikon.
