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
- `.empty(EmptyState)` — boş ızgaranın gösterdiği.
- `.disabled(bool)` — hover, odak ve basma yok; kartlar solar, seçim görünmeye devam eder.
- `.scrollbar(ScrollbarStyle)` — temanın yerine sabit bir kaydırma çubuğu stili kullanır.

## Tuşlar

- Oklar kartlar arasında gezer ve kenarda durur; `home` `end` ilk ve son karta gider; `pgup` `pgdn` sığan satır kadar atlar; `enter` açar; `space` işaretler açıkken işaretler, değilse açar.

## Fare

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
