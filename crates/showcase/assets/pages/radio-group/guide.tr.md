## Ne zaman kullanılır

Birkaç seçenekten tam olarak birinin seçilmesi gerektiğinde ve hepsinin aynı anda görünmesi gerektiğinde radyo grubu kullan: container motoru, yoğunluk. Beşten fazla seçenek varsa ya da yer darsa açılır liste kullan.

## Adım adım

1. Seçimi uygulamanda tut: `engine: Option<usize>`.
2. Grubu çiz: `RadioGroup::new(["Podman", "Docker"]).selected(state.engine)`.
3. Seçimleri işle: `.on_select(|index| Msg::Engine(index))`.
4. Kısa seçenekleri `.horizontal(true)` ile tek satıra diz.
5. Mantıklı bir varsayılan yoksa `None` ile başla; o zaman insanlar seçmek zorunda kalır.
6. Varsayılan stil karedir. Kutuya büyüyen kare için `.style(RadioStyle::Mark)`, onay kutusunun kutusu için `.style(RadioStyle::Box)`, nokta için `.style(RadioStyle::Dot)` ekle.

## Nasıl çalışır

- **Tek kontrol, tek odak durağı.** Tab gruba bir kez gelir; ok tuşları (Yukarı ve Aşağı, satırda Sol ve Sağ) komşuyu hemen seçer, Home ve End uçları seçer.
- **Dört stil, hiç parantez yok.** `Square` (varsayılan): her seçenekte iki hücrenin ortasında küçük bir kare durur; seçili olmayanlarda sakin, seçilide seçili renkte. `Mark`: aynısı, ama seçili kare vurgu renginde iki hücrelik dolu kutuya büyür. `Box`: her seçenek onay kutusunun iki hücrelik kutusudur, boş ya da dolu. `Dot`: seçili seçenekte dolu nokta, diğerlerinde silik halka.
- **Yumuşak renk geçişleri.** `Square` ile seçim yapınca yeni karenin rengi iki `motion.step` boyunca sakin tondan seçili tona karışır, eski kare aynı anda geri döner; hiçbir şeyin biçimi değişmez. `Mark` ile aynı karışım olur ve kare yolun yarısında dolu kutuya döner, arada başka boy yoktur, hiçbir şey sallanmaz. Hareket azaltılmışsa değişim anında olur. İşaret her durumda iki hücre genişliğindedir, yazılar kaymaz. Üzerine gelinen kare seçili tonun altında bir tona açılır, seçilmiş gibi görünmez.
- **Küçük kare iki sekstanttır.** `🬇🬃`, Symbols for Legacy Computing bloğundandır. kitty, WezTerm, Ghostty ve foot bu karakterleri kendisi çizer, hücreye hep tam oturur. Diğer terminaller onları yazı tipinden alır; seninkinde yoksa bu bloğu içeren bir yazı tipi kur ya da ikonları değiştir.
- **Kare ikon setindedir.** `radio-mark-small` ikonudur; bir temanın `[icons]` tablosu değiştirebilir. Boş karakter (ASCII'deki gibi) yerine bir kutu çizer, tonu kutu stilinin boş tonundan seçili tona karışır.
- **Kutu bilerek onay kutusuna benzer.** Fark anlamdadır: radyo grubunda tek bir seçenek seçili kalır, onay kutusu ise kendi başına açık ya da kapalıdır. Yalnızca birinin seçilebildiği anlaşılsın diye gruba bir başlık ver.
- **Hover seçenek başınadır.** İmlecin altındaki seçeneğin karesi (ya da boş kutusu) bir ton açılır; seçeneğin herhangi bir yerine, işaretine ya da yazısına tıklamak onu seçer. Vurgu çubuğu yok.
- **Odak seçimin üzerinde nefes alır.** Klavye odağı seçili kutuda nefes alır. Hiçbir şey seçili değilse odak ilk seçeneğin karesini ısıtır; Boşluk ya da Enter onu seçer.

## Sık yapılan hatalar

- **Açık/kapalı için radyo grubu.** "Açık" ve "Kapalı" diye iki seçenek bir anahtardır.
- **Uzun seçenek listeleri.** Yedi seçenek formun geri kalanını iter; açılır liste kullan.
- **Kareyi kodda çizmek.** Onun yerine temada `radio-mark-*` ikonlarını değiştir; ASCII ve her terminal çalışmaya devam eder.
