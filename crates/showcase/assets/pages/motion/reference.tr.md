## Çizim yardımcıları

- `cx.animate(isim, hedef, süre, yumuşatma) -> f32` — `hedef`e süzülen bir değer; ilk hedefinde durarak başlar.
- `cx.progress_since(başlangıç, süre, yumuşatma) -> f32` — `başlangıç`tan beri 0'dan 1'e yumuşatılmış ilerleme.
- `cx.cycle(periyot) -> f32` — tekrarlayan bir animasyonun 0 ile 1 arasındaki evresi.
- `cx.ticks(aralık) -> u128` — geçen tam aralık sayısı; sonraki kare tam bir sonraki adıma denk gelir.
- `cx.pulse_phase() -> f32` — temanın nefes alma atımının 0 ile 1 arasındaki evresi; hareket azaltılmışsa 0.
- `cx.now()` — `progress_since` için karenin zamanı; `cx.request_frame_in(gecikme)` — yardımcıların kapsamadığı hareketler için `gecikme` sonra bir kare daha.
- `cx.reduced_motion() -> bool` — kullanıcının hareketi azaltmak isteyip istemediği.

## Yapı taşları

- `Easing` — `Linear`, `EaseIn`, `EaseOut` (varsayılan), `EaseInOut`; `.apply(t)`, `.name()`, `Easing::ALL`.
- `Tween` — iki sayı arasında hareket eden değer: `settled`, `value(now)`, `target`, `is_running(now)`, `retarget`.
- `steps(ilerleme, sayı) -> u16` — ilerlemenin `sayı` hücreden kaçıncısına ulaştığı.
- `Command::set_reduced_motion(bool)` — hareketi azaltmayı çalışırken açar ya da kapatır; bir sonraki açılışa kalması için `Settings::REDUCED_MOTION` ile sakla.

## Tema anahtarları

`[motion]` — `"140ms"` ya da `"1.6s"` gibi süreler olarak `enter`, `spinner`, `shimmer`, `pulse-period`, `flash`, `cursor-blink`, `step`, `page`, `hover-delay`; `true` ya da `false` olarak `slide`.

## Ortam

- `QUVYTA_REDUCED_MOTION=1` — hareket azaltılmış, `=0` — hareket açık; ikisi de kayıtlı `reduced-motion` ayarını ve `Command::set_reduced_motion`'ı geçersiz kılar. Tanımsız ya da boşsa karar onlarındır.
- `Env::reduced_motion_forced() -> bool` — kararı değişkenin verip vermediği; veriyorsa anahtarı nedeniyle birlikte pasif göster.
