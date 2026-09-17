## Ne zaman kullanılır

İnsanların okuduğu, kopyaladığı ya da karşılaştırdığı kodu göster: yardımdaki örnekler, yapılandırma önizlemeleri, ayar farkları. Bu showcase'teki her sayfanın Kod bölümü, sayfanın kendi kaynağını gösteren bir kod görünümüdür.

## Adım adım

1. Kod ve diliyle oluştur: `CodeView::new(kaynak, Language::Rust)`.
2. Kısa örneklerde satır numaralarını gizle: `.line_numbers(false)`.
3. Kopyalamaya tepki ver: `.on_copy(Msg::Copied)`, örneğin bir bildirim göstermek için.

## Nasıl çalışır

- **Renkler temadan.** Rust için anahtar kelimeler, türler, fonksiyonlar, makrolar, metinler, sayılar, yorumlar, öznitelikler ve lifetime'lar; TOML için tablolar, anahtarlar, metinler, sayılar ve boolean'lar. Her tür bir `code-token` varyantıdır; tema kodu diğer bileşenler gibi yeniden boyar.
- **Uzun satırlar** kesilmek yerine iki hücrelik girintiyle sarılır; satır numarası yalnızca ilk satırda görünür.
- **Tek tuşla kopyala.** Bloğa Tab ile odaklan ve `c`'ye bas: kod panoya gider, SSH üzerinden de, blok parlar.
- **Yüzey onun çerçevesidir.** Kod bloğu iç boşluklu, hafifçe yükseltilmiş bir yüzeydir; kenarlık yok.
- **Kendiliğinden seçilebilir.** Kodun içinde sürüklemek metin seçer ve blokta kalır, iç boşluğa taşmaz. Kopyala (`ctrl c` ya da sağ tık menüsü) satır numaralarını almaz; Ham kopyala alır. Seçimi düğümde `.selectable(false)` ile kapat.

## Temayla özelleştirme

```toml
[style."code-token.keyword"]
fg = "$accent"

[style."code-token.comment"]
fg = "$muted"
italic = true
```

## Sık yapılan hatalar

- **Düz metni kod gibi göstermek.** Loglar ve çıktılar için `Language::Plain` kullan; renklendirme yanıltır.
- **Devasa dosyalar.** Kod görünümü tüm satırlarını çizer; bir kaydırma alanına koy ve önemli kısmı göster.
