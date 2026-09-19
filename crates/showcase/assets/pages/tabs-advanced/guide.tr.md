## Ne zaman kullanılır

Sekmeler kullanıcının açıp kapattığı şeyleri temsil ediyorsa bu seçenekleri kullan: editördeki dosyalar, terminaller, açık sorgular. Genel bakış ve Ayarlar gibi sabit görünümler için sade `Tabs` yeterlidir; yalnızca sekmelerinin ihtiyaç duyduğu seçenekleri ekle.

## Adım adım

1. Sekmeleri durumunda bir liste, açık olanı bir sıra numarası olarak tut.
2. Sade başla: `Tabs::new(labels).active(self.active).on_select(Msg::Open)`.
3. Kapatmaya izin ver: `.closable(|i| Msg::Edit(TabEdit::Close(i)))`. Başlangıç sayfası açık kalsın istiyorsan `.pinned([0])` ekle.
4. Sıralamaya izin ver: `.reorderable(|from, to| Msg::Edit(TabEdit::Move { from, to }))`.
5. İkisini de `update` içinde tek satırla uygula: `edit.apply(&mut self.tabs, &mut self.active)`. Sekmeyi siler ya da taşır; açık sekme de aynı sekmeyi göstermeye devam eder.
6. Genişliği `.tab_width(TabWidth::Fixed(16))` ya da `TabWidth::Fill` ile, sekmeler sığmayınca ne olacağını `.overflow(Overflow::Arrows)` ya da `Overflow::Menu` ile seç.
7. `.context_menu(|index| items)` ile sağ tık menüsü ekle: `index` numaralı sekme için `ContextItem` listesini döndür (Kapat, Diğerlerini kapat, Sağdakileri kapat, Sabitle, Çoğalt) ve mesajlarını `update` içinde işle.
8. `.on_add(|| Msg::NewTab)` ile yeni sekme açma yolu sun: son sekmenin hemen ardında bir `+` durur; yeni sekmeyi `update` içinde listeye ekle ve aç.

## Nasıl çalışır

