## Ne zaman kullanılır

Bir ekranın yerini başka bir ekranın aldığı her yerde sayfa geçişi kullan: bir router'ın sayfaları, bir akışın adımları, bir ayarlar ekranının bölmeleri. Kullanıcıya yerin değiştiğini, kayma açıksa hangi yöne gittiğini anlatır. Yerinde güncellenen içerik için, örneğin yenilenen bir liste için kullanma.

Katmanlar (açılan bir pencere, kayarak gelen bir bildirim, açılan bir liste) kendi bileşenlerinin içinde canlanır; bu sayfa düzeyindeki mekanizmadır.

## Adım adım

1. Durumunda bir `Router` tut ve `push` ile `back` kullanarak gezin.
2. `view` içinde sayfayı sarmala: `ui.add_with(PageTransition::new(sayfa.clone()), |ui| ui.page(sayfa, kur)).fill()`.
3. Gezinmeyi izleyen kısa bir kayma için `.slide(true).direction(router.direction())` ekle.
4. Süreyi temaya bırak: `[motion] page = "260ms"`.

## Nasıl çalışır

- **Karar anahtarındır.** `PageTransition::new`'a verilen anahtar önceki karedekinden farklıysa geçiş başlar; aynı sayfada değişen başka her şey yalnızca yeniden çizilir.
- **Hücre hücre.** Giden ekran hatırlanır. İlk yarıda eski yazı karışmakta olan zemine doğru solar, ikinci yarıda yeni yazı zeminden belirir. Zeminler baştan sona karışır, böylece yüzeyler asla sıçramaz.
- **Tam hücrelerle kayma.** Kayma açıkken gelen sayfa en fazla altı hücre öteden başlar ve yerine hücre hücre gelir; renkleri aynı ilerlemeyle karışır, yani her adım renkte de bir adımdır.
- **Yön.** `Router::direction()`, `push` ya da `replace` sonrasında `Forward`, `back` sonrasında `Back` olur. İleri giden sayfalar sağdan, geri dönenler soldan gelir.
- **Asla engel olmaz.** Yeni sayfa ilk kareden itibaren canlıdır. Hareketi azaltma, boyutu değişen alan ya da ilk çizim yeni sayfayı hemen gösterir.

## Sık yapılan hatalar

- **Her karede değişen bir anahtar kullanmak**, örneğin bir sayaç ya da zaman: sayfa durmadan geçiş yapar.
- **Sayfa yerine her bölümü ayrı sarmalamak.** Geçişi bütün olarak değişen içeriğin çevresine koy.
- **Uzun süreler.** Sayfalar sık değişir; gezinme çevik kalsın diye `motion.page` kısa olsun.
