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
