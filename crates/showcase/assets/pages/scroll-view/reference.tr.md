## Metotlar

- `ScrollView::new()` — içeriği `ui.add_with` ile ekle.
- Boyutunu `.fill()`, `.height(Length::Cells(n))` ya da herhangi bir uzunlukla ver.
- `.scrollbar(ScrollbarStyle)` — temanın yerine sabit bir kaydırma çubuğu stili kullanır.
- `.follow_end(bool)` — kişi sondayken büyüyen içeriğin sonunu görünür tutar; varsayılan olarak kapalı.

## Tuşlar

- Odaktayken: `up` `down` bir satır, `pgup` `pgdn` bir sayfa, `home` `end` kenarlara. `follow_end` açıkken `end` izlemeyi de yeniden başlatır.

## Fare

- Tekerlek üç satır kaydırır; kaydırma çubuğuna tıkla ya da sürükle.
- `follow_end` açıkken aşağıdaki satırları sayan nota tıklamak izlemeyi yeniden başlatır.

## Davranış

- İçinde yeni odaklanan bir bileşeni göstermek için kayar.
- Konumunu kimliğine göre korur; `ui.page` içinde sayfa gizliyken de korunur.
- `follow_end` açıkken: sonunda açılır; içerik büyüyünce temanın `page` süresiyle yeni sona kayar, hareketi azalt açıksa atlar. Yukarı kaydırmak (tekerlek, tuşlar, çubuk) izlemeyi durdurur ve aşağıdaki satırları sayan notu gösterir; yeniden en alta inmek izlemeyi başlatır. Odaklanan bir bileşeni ya da gösterilmesi istenen bir alanı göstermek için yapılan hareket sondan ayrılırsa izlemeyi durdurur, sonda biterse sürdürür. Alana sığan içerik her zaman sonda sayılır.

## Tema anahtarları

- `hover` ile `scrollbar` — `style` (varsayılan `block`, `half`, `thin`, `dots`), `track`, `thumb`; `scrollbar.<stil>` tek bir stili ayarlar.
- `log-more` — aşağıdaki satırları sayan not, `LogView` ile ortak. Metni framework'ün `quvyta.log.below` dizgesidir.

## İlgili

- `ScrollMetrics` — özel kaydırma bileşenleri için `total`, `visible`, `offset`, `.overflows()`, `.thumb(iz)`, `.offset_at(satır, iz)`, `.max_offset()`.
