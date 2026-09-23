## Metotlar

- `FolderWatch::new() -> io::Result<FolderWatch>` — klasörsüz bir izleme; Linux dışındaki platformlarda `Unsupported`.
- `watch.watch(&Path) -> io::Result<()>` — bir klasörün girdilerini özyinelemesiz izler; yeniden izlemek bir şey yapmaz. Hatalı yol için `NotFound` ya da `NotADirectory`, sistemin izleme sınırı dolunca `QuotaExceeded`.
- `watch.unwatch(&Path)` — bir klasörü izlemeyi bırakır; izlenmeyen klasöre dokunulmaz.
- `watch.changes() -> FolderChanges` — bekleyen taraf; kopyalaması ucuz, `Send`, `Command::perform` için.
- `changes.next() -> Vec<FolderChange>` — bir şey değişene kadar bekler, saniyenin onda biri kadar toplar ve topluluğu döndürür; izleme bırakılınca boştur.
- `changes.next_within(süre) -> Option<Vec<FolderChange>>` — aynı bekleme, ilk değişiklik için en fazla `süre`; hiçbir şey değişmediyse `None`, izleme sürer. Bekleyişi olduğu yerde çalıştıran ekran testleri için.
- `FolderChange { folder: PathBuf, name: Option<OsString>, kind: FolderChangeKind }` — `folder`, `watch`'a verildiği gibidir; `Gone` ve `Overflow` için `name` `None`'dır.
- `FolderChangeKind::Created`, `Removed`, `Renamed { from: OsString }`, `Modified`, `Gone`, `Overflow`.

## Davranış

- Linux'ta inotify exec'te kapanacak biçimde açılır, çocuk süreç izlemeyi devralmaz.
- Topluluk değişikliklerin oluş sırasını korur; içinde tekrarlanan bir değişiklik son yerinde kalır, "oluştu, silindi, oluştu" girdinin var olmasıyla biter.
- `Modified` içeriği ve nitelikleri (izinler, zamanlar) kapsar.
- Yeniden adlandırma bir topluluk içinde eşlenir; iki yarısı çekirdekten birlikte gelir.
- İzlenen iki klasör arasında taşıma, birincide `Removed`, ikincide `Created` olur; izlenmeyen bir klasöre ya da klasörden taşımada yalnızca görülen yarı gelir.
- `Overflow`, izlenen her klasör için bir kez, yola göre sıralı gelir.
- `Gone` bir kez gelir ve klasör artık izlenmez: aynı yolda yeni bir klasör, yeniden izlenene kadar takip edilmez.
- İzlemesi az önce bırakılan bir klasör için kuyrukta kalan olaylar atılır.
- Aynı `FolderChanges`'ın kopyalarında bekleyen iki iş parçacığı sırayla alır; bir topluluk aralarında bölünmez.
