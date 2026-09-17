## Ne zaman kullanılır

Bir sıranın nerede olduğunu göstermek için adım göstergesi kullan: sihirbaz sayfaları, yayın hattının aşamaları, başlangıç kontrol listesi. "Ne kadar ilerledim, ne kaldı" sorusunu yanıtlar. İlerleme adlandırılmış aşamalar değil de bir miktarsa ilerleme çubuğu kullan.

## Adım adım

1. Güncel adımı durumunda tut: `current: usize`.
2. Sırayı çiz: `Steps::new(etiketler).current(state.current)`.
3. İş ilerledikçe `update` içinde `current` değerini artır; son adımı geçen bir değer her şeyi bitmiş gösterir.
4. Yetenekleri yalnızca uyduğu yerde aç: yan sütundaki kontrol listesi için `.vertical(true)`, güncel aşama çalışırken `.running(true)`, durduğunda `.failed(true)`, bitmiş bir adıma dönülebilsin diye `.on_select(Msg::GoTo)`.

## Nasıl çalışır

- **Durumu işaret ve ton taşır.** Bitmiş adımlar başarı renginde bir onay, güncel adım vurgu renginde bir nokta ve kalın etiket, gelecek adımlar silik bir halka gösterir. Renk hiçbir zaman tek başına kalmaz.
- **Adımları hiçbir şey bağlamaz.** Çizgi ya da ok yok; adımları boşluk ayırır.
- **Dar satır kendini sıkıştırır.** Etiketler sığmazsa bütün işaretler kalır, etiketini yalnızca güncel adım korur.
- **Çalışan adım nefes alır.** Güncel işaret iki vurgu rengi arasında atar; hareket azaltılmışsa sabit durur.
- **Geri dönmek isteğe bağlıdır.** `on_select` yoksa adımlar yalnızca gösterimdir ve odak almaz. Varsa bitmiş adımlar, basılabilen her şey gibi, fare altında vurgu çubuğuyla yükselir; klavye ok tuşlarıyla aralarında gezer; Enter ya da Boşluk seçer.
- **Kararı uygulama verir.** Seçim bir mesaj gönderir; `current` değerini senin `update` fonksiyonun ayarlar.

## Sık yapılan hatalar

- **İleri atlamaya izin vermek.** Yalnızca bitmiş adımlar seçilebilir; doğrulamayı atlamak özellik değil akış hatasıdır.
- **Uzun etiketler.** Adım başına bir iki sözcük; ayrıntı sayfanın kendisine aittir.
- **Tek bir iş için adım göstergesi.** Tek adım sıra değildir; spinner ya da ilerleme çubuğu kullan.
