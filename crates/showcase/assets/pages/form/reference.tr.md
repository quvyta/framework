## Metotlar

- `Form::new()` — etiketler kontrollerin üstünde, alanlar arasında bir satır, özet yok.
- `.label_width(hücre)` — form en az `hücre + 18` genişlikteyken etiketler kontrollerin yanında bir sütunda. Kontrolü etiketin yanındaki yerden geniş isteyen alanın etiketi üstüne geçer.
- `.gap(satır)` — alanlar arasındaki satır; varsayılan 1.
- `.summary(&errors)` — `errors` içindeki her mesajı alanların üstünde listeler; boşken görünmez.
- `.show(ui, |form| …)` — formu ekler; `form.field(alan, |ui| …)` kontrolüyle bir alan ekler, `form.ui()` başka her şeyi.
- `Field::new(etiket)` — bir etiket ve içine eklenen kontrol, başka bir şey yok.
- `.hint(metin)` — ne uyarı ne hata varken kontrolün altında silik yardım.
- `.value(metin)` — kontrolün yerinde okunacak bir değer, kontrollerle aynı hizada; etiketi silik.
- `.warning(Option<metin>)` — kontrolün altında, hatanın alacağı yerde, uyarı renginde ve kendi işaretiyle çizilen uyarı. `FormErrors` içine hiç girmez; yalnızca uyarı taşıyan alanlardan oluşan form yine gönderilir. Yanında hata varsa hata o yeri alır.
- `.error(Option<metin>)` — kontrolün altında, tehlike renginde ve kendi işaretiyle çizilen hata; uyarının ve ipucunun yerine geçer.
- `.required(bool)` — etiketin ardından silik "zorunlu" sözcüğü.
- `.disabled(bool)` — soluk etiket; kontrolü de pasif yap.
- `.label_width(hücre)` — yalnızca bu alanın etiket sütunu; yoksa formunki geçerlidir.
- `FormErrors::new()`, `.set(ad, mesaj)`, `.check(ad, geçerli, mesaj)`, `.remove(ad)`, `.clear()`.
- `.get(ad)`, `.has(ad)`, `.first()`, `.len()`, `.is_empty()`, `.iter()`.
- `.focus_first()` — ilk soruna `Command::focus`, sorun yoksa `Command::none()`.

## Davranış

- Odaktaki kontrolün kullanmadığı Enter, odağı sonraki odaklanabilir bileşene taşır.
- Odak kontrolünde ya da alanın içindeki herhangi bir şeydeyken etiket `focus` durumunu alır.
- Kontrolün yanındaki düzende zorunlu sözcüğü etiketin, mesaj kontrolün altına gelir.
- Alan başına tek mesaj, bu sırayla: hata, uyarı, ipucu. İlk olan çizilir, ötekiler gösterilmez.
- İpuçları, uyarılar ve hatalar alan genişliğinde satıra bölünür; özet uzun mesajları `…` ile keser.
- Uyarı hata gibi çizilir ve renklenir ama hata değildir: `FormErrors` içine, `Form::summary` içine, `TextInput::invalid` içine ve `focus_first` içine girmez, form gönderilir.
- Henüz ekranda olmayan bir bileşen için `Command::focus(ad)`, bir sonraki karede belirirse orada uygulanır.

## Tema anahtarları

- `field-label` — `fg`, `bold`; durumlar `focus`, `disabled`.
- `field-required`, `field-hint`, `field-warning`, `field-error` — `fg`. `field-warning` için renk vermeyen tema, mesajın bildirdiği durumun uyarı rengini yine alır.
- `form-summary` — `bg`, `padding`.
- `form-summary-title` — `fg`, `bold`; `form-summary-marker`, `form-summary-item` — `fg`.
- `[icons]` — hata işareti için `error`, uyarı işareti için `warning`; her glyph kipinde birer hücre.
- Dil — `quvyta.form.required`, `quvyta.form.summary` (çoğul, `n`).
