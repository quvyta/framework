## Metotlar

- `.closable(|index| msg)` — kapatma işaretleri, orta tık ve ctrl+w; mesaj `index` sekmesinin kapatılmasını ister.
- `.pinned(indices)` — kapatılamayan, işaret göstermeyen sekmeler.
- `.tab_width(TabWidth)` — `Fit` (varsayılan), `Fixed(hücre)` ya da `Fill`.
- `.overflow(Overflow)` — `Arrows` (varsayılan) ya da `Menu`.
- `.reorderable(|from, to| msg)` — sürükleyerek ve klavyeyle sıralama; `to` sekmenin taşındıktan sonraki sırasıdır.
- `.on_drag_scroll(|first| msg)` — sürüklenen sekme şeridi her kaydırdığında gönderilir; `first` artık görünen ilk sekmenin sırasıdır.
- `.context_menu(|index| items)` — `index` sekmesinin `Vec<ContextItem>` listesi; sekmeye sağ tık ya da menü tuşu açar, seçilen girdinin mesajı gönderilir.
- `.on_add(|| msg)` — son sekmenin hemen ardında (sekmeler gizlenirken şeridin sağ ucunda, hiç sekme yokken başında) duran, basılınca `msg` gönderen bir `+` düğmesi.
- `TabEdit::Close(index)`, `TabEdit::Move { from, to }` — `.apply(&mut tabs, &mut active)` listeni düzenler, açık sekmeyi korur.
- Sade metotlar aynen kalır: `Tabs::new`, `.active`, `.numbered`, `.on_select`.

## Davranış

- `apply` ile açık sekme kapanınca yerine geçen sekme, sondaysa yeni son sekme açılır.
- `Fixed(n)` etikete bir hücreden az yer bırakacak kadar küçülmez; o hücrede uzun etiketten yalnızca `…` görünür; `Fill` 12 hücrede ya da etiketin kendi genişliğinde küçülmeyi bırakır.
- `Arrows` ile şerit açık sekmeyi yalnızca o değiştiğinde izler; böylece oklarla serbestçe gezilebilir. Gösterecek sekmesi kalmayan ok zemine iner ve basışları yok sayar. Sekmeler kapanınca kalanlar sığdığı anda şerit geri kayar.
- Denetimlerine yer kalmayacak kadar dar şerit açık sekmeyi kısaltır; hiçbir sekmeye yer kalmayan menülü şerit bütün sekmeleri listeler ve açık olanı işaretler.
- `Menu` yalnızca gizli sekmeleri listeler; ↑ ↓ Home End gezer, harf yazmak atlar, Enter seçer, Esc ya da başka yere tık kapatır.
- Sürükleme imleç iki hücre kayınca başlar. Hedef, imlecin altındaki yerinde duran sekmedir; önizleme bu yüzden titremez.
- `Arrows` ile gizli sekme varken bir okun üstünde ya da şeridin iki ucundan birinin ötesinde tutulan sürüklenen sekme şeridi kaydırır: ilk adım 400 ms sonra, sonra her 150 ms'de bir; şeridin kenarının ötesindeki her hücre adımı 30 ms öne çeker, en kısası 60 ms. Ok üstüne gelinmiş tonunu gösterir ve her adımda parlar; oktan ayrılınca kayma hemen durur, tekrar gelince yine 400 ms beklenir. Sonda ok zemine iner ve adımlar durur. Oka bırakılan sekme görünen en yakın sekmenin yerine düşer. `Menu` şeridi böyle kaymaz: önce gizli sekmeyi aç.
- `on_add` düğmesi üç hücredir: bir boşluk, `+` ve bir boşluk; son sekmeden bir boşluk sonra gelir. Sekmeler gizlenirken sekmeler, oklar ve menü düğmesi onun yeri düşülerek yerleşir, düğme şeridin sağ ucunda durur; `Fill` sekmeleri şeridi düğmenin yeri dışında paylaşır. Sekmelerdeyken Tab ona odaklanır, bir sonraki Tab şeritten çıkar; shift+Tab ya da ← geri döner, başka bir tuş sekmelere döner ve orada işler. Enter, Space ve sol tık basar, düğme bir ton parlar. İpucu imleç altında `motion.hover-delay` sonra, klavyeyle gelince hemen görünür. Üstüne bırakılan sekme, son sekmeler görünmüyor olsa bile en sona gider. Şerit düğmedeki odağı yalnızca kendisi odaktayken hatırlar; geri gelince odak sekmelerden başlar.
- `context_menu` açıkken bir sekmeye (kapatma işareti dahil) sağ tık o sekmenin menüsünü imlecin yanında açar ve sekme açmaz; başka bir sekmeye sağ tık menüyü taşır, yanına sol tık menüyü kapatıp bastığı şeyi yapar, Esc kapatır. Menü tuşu ve shift+F10 açık sekmenin menüsünü altında, ilk girdi seçili açar. İki menüden biri açılınca diğeri kapanır. Menünün sekmesi ortadan kalkarsa menü kapanır. Seçenek yoksa sağ tık, menü tuşu ve shift+F10 hiçbir şey yapmaz.