- **Her seçenek bağımsızdır.** Açılmayan seçenekten iz kalmaz: kapatma işareti, ok, fazladan tuş yoktur. İstediğin birleşimi aç.
- **Kapatma.** Her sekmenin sonunda silik bir `×` durur. Açık sekmede ve üstüne gelinen sekmede belirginleşir, imleç işaretin tam üstündeyken daha da parlar. İşarete tıklamak, sekmenin herhangi bir yerine orta tıklamak ya da odaklı şeritte ctrl+w kapatır. Sabitlenmiş sekmelerde işaret yoktur ve üçü de işlemez.
- **Genişlik.** `Fit` etikete uyar. `Fixed(n)` her sekmeyi n hücre yapar, uzun etiketi `…` ile keser. `Fill` şeridi eşit paylaştırır ve okunur bir alt sınırda küçülmeyi bırakır; sonrasında şerit taşar.
- **Taşma.** Varsayılan `Arrows` iki uca, hiçbir şey açmadan bir sekme kaydıran birer ok düğmesi koyar; ctrl+PgUp, ctrl+PgDn ve fare tekerleği de aynısını yapar. `Menu` sona gizli sekme sayısını gösteren bir düğme koyar; tıklayınca ya da ↓ ile gizli sekmeler listelenir. İki denetim de küçük birer düğmedir: üstüne gelince aydınlanır ve ilk hücresinde vurgu çubuğu belirir, ok basınca parlar, menü düğmesi de menü açıkken sabit bir çubukla aydınlık kalır.
- **Fare.** Tuşların yaptığı her şeyin fareyle bir yolu vardır: açmak için sekmeye, kaydırmak için oka ya da tekerleğe, gizli sekmeler için sayıya tıklama; kapatmak için `×` işareti ya da orta tık; sıralamak için sürükleme. Menü açıkken bir sekmeye tek tıklama menüyü kapatır ve sekmeyi açar; düğmeye tıklamak yalnızca menüyü kapatır.
- **Sıralama.** Bir sekmeyi sürükle: imlecin altında hayalet olarak süzülür, diğer sekmeler bırakılacağı yerdeki renkli boşluğa yer açar. Bırakınca `from` ve `to` gönderilir. Klavyede ctrl+shift+← ve → açık sekmeyi taşır.
- **Ekranın ötesine sürükleme.** Gizli sekme varken sürüklediğin sekmeyi bir okun üstünde (ya da şeridin ucunun ötesinde) tut: ok aydınlanır, 400 ms sonra şerit bir sekme kayar, ardından her 150 ms'de bir sekme daha; ucun ne kadar ötesine çekersen o kadar hızlı. Renkli boşluk görünür hale gelen sekmelerle birlikte ilerler; bıraktığında sekme tam boşluğun olduğu yere düşer. Oktan ayrılınca kayma hemen durur; son sekmeye gelince ok zemine iner ve başka bir şey olmaz. Bekleme sayesinde bırakmaya giderken okun üstünden geçmek şeridi kaydırmaz. Hareketi azaltmak burada bir şey değiştirmez, çünkü her adım zaten bir sıçramadır. `.on_drag_scroll(|first| msg)` her adımı bildirir; bu sayfa onları olay günlüğüne yazar.
- **Sağ tık menüsü.** Bir sekmeye sağ tıklamak o sekmenin menüsünü imlecin yanında açar, açık sekmeye dokunmaz; menü tuşu ya da shift+F10 açık sekmenin menüsünü onun altında açar. Girdiler senindir: sabitlenmiş sekmede "Sabitlemeyi kaldır" yazabilir, son sekmede "Sağdakileri kapat" pasif görünebilir. Başka bir sekmeye sağ tıklamak menüyü oraya taşır; menünün dışına tek sol tıklama menüyü kapatır ve tıklanan şeyi yapar. Gizli sekmeler menüsüyle sağ tık menüsü hiçbir zaman birlikte görünmez. Seçenek yoksa sağ tık hiçbir şey yapmaz.
- **Ekleme düğmesi.** `on_add` açıkken küçük bir `+` düğmesi son sekmenin bir boşluk ardından gelir; sekmeler açılıp kapandıkça onunla birlikte yer değiştirir. Sekmeler artık sığmayınca şeridin sağ ucunda, okların ya da menü düğmesinin ardında yerinde kalır; sekmeler ve oklar ona yer bırakır. Hiç sekme yokken şeridin başında durur, böylece boş şerit de bir sekme açmanın yolunu gösterir. Oklar gibi imleç üstündeyken aydınlanır ve vurgu çubuğunu gösterir. Tab klavye odağını sekmelerden ona taşır, orada çubuk nefes alır; Enter ya da Space basar, shift+Tab ya da ← sekmelere döner, bir sonraki Tab şeritten çıkar. Üstünde bir süre durunca ya da klavyeyle gelince altında kısa bir ipucu belirir. Üstüne bırakılan sürüklenen sekme sona gider.
- **Tek model.** `TabRail` aynı seçenekleri ve aynı davranışı dikey yerleşimle kullanır.

## Sık yapılan hatalar

- **Listeyi değiştirip açık sekme numarasını güncellememek.** `TabEdit::apply` kullan; açık sekme kapanınca ilk sekmeye atlamak yerine komşusu açılmalıdır.
- **Sabit görünümleri kapatılabilir yapmak.** Geri getirilemeyen bir sekme kapatılmamalı; ya kapatmayı açma ya da o sekmeyi sabitle.
- **Her zaman sığan şeritte oklara yer ayırmak.** Oklar yalnızca gerektiğinde çıkar; onlar için yer ayırma.
- **`+` düğmesini şeridin yanına koymak.** Aynı satırdaki bir düğme sekmeler taşınca satırdan düşer ya da şerit satırı doldurunca en sağa yaslanır. `on_add` onu her durumda son sekmenin hemen ardında ve şeridin üstünde tutar.
- **Menünün işi kendisinin yapacağını sanmak.** Sağ tık menüsü senin mesajlarını gönderir; kendiliğinden kapatmaz ya da sabitlemez. Menüden sabitlemek için sabitlenen sekmeleri durumunda tutup `.pinned(..)` ile ver.
