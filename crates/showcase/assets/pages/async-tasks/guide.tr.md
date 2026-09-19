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

## Alt süreci akıtmak

`Process`, bir alt süreci bir iş içinde çalıştırır ve çıktısını uygulamaya satır satır verir; bir paket yöneticisinin, bir derlemenin ya da bir dağıtım betiğinin ihtiyacı budur.

1. Kur: `Process::new("sh").arg("-c").arg(betik).env("LC_ALL", "C")`; başka bir yerde çalışması gerekiyorsa `.dir(yol)` ekle.
2. Bir işin içinde çalıştır: `process.run(&|| cx.is_cancelled(), &mut |satir| cx.send(Msg::Satir(satir)))`. Çağrı yalnızca kendi iş parçacığını bekletir, ekranı asla.
3. Her `Line` değerini bir log satırına çevir: `Line::Out` sıradan çıktı, `Line::Err` çocuğun standart hatasıdır; ayrı tutulur ki hata hata olarak tanınsın.
4. İptal butonuna `Command::cancel_task(id)` ile cevap ver; işin bayrağı `run` çağrısının çocuğu öldürüp `ProcessOutcome::Cancelled` döndürmesini sağlar.
5. Çocuğun gerçekten klavyeyi okuması gerekmiyorsa `.no_stdin()` ekle. Çocuk terminal yerine boş bir girdi okur; soru soran bir program kullanıcının tuşlarını uygulamadan çalamaz. Kendi süreç grubunda da çalışır; iptal, onun başlattığı programları da bitirir (`podman`, ardından `buildah`, ardından derlemenin adımları).
6. Ayrıştıracağın çıktının dilini `LC_ALL` **ve** `LANG` ile birlikte sabitle; bazı programlar yalnızca birine bakar.

Çocuğun ilerleme çubuğunu ve rengini koruması gerekiyorsa `.pty(sütun, satır)` ekle: programlar terminal denetimi yapar ve boruya yazarken süssüz çıktıya döner. Çocuk böylece gerçek terminalin değil, senin verdiğin boyutta bir terminal görür; ilerleme çubuğu da çizeceğin alana göre kurulur. Standart girdi uygulamanın kendi girdisi kalır ve çocuk kontrol eden terminali korur; sıcak bir `sudo` biletinin paylaşılmasını sağlayan budur. Sözde terminalde iki akış aynı satıra düştüğü için her satır `Line::Out` olarak gelir.

İlerleme çubuğunun yalnızca nerede bittiğini değil ne dediğini de istiyorsan (`cargo`'nun `Building [=>  ] 12/46` satırı, `pacman` ya da `curl` indirmesinin yüzdesi), `.run(..)` yerine `.run_with_overwritten(&cancel, &mut on_line, &mut on_frame)` ile çalıştır. Bir `\r`'nin ezmek üzere olduğu her kare `on_frame`'e gelir; satır gibi `Line::Out` ya da `Line::Err` etiketini taşır ve çocuğun yazdığı sırayla gelir. `on_line` ise `run`'ın vereceği satırların aynısını alır. Renk kodları ve `ESC [K` metinde kalır; ayrıştırmadan önce temizle.

## Sık yapılan hatalar

- **İki akışı `2>&1` ile birleştirmek.** Tanı böylece kaybolur: birçok program bir şey ters gitmedikçe standart hatayı boş bırakır.
- **Borudan ilerleme çubuğu beklemek.** `.pty(..)` olmadan çocuk terminal görmez ve süssüz satırlar yazar; bu bizim değil, çocuğun kararıdır.
- **İlerlemeyi görmek için yeni satır beklemek.** Satır başı yazılmakta olan satırı ezer; `run` ile eline biten satır, bir kez geçer. Kareleri `run_with_overwritten` ile iste.
- **Çocuğu öldürüp çocuklarının da gideceğini sanmak.** Yalnızca `.no_stdin()` ile başlatılan bir çocuk kendi çocuklarını da götürür; onsuz yalnızca çocuğun kendisi öldürülür ve kendi çocuklarını başlatan bir program onları çalışır bırakabilir.
- **Çocuğun terminal girdisini paylaşması.** `.no_stdin()` olmadan soru soran bir çocuk uygulamana gelen tuşları okur; iptal edildiğinde de kendi çocukları çalışmaya devam eder.
- **Girdisiz bir çocuğa `sudo` parolası sordurmak.** Onun grubu terminalin sahibi değildir; parolayı okumaya çalıştığında sistem onu durdurur ve iptal edilene kadar bekler. Bileti önce bir `sudo -v` devriyle ısıt ya da `sudo -n` kullan.
