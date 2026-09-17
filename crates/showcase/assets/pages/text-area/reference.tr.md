## Metotlar

- `TextArea::new(metin)` — `metin`i gösteren alan.
- `.placeholder(metin)` — alan boşken görünen soluk metin.
- `.max_length(n)` — en çok `n` karakter; satır sonu bir karakter sayılır.
- `.counter(bool)` — metnin altındaki satırda karakter sayısı; sınır varsa `sayı / sınır`.
- `.line_numbers(bool)` — her satırın ilk görünen satırının önünde soluk bir numara.
- `.invalid(bool)` — metni doğrulamadan geçemedi diye işaretler.
- `.disabled(bool)` — salt okunur ve odak alamaz.
- `.on_change(|metin| mesaj)` — her düzenlemeden sonra yeni metinle mesaj.
- `.on_submit(|metin| mesaj)` — Ctrl+Enter'da metinle mesaj.

## Davranış

- Verilen genişliğin tamamını kullanır; yüksekliği 3..8 arasına sıkıştırılmış satır sayısı, sayaç satırı ve iç boşluktur. Düğüme verilen yükseklik bunu geçersiz kılar.
- Satırlar, metin genişliğinden bir hücre eksiğinde sözcük aralarından kırılır; o hücre dolu satırın ardındaki imleç içindir. Kırılma yerindeki boşluklar satırın sonunda kalır.
- Satırlar sığmayınca sağ sütunda kaydırma çubuğu belirir ve metin bir hücre daha dar kırılır.
- Enter satır sonu ekler; Ctrl+Enter `on_submit` verilmişse gönderir, verilmemişse kullanılmaz.
- ↑ ↓ ve Page Up / Page Down sütunu bir hedefte tutar, başka bir tuşla hedef sıfırlanır; ilk ya da son satırın ötesinde metnin başına ya da sonuna gider.
- Home / End: görünen satırın başı ve sonu (End, kırılan satırın sondaki boşluğundan önce durur); Ctrl ile metnin başı ve sonu.
- Düzenleme tuşları `TextInput` ile aynıdır; Ctrl+U satır başına kadar siler. Yapıştırılan `\r\n` `\n` olur, sekme dört boşluk olur.
- Her tuş ve yapıştırma imleci görünür alana kaydırır; tekerlek üç satır kaydırır; kaydırma çubuğuna basmak ya da sürüklemek kaydırır; metne basmak imleci yerleştirir, sürüklemek seçer.
- Shift olmadan, seçim varken ← → onu kaldırır ve sol ya da sağ ucundan ilerler; ↑ ↓ ve Page Up / Page Down üst ya da alt ucundan ilerler.
- Sağ tık, Shift+F10 ya da menü tuşu `TextInput` düzenleme menüsünü açar (Kes, Kopyala, Yapıştır, Tümünü seç); seçimin içindeki sağ tık seçimi korur, başka yerde önce imleci oraya koyar. Dil anahtarları `quvyta.edit.*`.

## Tema anahtarları

- `text-area` — `bg`, `fg`, `padding`; durumlar `hover`, `focus`, `invalid`, `disabled`.
- `text-area-line-number` — `fg`; imlecin satırı için `selected`.
- `text-area-counter` — `fg`.
- `text-input-placeholder`, `text-input-selection`, `text-input-cursor` — `TextInput` ile ortak.
- `scrollbar` — `track`, `thumb`; `[icons]` `scroll-thumb`, `scroll-track`.
- `[motion]` — `cursor-blink`.
