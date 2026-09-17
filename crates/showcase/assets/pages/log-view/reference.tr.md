## Metotlar

- `LogView::new(&tampon)` — bir `LogBuffer` gösterir.
- `.min_level(LogLevel)` — daha önemsiz satırları gizler.
- `.search(sorgu)` — `sorgu`yu içeren satırları bırakır ve vurgular; küçük harf büyük-küçük farkına bakmaz.
- `.empty_text(metin)` — tampon boşken görünür.
- `.on_copy(|satır| mesaj)` — `c` satırları kopyaladıktan sonra gönderilir.
- `LogBuffer::new(kapasite)`, `.push(satır)`, `.clear()`, `.len()`, `.is_empty()`, `.capacity()`, `.get(index)`, `.iter()`.
- `LogLine::new(seviye, metin)`, `.time(zaman)`; `.level()`, `.timestamp()`, `.text()`.
- `LogLevel::Trace | Debug | Info | Warn | Error`, `LogLevel::ALL`, `.name()`.

## Davranış

- En alttayken sonu takip eder; tekerlek, kaydırma çubuğu ya da imleci yukarı taşımak takibi durdurur; en alta inmek, `end` ya da aşağıdaki satırlar notuna tıklamak yeniden başlatır.
- Odaktayken tuşlar: `up` `down` ya da `k` `j` imleci taşır, `shift` ile seçimi genişletir; `pgup` `pgdn` sayfa atlar; `home` en eski satıra gider; `end` takibe döner; `esc` imleci kaldırır; `c` kopyalar.
- Fare: tıklama imleci koyar, `shift` ile tıklama ya da sürükleme genişletir, tekerlek kaydırır.
- Kopyalanan satırlar `zaman seviye mesaj` biçimindedir, her biri ayrı satırda. Yalnızca filtrelerden geçen satırlar kopyalanır.
- Hiçbir satır filtrelerden geçmezse görünüm `quvyta.log.no-match` metnini gösterir.
- Bir satıra sağ tık, o satırı içeren seçimi korur ya da o satırı seçer, sonra Kopyala ve Ham kopyala menüsünü açar; satırlar seçiliyken Shift+F10 ve menü tuşu da açar. İki kopya da satır sayısıyla `on_copy` gönderir. Dil anahtarları `quvyta.edit.copy`, `quvyta.edit.raw-copy`.

## Tema anahtarları

- `list-item` (`hover`, `selected`, `focus`) — satırlar, List ile ortak.
- `log-time` — `fg`; `log-level.trace`, `.debug`, `.info`, `.warn`, `.error` — `fg`, `bold`.
- `log-match` — arama eşleşmelerinin `bg`, `fg` renkleri; `log-more` — metni `quvyta.log.below` olan aşağıdaki satırlar notunun `bg`, `fg` renkleri.
- `list-header` — boş metin; `scrollbar` — `track`, `thumb`.
