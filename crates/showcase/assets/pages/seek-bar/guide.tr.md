## Ne zaman kullanılır

Bir kişi bir şeyi izliyor ya da dinliyorsa ve içinde başka bir yere geçmek istiyorsa oynatma çubuğu kullanın: bir videonun altındaki taşıma satırı, bir ses dosyasının oynatma başlığı. Bu, basılıp sürüklenebilen bir ilerleme çubuğudur. İş başka bir yere taşınamıyorsa — bir yükleme, bir kopyalama, bir test çalıştırması — ilerleme çubuğu kullanın; değer zaman içindeki bir konum değil de bir sayıysa sürgü kullanın.

## Adım adım

1. Konumu uygulamanızda tutun: 0'dan 1'e `position: f32`.
2. Çizin: `ui.add(SeekBar::new(state.position)).width(Length::Fill(1))`.
3. Bir aramanın ne anlama geldiğini söyleyin: `.on_seek(|fraction| Msg::Seek(fraction))` ve kesri `update` içinde saklayın.
4. Konumu işaretçinin altında adlandırın: `.hover_label(|fraction| clock(fraction))`, burada `clock` parçanın uzunluğuna göre `m:ss` yazar.
5. Durumu anlatan bir ton verin: sona gelindiğinde `.variant("success")`.

## Nasıl çalışır

- **İlerleme çubuğuyla aynı görüntü.** Oynatma çubuğu, ilerleme çubuğunun kendi boyama kodunu kullanır; birini diğerinin durduğu yere koymak ekranda hiçbir şeyi değiştirmez, yalnızca işaretçinin ne yapabildiği değişir. Çubuğun sonundaki yüzde ikisinin de yazdığı yüzdedir, bu yüzden aynı `progress-label` stilini alır.
- **Basış, düştüğü hücreye gider**, birleşme yerine o hücrenin merkezine: yirmi hücrelik bir çubukta dördüncü hücre `4.5 / 20` olur.
- **Sürükleme, bırakılana kadar hücre hücre ilerler.** İşaretçi çubuktan ve çubuğun bulunduğu satırdan çıkabilir; kesir 0'a ya da 1'e kırpılır, böylece bir uçtan öteye sürükleme orada tutar ve kaçmaz.
- **Klavye, mesaj varsa** ona ulaşır: ← ve → çubuğun beşte birini taşır, Home ve End uçlara gider. Uçta, taşınacak bir yer kalmadığında, çubuk bir şey söylemez.
- **İşaretçinin altında** çubuk bir ton açılır ve işaretçinin altındaki hücre vurgu rengini alır; bir aramanın nereye gideceği tek başına duran hücre olur.
- **`on_seek` olmadan** çubuk yalnızca bir görüntüdür: odak yok, üzerine gelme yok, işaretçi yok; tam olarak bir ilerleme çubuğu gibi.
- **Etiket**, kütüphanenin ipucu mekanizmasıyla, işaretçinin kendi hücresine bağlı olarak çizilir ve üzerine gelme gecikmesinden sonra değil, hemen görünür: bileşeni açıklamak yerine işaretçinin nerede olduğunu adlandırır.

## Yaygın hatalar

- **Konumu saniyeye yuvarlamak.** Çubuk kesri gönderir, zamanı değil; çizmeden önce tam saniyeye yuvarlayan bir oynatıcı düzgün sıralayamaz.
- **Uygulamanın yok saydığı bir mesajı göndermek.** Çubuk kendini hareket ettirmez, bir kişinin nereye gitmek istediğini söyler. `update` içinde taşıyın, yoksa işaretçi ilerlerken dolgu yerinde kalır.
- **Çıplak kesir için etiket.** "0.42" bir kişinin tanıdığı hiçbir şeyi adlandırmaz; parçanın zamanını yazın.
- **Çok dar çubuk.** Her hücre bir adımdır; on hücrenin altında bir sürükleme sıçrar.