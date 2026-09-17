## Ne zaman kullanılır

Birkaç satıra yayılan metin için metin alanı kullan: sürüm notları, commit mesajı, dağıtım betiği, yorum. Bir ad, bir arama ya da tek satırlık her şey için metin kutusu kullan; orada Enter satır açmak yerine gönderir.

## Adım adım

1. Metni uygulamanda tut: `summary: String`.
2. Çiz: `TextArea::new(&state.summary)`. Seçeneksiz alan satır kırar, kaydırır ve düzenler; metniyle büyüyerek üç ile sekiz satır arası yer kaplar.
3. Değişiklikleri işle: `.on_change(|text| Msg::Summary(text))`, metni `update` içinde sakla.
4. Genişliği yerleşimle ver; büyümemesi gerekiyorsa yüksekliği de: `.height(Length::Cells(5))`.
5. Yetenekleri yalnızca gerektiğinde ekle: `.placeholder(..)`, `.max_length(280).counter(true)`, betikler ve kod için `.line_numbers(true)`.
6. Metni bir yere göndermek için Ctrl+Enter'a `.on_submit(|text| Msg::Publish(text))` bağla, yanına bir de buton koy.

## Nasıl çalışır

- **Aynı alan, daha uzun.** Yüzey, odak, geçersiz tonu, imleç, seçim renkleri ve geri alma `TextInput` ile aynıdır.
- **Sözcükler alt satıra geçer, hiçbir şey kaybolmaz.** Uzun satırlar sözcük aralarından kırılır, satırdan uzun bir sözcük karakter aralarından bölünür, satır başındaki girinti korunur. Kırılan satırlar yalnızca görünüştür: metnin kendi satır sonları değişmez.
- **Enter yeni satır açar.** Göndermek sana kalmış: `on_submit` verilmişse Ctrl+Enter onu gönderir. Kitty klavye protokolünü desteklemeyen terminaller Ctrl+Enter'ı düz Enter olarak bildirebilir; bu yüzden yanına bir buton koymak gerekir.
- **Gezinme:** ↑ ve ↓ satırlar arasında gezerken sütunu korur, kısa satırlardan geçerken bile; Home ve End görünen satırın başına ve sonuna, Ctrl ile metnin başına ve sonuna gider; Page Up ve Page Down bir sayfa kaydırır. Hepsi Shift ile seçer. Ctrl+U satır başına kadar siler.
- **Kaydırma imleci izler.** Metin alandan uzun olunca sağda kaydırma çubuğu belirir; tekerlek ve çubuğu sürüklemek imleci yerinden oynatmadan kaydırır.
- **Sınırlar karakter sayar.** Satır sonu bir karakterdir, yapıştırılan metin sınırda kesilir, sayaç metnin altında `sayı / sınır` gösterir.
- **Satır numaraları** görünen satırlara değil metnin satırlarına aittir: kırılan bir satır bir kez numaralanır, imlecin bulunduğu satırın numarası daha parlaktır.
- **Oklar seçimi arkada bırakır.** Shift olmadan ← → seçimi kaldırır ve sol ya da sağ ucundan bir adım öteye, ↑ ↓ üst ya da alt ucundan bir satır öteye gider.
- **Aynı düzenleme menüsü.** Sağ tık `TextInput` kurallarıyla Kes, Kopyala, Yapıştır ve Tümünü seç menüsünü açar; yapıştırılan metin satır sonlarını korur.

## Sık yapılan hatalar

- **Enter ile göndermek.** Metin alanında insanlar Enter'ın yeni satır açmasını bekler; Ctrl+Enter ve bir buton kullan.
- **Yalnızca Ctrl+Enter'a güvenmek.** Her terminal bunu bildirmez; her zaman başka bir yol sun.
- **Sınırsız büyümek.** Metniyle büyüyen alan sayfanın geri kalanını iter; başka kontrollerin arasındaysa yüksekliğini sabitle.
