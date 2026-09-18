## Metotlar

- `Command::confirm(soru)` — soruyu gösterir, cevabın mesajını iletir.
- `Confirm::new(başlık, onay_mesajı)` — soru ve onaylayınca gönderilecek mesaj.
- `.message(metin)` — başlığın altında açıklama.
- `.danger()` — sol kenar boyunca tehlike renginde çubuk ve tehlike renginde onay butonu.
- `.confirm_label(metin)` — onay butonu. Varsayılan: `quvyta.confirm.confirm`.
- `.cancel_label(metin)` — vazgeç butonu. Varsayılan: `quvyta.confirm.cancel`.
- `.on_cancel(mesaj)` — Vazgeç, Esc ve × işaretinde gönderilir. Yoksa vazgeçmek yalnızca kapatır.
- `.dismissable(bool)` — Esc ve × işareti birlikte vazgeçer mi. Varsayılan: `true`. `false` ile ikisi de çalışmaz, işaret gizlenir, yalnızca butonlar cevaplar.
- `.alternative(etiket, mesaj)` — Vazgeç ile onay butonu arasında üçüncü bir buton; `mesaj` gönderen düz bir butondur. Verilmezse pencere iki butonuyla eskisinin aynısıdır.
- `.require_word(sözcük)` — onay butonu çalışmadan önce kullanıcı `sözcük`ü bir alana yazar. Önce alan odaklıdır; metin eşleşene kadar onay butonu pasiftir. Eşleşmede baştaki ve sondaki boşluklar ile büyük-küçük harf yok sayılır; `İ`, `I`, `ı` ve `i` aynı harftir. Üçüncü yol beklemez. Boş sözcük hiçbir şey istemez.

## Tuşlar

- `enter` / `space` odaklı butonla cevaplar; önce Vazgeç odaklıdır.
- `tab` / `shift tab` butonlar arasında soldan sağa geçer: Vazgeç, varsa üçüncü yol, onay butonu.
- `esc` soru kapatılabilirken vazgeçer.
- Yazılacak bir sözcük varsa önce alan odaklıdır; Tab alanı, Vazgeç'i, üçüncü yolu ve onay butonunu gezer, beklerken onay butonunu atlar. Alanda `enter` sözcük eşleşince onaylar, önce bir şey yapmaz; alanın düzenleme tuşları, yapıştırma ve düzenleme menüsü yerindedir.

## Fare

- Butona tıklamak cevaplar. × işaretine tıklamak vazgeçer; üç hücresi fare üstüne gelince aydınlanır. Karartılmış ekrana tıklamak yok sayılır. Sözcüğünü bekleyen onay butonu tıklamayı yok sayar.

## Davranış

- Çalışma motoru pencereyi uygulamanın görünümünden sonra çizer; uygulamanın çizdiği her şeyin üstündedir.
- Birden çok soru üst üste biner; önce en yenisi gösterilip cevaplanır, sonra sıradaki.
- Soru açıkken uygulama kısayolları durur; sonra odak geri döner.
- `Msg` için `Clone` gerekmez: her mesaj bir kez iletilir.

## Tema anahtarları

- Pencere `Modal` anahtarlarını kullanır: `modal`, `modal.danger`, `modal-title`, `close-mark`, `layer-backdrop`, `layer-hint-key`, `layer-hint-label`.
- Butonlar: yıkıcı sorularda `button.danger`, diğerlerinde `button.primary`; sözcük henüz yazılmamışken `button:disabled`.
- Sözcük satırı `secondary` tipografi rolünü, sözcüğün kendisi `title` rolünü kullanır; alanı `text-input` anahtarlarını.
- Dil — `quvyta.confirm.confirm`, `quvyta.confirm.cancel`, `quvyta.confirm.type-word` (`{word}` ile), `quvyta.layer.close`, `quvyta.layer.switch`.
