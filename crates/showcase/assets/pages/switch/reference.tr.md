## Metotlar

- `Switch::new(açık)` — `açık` durumunu gösteren kapsül anahtar.
- `.label(metin)` — anahtardan iki hücre sonra metin; tıklamak değiştirir.
- `.style(SwitchStyle)` — `Capsule` (varsayılan), `Rail` ya da `Labeled`.
- `.disabled(bool)` — odak alamaz, değiştirilemez.
- `.on_toggle(|on| mesaj)` — yeni durumla mesaj.

## Davranış

- Kapsül ve ray beş hücre genişliğindedir; etiketli stil uzun sözcük artı altı hücredir.
- Topuz her `motion.step` süresinde bir hücre olmak üzere üç hücre gider; renkler her hücrede karışır.
- Enter, Boşluk ya da bileşen üzerinde bırakılan tıklama değiştirir.

## Tema anahtarları

- `switch` — `track`, `track-on`, `knob`, `knob-on`; durumlar `hover`, `focus`, `checked`, `disabled`.
- `switch-labeled` — `bg`, `fg`, `bold`, `dot`; aynı durumlar.
- `switch-label` — `fg`; aynı durumlar.
- `[motion]` — `step`.
- `[icons]` — `switch-rail`, `switch-knob`, `cap-left`, `cap-right`, `dot`.
- Dil — `quvyta.switch.on`, `quvyta.switch.off`.
