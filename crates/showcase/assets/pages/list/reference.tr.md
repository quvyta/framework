## List

- `List::new(öğeler)` — `ListItem` listesi.
- `List::shared(öğeler)` — durumda `Arc<[ListItem]>` olarak tutulan öğeler; her karede yalnızca `Arc` kopyalanır, uzun liste yeniden kurulmaz.
- `.selected(Option<usize>)` — seçili satır.
- `.on_select(|sıra| msg)` — seçim değişti.
- `.on_activate(|sıra| msg)` — bir satır Enter ya da tıklamayla açıldı.
- `.checked(Vec<bool>)` ve `.on_toggle(|sıra| msg)` — çoklu seçim.
- `.empty_text(metin)` — öğe yokken gösterilir.
- `.scrollbar(ScrollbarStyle)` — temanın yerine sabit bir kaydırma çubuğu stili kullanır.

## ListItem

- `ListItem::new(etiket)`, `ListItem::header(başlık)`, `ListItem::gap()`.
- `.icon(anahtar, Some(değişken))` — etiketten önce, istenirse renkli ikon.
- `.detail(metin)` — sağa hizalı silik metin.
- `.faint(bool)` — silik ama seçilebilir.

## Tuşlar

- `up` `down` ya da `k` `j` gezinir; `home` `end` atlar; `pgup` `pgdn` sayfa geçer; `enter` açar; çoklu seçimde `space` işaretler, diğer durumda açar.

## Fare

- Tıklama seçer ve açar; işarete ya da hemen sonraki hücreye tıklamak yalnızca işaretler; tekerlek kaydırır; kaydırma çubuğu sürüklenir.

## Satırın yapısı

- Çubuk, sonra sabit işaretler (çoklu seçim işareti), sonra kayan kısım (ikon ve etiket), en sağda sabit detay.
- `motion.slide` açıkken hover edilen ya da seçili satırda yalnızca kayan kısım bir hücre sağa geçer. Etiket bir boş hücre payı ayırır; dururken de kayarken de aynı yerden kesilir.

## Tema anahtarları

- `hover`, `selected`, `focus`, `pressed` ile `list-item` — `bg`, `fg`, `bold`, `pillar`.
- `list-item.faint`, `list-header`, `list-detail`, `scrollbar` (`style`, `track`, `thumb`) ve `scrollbar.<stil>`.
- `[icons]` — `select-on`, `select-off`: çoklu seçim işaretleri (işaretli ve boş kutu).
