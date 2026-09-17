## Ne zaman kullanılır

Kaydırılan her bileşen aynı kaydırma çubuğunu çizer; nasıl görüneceğine tema karar verir. Stili tema yazarken seç: varsayılan `block` her yerde iyi okunur, `half` daha hafiftir, yoğun araçlara `thin` yakışır, en sessizi `dots`. Bir bileşende stili yalnızca o yer uygulamanın geri kalanından farklı görünmeliyse sabitle.

## Adım adım

1. Tema dosyasında stili seç: `[style.scrollbar]` altında `style = "thin"`.
2. Gerekirse renkleri ayarla: `track` ve `thumb`; çubuğun üstüne gelinince ya da sürüklenince parlayan başparmak için `[style."scrollbar:hover"]`.
3. Diğerlerine dokunmadan tek bir stili ayarlamak için onun adıyla bir varyant kuralı yaz: `[style."scrollbar.dots"]`.
4. Stili tek bir bileşende sabitlemek için `List` ya da `ScrollView` üzerinde `.scrollbar(ScrollbarStyle::Thin)` çağır.

## Nasıl çalışır

- **Tek sütun, yalnızca taşınca.** Her şey sığıyorsa hiçbir stil bir şey çizmez; sütun içeriğe geri döner.
- **Çerçeve yok.** İz bir ton ya da silik bir karakterdir, içeriğin etrafında çizgi asla değildir.
- **`block`** hiç karakter kullanmaz: iz ve başparmak arka plan rengidir. Her karakter modunda aynı görünür; sütunda karakter olmadığı için metin seçimi kaydırma çubuğunu asla almaz. **`half`** ince bir iz `▕` ve yarım blok başparmak `▐` çizer. **`thin`** iz çizmez, yalnızca ince başparmak. **`dots`** noktalı bir iz `·` ve dolu bir başparmak `•` çizer.
- **ASCII terminaller** blok karakterlerin yerine renkli hücre alır; her stil yine çalışır.
- **Bilinmeyen sözcükler bildirilir.** `style = "wavy"` yazan bir tema dosya, satır ve sütunla bir uyarı alır; bileşen varsayılanda kalır.

## Sık yapılan hatalar

- **Stili her yerde sabitlemek.** O zaman tema onu değiştiremez; yalnızca gerçekten farklı olan yerde sabitle.
- **İze çok yakın başparmak.** Her temada `thumb` rengini `track` ile karşılaştır; renkten başka bir şeyi olmayan `block` stilinde özellikle. Yükseltilmiş bir yüzeyde (çok satırlı metin gibi) iz yüzeyle aynı renk olabilir; başparmak konumu yine gösterir.
