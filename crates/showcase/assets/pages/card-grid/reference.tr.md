## CardGrid

- `CardGrid::new(sayı)` — `sayı` kartlık bir ızgara; kartlar 24 ile 32 hücre geniş, üç satır içerikli, aralarında iki hücre ve bir satır boşluk.
- `.card(|ui, sıra| ..)` — `sıra` numaralı kartın gösterdiklerini kurar; ızgara çizilirken yalnızca ekrandaki kartlar için çağrılır. Kapatma `'static`'tir.
- `.card_width(en_az, en_fazla)` — bir kartın alabileceği en az ve en fazla genişlik; sütun sayısı `en_az`'dan çıkar.
- `.card_height(satır)` — her kartın iç boşluğu hariç içerik satırı.
- `.gap(sütun, satır)` — bir satırdaki kartlar arasındaki hücre, kart satırları arasındaki satır.
- `.selected(Option<usize>)` — seçili kart.
- `.on_select(|sıra| msg)` — seçim değişti.
- `.on_activate(|sıra| msg)` — bir kart Enter ya da tıklamayla açıldı.
- `.checked(Vec<bool>)` ve `.on_toggle(|sıra| msg)` — işaretler; varsayılan olarak kapalı.
- `.activate_on(Click::Single | Click::Double)` — varsayılanı `Single`: tık seçer ve açar. `Double` ile tık yalnızca seçer, aynı karta `Click::INTERVAL` (400 ms) içinde ikinci basış onu açar; Enter her iki durumda da açar.
- `.multi_select(&seçili, |kartlar| mesaj)` — birden çok kart birlikte seçilir: Ctrl+tık kartı ekler ya da çıkarır, Shift+tık okuma sırasında aralık seçer, Shift ile oklar, `pgup` `pgdn`, `home` ya da `end` onu uzatır, `ctrl+a` her kartı seçer, Space tuşların üstünde durduğu kartı ekler ya da çıkarır, `esc` birden çok seçili kartı onunkine indirir; `kartlar` her zaman seçimin yeni halinin tamamıdır. Seçili kartlar seçili yüzeyi alır.
- `.box_select(bool)` — `multi_select` ile, kartların arasındaki ve sonrasındaki boş yerden sürüklemek `text-selection` tonunda bir alan çizer, değdiği her kart seçim olur ya da basarken Ctrl basılıysa seçime eklenir; orada tıklamak seçimi bırakır.
- `.droppable(|bırakma| mesaj, |sıra| kabul)` — basılan kart ya da içinde olduğu seçim, `kabul`ün evet dediği bir kartın üstüne sürüklenir; o kart `tree-drop` tonunu alır. `bırakma` bir `RowDrop { rows, into }`'dur. Başka bir yerde bırakmak hiçbir şey yapmaz.
- `.on_copy_drop(|bırakma| mesaj)` — Ctrl basılıyken yapılan bırakma, taşıma yerine bununla kopyalama ister.
- `.empty(EmptyState)` — boş ızgaranın gösterdiği.
- `.context_menu(|index| Vec<ContextItem<Msg>>)` — her karta kendi menüsünü verir; menü, açıldığı kart için kurulur.
- `.disabled(bool)` — hover, odak ve basma yok; kartlar solar, seçim görünmeye devam eder.
- `.scrollbar(ScrollbarStyle)` — temanın yerine sabit bir kaydırma çubuğu stili kullanır.

## Tuşlar

- Oklar kartlar arasında gezer ve kenarda durur; `home` `end` ilk ve son karta gider; `pgup` `pgdn` sığan satır kadar atlar; `enter` açar; `space` işaretler açıkken işaretler, değilse açar.

## Fare

- Kart menüsü varken: bir karta sağ tıklamak o kartın menüsünü imlecin yanında açar ve kart işaretli değilse onu seçim yapar; menü tuşu ya da `shift f10` tuşların üzerinde olduğu kartın menüsünü açar, önce kartı görünür kılar. O kart menü açıkken yükselmiş kalır.
- Tıklamak kartı seçer ve açar; kartın sağ üst köşesindeki işarete (ya da yanındaki bir hücreye) tıklamak yalnızca işaretler; tekerlek bir sıra kart kaydırır; kaydırma çubuğu sürüklenir.

## Yerleşim

- Sütunlar: `(genişlik + boşluk) / (en_az + boşluk)`, en az bir. Her kart `(genişlik - boşluklar) / sütunlar` genişliğindedir, en fazla `en_fazla`.
- `en_az`'dan dar bir alan, alan kadar geniş tek sütun gösterir.
- Kaydırma çubuğu son sütunu yalnızca satırlar taşınca alır; tekerlek ve kaydırma çubuğu kart satırı sayar.

## Tema anahtarları

- `card`; `hover`, `selected`, `focus`, `pressed` durumlarıyla — `bg`, `padding`, `pillar`.
- `card-mark` — işaretli kartın işareti; `card-mark.off` — işaretler açıkken yanan kartın gösterdiği soluk işaret.
- `scrollbar` (`style`, `track`, `thumb`) ve `scrollbar.<stil>`.
- `[icons]` — `check`: işaret (`✓`, ASCII'de `v`); `pillar`.
