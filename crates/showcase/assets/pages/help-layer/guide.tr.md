## Ne zaman kullanılır

Her uygulamaya bir yardım katmanı koy. İnsanlar bir tuşu unutunca `?` tuşuna basar; katman, uygulamanın çalıştığı kısayol haritasının kendisinden cevap verir, bu yüzden hiç eskimez. Yalnızca o ekranda anlamı olan tuşlar için ekran ipuçları ekle.

## Adım adım

1. Açık olup olmadığını tut: `help_open: bool`.
2. `?` tuşuna bağlı genel `help` eyleminden aç: `App::action` içinde `"help"` eylemini `Msg::Help(true)` mesajına çevir.
3. `view` içinde açıkken ekle: `ui.add(HelpLayer::new(Msg::Help(false)))`.
4. Kısayol haritasında olmayan, o ekrana özgü tuşları ekle: `.hint("↑↓", t!("hints.move"))`.
5. Kendi eylemlerinin etiketlerini dil dosyalarında `[keys]` altına yaz; framework eylemlerinin etiketleri hazırdır.

## Nasıl çalışır

- **Üç grup.** "Bu ekran" ipuçlarını, "Uygulama" `[app]` eylemlerini, "Genel" framework'ün `[global]` eylemlerini listeler. Boş gruplar gizlenir.
- **Tuşlar tuş gibi görünür.** Her birleşim yükseltilmiş bir yüzeyde durur, etiket daha sakin bir renkte gelir; birden çok tuşu olan bağlantılar yan yana gösterilir.
- **Yazarak süz.** Her harf listeyi etiketler ve tuşlar üzerinde bulanık eşleşmeyle daraltır; eşleşen karakterler vurgu rengini alır.
- **Uzun listeler kayar.** Oklar, PgUp/PgDn ve tekerlek kaydırır; kaydırma çubuğu yalnızca gerektiğinde çıkar, fareyle basılıp sürüklenebilir.
- **Modal bir katmandır.** Ekran kararır, katmanın sol kenarı boyunca bir çubuk uzanır, odak içeride kalır, uygulama kısayolları durur, kapanınca odak geri döner.
- **Kapatılabilir demek Esc ve × birlikte demek.** Hem Esc hem sağ üst köşedeki × işareti kapatma mesajını gönderir. `.dismissable(false)` ikisini birden kapatır ve işareti gizler; katmanı uygulamanın kendisi kapatacaksa, örneğin birkaç saniye ekranda kalan bir ilk açılış turunda.
- **Her seferinde temiz açılır.** Yeniden açınca süzgeç boştur, liste en baştadır.

## Sık yapılan hatalar

- **Ayrı bir yardım metni yazmak.** Gerçek tuşlardan kopar; katmanı kısayol haritası doldursun.
- **Kısayol eylemleri için ipucu eklemek.** Zaten listelenirler; ipuçları haritanın bilmediği tuşlar içindir.
- **Kimsenin kapatamadığı katman.** `.dismissable(false)` verdiysen katmanı uygulama kaldırmalı; ekrandaki hiçbir şey kaldırmaz.
- **Eksik etiketler.** `keys.<eylem>` girdisi olmayan eylem, anahtar yolunu `⟦keys.<eylem>⟧` biçiminde gösterir; dil testleri bunu yakalar.
