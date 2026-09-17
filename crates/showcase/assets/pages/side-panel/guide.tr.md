## Ne zaman kullanılır

Kullanıcının gerektiğinde açtığı, yer açmak için kapattığı ikincil bir alan için yan panel kullan: proje gezgini, arama görünümü, ana hatlar. İki alan da eşit önemdeyse ve yalnızca boyutları değişiyorsa bölücü kullan. Birden fazla görünüm varsa ona, VS Code gibi editörlerdeki etkinlik çubuğuna benzeyen bir ikon şeridi ver.

## Adım adım

1. Panelin durumunu uygulamanda tut: `open: bool`, `width: u16` ve birden fazla görünüm varsa `view: u16`.
2. Kur: `SidePanel::new(state.width).open(state.open).panel(|ui| ...).body(|ui| ...).show(ui)`.
3. Gerekirse sağa yerleştir: `.side(Side::Right)`.
4. Açılıp kapanabilsin: `.on_toggle(|açık| Msg::Panel(açık))`.
5. Boyutlanabilsin: `.on_resize(|genişlik| Msg::PanelWidth(genişlik))`. Sınır vermezsen panel, gövdeye alanın dörtte biri kalana kadar büyüyebilir; kendi sınırlarını `.limits(18, 48)` ile, yalnızca en küçük genişliği `.limits(18, None)` ile seç.
6. Birden fazla görünüme ikon şeridi ver: `.strip(["folder", "search"], |sıra| Msg::ShowView(sıra)).active_view(state.view)`. `update` içinde `ShowView(sıra)` `view = sıra` ve `open = true` yapar; başka bir şey gerekmez, bir tıklamanın ne zaman kapatacağına şerit karar verir.
7. Kapanınca ne kalacağını seç: `.closed(Closed::Collapse)` (varsayılan) şeridi bırakır, `.closed(Closed::Hide)` yalnızca kenarı bırakır.

## Nasıl çalışır

- **Kenar bir ton farkıdır.** Panel gövdenin yanında kendi yüzeyinde durur; aralarına hiçbir şey çizilmez. Kenara gelince o sütun bir kademe aydınlanır ve ortasında iki hücrelik bir açma kapama düğmesi yükselir: yükseltilmiş bir zeminde bir `‹` ya da `›`; ilk hücre düğmenin tonunda boş kalır. Düğme gövdeye bir hücre taşar, panelin kendi içeriğine dokunmaz. Düğmenin kendisine gelince bir kademe parlar ve ilk hücrede vurgu çubuğu `▌` belirir, basınca bir kademe daha parlar; tıklamak paneli açar ya da kapatır. Kenarın geri kalanını sürüklemek, vurgu rengiyle boyutlar; düğmede başlayan basış hiçbir zaman boyutlamaz.
- **Klavye.** Tab kenara ulaşır ve aynı düğmeyi, basılabilen her şeyin klavye odağını gösterdiği gibi, nefes alan çubukla gösterir: Enter ya da Boşluk açıp kapatır ve düğme bir kademe parlayıp söner, ← → boyutlar. Genel `toggle-panel` eylemi (`alt+b`) panelin ya da gövdesinin içindeki her yerden açıp kapatır; her yerde çalışan bir kısayol için kendi uygulama eylemini bağla.
- **Kayar.** Açılış ve kapanış paneli `motion.enter` süresinin iki katında hücre hücre taşır; içerik genişliğini korur, yeniden dizilmek yerine kenardan kayarak çıkar. Hareket azaltılmışsa bir anda değişir.
- **Şerit bir etkinlik çubuğudur.** Panel açık da olsa kapalı da dış kenarda durur; açık panel onunla gövdenin arasındadır. Gösterilen görünümün ikonu sabit bir vurgu çubuğu `▌` ile yükselir; başka bir ikona gelince o bir kademe yükselir. Başka bir ikona tıklamak açık paneli o görünüme geçirir, gösterilen görünümün ikonuna tıklamak paneli kapatır, panel kapalıyken her ikon paneli kendi görünümüyle açar. Tab şeride kenardan sonra ulaşır: ↑ ↓ gezer, Enter ya da Boşluk tıklama gibi çalışır, klavyenin üstünde durduğu ikonun çubuğu nefes alır.
- **Küçülür ya da gizlenir.** `Closed::Collapse` ile panel şeride katlanır, şerit kalır. `Closed::Hide` ile şerit de gider, gövde kenardaki tek sütun dışında bütün genişliği alır. O sütun gövdeyle tıpatıp aynı görünür; imleç ancak ona gelince aydınlanır ve açık panelin kenarındaki `▌›` düğmesinin aynısını gösterir. Paneli yeniden açmak için düğmeye tıkla, `alt+b`'ye bas ya da kenarı gövdeye doğru sürükle; panel en son gösterdiği görünümle açılır. Şerit yoksa ikisi aynıdır: tek bir kenar sütunu kalır.
- **Kapalı kenarı sürüklemek açar.** İmleç en küçük genişliğin yarısı kadar gövdeye girince panel orada açılır.
- **Sınırları sen seçersin.** Varsayılan olarak panel en az 8 sütundur ve üst sınırı yoktur. `.limits(min, max)` `max` için bir sayı ya da `None` alır; oyun alanı sınırsız, 18–48 ve 30–60 arasında geçiş yapar.
- **Karar senin durumunda.** Her değişiklik bir mesajdır; gelen genişlik zaten sınırlar içindedir ve gövde alanın her zaman dörtte birini korur.

## Sık yapılan hatalar

- **Geri dönüş yolu olmadan tamamen kapatmak.** `Closed::Hide` ile ya da şerit yoksa paneli yalnızca kenar ve kısayol yeniden açar; kısayolu ipucu çubuğunda göster.
- **`active_view`'ı unutmak.** O olmadan hiçbir ikon seçili görünmez ve gösterilen görünümün ikonu paneli kapatamaz.
- **`ShowView` içinde kapatmaya da çalışmak.** Kararı şeride bırak: `ShowView`'ı yalnızca bir görünümü göstermek için gönderir, kapatmayı `on_toggle` ile yapar.
- **Önemli durumu yalnızca panel içeriğinde tutmak.** Tamamen kapalı panel çizilmez; önemli olanı uygulama durumunda tut.
- **Ayırıcı çizmek.** Ayırıcı zaten ton farkının kendisidir.
