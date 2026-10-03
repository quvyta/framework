## Metotlar

- `StatusLine::new(metin)` — `metin` yazan, nötr `Info` tonunda bir durum satırı.
- `.tone(ToastKind)` — cümlenin bildirdiği durum: `Success`, `Warning`, `Danger` ya da `Info`. Satıra işaretini ve rengini verir.
- `.action(buton)` — satırın sonunda bir Button; kendi varyantını, kısayolunu ve mesajını korur ve basıldığında o mesajı gönderir.
- `ToastKind::ALL` ve `ToastKind::name()` — bütün türler ve bir türün kısa adı; bir ayar ekranı ya da dil anahtarı onu böyle yazar.

## Davranış

- Cümle ve buton sığdığı sürece tek satır: işaret, bir boşluk, cümle, sonra iki hücrelik boşluktan sonra buton.
- Daha dar alanda cümle kendi altına sarılır, sütununu korur ve buton kendi satırına geçer. Hiçbir şey kesilmez.
- Sığdığı sürece tek satırın genişliğini, yoksa verildiği genişliği ölçer; satır cümlenin, altında butonun bir satır daha yer alır.
- İşaret ve cümle metindir; yalnızca buton odak alır ve girdi alır.
- Konduğu yere göre sarılır: dar bir satır için `.width(Length::Cells(n))`, bütün satır için `.fill_width()`.

## Tema anahtarları

- `status-line` — `fg`, `bold`; `success`, `warning`, `danger` ve `info` varyantlarıyla. Tema bir varyant adı vermediğinde cümle türün rengini alır; işaret her zaman alır.
- `success`, `warning`, `danger`, `info` — hem işaretin hem cümlenin kullandığı renkler.
- `[icons]` — işaretler için `success`, `warning`, `error`, `info`; her glif kipinde birer hücre.