## Tuşlar

- `ctrl w` kapatır, `ctrl shift left` / `right` açık sekmeyi taşır, gizli sekme varken `ctrl pgup` / `pgdn` kaydırır, `down` gizli sekmeler menüsünü, `menu` ya da `shift f10` açık sekmenin bağlam menüsünü açar. `on_add` açıkken `tab` sekmelerden `+` düğmesine, oradan şeridin dışına geçer, `shift tab` ya da `left` geri döner, `enter` ya da `space` basar.

## Fare

- Kapatmak için `×` işaretine tıkla ya da sekmeye orta tıkla; taşımak için sürükle, gizli sekmelere ulaşmak için okun üstünde tut; menüsü için sekmeye sağ tıkla; kaydırmak için oklara tıkla ya da tekerleği çevir; gizli sekmeler için sayıya tıkla; yeni sekme istemek için `+` düğmesine tıkla ya da sürüklediğin sekmeyi sona taşımak için üstüne bırak. Menü açıkken bir sekmeye tık menüyü kapatıp sekmeyi açar, sayıya tık yalnızca menüyü kapatır.

## Tema anahtarları

- `close-mark` — kapatma işaretinin üç hücresi: `fg`, `bg`, `bold`; sekmenin üstüne gelinince ya da sekme açıkken `active`, imleç üstündeyken `hover` (üç hücre birlikte aydınlanır).
- Birlikte `active` ve `hover` ile `close-mark` — yükselmiş sekmede aydınlanan işaret, açık sekmenin tonundan bir kademe yukarıda.
- `tab-arrow` — `bg`, `fg`, `pillar`; `hover`, `pressed`, `disabled`.
- `tab-menu` — `bg`, `fg`, `pillar`; `hover`, açıkken `active`.
- `tab-add` — ekleme düğmesinin `bg`, `fg`, `pillar` değerleri; `hover`, `focus` (çubuk nefes alır), `pressed`. İpucu için `tooltip`; ipucunun sözleri `quvyta.tabs.add` dil anahtarıdır.
- `tab-ghost` — sürüklenen sekmenin `bg`, `fg`, `bold` değerleri; `tab-drop` — bırakılacağı boşluğun `bg` değeri.
- `popup-menu`, `popup-item` (`hover`, `checked`), `popup-check` — gizli sekmeler menüsü.
- `context-menu`, `context-item` (`hover`, `disabled`, `danger`), `context-item-shortcut`, `context-item-chevron` — bağlam menüsü.

## İkonlar

- İşaret için `close`, oklar için `chevron-left` / `chevron-right`, menü düğmesi için `chevron-down`, ekleme düğmesi için `add`, menü bütün sekmeleri listelediğinde açık sekme için `check`.
