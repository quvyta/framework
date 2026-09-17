## Ne zaman kullanılır

İnsanlar yardımı açmadan tuşları öğrensin diye ekranın altına bir kısayol çubuğu koy. Her bağlamayı değil, bu ekranda önemli olan birkaç tanesini göster.

## Adım adım

1. Ekranın kendisinin işlediği tuşlar için ipucu ekle: `.hint("↑↓", t!("hints.move"))`.
2. Kısayol eylemlerini adıyla ekle: `.action(Scope::App, "search")`. Tuşlar kısayol haritasından, etiket dilden gelir; yeniden bağlamak ya da çevirmek kod gerektirmez.
3. Birkaç global eylemi sağa sabitle: `.action_right(Scope::Global, "quit")`.

## Nasıl çalışır

- **Tuşlar küçük yüzeyler, etiketler siliktir.** Hiçbir şey parantez içinde değildir; tuşun yükseltilmiş tonu onun şeklidir.
- **Çubuk uyum sağlar.** Daraldığında soldaki ipuçları sondan başlayarak düşer; sağ grup her zaman kalır.
- **Kullanıcıyı takip eder.** Kısayol dosyasında bir bağlamayı değiştir ya da dili değiştir, çubuk güncellenir.

## Sık yapılan hatalar

- **Her şeyi listelemek.** Beş altı ipucu okunur; on beş okunmaz.
- **Yalan söyleyen ipuçları.** Tuşu elle yazmak yerine kısayol eylemleri için `.action` kullan; bağlama değişince ipucu da değişsin.
