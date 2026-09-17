## Metotlar

- `RailTab::new(ad)` — bir sekme; `.icon(anahtar)`, `.badge(metin)`, `.status(değişken)` ikon, sağda silik yazı ve durum noktası ekler.
- `TabRail::new(sekmeler)` — ilk sekmesi açık ray.
- `.active(sıra)` — açık sekme.
- `.on_select(|sıra| msg)` — başka bir sekme açılınca gönderilir.
- `.collapsed(bool)` — ince şerit: çubuk, hiç kaymayan tek hücrelik ikon (yoksa adın ilk harfi; durum rengi ikonu boyar), bir hücre hava ve kaydırma çubuğu sütunu; adlar üstüne gelince. Varsayılan: `false`.
- `.collapsed_marker(işaret)` — kendi işareti olmayan sekmeler için daraltılmış şeridin gösterdiği: `CollapsedMarker::Icon` (ikon, ikon yoksa baş harf), `Initial` ya da `Number` (1'den başlayan sıra; 9'dan sonra `…`). Varsayılan: `Icon`.
- `RailTab::marker(işaret)` — aynı seçim tek bir sekme için; rayınkinden önce gelir.
- `.on_add(|| msg)` — rayın sonuna tıklanınca `msg` gönderen bir ekle (`+`) satırı koyar.
- `.closable(|sıra| msg)`, `.pinned(sıralar)`, `.reorderable(|nereden, nereye| msg)` — `Tabs` ile aynı seçenekler; mesajları `TabEdit::apply` ile uygula.
- `.on_drag_scroll(|ilk| msg)` — sürüklenen sekme rayı her kaydırdığında gönderilir; `ilk` artık görünen ilk satırdır.
- `.row_height(satır)` — her satır bu kadar satır yüksekliğinde bir bloktur (en az bir, üst sınır yok). Varsayılan: `1`.
- `.gap(satır)` — satırlar arasında istenen sayıda boş satır. Varsayılan: `row_height` 1'den büyükse bir satır, değilse hiç; `gap(0)` blokları üst üste dizer.
- `.context_menu(|sıra| öğeler)` — `sıra` numaralı sekme için bir `Vec<ContextItem>`; sekmeye sağ tıklayınca ya da menü tuşuyla açılır, seçilen girdinin mesajı gönderilir. `Tabs` ile aynıdır.

## Davranış

