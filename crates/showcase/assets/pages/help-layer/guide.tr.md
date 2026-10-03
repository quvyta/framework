## Ne zaman kullanılır

Her uygulamaya bir yardım katmanı koy. İnsanlar bir tuşu unutunca `?` tuşuna basar; katman, uygulamanın çalıştığı kısayol haritasının kendisinden cevap verir, bu yüzden hiç eskimez. Yalnızca o ekranda anlamı olan tuşlar da listelenir: odaklı bileşen hangilerini aldığını kendisi söyler, kalanı için uygulama ipucu ekler.

## Adım adım

1. Açık olup olmadığını tut: `help_open: bool`.
2. `?` tuşuna bağlı genel `help` eyleminden aç: `App::action` içinde `"help"` eylemini `Msg::Help(true)` mesajına çevir.
3. `view` içinde açıkken ekle: `ui.add(HelpLayer::new(Msg::Help(false)))`.
4. Bileşenlerinin tuşlarını sen yazma: tablo, liste, ağaç ya da alan aldığı tuşları kendisi bildirir.
5. Hangi bileşenin de almayan, o ekrana özgü tuşlar için ipucu ekle; örneğin seçili satırı yeniden baştan başlatan bir uygulamanın `r` tuşu: `.hint("r", t!("hints.restart"))`.
6. Kendi eylemlerinin etiketlerini dil dosyalarında `[keys]` altına yaz; framework eylemlerinin etiketleri hazırdır.

## Nasıl çalışır

- **Üç grup.** "Bu ekran" önce odaklı bileşenin tuşlarını, sonra senin ipuçlarını listeler; "Uygulama" `[app]` eylemlerini, "Genel" framework'ün `[global]` eylemlerini. Boş gruplar gizlenir.
- **Odaklı bileşen kendini doldurur.** Odakta olan bileşen çizilirken hangi tuşları aldığı sorulur ve bunlar "Bu ekran"ın başına konur. Kendi bileşenin `Widget::keys` ile cevap verir.
- **Sorulan, klavyenin geldiği bileşendir.** Katman klavyeyi kendine alır; listelenen tuşlar katman açıldığı anda odakta olan bileşenin tuşlarıdır, katmanın kendi süzgecinin değil. Yardımı tablo odaktayken bir tuşa basarak aç; onu açan bir butona tıklamak odağı butonda bırakır, buton da hiçbir şey bildirmez.
- **Gerçekten var olan tuşlar.** Satır açılmayan bir tablo Enter'dan söz etmez, işaret kutusu olmayan bir liste Boşluk'tan söz etmez, açılıp kapanmayan bir ağaç yan oklardan söz etmez. Bir yetenek açıldığında tuşu da gelir.
- **Tuşlar tuş gibi görünür.** Her birleşim yükseltilmiş bir yüzeyde durur, etiket daha sakin bir renkte gelir; birden çok tuşu olan bağlantılar yan yana gösterilir.
- **Yazarak süz.** Her harf listeyi etiketler ve tuşlar üzerinde bulanık eşleşmeyle daraltır; eşleşen karakterler vurgu rengini alır.
- **Uzun listeler kayar.** Oklar, PgUp/PgDn ve tekerlek kaydırır; kaydırma çubuğu yalnızca gerektiğinde çıkar, fareyle basılıp sürüklenebilir.
- **Modal bir katmandır.** Ekran kararır, katmanın sol kenarı boyunca bir çubuk uzanır, odak içeride kalır, uygulama kısayolları durur, kapanınca odak geri döner.
- **Kapatılabilir demek Esc ve × birlikte demek.** Hem Esc hem sağ üst köşedeki × işareti kapatma mesajını gönderir. `.dismissable(false)` ikisini birden kapatır ve işareti gizler; katmanı uygulamanın kendisi kapatacaksa, örneğin birkaç saniye ekranda kalan bir ilk açılış turunda.
- **Her seferinde temiz açılır.** Yeniden açınca süzgeç boştur, liste en baştadır.

## Sık yapılan hatalar

- **Ayrı bir yardım metni yazmak.** Gerçek tuşlardan kopar; katmanı kısayol haritası ve bileşenler doldursun.
- **Bileşenin tuşları için ipucu eklemek.** Zaten listelenirler; bir bileşenin artık almadığı bir tuş için yazılan ipucu, katmanın göremediği bir yalandır.
- **Kimsenin kapatamadığı katman.** `.dismissable(false)` verdiysen katmanı uygulama kaldırmalı; ekrandaki hiçbir şey kaldırmaz.
- **Eksik etiketler.** `keys.<eylem>` girdisi olmayan eylem, anahtar yolunu `⟦keys.<eylem>⟧` biçiminde gösterir; dil testleri bunu yakalar.
