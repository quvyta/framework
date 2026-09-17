## Metotlar

- `Command::confirm(soru)` — soruyu gösterir, cevabın mesajını iletir.
- `Confirm::new(başlık, onay_mesajı)` — soru ve onaylayınca gönderilecek mesaj.
- `.message(metin)` — başlığın altında açıklama.
- `.danger()` — sol kenar boyunca tehlike renginde çubuk ve tehlike renginde onay butonu.
- `.confirm_label(metin)` — onay butonu. Varsayılan: `quvyta.confirm.confirm`.
- `.cancel_label(metin)` — vazgeç butonu. Varsayılan: `quvyta.confirm.cancel`.
- `.on_cancel(mesaj)` — Vazgeç, Esc ve × işaretinde gönderilir. Yoksa vazgeçmek yalnızca kapatır.
- `.dismissable(bool)` — Esc ve × işareti birlikte vazgeçer mi. Varsayılan: `true`. `false` ile ikisi de çalışmaz, işaret gizlenir, yalnızca butonlar cevaplar.

## Tuşlar

- `enter` / `space` odaklı butonla cevaplar; önce Vazgeç odaklıdır.
- `tab` / `shift tab` iki buton arasında geçer.
- `esc` soru kapatılabilirken vazgeçer.

## Fare

- Butona tıklamak cevaplar. × işaretine tıklamak vazgeçer; üç hücresi fare üstüne gelince aydınlanır. Karartılmış ekrana tıklamak yok sayılır.

## Davranış

- Çalışma motoru pencereyi uygulamanın görünümünden sonra çizer; uygulamanın çizdiği her şeyin üstündedir.
- Birden çok soru üst üste biner; önce en yenisi gösterilip cevaplanır, sonra sıradaki.
- Soru açıkken uygulama kısayolları durur; sonra odak geri döner.
- `Msg` için `Clone` gerekmez: her mesaj bir kez iletilir.

## Tema anahtarları

- Pencere `Modal` anahtarlarını kullanır: `modal`, `modal.danger`, `modal-title`, `close-mark`, `layer-backdrop`, `layer-hint-key`, `layer-hint-label`.
- Butonlar: yıkıcı sorularda `button.danger`, diğerlerinde `button.primary`.
- Dil — `quvyta.confirm.confirm`, `quvyta.confirm.cancel`, `quvyta.layer.close`, `quvyta.layer.switch`.
