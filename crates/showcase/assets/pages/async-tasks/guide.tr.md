## Ne zaman kullanılır

Bir kareden uzun süren her iş için `Task` kullan: imaj derlemek, migration çalıştırmak, bir API'yi çağırmak, bir klasörü taramak. İş sürerken ekran çizilmeye ve tuşlara cevap vermeye devam eder.

- **`Command::perform`**, sonucu yalnızca bir mesaj olan kısa bir çağrı için yeterlidir.
- **`Task`**, kullanıcının izlediği işler içindir: ilerlemesini ve o anki adımı bildirir, iptal edilebilir ve bir sonuçla biter (bitti, bir sebeple başarısız oldu, iptal edildi).
- **`TaskList`**, çalışan ve biten işleri zaten tanıdığın Spinner ve ProgressBar ile gösterir.

## Adım adım

1. İşi yaz: `Task::new("İmajı derle", |cx| { ...; Ok(Msg::Olusturuldu(ozet)) })`. Başarısız olmak için `Err(sebep)` döndür.
2. İş ilerledikçe içeriden `cx.note("katmanlar gönderiliyor")` ve `cx.progress(0.4)` çağır; log satırları gibi başka her şey için `cx.send(mesaj)` kullan.
3. Beklemek için `if !cx.sleep(süre) { return Err("durduruldu".into()) }` yaz ya da adımlar arasında `cx.is_cancelled()` değerine bak; böylece iptal işi hemen durdurur.
4. `.on_event(Msg::Task)` ekle, iptal etmek istiyorsan `task.id()` değerini sakla ve `update` içinden `Command::task(task)` döndür.
5. Durumunda bir `Tasks` modeli tut ve her `TaskEvent` için `tasks.apply(&olay)` çağır.
6. Göster: `TaskList::new(&self.tasks).on_cancel(Msg::Iptal).show(ui)`; `Msg::Iptal(id)` mesajına `Command::cancel_task(id)` ile cevap ver.

## Nasıl çalışır

- **Her işin kendi iş parçacığı vardır.** Uygulamayla yalnızca mesajlarla konuşur; çalışma zamanı bu mesajları kareler arasında uygular, böylece `update` ve `view` durumunla asla aynı anda çalışmaz.
- **Olaylar sırayla gelir:** hemen `Started`, bildirildikçe `Progress`, varsa `Ok` mesajı ve en sonunda sonucuyla `Finished`.
- **İptal iş birliğiyle olur.** `Command::cancel_task`, `cx.sleep` beklemesini hemen uyandırır ve `cx.is_cancelled()` değerini true yapar. İş bundan sonra ne döndürürse döndürsün sonuç `Cancelled` olur ve mesajı atılır.
- **Panik uygulamayı çökertmez.** Kısa bir sebeple `Failed` olur.
- **Testler hızlı ve kesin kalır.** `Harness` içinde `cx.sleep` sahte saati izler: `advance(Duration::from_millis(500))` her işin tam yarım saniyelik beklemesini çalıştırır, ardından test sürücüsü çizmeden önce işlerin durulmasını bekler.

## Sık yapılan hatalar

- **`std::thread::sleep` ile beklemek.** İptal edilemez ve test saatini yok sayar; `cx.sleep` kullan.
- **`on_event` eklemeyi unutmak.** Onsuz yalnızca son mesajı alırsın; ilerlemeyi ve hataları hiç görmezsin.
- **İlerlemeyi işin içinde tutmak.** Model `Tasks::apply` ile senin durumunda yaşar; iş yalnızca bildirir.
- **Dosya ya da ağ işini `view` veya `update` içinde yapmak.** Bir `Task` içine taşı ki çizim hiç beklemesin.
