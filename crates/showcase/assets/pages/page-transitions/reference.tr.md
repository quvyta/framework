## Metotlar

- `PageTransition::new(anahtar)` — `anahtar` değişince canlanan bir kap; varsayılan olarak yumuşak geçiş yapar.
- `.slide(bool)` — gelen sayfa yerine kayarak da gelir. Varsayılan: `false`.
- `.direction(Navigation)` — kayan sayfanın hangi taraftan geleceği. Varsayılan: `Forward`.
- `Router::direction() -> Navigation` — `push` ya da `replace` sonrasında `Forward`, `back` sonrasında `Back`.
- `Navigation::Forward`, `Navigation::Back`.

## Davranış

- Süre: `motion.page`, sona doğru yavaşlayarak. `page` tanımlamayan temalar `motion.enter` süresinin iki katını kullanır.
- İlk yarı: eski karakterler, yazı renkleri zemine doğru karışarak. İkinci yarı: yeni karakterler zeminden belirerek. Zeminler baştan sona karışır.
- Kayma mesafesi: genişliğin sekizde biri, en fazla 6 hücre; renklerle aynı ilerlemeyle hücre hücre; ileri giderken sağdan, geri dönerken soldan.
- İlk çizimde, alanın boyutu değiştiğinde ya da hareket azaltılmışken geçiş olmaz.
- 24-bit olmayan renkler karışmak yerine yarı yolda değişir.
- Alanın kenarında kesilen geniş karakterler geçiş boyunca boşluk olarak çizilir.

## Tema anahtarları

- `[motion] page` — `"260ms"` gibi bir süre.
