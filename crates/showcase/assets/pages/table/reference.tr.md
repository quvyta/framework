## Metotlar

- `Table::new(sütunlar, satırlar)` — `satırlar` bir `Vec<TableRow>` ya da `Arc<[TableRow]>`.
- `.selected(Option<usize>)`, `.on_select(|index| mesaj)`, `.on_activate(|index| mesaj)`.
- `.checked(Vec<bool>)` ve `.on_toggle(|index| mesaj)` — çoklu seçim.
- `.sort(sütun, SortDirection)` — sıralama okunu gösterir; satırlar zaten bu sırada olmalı.
- `.on_sort(|sütun, yön| mesaj)` — başlık tıklaması ve tuşlarla sıralamayı açar.
- `.empty_text(metin)` — satır yokken başlığın altında görünür.
- `.context_menu(|index| Vec<ContextItem<Msg>>)` — her satıra kendi menüsünü verir; menü, açıldığı satır için kurulur.
- `.menu_on_activate(bool)` — Enter ve tık `on_activate` göndermek yerine satırın menüsünü açar; boş menü hiçbir şey açmaz. Varsayılan kapalı.
- `.activate_on(Click::Single | Click::Double)` — varsayılanı `Single`: tık seçer ve etkinleştirir. `Double` ile tık yalnızca seçer, aynı satıra `Click::INTERVAL` (400 ms) içinde ikinci basış onu etkinleştirir; Enter her iki durumda da etkinleştirir.
- `.multi_select(&seçili, |satırlar| mesaj)` — birden çok satır birlikte seçilir: Ctrl+tık satırı ekler ya da çıkarır, Shift+tık aralık seçer, `shift+up` `shift+down` `shift+pgup` `shift+pgdn` `shift+home` `shift+end` onu uzatır, `ctrl+a` her satırı seçer, Space imlecin satırını ekler ya da çıkarır, `esc` birden çok seçili satırı imlecinkine indirir; `satırlar` her zaman seçimin yeni halinin tamamıdır, `.selected(..)` imleçtir. Seçili satırlar seçim tonunu paylaşır, çubuğu yalnızca imlecin satırı taşır.
- `.box_select(bool)` — `multi_select` ile, satırların altındaki boş yerden sürüklemek `text-selection` tonunda bir alan çizer, kapladığı satırlar seçim olur ya da basarken Ctrl basılıysa seçime eklenir; orada tıklamak seçimi bırakır.
- `.droppable(|bırakma| mesaj, |sıra| kabul)` — basılan satır ya da içinde olduğu seçim, `kabul`ün evet dediği bir satırın üstüne sürüklenir; o satır `tree-drop` tonunu alır. `bırakma` bir `RowDrop { rows, into }`'dur. Başka bir yerde bırakmak hiçbir şey yapmaz. `Click::Single` ile satır bu durumda bırakırken etkinleşir, böylece sürüklemeye dönen basış hiçbir şeyi etkinleştirmez.
- `.on_copy_drop(|bırakma| mesaj)` — Ctrl basılıyken yapılan bırakma, taşıma yerine bununla kopyalama ister.
- `Column::new(başlık)`, `.width(ColumnWidth::Fixed(n) | Fit | Fill(ağırlık))`, `.min(hücre)`, `.align(Align)`, `.sortable(bool)`.
- `TableRow::new(hücreler)`, `.faint(bool)`; `TableCell::new(metin)`, `.icon(glif, renk)`, `.color(token)`; metinler hücreye dönüşür. `glif` bir ikon anahtarı (`"dot"`, `Glyph::key(..)`) ya da `Glyph::literal(..)` olur; glif, bir boşluk ve metin olarak çizilir. `Some(token)` ile o renkte, `None` ile `muted` çizilir ve satır seçiliyken satırın metin rengini alır. Kesme yalnızca metne uygulanır.
- `SortDirection::Ascending | Descending`, `.reversed()`.

## Davranış

- Odaktayken tuşlar: `up` `down` ya da `k` `j`, `pgup` `pgdn`, `home` `end` gezinir; `enter` etkinleştirir; `space` çoklu seçimde işaretler, değilse etkinleştirir; `left` `right` taşan sütunları kaydırır; `s` sonraki sıralanabilir sütuna göre sıralar, `shift s` yönü çevirir.
- Satır menüsü varken: bir satıra sağ tıklama o satırın menüsünü imlecin yanında açar ve satır işaretli değilse onu seçim yapar; menü tuşu ya da `shift f10` seçili satırın menüsünü satırın altında açar, önce satırı görünür kılar. Menünün ait olduğu satır menü açıkken yükselmiş kalır. Menünün yanına yapılan bir basış menüyü kapatır ve altındakine yine ulaşır.
- Fare: tıklama satırı seçer ve etkinleştirir; işarete ya da hemen sonraki hücreye tıklama yalnızca işaretler; sıralanabilir başlığa tıklama sıralar, tekrar tıklama çevirir; başlıktaki bir oka tıklama sütunları bir adım kaydırır; tekerlek ve kaydırma çubuğu kaydırır.
- Önce sabit ve içeriğe uyan genişlikler yerleşir, dolduran sütunlar kalanı ağırlıklarına göre paylaşır. En küçük genişlikler sığmazsa sütunlar küçülmez, yana kayar.
- Görünen ilk hücre seçim kayması için bir boş hücre ayırır ve uzun metni `…` ile keser. Çoklu seçim işareti hiç kaymaz.

## Tema anahtarları

- `list-item` (`hover`, `selected`, `focus`, `pressed`) ve `list-item.faint` — satırlar, List ile ortak.
- `table-header` — `bg`, `fg`; sıralanabilir başlık üstünde `hover`, sıralı başlıkta `selected`.
- `table-sort` — sıralama okunun `fg` rengi.
- `table-scroll` — başlık oklarının `fg`, `bg` rengi; `hover`.
- `list-header` — boş metin; `scrollbar` — `track`, `thumb`.
- `[icons]` — `select-on`, `select-off`: çoklu seçim işaretleri (işaretli ve boş kutu).
