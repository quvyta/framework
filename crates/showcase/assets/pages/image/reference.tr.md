## Özellik

- `quvyta-framework`'te `image`, varsayılan olarak kapalı. PNG, JPEG, GIF ve WebP çözücülerini ve kitty terminaline giden resimleri sıkıştırmak için zlib'i getirir, başka bir şey getirmez.

## ImageData

- `ImageData::decode_file(path, max)` — `path`'teki resmi okur ve çözer, şeklini koruyarak `max`'a (genişlik, yükseklik, piksel) sığacak kadar küçültür. Biçim dosyanın ilk baytlarından okunur. GIF'in ilk karesi alınır. Bekletir: `Command::perform` içinden çağırın.
- `ImageData::from_rgb(width, height, &bytes)` — piksel başına üç bayttan (kırmızı, yeşil, mavi), satır satır bir resim; bir kenar sıfırsa ya da uzunluk yanlışsa `None`.
- `.width()`, `.height()` — saklanan boyut, piksel.
- `.original_size()` — küçültmeden önceki boyut.
- `.name()` — çözülen resmin dosya adı.
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

## Davranış

- Verilen alan içinde yerleşimin gerektirdiği hücreleri ister: `Cover` için tamamını, `Contain` için resmin şeklini, `Center` için kendi boyutunu; bir hücre bir piksel eninde, iki piksel boyundadır.
- Her hücre `▀`'dir: rengi üstteki piksel, zemini alttaki piksel; resmin kenarındaki yarım hücre, altındakinin üstünde `▀` ya da `▄`'dür. Resmin ulaşmadığı hücrelere dokunulmaz.
- Kutu süzgeciyle yeniden örnekler. Hücreler bileşenin belleğinde tutulur, yalnızca alanın boyutu, yerleşim ya da resim değişince yeniden hesaplanır.
- `Env::graphics()` `Graphics::Kitty` ise yarım blok yok. Resmin hücreleri temanın `canvas` zeminini ve bir boşluk alır, terminal resmi `z=-1` ile onların üstüne çizer: metnin altında, hücrelerin zemininin üstünde. Pikseller kendi numaraları altında RGB olarak bir kez gönderilir (`a=t,f=24,o=z`, 4096 baytlık parçalarda base64); sonraki her kare yalnızca yerleştirir (`a=p`, kaynak dikdörtgeni ve hücrelerle), artık kullanılmayan yerleşimi siler (`a=d,d=i`) ve hiçbir yerde gösterilmeyen resmi serbest bırakır (`a=d,d=I`). Yerleşimleri değişmeyen kare hiçbir şey yazmaz. Her komut `q=2` taşır, terminal hiç cevap vermez. Terminal bir programa verilip geri alınınca her şey yeniden gönderilir ve yerleştirilir.
- Kitty resminin üstüne çizilen şey onu örter: görünen hücreler tek bir dikdörtgen oluşturuyorsa (bir kenar boyunca açılan menü) resim ona kırpılır; oluşturmuyorsa (ortada ya da köşede bir şey, bir iletişim kutusunun karartılmış arka planı) o kare resmi yarım blokla, arka plan gibi karartılmış çizer. `Graphics::Sixel` şimdilik yarım blok çizer.
- Hücrelerini süs olarak işaretler: bir metin seçiminin temiz kopyası onları atlar.
- `ColorDepth::Ansi256`: iki yarı da en yakın palet rengini alır. `ColorDepth::Ansi16` ve `GlyphMode::Ascii`: dosya adıyla (ya da "Resim"), biçimle, özgün boyutla ve "Bu terminal resim gösteremiyor." cümlesiyle bir boş durum çizer.
- Odak almaz; mesaj göndermez.

## Tema anahtarları

- `empty-state-icon`, `empty-state-title`, `empty-state-message` — resim çizilemeyen yerde gösterilen boş durum üzerinden.
