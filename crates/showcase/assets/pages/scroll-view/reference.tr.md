## Metotlar

- `ScrollView::new()` — içeriği `ui.add_with` ile ekle.
- Boyutunu `.fill()`, `.height(Length::Cells(n))` ya da herhangi bir uzunlukla ver.
- `.scrollbar(ScrollbarStyle)` — temanın yerine sabit bir kaydırma çubuğu stili kullanır.

## Tuşlar

- Odaktayken: `up` `down` bir satır, `pgup` `pgdn` bir sayfa, `home` `end` kenarlara.

## Fare

- Tekerlek üç satır kaydırır; kaydırma çubuğuna tıkla ya da sürükle.

## Davranış

- İçinde yeni odaklanan bir bileşeni göstermek için kayar.
- Konumunu kimliğine göre korur; `ui.page` içinde sayfa gizliyken de korunur.

## Tema anahtarları

- `hover` ile `scrollbar` — `style` (varsayılan `block`, `half`, `thin`, `dots`), `track`, `thumb`; `scrollbar.<stil>` tek bir stili ayarlar.

## İlgili

- `ScrollMetrics` — özel kaydırma bileşenleri için `total`, `visible`, `offset`, `.overflows()`, `.thumb(iz)`, `.offset_at(satır, iz)`, `.max_offset()`.
