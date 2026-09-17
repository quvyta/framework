## Metotlar

- `Wizard::new(etiketler)` — `etiketler` adlı adımlar, ilk adımda, butonlara mesaj bağlı değil.
- `.current(sıra)` — güncel adım.
- `.on_back(mesaj)`, `.on_next(mesaj)`, `.on_finish(mesaj)` — buton mesajları.
- `.on_cancel(mesaj)` — solda Vazgeç butonu; sihirbaz içindeki Esc de aynı mesajı gönderir.
- `.on_step(|sıra| mesaj)` — bitmiş adımlar seçilebilir.
- `.busy(bool)` — İleri ya da Bitir bir spinner gösterir ve basışları yok sayar; Geri ve Vazgeç pasifleşir; güncel adım nefes alır.
- `.page_height(satır)` — her sayfa tam `satır` satır alır.
- `.show(ui, |ui| …)` — sihirbazı güncel adımın sayfasıyla ekler.

## Davranış

- Yerleşim: adımlar, bir boş satır, sayfa, bir boş satır, butonlar (Vazgeç solda; Geri ve İleri sağda).
- Geri ikinci adımdan itibaren görünür; son adımda İleri, Bitir yazar ve `on_finish` gönderir.
- Adlı bileşenler: `wizard-steps`, `wizard-page`, `wizard-cancel`, `wizard-back`, `wizard-next`.
- Esc yalnızca odak sihirbazın içindeyken ve `on_cancel` verilmişken işlenir.

## Tema anahtarları

- Sihirbaz `Steps` ve `Button` (`button.primary`) stilleriyle çizilir; kendi anahtarı yoktur.
- Dil — `quvyta.wizard.cancel`, `quvyta.wizard.back`, `quvyta.wizard.next`, `quvyta.wizard.finish`.
