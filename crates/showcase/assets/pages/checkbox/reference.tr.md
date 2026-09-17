## Metotlar

- `Checkbox::new(işaretli)` — `işaretli` durumunu gösteren kutu.
- `.label(metin)` — kutudan iki hücre sonra metin; tıklamak değiştirir.
- `.partial(bool)` — kutuyu kısmen işaretli gösterir; değiştirmek `true` ister.
- `.style(CheckboxStyle)` — `Box` (varsayılan): iki hücre renk. `Check`: tikli ya da çizgili üç hücre.
- `.disabled(bool)` — odak alamaz, değiştirilemez.
- `.on_toggle(|on| mesaj)` — yeni durumla mesaj.

## Davranış

- İki hücre (`Box`) ya da üç hücre (`Check`) ölçer; etiket varsa iki hücre ve etiket eklenir; bir satır. Dar alanda etiket `…` ile kesilir.
- `Box` her hücreyi boş renkten (`checked` olmadan durumlar) dolu renge (`checked` ile) `motion.step` × 3 sürede karıştırır; hareket azaltılmışsa hemen geçer. Kısmen işaretli kutu sol hücresini doldurur.
- Kutuya, aradaki boşluğa ya da etikete tıklamak değiştirir.
- Enter, Boşluk ya da bileşen üzerinde bırakılan tıklama değiştirir.
- `on_toggle` yoksa görünür ama odak alamaz.

## Tema anahtarları

- `checkbox` — `bg`, `fg`, `bold`; durumlar `hover`, `focus`, `checked`, `disabled`; varyant `partial` (tikli stil; kutu stili bunun yerine sol hücreyi doldurur).
- `checkbox-label` — `fg`, `bold`; aynı durumlar.
- `[motion]` — `step` (kutu geçişi).
- `[icons]` — `check`, `check-partial` (yalnızca tikli stil).
