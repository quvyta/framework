## Ne zaman kullanılır

Kullanıcının yazdığı tek satırlık metin için metin kutusu kullan: isimler, arama terimleri, yollar, kodlar. Değer uygulamana aittir; düzenlemeyle ilgili her şey bileşene aittir.

## Adım adım

1. Değerini göster: `TextInput::new(&self.name)`.
2. Her değişikliği al: `.on_change(Msg::NameChanged)`. Yeni değeri `update` içinde sakla; alan durumunda ne varsa onu gösterir.
3. Oraya ne yazılacağını söylemek için `.placeholder(t!("..."))` ekle.
4. Kodunda doğrula ve alanı işaretle: `.invalid(name.len() < 3)`, altına kısa bir mesajla.
5. Gizli bilgiler için `.password(true)`, sınırlar için `.max_length(n)`, Enter'da işlem için `.on_submit(msg)` kullan.
6. Yeniden adlandırma penceresinde alanı ad seçili açılsın: `main.rs` için `.select_on_focus(0..4)`, karakterle sayılır. Yazmaya başlayınca ad değişir, `.rs` kalır; `.select_all_on_focus()` hepsini seçer.
7. Adres çubuğunda ya da paket aramasında kişinin isteyebileceğini öner: `.suggestions(satırlar)` ve `.on_suggestion(|index| msg)`. Hangi satırların orada olacağına sen karar verirsin; alan listeyi çizer ve hangisinin seçildiğini söyler.

## Nasıl çalışır

- **Gerçek düzenleme.** İmleç karakterler arasında gezinir, asla birleşik bir harfin içine girmez. Ctrl ve oklar kelime atlar, Shift seçer, Home ve End kenarlara atlar.
- **Doğru hissettiren geri alma.** Bir kelime yazmak tek adımda geri alınır; bir kelime silmek kendi adımıdır. Ctrl+Z geri alır, Ctrl+Y ya da Ctrl+Shift+Z yineler.
- **Pano.** Ctrl+C ve Ctrl+X seçimi kopyalar ve keser; Ctrl+V önce sistem panosundan, sonra terminalin panosundan, en son uygulama içindeki son kopyadan yapıştırır. Terminalin kendi yapıştırması da metni ekler, satır sonları boşluğa döner.
- **Oklar seçimi arkada bırakır.** Seçim varken ← ve → onu kaldırır ve sol ya da sağ ucundan bir adım öteye gider; bir metin düzenleyicideki gibi.
- **Fare.** İmleci yerleştirmek için tıkla, seçmek için sürükle. Sağ tık Kes, Kopyala, Yapıştır ve Tümünü seç menüsünü açar: seçimin içindeyse seçimi korur, başka yerdeyse önce imleci oraya koyar. Seçim yokken Kes ve Kopyala, yapıştıracak bir şey yokken Yapıştır pasiftir. Shift+F10 ve menü tuşu aynı menüyü açar.
- **Parola kopyalanmaz.** Parola alanında Kes, Kopyala, Ctrl+C ve Ctrl+X hiçbir şey yapmaz.
- **Yazarken imleç sabit kalır**, yalnızca durduğunda temanın `motion.cursor-blink` hızında yanıp söner.
- **Uzun değerler** imleci görünür tutmak için yatayda kayar.
- **Başlangıçta bir seçim.** `select_on_focus` ile alan her odak aldığında metnin o kısmını seçer, imleç o kısmın sonundadır. Bundan sonra seçim kullanıcınındır: yazmak onu değiştirir, oklar bırakır. Odağı getiren bir tıklama ise imleci tıklanan yere koyar; metnin dışına taşan aralık metinle sınırlanır.
- **Dışarıdan değişiklik kazanır.** Uygulaman değeri değiştirirse alan onu gösterir ve imleci sona koyar.
- **Sırada ne var diye bir liste.** `.suggestions(...)` kişi yazar yazmaz alanın altında, tam alan kadar geniş bir liste açar: kenarları alanın kenarlarıyla aynı yerde durur, alandan uzun bir ad orada kesilir. Liste bir menüdür; seçilen satır tonla yükselir ve yanında `▌` sütunu vardır, notu ve ikonu da menünün kendi olanlarıdır. `max_suggestions(n)` listenin kaç satır tuttuğunu söyler, varsayılan sekiz.
- **İstenmeden hiçbir şey sunulmaz.** Odağa girmek liste açmaz, değeri hazır olan alan da sunmaz; liste metin değiştikçe açılır. Liste `.on_suggestion`'ı da ister, ikisini de vermeyen alan tam olarak önce gibi davranır.
- **Yazdığın metin durur.** `↑` ve `↓` seçili satırı taşır, seçili satır varken `Enter` onu alır. İlk satırdan `up` ve son satırdan `down` yazdığını geri getirir, o zaman `Enter` her zaman yaptığı gibi gönderir. Yazmak yeniden seçmez, `esc` listeyi kapatıp metni korur, başka bir yere basmak listeyi kapatır ve tıklanan yine basılır.

## Temayla özelleştirme

```toml
[style."text-input:focus"]
bg = "$active"

[style."text-input:invalid"]
bg = "mix($danger, $surface, 14%)"

[style.text-input-cursor]
bg = "$accent"
fg = "$ink"
```

Geçersiz alan yüzeyini renklendirir; asla kırmızı bir çerçeve çizmez.

## Sık yapılan hatalar

- **Aralığı bayt olarak saymak.** `select_on_focus` karakter sayar; `şğü.txt` adını `0..3` ile seçer. `str::find` bayt döndürür, önce karakter sayısına çevir.
- **Her tuşta yüksek sesli doğrulama.** Boş alan henüz hata değildir; mesajları kullanıcı bir şey yazdıktan sonra göster.
- **İmleci kendi durumunda tutmak.** Motor bunu senin için tutar.
- **Seçimler için metin kutusu.** Cevaplar belliyse açılır liste ya da liste kullan; önerili metin kutusu henüz bilinmeyen cevaplar içindir.
- **Her şeyi birden sunmak.** Liste `max_suggestions` satır tutar, fazlasını değil; satırlarını kişinin yazdığına göre daralt ve ilk sıralara kendin seçeceklerini koy. `qframe::text::fuzzy` süzgecin, komut paletinin ve seçicilerin bulduğu eşleştiricidir.
