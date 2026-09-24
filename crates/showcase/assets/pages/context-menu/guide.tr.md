## Ne zaman kullanılır

İmlecin altındaki şeyle ilgili eylemler için bağlam menüsü kullan: bir container'ı yeniden başlatmak, bir kimliği kopyalamak, bir dosyayı silmek. Eylemler yalnızca istenince göründüğü için satırlar sade kalır. İçindeki her eyleme başka bir yoldan da (bir tuş, bir araç çubuğu butonu) ulaşılabilmeli; herkes sağ tıklamaz.

## Adım adım

1. Öğeleri kur: `ContextItem::new(t!("restart"), Msg::Restart)`; gerekirse `.icon("…")`, `.shortcut("ctrl r")`, `.detail(t!("no-shell"))`, `.disabled(true)` ya da `.danger(true)` ekle.
2. İlgili eylemleri `ContextItem::gap()` ile grupla, `ContextItem::submenu(t!("move"), [...])` ile iç içe koy.
3. Alanı sar: `ui.add_with(ContextMenu::new(items), |ui| { … })`.
4. Mesajları `update` içinde, herhangi bir butonunkiler gibi işle.

## Nasıl çalışır

- **Sağ tık menüyü imlecin yanında açar**, tıklanan hücrenin bir satır altında. Shift+F10 ya da menü tuşu menüyü alandaki odaklı bileşenin altında, ilk eylem vurgulu olarak açar.
- **Katman yüzeyinde bir katmandır**; `motion.enter` süresince açılır, yer yoksa üste ya da sola geçer. Gruplar çizgiyle değil, boş bir satırla ayrılır.
- **Satırlar liste satırı gibi davranır.** Vurgulanan satır yüzeyini yükseltir, çubuğu gösterir ve etiketini bir hücre kaydırır; kısayollar ve alt menü oku silik tonda sağ kenarda sabit kalır.
- **Satır nedenini söyleyebilir.** `.detail(…)` sağa, sönük tonda kısa bir not koyar; kısayol ya da ok varsa onlardan önce: pasif bir satır sebepsiz gri durmak yerine `Terminale bağlan   imajda kabuk yok` diye okunur. Menü notu sığdıracak kadar genişler; ekran dar kalırsa önce not kesilir, sonra hiç gösterilmez, etiket bütün kalır.
- **Klavye içeride kalır.** ↑ ve ↓ gezinir, boşlukları ve pasif satırları atlar; Home ve End uçlara atlar; bir harf o harfle başlayan sonraki satıra atlar; →, Enter ya da Boşluk alt menüyü açar, ← ya da Esc kapatır; Enter ya da Boşluk seçer.
- **Fare de çalışır.** Alt menü satırının üstüne gelmek onu açar, bir satıra tıklamak onu seçer; boşluğa ya da pasif satıra tıklamak bir şey yapmaz. Başka herhangi bir yere basmak menüyü kapatır ve basış yine de düştüğü yere ulaşır; tek tıklama hem menüyü kapatır hem de hedeflediğin şeyi yapar. Alanın içinde yeniden sağ tıklamak menüyü yeni yerde açar.
- **Yıkıcı eylemler işaretlenir**: tehlike rengiyle çizilir ve bir ikon taşımalıdır; renk hiçbir zaman tek işaret olmaz.
- **Metnin kendi menüsü vardır.** Metin alanları Kes, Kopyala, Yapıştır ve Tümünü seç; seçili metin Kopyala ve Ham kopyala menüsünü açar. İkisi de bu aynı menüyle çizilir, seninkilerle birebir aynı görünür ve davranır.

## Sık yapılan hatalar

- **Eylemleri yalnızca menüye koymak.** Sık kullanılan eylemlere bir tuş ya da buton da ver.
- **Kısayol etiketinin tuşu bağladığını sanmak.** `.shortcut(…)` yalnızca gösterir; tuşu kısayol haritasında bağla.
- **Notu kısayol yuvasına yazmak.** `.shortcut(…)` tuş adları içindir; bir sebep `.detail(…)`'e gider ve tuştan önce durur.
- **Derin iç içe menüler.** Tek seviye alt menü yeter; fazlası terminalde fareyle zor yönlendirilir.
