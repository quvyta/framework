## Metotlar

- `Tree::new(kökler)` — en üst seviyedeki `TreeNode`'lar.
- `.selected(Option<&str>)` — seçili düğümün anahtarı.
- `.on_select(|anahtar| mesaj)`, `.on_activate(|anahtar| mesaj)`, `.on_expand(|anahtar, açık| mesaj)`.
- `.empty_text(metin)` — düğüm yokken görünür.
- `.reorderable(|adım| mesaj)` — düğümler kardeşleri arasında taşınabilir; `adım`, `key`, `parent` (en üst seviyede `None`), `from` ve `to` taşıyan bir `TreeMove`'dur. `adım.apply(&mut kardeşler)` öğeyi senin listende taşır.
- `.context_menu(|anahtar| öğeler)` — o anahtarlı düğümün `ContextItem`'ları.
- `.multi_select(&seçili, |anahtarlar| mesaj)` — birden çok düğüm seçilebilir; `seçili` onların anahtarlarıdır, `anahtarlar` her zaman seçimin yeni halinin tamamıdır. `.selected(..)` imleçtir.
- `.droppable(|bırakma| mesaj, |anahtar| kabul)` — sürüklenen düğümler `kabul`ün evet dediği düğümlere bırakılır; `bırakma`, `keys` (ağaçtaki sırasıyla, taşınan başka bir düğümün içindekiler hariç) ve `into` (en üst seviye için `None`) taşıyan bir `TreeDrop`'tur.
- `.on_copy_drop(|bırakma| mesaj)` — Ctrl basılıyken yapılan bırakma, taşıma yerine bununla kopyalama ister; Ctrl'yi fare olayında bildirmeyen terminal her zaman taşır.
- `.activate_on(Click::Single | Click::Double)` — varsayılanı `Single`: tık seçer ve Enter'ın yaptığını yapar. `Double` ile tık yalnızca seçer, aynı satıra `Click::INTERVAL` (400 ms) içinde ikinci basış onu açar, kapatır ya da etkinleştirir; ok işareti, ← ve → yine tek tıkla açıp kapatır.
- `.box_select(bool)` — `multi_select` ile, satırların altındaki boş yerden sürüklemek `text-selection` tonunda bir alan çizer, kapladığı satırlar seçim olur ya da basarken Ctrl basılıysa seçime eklenir; orada tıklamak seçimi bırakır.
- `TreeNode::new(anahtar, etiket)`, `.children(düğümler)`, `.expanded(bool)`, `.expandable(bool)`, `.loading(bool)`.
- `.icon(anahtar, Some(token))`, `.detail(metin)`, `.faint(bool)`.

## Davranış

