## Ne zaman kullanılır

Uygulaman zamanı ölçüyor ya da saklıyorsa bunları kullan: birinin ne kadar çalıştığı, bir kaydın hangi güne yazıldığı, bir dosyaya yazılan tarih. Hiçbiri bir bileşene bağlı değildir; `Date` ve `TimeOfDay` tarih seçicinin ve saat girişinin sana verdiği, senin de kendi durumunda tuttuğun sade değerlerdir.

Dört soru, dört cevap:

- **Bu ne kadar süredir çalışıyor?** `Uptime`, çünkü çalışılan süreyi makinenin uyuduğu süreden ayırır.
- **Burada hangi gün?** `Date::today_local()` ve `DateTime::now_local()`, çünkü duvar saati tek başına saat dilimini bilmez.
- **Başında biri var mı?** `ui.idle_for()` ve `ui.on_idle`, çünkü uygulama tuşları kendisi hiç görmez.
- **Dosyada ne yazıyor?** `Date::parse` ve `Display`, çünkü saklanan tarih metindir.

## Adım adım

1. İş başlarken bir ölçüm al: `let started = Uptime::now();`.
2. Sonra bir ölçüm daha al ve farkı sor: `now.awake - started.awake` çalışılan süredir, `now.suspended_since(&started)` makinenin uyuduğu süre.
3. Cevap bilinmiyorsa kullanıcıya söyle: `Uptime::detects_suspend()` Linux ve Android dışında `false` döner; orada uykuda geçen süre uydurulmaz, sıfır kalır.
4. Bir kaydın günü için `Date::today_local()`, bir an için `DateTime::now_local()` kullan; ikincisi farkı da yanında taşır.
5. Bir anı `to_unix()` ile sakla, `DateTime::from_unix(saniye, fark)` ile geri oku. Bir günü `date.to_string()` ile sakla, `Date::parse` ile geri oku.

6. Klavyenin başında kimsenin olmadığını fark etmek için `view` içinde bir izleyici tanımla: `ui.on_idle(Duration::from_secs(300), Msg::Away)`. Beş dakika girdi gelmeyince `update` `Msg::Away(true)`, ilk tuşta ya da imleç hareketinde `Msg::Away(false)` alır. Sessizliği göstermek için `ui.idle_for()` oku.

## Nasıl çalışır

- **Duvar saati değil, iki tekdüze saat.** Duvar saati iki kez yanıltır: saat düzeltmesi kimse uyumadan saati kaydırır; tekdüze saati uyku boyunca işleyen bir platformda ise duvar saati de aynı kadar ilerler, fark sıfır çıkar ve sekiz saatlik uyku sekiz saat çalışma sayılır. Uykuda duran `CLOCK_MONOTONIC` ile durmayan `CLOCK_BOOTTIME` arasındaki fark tanım gereği uykudur ve hiçbir saat düzeltmesi ona dokunmaz.
- **Cevap veremeyen platform bunu söyler.** Bu çifti güvenli koddan yalnızca Linux ve Android sunar; diğerlerinde `elapsed`, `awake`'e eşittir, uyku süresi sıfır kalır ve `Uptime::detects_suspend()` `false` döner. Kullanıcıya kimsenin ölçmediği bir sayı değil, bunu göster.
- **Tek bir ölçüm tek başına bir şey söylemez.** `awake` ve `elapsed` makinenin açılışından sayar; platform açılış saati sunmuyorsa süreçteki ilk ölçümden. Başladığın ölçümü sakla ve karşılaştır.
- **Yerel fark sistemden gelir.** `TZ`'nin gösterdiği ya da `/etc/localtime` olan saat dilimi dosyasından süreç başına bir kez okunur; dosya dilimin bütün fark değişimlerini tuttuğu için yaz saati bizim bir kuralımız olmadan doğru çıkar. Böyle bir dosya yoksa `local_offset()` `None` olur; `local_offset_minutes()` o zaman 0, yani UTC der, bu yüzden ayrım önemliyse `local_offset()` sor.
- **`DateTime` farkını yanında taşır.** İçindeki tarih ve saat duvardaki saatin gösterdiğidir; `offset_minutes` onları yeniden bir ana çevirendir. Farkları başka olan iki `DateTime` aynı an olabilir; oyun alanı bunu gösteriyor.
- **ISO tarihleri takvime karşı denetlenir.** `Date::parse` `YYYY-MM-DD` ister; 2026-13-40 ve 2026-02-30 reddedilir. 29 Şubat 2024'te ve 2000'de vardır, 2026'da ve 2100'de yoktur. `Display` aynı biçimi geri yazar, böylece metin geldiği gibi gider.

- **Boşta kalma çalışma zamanının saatidir.** Uygulama tuşları ve fareyi hiç görmez; çalışma zamanı görür ve her tuşun (basma, tekrar, bırakma), imlecin pencere üzerinde gezinmesi dahil her fare olayının, her yapıştırmanın ve bir devrin bitişinin anını not eder. Yeniden boyutlandırma kullanıcı değildir: pencere yöneticisi başında kimse olmayan pencereleri de boyutlandırır. Hiç girdi gelmeden önce sessizlik başlangıçtan sayılır.
- **Uygulama yoklanmaz, uyandırılır.** `on_idle(after, …)` sessizliğin `after`'a ulaştığı ana tek bir uyanma koyar; arada hiçbir şey çizilmez. `idle_for()` okuyan bir görünüm, okuduğu sürece her tam saniyede yeniden çizilir, böylece ekrandaki sayı hiç bayatlamaz; okumayı bırakınca çizim de durur. Bu sayfadaki demo ikisini de yapar.

## Sık yapılan hatalar

- **Çalışmayı duvar saatiyle ölçmek.** `SystemTime` sistem saati düzeltilince kayar; doksan saniyelik bir düzeltme doksan saniyelik çalışmayı sessizce siler.
- **Bilinmeyen farkı UTC saymak.** Sistem bir şey söylemediğinde `local_offset_minutes()` 0 döner. Uygulaman cevabı bir yere yazıyorsa önce `local_offset()` sor.
- **Yerel saati farkı olmadan saklamak.** `09:00` diye yazılmış bir saat, hangi `09:00` olduğunu fark söylemeden bir an değildir. `to_unix()` sakla ya da farkı yanında sakla.
- **"Bugün" için UTC'ye güvenmek.** Gece yarısına yakın UTC günü ile yerel gün başka günlerdir; bu, bir gün geç dosyalanmış bir kayıttır.
- **Tarihi elle yazmak.** `format!("{:04}-{:02}-{:02}", …)` zaten `Display`'in yaptığıdır, `Date::parse` de onu geri okuyandır.
- **Boşta kalmayı kendi zamanlayıcınla ölçmek.** Her saniye tıklayan bir görev tuşlardan habersizdir; `on_idle` sessizliği öğrenir ve beklerken uygulamayı uyanık tutmaz.
- **İzleyiciyi yalnızca beklerken tanımlamak.** `on_idle`'ı izleyicinin yaşaması gereken her karede tut; yoksa girdi geri geldiğinde ona haber verilmez.
