## Ne zaman kullanılır

İnsanların okuduğu, kopyaladığı ya da karşılaştırdığı kodu göster: yardımdaki örnekler, yapılandırma önizlemeleri, ayar farkları, çalışmadan önce incelenecek bir paket tarifi. Bu showcase'teki her sayfanın Kod bölümü, sayfanın kendi kaynağını gösteren bir kod görünümüdür.

## Adım adım

1. Kod ve diliyle oluştur: `CodeView::new(kaynak, Language::Rust)`.
2. Kısa örneklerde satır numaralarını gizle: `.line_numbers(false)`.
3. Kopyalamaya tepki ver: `.on_copy(Msg::Copied)`, örneğin bir bildirim göstermek için.
4. Fark göster: `.line_marks(işaretler)`, her satıra bir `LineMark::Added`, `Removed` ya da `Unchanged`.
5. Satırları göster: bulgular için `.highlight_lines(12..=14, LineTone::Warning)`, gidilen satır için `LineTone::Accent`.
6. Satıra git: kod görünümünü bir `ScrollView` içine koy ve `.reveal(satır)` ekle; kaydırma alanı satırı gösterecek kadar kayar. Farkta bunun yerine dosyanın kendi numarasıyla git: `.reveal_number(22)`.
7. Dosyanın ortasından başlayan bir fark: `.line_numbers_from(numaralar)`, her satıra bir numara ve numarası olmayan satıra (parça başlığı gibi) `None`.

## Nasıl çalışır

- **Renkler temadan.** Rust için anahtar kelimeler, türler, fonksiyonlar, makrolar, metinler, sayılar, yorumlar, öznitelikler ve lifetime'lar; TOML için tablolar, anahtarlar, metinler, sayılar ve boolean'lar; kabuk betikleri için yorumlar, metinler, değişkenler ve açılımlar, anahtar kelimeler, fonksiyon tanımları ve heredoc'lar. Her tür bir `code-token` varyantıdır; tema kodu diğer bileşenler gibi yeniden boyar.
- **Dil dosya adından.** `Language::from_file_name("PKGBUILD")` kabuktur, `.sh`, `.bash`, `.zsh` ve `.install` de öyle; `.rs` Rust, `.toml` TOML, gerisi düz metin.
- **Farklar ve bulgular işaretli bir tonla.** İşaretli ya da vurgulu satırın bütün sırası, sarılan sıraları da dahil, bir tonla boyanır ve soldaki tek hücrelik sütuna bir işaret gelir: eklenen satıra yeşil `+`, silinene kırmızı `−`, uyarıya uyarı ikonu, vurguya dikey çubuk. Aynı satırda vurgu fark işaretini geçer. Sütun ancak bir şey işaretlenince açılır; kopyalar işaretleri almaz.
- **Satıra gitmek kayarak olur.** `.reveal(satır)` çevreleyen kaydırma alanını satırı iki satır bağlamla gösterecek kadar, bir sayfa geçişi süresinde kaydırır; hareket azaltılmışsa bir anda atlar. Satır değişince olur, sonra kullanıcı istediği yere kaydırabilir.
- **Uzun satırlar** kesilmek yerine iki hücrelik girintiyle sarılır; satır numarası yalnızca ilk satırda görünür.
- **Tek tuşla kopyala.** Bloğa Tab ile odaklan ve `c`'ye bas: kod panoya gider, SSH üzerinden de, blok parlar.
- **Yüzey onun çerçevesidir.** Kod bloğu iç boşluklu, hafifçe yükseltilmiş bir yüzeydir; kenarlık yok.
- **Kendiliğinden seçilebilir.** Kodun içinde sürüklemek metin seçer ve blokta kalır, iç boşluğa taşmaz. Kopyala (`ctrl c` ya da sağ tık menüsü) satır numaralarını almaz; Ham kopyala alır. Seçimi düğümde `.selectable(false)` ile kapat.

- **Farkın numaraları metne değil, dosyalara aittir.** Fark, iki sürümün satırlarını arka arkaya dizer; baştan saymak bu yüzden ikisinden hiçbirini numaralamaz ve `PKGBUILD:22` diyen bir bulgu yanlış satırı gösterir. `.line_marks(...)` verildiğinde numaralar dosyaları izler: silinen satır eski dosyanın numarasını, eklenen satır yeni dosyanınkini, ikisinde de olan satır yeni dosyanınkini taşır. `.reveal_number(22)` o numaranın anlattığı satıra gider; silinen bir satırla eklenen bir satır aynı numarayı taşıdığında, yeni dosyanın o numarayı verdiği satıra gidilir, çünkü bulgu o dosya hakkındadır.

## Temayla özelleştirme

```toml
[style."code-token.keyword"]
fg = "$accent"

[style."code-token.comment"]
fg = "$muted"
italic = true

[style."code-line.added"]
bg = "mix($success, $surface, 10%)"
fg = "$success"
```

## Sık yapılan hatalar

- **Düz metni kod gibi göstermek.** Loglar ve çıktılar için `Language::Plain` kullan; renklendirme yanıltır.
- **Devasa dosyalar.** Kod görünümü tüm satırlarını çizer; bir kaydırma alanına koy ve önemli kısmı göster.
- **Her karede satıra gitmek.** `.reveal(satır)` satır değişince çalışır; satırı durumda tut ve kullanıcı bir yere gitmek istediğinde değiştir.
- **Anlamsız durum rengi.** Bir satırı göstermek için `LineTone::Accent` kullan; `Warning` dikkatle bakılacak bir şey olduğunu söyler.
