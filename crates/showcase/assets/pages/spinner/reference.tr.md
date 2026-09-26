## Metotlar

- `Spinner::new()` — vurgu renginde yay biçimli bir spinner.
- `.animation(ad)` — bunun yerine herhangi bir hücre animasyonunu adıyla oynatır, örneğin temanın tanımladığı bir animasyonu; bilinmeyen bir ad `⟦` çizer.
- `.style(SpinnerStyle)` — nasıl hareket edeceği.
- `.label(metin)` — spinner'dan iki hücre sonra, sığacak kadar kesilen metin.
- `.variant(isim)` — `"success"`, `"warning"`, `"danger"` gibi tema varyantı.
- `.done(bool)` — iş bitti: onay işaretini bir kez oynat ve onda kal. Varsayılan kapalı; kapanınca yeniden döner.
- `.delayed(busy)` — spinner'ı yalnızca yavaş işte gösterir: `busy` true olalı 300 ms geçmeden hiçbir şey, çıktıktan sonra `busy` daha önce false olsa da en az 500 ms. İşin sürüp sürmediğini her görünümde ver. Varsayılan kapalı: spinner her zaman görünür.

## Stiller

- `Arc` — dönerek süpüren yay (animasyon `spinner-arc`). Varsayılan. Nerd Font `nf-extra-progress_spinner_1..6`, U+EE06 ile U+EE0B arası (Nerd Font v3); Unicode `◜◠◝◞◡◟`; ASCII `-\|/`.
- `Dots` — dönen braille noktalar (animasyon `spinner-dots`).
- `Orbit` — hücrenin çevresinde dolaşan nokta (animasyon `spinner-orbit`).
- `Pop` — büyüyüp küçülen nokta (animasyon `spinner-pop`).
- `Pulse` — silik renk ile ton arasında nefes alan nokta (animasyon `spinner-pulse`, renk `pulse($muted, $fg)`).
- `Quarters` — hücrenin saat yönünde dönen dolu çeyreği, `▖▘▝▗` (animasyon `spinner-quarters`).
- `Slices` — dilim dilim dolup yeniden başlayan pasta (animasyon `spinner-slices`): Nerd Font `nf-md-circle_slice_1..8`, U+F0A9E ile U+F0AA5 arası, Nerd Font v3 ister; Unicode `◐◓◑◒`; ASCII `.oO@`.

`SpinnerStyle::ALL` hepsini bu alfabetik sırayla listeler, `.name()` kısa adı, `.animation()` animasyonun adını verir.

## Bitiş

`spinner-done` animasyonunda beş kare var; her biri bir `motion.step` sürer ve bir kez oynar (`playback = "once"`):

1. `nf-md-progress_check` U+F0995, Unicode `·`, ASCII `.` — spinner'ın rengi.
2. `nf-fa-check_circle_o` U+F05D, Unicode `∙`, ASCII `.` — `mix($success, $fg, 25%)`.
3. `nf-oct-check_circle` U+F49E, Unicode `✓`, ASCII `v` — `mix($success, $fg, 50%)`.
4. `nf-md-check_circle_outline` U+F05E1, Unicode `✔`, ASCII `v` — `mix($success, $fg, 75%)`.
5. `nf-md-checkbox_marked_circle` U+F0133, Unicode `✔`, ASCII `v` — `$success`; spinner burada kalır.

Nerd Font karakterleri Nerd Font v3 ister. Spinner'ın rengi `spinner` (ya da varyantı) anahtarından gelir; nabız, `done` açıldığı anda gösterdiği renkten başlar.

## Davranış

- Bir hücre ölçer; etiket varsa iki hücre ve etiket genişliği eklenir. Her stilin ve bitişin her karesi her karakter modunda bir hücre genişliğindedir; bitiş etiketi hiç kaydırmaz.
- Kareyi tam bir sonraki kare zamanı geldiğinde ister; işarette duran spinner kare istemez.
- Baştan `done(true)` ile (ilk çizildiğinde) gelen spinner oynatmadan işarette durur.
- Hareket azaltıldığında ilk kareyi, `Pulse` için tam tonu gösterir ve kare istemez; bitmiş spinner son kareyi kendi renginde hemen gösterir.
- `delayed(busy)` ile: görünse de görünmese de aynı ölçüdedir; gizliyken etiketi dahil hiçbir şey çizmez. Zamanı, `busy` diyen ilk görünümden itibaren spinner'ın kendi belleğinde tutulur. Çıkacağı an (iş başladıktan 300 ms sonra) ve kaybolacağı an (çıktıktan 500 ms sonra, iş bitmişse) için bir kare ister; gizli ve boştaki spinner hiçbir şey istemez.

## Tema anahtarları

- `spinner`, `spinner.<varyant>` — `fg`.
- `spinner-label`, `spinner-label.<varyant>` — `fg`, `bold`.
- `[motion]` — `spinner`, `pulse-period`, `step` (bitişin bir karesi).
- `[animations]` — `spinner-arc`, `spinner-dots`, `spinner-orbit`, `spinner-pop`, `spinner-pulse`, `spinner-quarters`, `spinner-slices`, `spinner-done`. Eski ikon anahtarları (`spinner`, `spinner-arc`, …, `spinner-done`) `[icons]` içinde hâlâ çalışır: animasyonlarının karakterlerini değiştirir, süreyi ve renkleri korur.
