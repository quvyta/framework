## Metotlar

- `ShimmerText::new(metin)` — üzerinden ışık süzülen metin.
- `.style(ShimmerStyle)` — `Sweep` (varsayılan) ya da `Dots`.

## Davranış

- Metnin genişliğini ve bir satırı ölçer; `Dots` noktalar için üç hücre ekler.
- `Sweep`: merkezinin iki yanında beşer hücrede sönen bir bant, her `motion.shimmer` süresinde harflerin üzerinden bir kez, yavaş başlayıp yavaş biterek geçer.
- `Dots`: `motion.shimmer` süresince sıfırdan üçe nokta.
- Yalnızca hareket azaltılmamışsa kare ister.

## Tema anahtarları

- `shimmer` — dinlenen harfler için `fg`, ışık için `highlight`.
- `[motion]` — `shimmer`.
