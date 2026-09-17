## Ne zaman kullanılır

Her biri ekranı dolduran birkaç açık çalışma alanı arasında geçiş için dikey sekmeler kullan: bir IDE'deki projeler, bir gösterge panelindeki kümeler, bir veritabanı istemcisindeki bağlantılar. Tek bir şeyin görünümleri için yatay `Tabs`, çok sayıda sayfada gezinmek için menü daha uygundur.

## Adım adım

1. Her çalışma alanı için bir `RailTab` kur: `RailTab::new("quvyta").icon("folder")`.
2. Bir bakışta görülmesi gerekeni ekle: üç çalışan container için `.status("success").badge("3")`.
3. Rayı açık sekmeyle göster: `TabRail::new(tabs).active(self.active).on_select(Msg::Open)`.
4. Bir genişlik ve eldeki yüksekliği ver: `.width(Length::Cells(24)).fill_height()`.
5. İhtiyacın olanı aç: ince bir ikon şeridi için `.collapsed(true)`, `+` satırı için `.on_add(..)`; `.closable(..)` ve `.reorderable(..)` tıpkı `Tabs`'taki gibi çalışır ve `TabEdit::apply` ile uygulanır.
6. Sık tıklanan proje sekmelerini kalınlaştır: `.row_height(3)` her sekmeyi üç satırlık bir bloğa çevirir, bloklar arasında bir boş satır kalır. Her yükseklik olur; başka bir aralık için `.gap(n)` ver, `.gap(0)` blokları üst üste dizer. İkisini de oyun alanındaki sayı alanlarıyla dene.
7. Daraltılmış şeridin ne göstereceğini seç: sıra numarası için `.collapsed_marker(CollapsedMarker::Number)`, harf için `Initial`, ikon için `Icon` (varsayılan). Editörlerin etkinlik çubuğundaki gibi kullanıcı sekme sekme seçebilsin: seçimini durumunda tutup `RailTab::marker(..)` olarak ver ya da sekmenin `.icon(..)` değerini değiştir.
8. Sekmelere sağ tık menüsü ver: `.context_menu(|index| vec![ContextItem::new("Diğerlerini kapat", Msg::CloseOthers(index)), ..])`. Girdileri durumundan kur, işe yaramayacak olanları pasif yap, mesajlarını da `update` içinde her mesaj gibi işle.

## Nasıl çalışır