- Her sekme bir satır, `row_height` birden büyükse bir blok. Sağdaki işaretler sağ kenara sabittir: durum noktası, rozet, kapatma işareti. Boşluksuz `row_height(1)` sade rayın birebir aynısını çizer.
- `row_height` 1'den büyükse, `gap` başka bir şey söylemedikçe sekmeler arasında bir boş satır kalır; `gap(0)` blokları üst üste dizer.
- Bir satırdan uzun blok dururken de yükseltilmiştir (`rail-tab.tall`); içeriği orta satırdadır (çift sayıda satırda üstteki orta satır), kapatma işareti ilk satırında sağa yaslıdır (iki satırlık blokta bu satır orta satırdır), çubuğu bütün satırları kaplar. Basma, üstüne gelme, sürükleme ve sağ tık bloğun her yerinde sayılır; boşluk satırları hiçbir sekmeye ait değildir.
- Kaydırma, kaydırma çubuğu ve açık sekmeyi izleme bütün bloklarla sayılır. Bir bloktan kısa ray her bloğu kendi yüksekliğine indirir.
- Ekle satırı, daraltılmış şerit ve daraltılmış rayın ad kartı bir blok yüksekliğindedir; şeridin ikonu, kartın yazısı ve kapatma işareti orta satırdadır.
- Daraltılmış rayda ikon, sekme dururken de, üstüne gelinince de, açıkken de çubuğun hemen sağındaki sütunda kalır; yalnızca çubuk belirir. Geniş ray ikon ve adın bir hücrelik kaymasını korur.
- Sağdaki işaretlere yetmeyecek kadar dar bir rayda çubuğa değecek işaret çizilmez; kapatma işareti de çubuğun yanında yer ister.
- Bir sekmeye ya da ad kartına sağ tık, o sekmenin menüsünü imlecin yanında açar ve sekmeyi açmaz; menü açıkken ad kartı gizlenir, başka bir sekmeye sağ tık menüyü oraya taşır, yanına sol tık menüyü kapatıp bastığı şeyi yapar, Esc kapatır. Menü tuşu ve shift+F10 açık sekmenin menüsünü satırının altında, ilk girdi seçili açar. Menünün sekmesi ortadan kalkarsa menü kapanır. `context_menu` yoksa sağ tık ve menü tuşu hiçbir şey yapmaz.
- Daraltılmış ray 4 sütun ölçer ve son sütununu her zaman kaydırma çubuğuna ayırır, böylece sekmeler taşınca genişliği değişmez; ad kartı üstüne gelinen satırın (ray klavyeyle odaklıyken açık satırın) yanında çizilir, imleç kartın üstündeyken kalır, tıklanınca sekmeyi açar ve sekme kapatılabiliyorsa kapatma işaretiyle biter.
- Ekle satırı bir sekme değildir: seçilmez, kapatılmaz, taşınmaz.
- Sekmeler sığmayınca ray açık sekme değiştiğinde onu izler, tekerlekle kayar ve son sütununda kaydırma çubuğu çizer.
- Sürükleme imleç bir satır kayınca başlar; hedef, imlecin altındaki yerinde duran bloktur.
- Satırlar sığmıyorsa, görünen ilk ya da son bloğun (sonuncunun altındaki satırlar dahil) üstünde veya rayın üst ya da alt ucunun ötesinde tutulan sürüklenen sekme rayı bir blok kaydırır: ilk adım 400 ms sonra, sonra her 150 ms'de bir; kenarın ötesindeki her satır adımı 30 ms öne çeker, en kısası 60 ms. Kayabildiği sürece kaydırma çubuğu aydınlanır. Uçtan ayrılınca kayma hemen durur, tekrar gelince yine 400 ms beklenir; ray uçlarında durur. Tek blok gösteren ray yalnızca kenarlarının ötesinde kayar.

## Tuşlar

- `up` `down` (ya da `k` `j`) komşuyu açar, `home` `end` ilk ve son sekmeyi, `ctrl w` kapatır, `ctrl shift up` / `down` açık sekmeyi taşır, `menu` ya da `shift f10` açık sekmenin bağlam menüsünü açar.

## Fare

- Açmak için sekmeye tıkla, kapatmak için `×` işaretine tıkla ya da orta tıkla, sıralamak için sürükle (kaydırmak için uç satırda tut), menüsü için sağ tıkla, kaydırmak için tekerleği çevir ya da kaydırma çubuğuna tıklayıp sürükle. Daraltılmış rayda ad kartı aynı tıklamaları alır.

## Tema anahtarları

- `rail-tab` — `bg`, `fg`, `bold`, `pillar`; `hover`, `selected`, `focus` durumları; sürüklenen sekme için `ghost` varyantı; bir satırdan uzun bloklar için `tall` varyantı (`hover` ve `selected` ile).
- `rail-badge` — `fg`; `selected`.
- `rail-hint` — daraltılmış rayın yanındaki ad kartının `bg`, `fg` değerleri; imleç altındayken `hover`.
- `rail-add` — ekle satırının `bg`, `fg`, `pillar` değerleri; `hover`; `tall` varyantı.
- `close-mark`, `tab-ghost`, `tab-drop` — `Tabs` ile ortak; `scrollbar`.
- `quvyta.tab-rail.add` — ekle satırının ve onun ad kartının yazısı.
- `context-menu`, `context-item` (`hover`, `disabled`, `danger`), `context-item-shortcut`, `context-item-chevron` — bağlam menüsü.

## İkonlar

- `pillar`, `close`, durum için `dot`, ekle satırı için `add` ve sekmelerde verdiğin ikonlar.
