## Metotlar

- `Form::new()` — etiketler kontrollerin üstünde, alanlar arasında bir satır, özet yok.
- `.label_width(hücre)` — form en az `hücre + 18` genişlikteyken etiketler kontrollerin yanında bir sütunda. Kontrolü etiketin yanındaki yerden geniş isteyen alanın etiketi üstüne geçer.
- `.gap(satır)` — alanlar arasındaki satır; varsayılan 1.
- `.summary(&errors)` — `errors` içindeki her mesajı alanların üstünde listeler; boşken görünmez.
- `.show(ui, |form| …)` — formu ekler; `form.field(alan, |ui| …)` kontrolüyle bir alan ekler, `form.ui()` başka her şeyi.
- `Field::new(etiket)` — bir etiket ve içine eklenen kontrol, başka bir şey yok.
- `.hint(metin)` — hata yokken kontrolün altında silik yardım.
- `.error(Option<metin>)` — ipucunun yerine kontrolün altında hata.
- `.required(bool)` — etiketin ardından silik "zorunlu" sözcüğü.
- `.disabled(bool)` — soluk etiket; kontrolü de pasif yap.
- `.label_width(hücre)` — yalnızca bu alanın etiket sütunu; yoksa formunki geçerlidir.
- `FormErrors::new()`, `.set(ad, mesaj)`, `.check(ad, geçerli, mesaj)`, `.remove(ad)`, `.clear()`.
- `.get(ad)`, `.has(ad)`, `.first()`, `.len()`, `.is_empty()`, `.iter()`.
- `.focus_first()` — ilk soruna `Command::focus`, sorun yoksa `Command::none()`.

## Davranış

- Odaktaki kontrolün kullanmadığı Enter, odağı sonraki odaklanabilir bileşene taşır.
- Odak kontrolünde ya da alanın içindeki herhangi bir şeydeyken etiket `focus` durumunu alır.
- Kontrolün yanındaki düzende zorunlu sözcüğü etiketin, ipucu ya da hata kontrolün altına gelir.
- İpuçları ve hatalar alan genişliğinde satıra bölünür; özet uzun mesajları `…` ile keser.
- Henüz ekranda olmayan bir bileşen için `Command::focus(ad)`, bir sonraki karede belirirse orada uygulanır.

## Tema anahtarları

- `field-label` — `fg`, `bold`; durumlar `focus`, `disabled`.
- `field-required`, `field-hint`, `field-error` — `fg`.
- `form-summary` — `bg`, `padding`.
- `form-summary-title` — `fg`, `bold`; `form-summary-marker`, `form-summary-item` — `fg`.
- `[icons]` — `error`.
- Dil — `quvyta.form.required`, `quvyta.form.summary` (çoğul, `n`).
