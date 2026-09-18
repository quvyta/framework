## Ne zaman kullanılır

Geri alması zor bir işten önce onay iste: canlı ortamdaki bir container'ı durdurmak, projeyi silmek, kaydedilmemiş değişiklikleri atmak. Kolayca geri alınan şeyler için sorma; geri alma, sorudan daha naziktir. Pencerenin evet ya da hayırdan fazlasına, örneğin bir alana ihtiyacı varsa `Modal` ile kur.

## Adım adım

1. Soruya kendi mesajını ver, örneğin `Msg::AskStop(id)`, ve butondan gönder.
2. `update` içinde soruyu döndür: `Command::confirm(Confirm::new(t!("stop.title"), Msg::Stop(id)))`.
3. Ne olacağını anlat: `.message(t!("stop.body"))`.
4. Yıkıcı işlerde `.danger()` ekle ve butona işin adını yaz: `.confirm_label(t!("stop"))`.
5. Vazgeçmek bir şey yapmalıysa (log yazmak, seçimi sıfırlamak) `.on_cancel(Msg::Kept(id))` ekle.
6. `Msg::Stop(id)` mesajını her zamanki gibi işle; yalnızca kullanıcı onaylayınca gelir.

## Nasıl çalışır

- **Uygulamanda pencere durumu yok.** Bekleyen soruyu çalışma motoru tutar ve pencereyi görünümünün üstüne çizer; `update` yalnızca cevabı görür.
- **Odak güvenli cevaptadır.** Pencere açılınca Vazgeç odaklıdır; aceleyle basılan Enter hiçbir şeyi bozmaz. Tab onay butonuna geçer.
- **Esc ve × vazgeçer, dışarı tıklamak bir şey yapmaz.** Kapatılabilir demek ikisi birden demek: hem Esc hem sağ üst köşedeki × işareti vazgeçer. `.dismissable(false)` ikisini birlikte kaldırır; yalnızca butonla cevaplanması gereken sorular için. Karartılmış ekrana kazara bir tıklama soruyu cevaplamaz.
- **Her pencere gibi görünür.** `Modal` ile aynı katman: karartılmış ekran, hafif beliriş, sol kenar boyunca bir çubuk (yıkıcı sorularda tehlike renginde) ve tehlike renginde onay butonu.
- **İki yol yetmeyince üçüncü yol.** Kurtarılan iş "At · Sürdür · Kaydet" diye sorar: `.alternative(etiket, mesaj)` Vazgeç ile onay butonu arasına düz bir buton koyar. Odak yine Vazgeç'tedir, Tab önce üçüncü yola, en son onay butonuna gider; Esc ve × yine vazgeçer.
- **Sorular üst üste biner.** Bir soru açıkken yeniden sormak yenisini üste koyar; önce en yenisi cevaplanır.
- **Odak geri döner**, sorudan önce onu tutan bileşene.

## Sık yapılan hatalar

- **Belirsiz butonlar.** "Durdur" ve "Çalışsın" ne olacağını söyler; "Tamam" söylemez.
- **Her şey için sormak.** İnsanlar okumadan Enter'a basmayı öğrenir. Soruları gerçek kayıplar için sakla.
- **Evet-hayır sorusuna üç yol.** Üçüncü buton gerçek bir üçüncü sonuç içindir, iki kez söylenmiş bir "Vazgeç" için değil.
- **İşi cevaptan önce yapmak.** İşi sorarken değil, onay mesajının işlendiği yerde yap.
