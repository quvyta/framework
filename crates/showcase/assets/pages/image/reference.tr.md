## Özellik

- `quvyta-framework`'te `image`, varsayılan olarak kapalı. PNG, JPEG, GIF ve WebP çözücülerini ve kitty terminaline giden resimleri sıkıştırmak için zlib'i getirir, başka bir şey getirmez; sixel kodlayıcısı framework'ün kendisinindir.

## ImageData

- `ImageData::decode_file(path, max)` — `path`'teki resmi okur ve çözer, şeklini koruyarak `max`'a (genişlik, yükseklik, piksel) sığacak kadar küçültür. Biçim dosyanın ilk baytlarından okunur. GIF'in ilk karesi alınır. Bekletir: `Command::perform` içinden çağırın.
- `ImageData::decode_bytes(&bytes, max)` — bellekteki bir resmi (`include_bytes!`) çözer, `decode_file` gibi küçültür. Adı yoktur. Hatalar: `UnknownFormat`, `Broken`, `Unreadable`.
- `ImageData::EXTENSIONS` — `["png", "jpg", "jpeg", "gif", "webp"]`: çözücünün okuduğu biçimlerin uzantıları, küçük harfle, `FileBrowser::extensions` için. Bir test onu derlenen biçimlere bağlı tutar.
- `ImageData::reads(path)` — `path`'in uzantısı, büyük küçük harf fark etmeden, bunlardan biri mi. Çözücünün kendisi adla değil ilk baytlarla karar verir.
- `ImageData::from_rgb(width, height, &bytes)` — piksel başına üç bayttan (kırmızı, yeşil, mavi), satır satır bir resim; bir kenar sıfırsa ya da uzunluk yanlışsa `None`.
- `.width()`, `.height()` — saklanan boyut, piksel.
- `.original_size()` — küçültmeden önceki boyut.
- `.name()` — dosyadan çözülen resmin dosya adı; öbürlerinde `None`.
- `.pixel(x, y)` — bir pikselin rengi, resmin dışında `None`.
- Klonlamak ucuzdur: pikseller paylaşılır. Saydamlık korunmaz.

## ImageError

- `Missing` — yolda hiçbir şey yok.
- `Unreadable` — okunamıyor: izin yok, bir klasör ya da çözülemeyecek kadar büyük.
- `UnknownFormat` — PNG, JPEG, GIF ya da WebP resmi değil.
- `Broken` — verisi bozuk ya da yarım kalmış bir resim.
- `Display` çıktısı dil dosyalarından bir cümledir (`quvyta.image.*`).

## Image

- `Image::new(&data)` — `data`'yı çizer, bir klonunu tutar.
- `.fit(Fit)` — `Fit::Contain` (varsayılan), `Fit::Cover` ya da `Fit::Center`.

## Graphics

- `Graphics::can_draw()` — resim çizilir mi: `Kitty`, `Sixel` ve `HalfBlock` için doğru, `None` için yanlış. Resmin kendi sorusudur; bir resmi göstermeden önce sorun, "gösteremiyor" halini atlayın.
- `App::graphics(graphics)` — grafiği ilk kareden önce (`App::resized`'dan sonra, `App::init`'ten önce) ve her değiştiğinde duyar, örneğin glif kipi ASCII'ye geçince. Terminalin gösterdiği boyutta orada çözün.

## Davranış

