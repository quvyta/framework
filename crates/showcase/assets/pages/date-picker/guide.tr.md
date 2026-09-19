## Ne zaman kullanılır

Kullanıcı tek bir gün seçecekse ve çevresindeki haftaları görmek işe yarıyorsa tarih seçici kullan: bir yayın tarihi, bir bakım aralığı, bir raporun başlangıcı. Doğum günü gibi çok eski tarihlerde yazmak daha hızlıdır; günün saati için saat alanı kullan.

## Adım adım

1. Değeri durumunda `Option<Date>` olarak tut; tarihleri `Date::new(2026, 9, 16)` ile kur.
2. Göster: `DatePicker::new(self.release)`.
3. Seçimleri al: `.on_change(Msg::Release)`.
4. Boşken ne seçileceğini söyle: `.placeholder(t!("choose-date"))`.
5. Bugün, makinenin kendi gününe göre işaretlenir: sistem saati ve saat dilimi. Başka bir gün bugün sayılmalıysa, örneğin başka bir dilimdeki kullanıcının günü, `.today(tarih)` ver.

## Nasıl çalışır

- **Alan bir açılır liste gibi görünür** ve tarihi dilin yazdığı gibi yazar: İngilizcede "September 16, 2026", Türkçede "16 Eylül 2026".
- **Takvim bir katmandır**; katman yüzeyinde, `motion.enter` süresince alanın altında, yer yoksa üstünde açılır.
- **Her zaman altı hafta.** Izgara aydan aya boyunu korur. Seçili gün vurgu rengiyle dolar, bugün vurgu renginde kalın ve altı çizili rakamlarla işaretlenir, komşu ayların günleri siliktir, vurgulanan gün yüzeyini yükseltir.
- **Haftayı bölge, bölge yoksa dil belirler.** Ay ve gün adları `quvyta.date` anahtarlarından gelir. Haftanın ilk günü `I18n::first_weekday()`'den gelir: sistem bir ülke söylüyorsa (`LANG=en_GB.UTF-8`) o ülkenin Unicode CLDR'deki günüdür, Birleşik Krallık'ta pazartesi, ABD'de pazar; söylemiyorsa dilin `quvyta.date.first-weekday` anahtarıdır, İngilizcede pazar, Türkçede pazartesi, bu anahtarı vermeyen bir dilde pazartesi. Uygulaman haftaları nerede sayıyorsa aynı metodu çağır; böylece haftalık hedef ile takvim aynı günden başlar.
- **Uygulama bölgeyi kendisi söyleyebilir.** `Command::set_locale("en-GB")` İngilizceye ve İngiliz haftasına birlikte geçer; `tr` gibi bölgesiz bir kod sistemin verdiği bölgeyi korur. `Command::set_region(Some("GB"))` yalnızca bölgeyi değiştirir, `Command::set_region(None)` haftayı yeniden dile bırakır.
- **Tuşlar:** ← ve → bir gün, ↑ ve ↓ bir hafta, PgUp ve PgDn bir ay, Shift ile birlikte bir yıl ilerletir; Home ve End haftanın uçlarına gider; Enter ya da Boşluk seçer; Esc kapatır. Ay adının yanındaki oklar (her biri fare üstüne gelince aydınlanan üç hücre) ve fare tekerleği ayı değiştirir; bir güne tıklamak onu seçer.
- **Tek vurgulu gün.** Klavye de fare de aynı vurguyu taşır. Komşu ayın bir gününün üstüne gelmek takvimi çevirmeden o günü aydınlatır; tuşlar, gösterilen ayda en son üstüne geldiğin günden devam eder.
- **Çubuk üstüne geldiğin şeyi gösterir, hiçbir şey kaymaz.** Alan, ay okları ve vurgulu gün en soldaki hücrelerinde `▌` çubuğunu gösterir. O hücre hep boştur (bir gün dört hücredir: boşluk, iki rakam, boşluk), bu yüzden ayarlardan kaydırma açık olsa bile ne yazı ne rakam yer değiştirir. Çubuk fare altındayken yumuşaktır; vurguyu en son klavye taşıdıysa nefes alır. Seçili gün vurgu rengi dolgusunu korur, vurgulanınca üstünde koyu bir çubuk belirir.
- **Başka yere tıklamak takvimi kapatır, tıklama da boşa gitmez**: bir butona, bir menü öğesine ya da başka bir alana. Tarih alanının kendisine tıklamak yalnızca kapatır.
- **Tarihler sade değerlerdir.** `Date` saat ya da dilim taşımaz. Gün ve ay ekler (31 Ocak artı bir ay, şubatın son günüdür), haftanın gününü söyler ve `to_days` ile iki tarih arasındaki günleri sayar.

## Sık yapılan hatalar

- **Biçimlenmiş metni saklamak.** `Date`'i tut; metin dille birlikte değişir.
- **Bugünü `Date::today_utc()` ile işaretlemek.** Gece yarısına yakın UTC günü kullanıcının günü değildir; yerel gün için `.today(…)` hiç verme ya da `Date::today_local()` ver.
- **Takvimin liste gibi kaymasını beklemek.** Kaydırma listeler, menüler ve sekmeler içindir. Tarih seçici sabit sütunlu bir ızgaradır; hiç kaymaz, yerine çubuğu gösterir.
- **Aralık için tek seçici beklemek.** Demodaki gibi iki seçici kullan; sırayı `update` içinde denetle.
