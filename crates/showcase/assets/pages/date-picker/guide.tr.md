## Ne zaman kullanılır

Kullanıcı tek bir gün seçecekse ve çevresindeki haftaları görmek işe yarıyorsa tarih seçici kullan: bir yayın tarihi, bir bakım aralığı, bir raporun başlangıcı. Doğum günü gibi çok eski tarihlerde yazmak daha hızlıdır; günün saati için saat alanı kullan.

## Adım adım

1. Değeri durumunda `Option<Date>` olarak tut; tarihleri `Date::new(2026, 9, 16)` ile kur.
2. Göster: `DatePicker::new(self.release)`.
3. Seçimleri al: `.on_change(Msg::Release)`.
4. Boşken ne seçileceğini söyle: `.placeholder(t!("choose-date"))`.
5. Uygulaman kullanıcının saat dilimini biliyorsa yerel günü `.today(tarih)` ile ver; yoksa bugün, sistem saatinden UTC olarak alınır.

## Nasıl çalışır

- **Alan bir açılır liste gibi görünür** ve tarihi dilin yazdığı gibi yazar: İngilizcede "September 16, 2026", Türkçede "16 Eylül 2026".
- **Takvim bir katmandır**; katman yüzeyinde, `motion.enter` süresince alanın altında, yer yoksa üstünde açılır.
- **Her zaman altı hafta.** Izgara aydan aya boyunu korur. Seçili gün vurgu rengiyle dolar, bugün vurgu renginde kalın ve altı çizili rakamlarla işaretlenir, komşu ayların günleri siliktir, vurgulanan gün yüzeyini yükseltir.
- **Haftayı dil belirler.** Ay ve gün adları ile haftanın ilk günü `quvyta.date` anahtarlarından gelir: İngilizcede hafta pazar, Türkçede pazartesi başlar. Kendi dil dosyaların bunları değiştirebilir.
- **Tuşlar:** ← ve → bir gün, ↑ ve ↓ bir hafta, PgUp ve PgDn bir ay, Shift ile birlikte bir yıl ilerletir; Home ve End haftanın uçlarına gider; Enter ya da Boşluk seçer; Esc kapatır. Ay adının yanındaki oklar (her biri fare üstüne gelince aydınlanan üç hücre) ve fare tekerleği ayı değiştirir; bir güne tıklamak onu seçer.
- **Tek vurgulu gün.** Klavye de fare de aynı vurguyu taşır. Komşu ayın bir gününün üstüne gelmek takvimi çevirmeden o günü aydınlatır; tuşlar, gösterilen ayda en son üstüne geldiğin günden devam eder.
- **Çubuk üstüne geldiğin şeyi gösterir, hiçbir şey kaymaz.** Alan, ay okları ve vurgulu gün en soldaki hücrelerinde `▌` çubuğunu gösterir. O hücre hep boştur (bir gün dört hücredir: boşluk, iki rakam, boşluk), bu yüzden ayarlardan kaydırma açık olsa bile ne yazı ne rakam yer değiştirir. Çubuk fare altındayken yumuşaktır; vurguyu en son klavye taşıdıysa nefes alır. Seçili gün vurgu rengi dolgusunu korur, vurgulanınca üstünde koyu bir çubuk belirir.
- **Başka yere tıklamak takvimi kapatır, tıklama da boşa gitmez**: bir butona, bir menü öğesine ya da başka bir alana. Tarih alanının kendisine tıklamak yalnızca kapatır.
- **Tarihler sade değerlerdir.** `Date` saat ya da dilim taşımaz. Gün ve ay ekler (31 Ocak artı bir ay, şubatın son günüdür), haftanın gününü söyler ve `to_days` ile iki tarih arasındaki günleri sayar.

## Sık yapılan hatalar

- **Biçimlenmiş metni saklamak.** `Date`'i tut; metin dille birlikte değişir.
- **Gece yarısına yakın "bugün" için UTC'ye güvenmek.** Yerel gün önemliyse `.today(…)` ver.
- **Takvimin liste gibi kaymasını beklemek.** Kaydırma listeler, menüler ve sekmeler içindir. Tarih seçici sabit sütunlu bir ızgaradır; hiç kaymaz, yerine çubuğu gösterir.
- **Aralık için tek seçici beklemek.** Demodaki gibi iki seçici kullan; sırayı `update` içinde denetle.