- Odaktayken tuşlar: `up` `down` ya da `k` `j`, `pgup` `pgdn`, `home` `end` gezinir; `right` ya da `l` açar veya ilk çocuğa geçer; `left` ya da `h` kapatır veya üst düğüme çıkar; `enter` klasörü açıp kapatır, yaprağı etkinleştirir; `space` etkinleştirir.
- Fare: oka tıklama açar ya da kapatır; satıra tıklama önce seçer, sonra `enter` gibi davranır; tekerlek ve kaydırma çubuğu kaydırır.
- Sıralama: bir satıra (okuna değil) basmak onu seçer; imleci bir satır oynatmak sürüklemeye çevirir. İnilecek yer imlecin altında duran kardeş, yoksa ekrandaki en yakın kardeştir; kardeşler bırakmanın vereceği sırayla gösterilir, inilecek yer renklenir ve imleci bir hayalet satır izler. Sürüklenen düğümün çocukları sürükleme boyunca katlanır. Bırakılınca ağaç tek bir `TreeMove` gönderir; başladığı yere bırakılırsa hiçbir şey. Sürükleme olmadan bırakmak satırı `enter` gibi açar, kapatır ya da etkinleştirir. `ctrl+shift+up` ve `ctrl+shift+down` seçili düğümü bir yer taşır, uçlarda durur. Sıralama düğümün ebeveynini hiçbir zaman değiştirmez.
- Üst ya da alt satırda veya ötesinde tutulan sürükleme 400 ms sonra bir satır, sonra her 150 ms'de bir kaydırır; kenardan ne kadar uzaksa o kadar sık.
- Çoklu seçim: düz bir tıklama ya da ok tuşu bir satırı tek seçili satır yapar. `ctrl` + tık bir satırı ekler ya da çıkarır ve imleci oraya taşır; `shift` + tık son düz ya da `ctrl` tıktan bu satıra kadar olanları seçer. `shift+up`, `shift+down`, `shift+pgup`, `shift+pgdn`, `shift+home` ve `shift+end` bu aralığı uzatır; `ctrl+a` gösterilen her satırı seçer; `space` imlecin satırını etkinleştirmek yerine ekler ya da çıkarır; `esc` birden çok seçili düğümü imlecinkine indirir, yoksa üst bileşenlere geçer. Seçili satırların hepsi seçim tonunu alır; çubuğu yalnızca imlecin satırı taşır ve yalnızca o kayar.
- Bırakma: seçili bir satıra basmak seçimi korur ve sürükleme hepsini taşır; başka bir satıra basmak yalnızca onu seçer. İmlecin altındaki satır, `kabul` ona evet diyorsa ve sürüklenen düğümlerden biri, onların içinde ya da hepsinin zaten içinde durduğu düğüm değilse `tree-drop` alır; reddedilen satır `list-item.faint` ile çizilir. Sürüklemenin 400 ms beklediği kapalı düğüm açılır. Son satırın altındaki boş satırlar en üst seviyedir. Kabul eden bir satıra bırakılınca ağaç tek bir `TreeDrop` gönderir; başka yerde hiçbir şey. Sürüklemeden bırakılan tıklama seçimi o satıra indirir.
- `reorderable` ve `droppable` birlikte açıksa tek düğümlü sürükleme `kabul`ün evet dediği satıra bırakılınca içine girer, diğer satırlarda yukarıdaki gibi kardeşleri arasına iner; birden çok düğümlü sürükleme yalnızca bırakır.
- Bağlam menüsü: bir satıra sağ tık, düğümünün menüsünü imleçte açar ve satırı yükseltilmiş tutar; `shift+f10` ya da menü tuşu seçili düğümün menüsünü, onu görünüme kaydırarak satırının altında açar. Bir öğeyi seçmek mesajını gönderir. Satırların altına sağ tık hiçbir şey yapmaz. Çoklu seçimde seçili bir satıra sağ tık seçimi korur; başka bir satırda önce yalnızca o satırı seçer.
- Her seviye iki hücre girintilidir. Yapraklar okun sütununu boş bırakır; aynı seviyedeki adlar alt alta gelir.
- Hover edilen ya da seçili satırda yalnızca ikon ve etiket bir hücre sağa kayar; girinti, ok (ya da yükleme göstergesi) ve detay yerinde kalır. Etiket bir boş hücre payı ayırır; dururken de kayarken de aynı yerden kesilir.

## Tema anahtarları

- `list-item` (`hover`, `selected`, `focus`, `pressed`); `list-item.faint`; `list-detail`; boş metin için `list-header`.
- `tree-chevron` — `fg`; `hover` ve `selected` ile.
- `spinner` — yüklenen düğümün oku; `scrollbar` — `track`, `thumb`.
- `tab-drop` — sürüklenen düğümün ineceği yer; `tab-ghost` — imleci izleyen satır. Menü `ContextItem` anahtarlarını kullanır.
- `tree-drop` — bırakmanın gireceği düğümün `bg`, `fg`, `bold` değerleri; en üst seviyeye bırakılırken boş satırların `bg` değeri.
- İkonlar: `tree-collapsed`, `tree-expanded`, `spinner`.