- **Kutu değil satır.** Varsayılan olarak her sekme bir satırdır. Açık sekme vurgu çubuğuyla yükselir; ray klavyeyle odaklıyken çubuk nefes alır. Üstüne gelinen sekme hafifçe yükselir. Açık sekmenin ve üstüne gelinen sekmenin ikonu ile adı bir hücre kayar; durum noktası, rozet ve kapatma işareti yerinde kalır.
- **Kalın bloklar.** `row_height(n)` birden büyükse her sekme `n` satırlık, yükseltilmiş bir bloktur; yine çerçeve yoktur, şekli ton verir. İkon, ad, durum ve rozet orta satırda durur; kapatma işareti, pencerelerdeki gibi bloğun sağ üst köşesindedir; üstüne gelinen ya da açık sekmenin çubuğu bloğun tamamı boyunca iner. `row_height` 1'den büyükse, `gap` başka bir şey söylemedikçe sekmeler arasında bir boş satır kalır; `gap(0)` blokları üst üste dizer. Bloğun her satırı sekmenin kendisidir: üstüne gelme, tıklama, orta tık, sürükleme ve sağ tık, kapatma işaretinin kendisi dışında her yerinde çalışır. İki satırlık blokta ilk satır aynı zamanda orta satırdır; işaret, tek satırlık sekmedeki gibi adın satırının sonunda durur. Ekle satırı ve daraltılmış şerit aynı yüksekliği korur, daraltılmış rayın ad kartı da bloğu kadar uzundur. Bir bloktan kısa ray yarım sekme göstermez, blokları kendi boyuna indirir.
- **Sağ tık menüsü.** `context_menu` verildiyse bir sekmeye (ya da daraltılmış rayda ad kartına) sağ tıklamak, sekmeyi açmadan o sekmenin menüsünü imlecin yanında açar; menü tuşu ya da shift+F10 açık sekmenin menüsünü onun altında, ilk girdi seçili olarak açar. Menü her yerdeki `ContextMenu` ile aynıdır: oklar, harf yazma, Enter, Esc; menünün dışına tıklamak onu kapatır ve tıklanan şeyi de yapar. Seçenek yoksa sağ tık hiçbir şey yapmaz.
- **İşaretler.** Daraltılmış sekme tek hücre gösterir: ikonunu (yoksa baş harfini), baş harfini ya da 1'den başlayan sırasını. Baş harf aksanlarını korur, tek karakter kalıyorsa büyük harfe döner. 9'dan sonraki sıralar, başka bir sekmeyi gösterecek bir rakam yerine `…` olur; ad kartı yine sekmenin adını söyler. Sekmenin kendi `marker` değeri rayın `collapsed_marker` seçiminden önce gelir; numara, sekme taşınınca onunla birlikte değişir. Demodaki sekme menüsünün İşaret alt menüsü bir projenin ikonunu, harfini ya da numarasını seçer veya rayın seçimine döndürür.
- **Durumun bir işareti olur.** Durum rengi her zaman bir noktadır; rozet yanında duran silik bir yazıdır.
- **Daraltılmış ray.** Ray ikonlara iner; durum rengi ikonu boyar. İkon hiç kaymaz: üstüne gelinen ya da açık sekmede çubuk ikonun hemen solundaki hücrede belirir; şerit genişlemiş gibi görünmez, ikon kaydırma çubuğu sütununa yaklaşmaz. İkonu olmayan sekme adının ilk harfini gösterir. Bir sekmenin üstüne gelince rayın yanında adını ve rozetini gösteren bir ad kartı belirir. Kart rayın bir parçasıdır: imleci üstüne götürünce kaybolmaz, imleç altındayken yükselir, tıklayınca sekmeyi açar ve kapatma açıksa orta satırında sekmenin kapatma işaretiyle biter. İmleç hem satırdan hem karttan çıkınca kaybolur. Ray klavyeyle odaklıyken kart açık sekmenin adını gösterir; ikonlar tahmin oyununa dönmez.
- **Tabs ile aynı model.** Kapatma (silik `×`, orta tık, ctrl+w), sabitleme ve sürükleme `Tabs` ile birebir aynıdır; sürüklerken diğer sekmeler kayar ve sekmenin bırakılacağı yerde renkli bir boşluk açılır. ctrl+shift+↑ ve ↓ açık sekmeyi taşır.
- **Çok sekme.** Satırlar sığmayınca ray açık sekmeyi görünür tutar ve kaydırma çubuğu gösterir; tekerlek kaydırır, kaydırma çubuğuna tıklamak ya da onu sürüklemek doğrudan oraya götürür.
- **Gizli satırlara sürükleme.** Sürüklediğin sekmeyi görünen ilk ya da son satırın üstünde, ya da rayın üst veya alt ucunun ötesinde tut: kaydırma çubuğu aydınlanır, 400 ms sonra ray bir satır kayar (kalın satırlarda bütün bir blok), ardından her 150 ms'de bir satır daha; kenarın ne kadar ötesine çekersen o kadar hızlı. Renkli boşluk da birlikte ilerler, sekme gördüğün yere düşer. Ortaya dönünce kayma hemen durur; ray ilk ve son satırında da durur. Bekleme sayesinde bırakmaya giderken uç satırın üstünden geçmek hiçbir şeyi kaydırmaz.
- **Kalın bloklarla kaydırma.** Kalın bloklar her seferinde bütün bir blok kayar, hiçbir zaman yarıda durmaz.
- **Fare.** Açmak için tıklama, kapatmak için `×` ya da orta tık, sıralamak için sürükleme, menü için sağ tık, kaydırmak için tekerlek ya da kaydırma çubuğu; daraltılmış rayda aynıları ad kartında da çalışır.

## Sık yapılan hatalar

- **Rayı sayfa menüsü gibi kullanmak.** Sekmeler açılıp kapanan şeylerdir; menü her zaman orada olan yerleri listeler.
- **Noktasız renk.** Durumu göstermek için adı boyama; `.status(..)` kullan.
- **Dar rayda uzun adlar.** Adlar `…` ile kesilir; raya yer aç ya da onu daralt.
- **Kısa rayda kalın bloklar.** Üç satırlık bir blok, arkasındaki boş satırla birlikte dört satır tutar; beklediğin sekme sayısına yetecek yükseklik ver ya da tek satırlık sekmelerde kal. Bir bloktan kısa ray da çalışır, yalnızca bir seferde kesilmiş tek blok gösterir.
- **Çok sekmede numara.** Dokuzdan fazla sekmede numaralar `…` olur; daha fazlasını bekliyorsan ikon ya da harf kullan.
- **Hiçbir şey yapmayan menü girdileri.** Kapatacak başka sekme yokken "Diğerlerini kapat" sessizce yok sayılmamalı, pasif görünmeli.