- Verilen alan içinde yerleşimin gerektirdiği hücreleri ister: `Cover` için tamamını, `Contain` için resmin şeklini, `Center` için kendi boyutunu; bir hücre bir piksel eninde, iki piksel boyundadır.
- Her hücre `▀`'dir: rengi üstteki piksel, zemini alttaki piksel; resmin kenarındaki yarım hücre, altındakinin üstünde `▀` ya da `▄`'dür. Resmin ulaşmadığı hücrelere dokunulmaz.
- Kutu süzgeciyle yeniden örnekler. Hücreler bileşenin belleğinde tutulur, yalnızca alanın boyutu, yerleşim ya da resim değişince yeniden hesaplanır.
- `Env::graphics()` `Graphics::Kitty` ise yarım blok yok. Resmin hücreleri temanın `canvas` zeminini ve bir boşluk alır, terminal resmi `z=-1` ile onların üstüne çizer: metnin altında, hücrelerin zemininin üstünde. Pikseller kendi numaraları altında RGB olarak bir kez gönderilir (`a=t,f=24,o=z`, 4096 baytlık parçalarda base64); sonraki her kare yalnızca yerleştirir (`a=p`, kaynak dikdörtgeni ve hücrelerle), artık kullanılmayan yerleşimi siler (`a=d,d=i`) ve hiçbir yerde gösterilmeyen resmi serbest bırakır (`a=d,d=I`). Yerleşimleri değişmeyen kare hiçbir şey yazmaz. Her komut `q=2` taşır, terminal hiç cevap vermez. Terminal bir programa verilip geri alınınca her şey yeniden gönderilir ve yerleştirilir.
- Kitty resminin üstüne çizilen şey onu örter. Görünen hücreler dikdörtgenlere bölünür (her satırın dizileri, hizalı kaldıkça aşağı doğru birleştirilerek) ve resim her birine kaynağı ona göre kırpılarak bir kez yerleştirilir (`p=1`, `p=2`, …); komşu kırpmalar aynı pikselde buluşur. `PaintCx::tint`'in kaydettiği bir karışımın (iletişim kutusunun arka planı, pencere gölgesi) altındaki hücreler aynı karede yarım blokla, o karışımla karartılmış çizilir; bir hücre yalnızca kaydedilen karışımlar işareti ve zemini tam olarak onun renklerine götürüyorsa karartılmış sayılır, oraya çizilen başka her şey onu örter. 64'ten çok dikdörtgen o kare bütün resmi yarım blokla çizdirir.
- `Env::graphics()` `Graphics::Sixel`: hücreler kitty'deki gibi işaretlenir; kare boyandıktan sonra resim yalnızca çizebileceği her hücre (kırpma alanının içinde) işaretini hâlâ taşıyorsa gösterilir. Herhangi bir yerinin üstündeki bir şey, bir karışım dahil, o kare resmin tamamını yarım blokla çizdirir. Gösterildiğinde kutu süzgeciyle hücrelerinin piksellerine küçültülür: bir hücre, terminalin penceresi için bildirdiği boyutun hücre sayısına bölümüdür, bildirmiyorsa 10 × 20; yükseklik altılık bantların tamına indirilir, resim son satırının altına hiç taşmaz. Renkler 6 × 7 × 6 düzeyli (kırmızı, yeşil, mavi; 252 girdi, yalnızca kullanılanlar tanımlanır) sabit bir palete gider. Hücrelerden sonra, karenin eşzamanlı güncellemesinin içinde yazılır: imleç ilk hücresine gider, sonra `ESC P 0;1;0 q "1;1;w;h`, palet, `!` tekrarlı altı piksellik bantlar, `ST`. Yeniyse, taşındıysa ya da başka kırpıldıysa, altındaki bir hücre değiştiyse, yeni boyuttan sonra ve terminali bir program kullandıktan sonra yeniden yazılır; aynı yerleşimli ve hücreleri değişmemiş kare hiçbir şey yazmaz. Bir yerleşim gidince altındaki, karenin değiştirmediği hücreler yeniden yazılır, pikselleri geride kalmaz. Kodlamalar resim boyandığı sürece saklanır.
- Hücrelerini süs olarak işaretler: bir metin seçiminin temiz kopyası onları atlar.
- `ColorDepth::Ansi256`: iki yarı da en yakın palet rengini alır. `Graphics::None` (`ColorDepth::Ansi16`, `GlyphMode::Ascii` ya da `QUVYTA_GRAPHICS=none`): dosya adıyla (ya da "Resim"), biçimle, özgün boyutla ve "Bu terminal resim gösteremiyor." cümlesiyle bir boş durum çizer.
- Odak almaz; mesaj göndermez.

## Tema anahtarları

- `empty-state-icon`, `empty-state-title`, `empty-state-message` — resim çizilemeyen yerde gösterilen boş durum üzerinden.
