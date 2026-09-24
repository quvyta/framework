## Klasörler

- `XdgDirs::from_env(arama) -> XdgDirs` — standart yedekleriyle `XDG_DATA_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_HOME`, `XDG_CONFIG_DIRS` ve `XDG_CURRENT_DESKTOP`. `arama` bir değişkeni döndürür, örneğin `|ad| std::env::var(ad).ok()`. Boş değer yok sayılır, göreli klasör atlanır.
- `XdgDirs { data_home, data_dirs, config_home, config_dirs, desktops }` — alanlar açık; test geçici bir klasör üzerine kendisi kurar.

## Türler

- `MimeDb::load(&XdgDirs) -> MimeDb` — her veri klasörünün `mime`'ından `globs2`, `aliases` ve `subclasses`, önce kişininki. `__NOGLOBS__` bir türün desenlerini sonraki klasörlerden düşürür.
- `db.guess(ad) -> Option<String>` — yalnızca addan: ağırlık, sonra harfe duyarlı desen duyarsızdan önce, sonra en uzun.
- `db.sniff(yol) -> String` — klasör `inode/directory`, boru, soket ya da aygıt kendi `inode/*`'u; değilse ad, hiçbir desen tutmazsa ilk 4 KiB: `text/plain` ya da `application/octet-stream`.
- `db.canonical(mime) -> String` — `mime` bir takma adsa türün kendi adı.
- `db.ancestors(mime) -> Vec<String>` — önce `mime`, sonra genişlik öncelikli olarak onun çeşidi olduğu her tür; her `text/*` bir `text/plain`'dir, `inode/*` dışındakilerin hepsi `application/octet-stream` ile biter.
- `db.diagnostics() -> &[Diagnostic]`.

## Programlar

- `Apps::load(&XdgDirs, dil, path_var) -> Apps` — her `applications` klasörünün altındaki masaüstü girdileri ve her `mimeapps.list`; adlar `dil`'de (`tr_TR.UTF-8`); `TryExec` `path_var`'da (bir `PATH` değeri) aranır.
- `apps.for_mime(&db, mime) -> Vec<&DesktopApp>` — tür için, sonra onun çeşidi olduğu her tür için: her dosyanın varsayılanı, eklenen programlar, sonra o türü sayan programlar. Çıkarılanlar ve gizliler asla.
- `apps.default_for(&db, mime) -> Option<&DesktopApp>` — en yakın türün varsayılanı, yoksa `for_mime`'ın ilk programı.
- `apps.all()`, `apps.get(kimlik)`, `apps.diagnostics()`.
- `DesktopApp { id, name, exec, terminal, mime_types, path, icon }` — alanlar açık.
- `app.command(&Path) -> Option<Vec<OsString>>` — doldurulmuş `Exec` satırı: `%f %F %u %U` dosya, `%c` ad, `%k` girdi, `%i` `--icon <ikon>`, `%%` yüzde işareti; eskimiş kodlar atılır; dosya kodu yoksa dosya sona eklenir. Komut vermeyen satır için `None`.

## Tek dosya

- `Openers::load(&XdgDirs, dil, path_var) -> Openers` — `mime` ve `apps` birlikte okunur; `openers.diagnostics()`.
- `openers.for_file(&Path) -> Choices` — `Choices { mime, apps, default }`; `default`, `apps` içinde bir sıra numarasıdır, yalnızca hiç program yoksa `None`.

## Açmak

- `graphical_session(arama) -> bool` — `DISPLAY` ya da `WAYLAND_DISPLAY` dolu.
- `app.can_start(grafik) -> bool` — terminal programı her zaman, grafik program yalnızca grafik oturumda açılır.
- `app.launch(&Path, grafik, bitince) -> Result<Command<Msg>, LaunchError>` — terminal programı için `Handoff`, grafik program için `Open::program`.
- `Launched::Returned { code }`, `Launched::Started`, `Launched::Failed(sebep)`.
- `LaunchError::NoGraphicalSession`, `LaunchError::NoCommand`.

## Davranış

- Eksik dosya olağandır. Okunamayan satır, dosyası ve satırıyla bir uyarıyla atlanır; `Name`'i, `Exec`'i olmayan ya da `Exec`'i komut vermeyen bir uygulama girdisi bir uyarıyla atlanır. Kimliği alınmış kalır; kişinin bozuk kopyası sistemin kopyasını geri getirmez.
- `Type=Application` olmayan ve `Hidden=true` girdiler kimliklerini alır ama program sunmaz. `NoDisplay` programlar kalır: dosya açmaya devam ederler.
- Bir dosyanın kendi `[Removed Associations]`'ı kendi eklemelerini geri almaz; daha az önemli dosyaların eklediğini ve girdilerin saydığını çıkarır.
- Yalnızca 16 MiB'a kadar düzenli dosyalar okunur; veritabanı adını taşıyan bir boru hiçbir şeyi bekletmez.
- İkili `magic` kuralları ve XML açıklamaları okunmaz. Hiçbir şey yazılmaz.
