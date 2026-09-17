## Ne zaman kullanılır

Bir sayfada birkaç grup ayrıntı varsa ve insanlar çoğu zaman bunların yalnızca bir ikisine bakıyorsa akordeon kullan: bir container'ın genel bakışı, portları, birimleri ve ortamı gibi. Bütün gruplar aynı anda gerekiyorsa panel olarak göster; gruplar aynı şeyin farklı görünümleriyse sekme kullan.

## Adım adım

1. Hangi bölümlerin açık olduğunu uygulamanda tut: `open: [bool; 4]`.
2. Başlıkları ver: `Accordion::new(["Genel bakış", "Portlar", "Birimler"])`.
3. Her bölüm için aynı sırayla bir çocuk ekle: `ui.add_with(accordion, |ui| { ui.column(..); ui.column(..); ... })`.
4. Durumu ver, değişiklikleri işle: `.open(state.open).on_toggle(|sıra, açık| Msg::Section(sıra, açık))`.
5. Gerekeni aç: `.single(true)` aynı anda tek bölüm açık tutar; `Section::new(başlık).icon("folder").detail("3 bağlama")` ikon ve silik bir ayrıntı ekler.

## Nasıl çalışır

- **Çerçeve değil yüzey.** Başlık satırı sayfadan bir ton yüksektir, açık gövde altında yüzey tonundadır, bölümleri bir satırlık zemin ayırır.
- **Başlık satırları liste satırı gibi davranır.** Fare üstüne gelince ya da klavyeyle odaklanınca satır yükselir, vurgu çubuğu belirir, ikon ve başlık bir hücre sağa kayar. Ok ve sağdaki ayrıntı yerinde kalır.
- **Açılış yavaşça serilir.** Gövdenin satırları hemen ayrılır, içerik `motion.enter` süresinin iki katında satır satır görünür. Kapanış anında olur, böylece alttaki içerik ileri geri zıplamaz. Hareket azaltılmışsa bir anda açılır.
- **Karar senin durumunda.** Tıklama yalnızca mesaj gönderir; bölüm, `update` bunu kaydedince açılır.
- **Aynı anda tek bölüm** seçeneği, açma mesajıyla birlikte diğer açık bölümler için kapama mesajları gönderir.

## Sık yapılan hatalar

- **Başlıklarla uyuşmayan çocuklar.** n. çocuk n. başlığın gövdesidir; sırayı aynı tut.
- **Kapalı bölümlerde önemli eylemler.** İnsanlar her bölümü açmaz; ana eylemi dışarıda tut.
- **Sabit ve kısa bir alanda akordeon.** Açık gövdeler doğal yüksekliklerini alır; akordeonu kaydırma alanına koy ya da sabit yüksekliği paylaştıran bileşen yuvasını kullan.
