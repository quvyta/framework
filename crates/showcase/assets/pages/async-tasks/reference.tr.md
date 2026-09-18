## Metotlar

- `Task::new(etiket, |cx: &TaskCx<Msg>| -> Result<Msg, String>)` — arka plan işi; `Ok` bir mesaj teslim eder, `Err` bir sebeple başarısız olur.
- `.on_event(|TaskEvent| msg)` — başlangıç, ilerleme ve sonuç mesaj olarak gelir.
- `.id()`, `.label()` — iş başlamadan önce bilinir.
- `Command::task(task)` — işi kendi iş parçacığında başlatır.
- `Command::cancel_task(id)` — durmasını ister; iş bittiyse bir şey yapmaz.
- `TaskCx::progress(oran)`, `TaskCx::note(metin)` — bildirir; `TaskCx::send(msg)` — başka her mesaj.
- `TaskCx::sleep(süre) -> bool` — bekler; iptal edilirse erken uyanır ve `false` döndürür.
- `TaskCx::is_cancelled()`, `TaskCx::id()`.
- `Tasks::new()`, `.apply(&olay)`, `.entries()`, `.get(id)`, `.running()`, `.clear_finished()`.
- `TaskEntry` — `id`, `label`, `fraction`, `note`, `outcome` (çalışırken `None`).
- `TaskEvent::Started`, `Progress`, `Finished`; `TaskOutcome::Done`, `Failed(sebep)`, `Cancelled`.
- `TaskList::new(&tasks).show(ui)` — satırlar; `.on_cancel(|id| msg)` iptal butonlarını ekler; `.empty_text(metin)`.

## Davranış

- Teslim sırası: komutu döndüren update sırasında `Started`, bildirildikçe `Progress`, `Ok` mesajı, sonra `Finished`.
- İptal edilen bir iş, işi `Ok` döndürse bile her zaman `Cancelled` ile biter.
- İşin içindeki panik `Failed("the task `etiket` panicked")` olur.
- `TaskList` satırları: spinner ya da durum ikonu, etiket, not ya da sonuç kelimesi, oran bildirildiyse 22 hücrelik ilerleme çubuğu ve `on_cancel` verildiyse iptal butonu. Biten satırlarda sütunlar hizalı kalır.
- `Harness` içinde `cx.sleep` sahte saati izler ve her çizim işler uyuyana ya da bitene kadar bekler; on saniye boyunca uyumadan çalışan bir iş testi açık bir mesajla düşürür.

## Tema ve ikonlar

- `Spinner`, `ProgressBar`, `Button` ve `Text` ile kurulur; onların tema anahtarları geçerlidir.
- `success`, `error`, `check-partial` ikonları; bu ikonlarla birlikte `success` ve `danger` renkleri.

## Dil anahtarları

- `quvyta.tasks.done`, `quvyta.tasks.cancelled`, `quvyta.tasks.cancel`, `quvyta.tasks.empty`.

## Alt süreç

- `Process::new(program)` — borularla ve uygulamanın kendi ortamıyla çalışan bir alt süreç.
- `.arg(arg)`, `.args(args)`, `.dir(yol)` — komut satırı ve çalışma klasörü.
- `.env(anahtar, değer)` — çocuk için bir değişken; ortamın kalanı devralınır.
- `.pty(sütun, satır)` — çocuğu boru yerine o boyutta bir sözde terminalde çalıştırır.
- `.no_stdin()` — çocuk terminal yerine boş bir girdi (`/dev/null`) okur ve Unix'te kendi süreç grubunda çalışır.
- `.run(&cancel, &mut on_line) -> io::Result<ProcessOutcome>` — çalıştırır ve her satırı teslim eder; çocuk başlatılamazsa ya da sözde terminal açılamazsa hata döner.
- `Line::Out(metin)`, `Line::Err(metin)` — standart çıktı ve standart hata; boru kipinde ayrı tutulur.
- `ProcessOutcome::Finished { code }` — çocuğu bir sinyal bitirdiyse `code` değeri `None` olur; `ProcessOutcome::Cancelled`.

## Davranış

- Satırlar tek tek, yeni satır karakteri olmadan gelir; son satır sonunda yeni satır olmasa da teslim edilir.
- Satır başı yeni satır açmak yerine yazılmakta olan satırı ezer, böylece ilerleme çubuğu tek satır kalır; terminalin `\r\n` dizisi ise satırı bitirir.
- Boru kipinde iki akış ayrı okunur ve birleştirilmez; sözde terminalde ikisi de aynı satıra düştüğü için yalnız `Line::Out` görünür.
- `cancel` satırlar arasında sorulur. Doğru döndüğünde çocuk öldürülür, bekleyen çıktı atılır ve sonuç `Cancelled` olur. Unix'te `no_stdin` ile çocuğun bütün süreç grubu öldürülür; başlattığı programlar da biter, kendi grubuna ya da oturumuna geçenler hariç. Onsuz yalnızca çocuğun kendisi öldürülür, çünkü terminali okuyan bir çocuk kendi grubunda yaşayamaz (sistem onu ilk okumasında durdurur).
- UTF-8 olmayan baytlar kaybolmaz, değiştirme karakterine çevrilir.
- Standart girdi, `no_stdin` istenmedikçe uygulamanın kendi girdisi kalır ve çocuk hiçbir zaman kendi oturumuna konmaz (süreç grubu oturum değildir); kontrol eden terminali korur ve sıcak `sudo` biletini paylaşır. Sözde terminal `rustix` ile, `unsafe` olmadan açılır ve bir Unix sistemi gerektirir.
