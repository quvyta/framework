## Ne zaman kullanılır

Bir değişikliği gözle takip etmek zor olacaksa hareket kullan: topuzun öbür tarafa geçmesi, açılır listenin açılması, bir değerin artması. Hareket *bir şeyin nereye gittiğini* anlatır; asla süs değildir.

Çoğu zaman hiç yazmadan hareket kullanırsın: Select açılırken, Spinner dönerken ve ProgressBar süpürürken bunu kendileri yapar. Bu sayfa hareket eden kendi bileşenini yazmak içindir.

## Adım adım

1. Bileşeninin `paint` metodunda göstermek istediğin değeri iste: `let x = cx.animate("knob", hedef, süre, Easing::EaseOut);`. İlk seferde doğrudan `hedef` döner; hedef sonra değişirse olduğu yerden oraya süzülür.
2. Değeri hücrelere çevir. Terminal tam hücrelerle hareket eder; bu yüzden yukarıdaki şeritlerdeki gibi rengi hücrenin kaplanma oranıyla karıştır: bir hücrenin yarısını kaplayan topuz o hücreyi iz ile topuz arasında yarı yarıya boyar.
3. Bir anda başlayan şeyler için (açılan bir katman) başlangıç zamanını `cx.now()` ile sakla ve `cx.progress_since(başlangıç, süre, yumuşatma)` kullan.
4. Tekrarlayan şeyler için (spinner, süpürme) yumuşak evreler için `cx.cycle(periyot)`, kare sayısı için `cx.ticks(aralık)` kullan.
5. Süreleri temadan al: `cx.env().theme().motion()`.

## Nasıl çalışır

- **Yalnızca hareket varken kare çizilir.** Her yardımcı, bir şey değişmeye devam ettiği sürece çalışma motorundan sonraki kareyi ister. Duran bir ekranın maliyeti yoktur.
- **Durum bileşenle yaşar.** `animate` ara değeri bileşenin belleğinde verdiğin isimle tutar; bir bileşen birden fazla değeri canlandırabilir.
- **Hareketi azaltma yerleşiktir.** Kullanıcı bunu açtığında (`QUVYTA_REDUCED_MOTION=1` ya da `Command::set_reduced_motion(true)`) `animate` ve `progress_since` bitiş durumunu döndürür, `cycle` ve `ticks` 0'da durur ve kare istenmez.
- **Seçimi hatırla.** Hareketi azaltma bir kullanıcı tercihidir: değişince `Settings::REDUCED_MOTION` ile sakla; `Runtime::settings` bir dahaki açılışta onu ilk kareden önce uygular. Kullanıcının kabuğundaki `QUVYTA_REDUCED_MOTION` kayıtlı ya da çalışırken verilen her seçimden güçlüdür: ayar ne derse desin `1` hareketi azaltılmış, `0` açık tutar; erişilebilirlik ayarları da böyle çalışır. Tanımsız ya da boşsa kayıtlı seçim karar verir. Bu sayfadaki anahtar da Tema, ikon, dil sayfasındaki anahtar da aynı ayarı saklar; biri neyi seçtiyse diğeri onu gösterir.
- **Karar ortamdaysa bunu göster.** `QUVYTA_REDUCED_MOTION` karar verirken `Env::reduced_motion_forced()` doğru döner. O sırada ayar ekranındaki anahtar basılınca geri döner; bu yüzden anahtarı pasif yap, `Env::reduced_motion()` değerini göstersin ve altına nedenini söyleyen silik bir satır ekle. Buradaki iki anahtar da böyle yapar: "QUVYTA_REDUCED_MOTION ortam değişkeni belirliyor." Pasif anahtar mesaj göndermez; hiçbir şey değişmez, kaydedilmez.
- **Yumuşatma.** Gelen şeyler için `EaseOut`, giden şeyler için `EaseIn`, gidip gelen şeyler için `EaseInOut`, döngüler için `Linear` doğrudur.

## Sık yapılan hatalar

- **Koda sabit süre yazmak.** Temanın sürelerini kullan; böylece bir tema bütün uygulamayı daha sakin ya da daha çevik yapabilir.
- **Karıştırmadan tam hücrelerle hareket.** Kesik kesik görünür. Kenar hücreleri kaplanan oranla karıştır.
- **Ortamı görmezden gelen hareketi azalt anahtarı.** `QUVYTA_REDUCED_MOTION` karar verirken canlı bir anahtar değişir, kaydeder ve nedensiz geri döner. `Env::reduced_motion_forced()` değerine bak.
- **Elle yazılan döngülerde hareketi azaltmayı unutmak.** Saati kendin okumak yerine `cycle` ve `ticks` kullan; bu ayara zaten uyar.
