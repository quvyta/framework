## Klasörler

- `XdgDirs::from_env(arama) -> XdgDirs` — standart yedekleriyle `XDG_DATA_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_HOME`, `XDG_CONFIG_DIRS` ve `XDG_CURRENT_DESKTOP`. `arama` bir değişkeni döndürür, örneğin `|ad| std::env::var(ad).ok()`. Boş değer yok sayılır, göreli klasör atlanır.
- `XdgDirs { data_home, data_dirs, config_home, config_dirs, desktops }` — alanlar açık; test geçici bir klasör üzerine kendisi kurar.

## Türler

- `MimeDb::load(&XdgDirs) -> MimeDb` — her veri klasörünün `mime`'ından `globs2`, `aliases` ve `subclasses`, önce kişininki. `__NOGLOBS__` bir türün desenlerini sonraki klasörlerden düşürür.
- `db.guess(ad) -> Option<String>` — yalnızca addan: ağırlık, sonra harfe duyarlı desen duyarsızdan önce, sonra en uzun.
- `db.sniff(yol) -> String` — klasör `inode/directory`, boru, soket ya da aygıt kendi `inode/*`'u; değilse ad, hiçbir desen tutmazsa ilk 4 KiB: `text/plain` ya da `application/octet-stream`.
- `db.canonical(mime) -> String` — `mime` bir takma adsa türün kendi adı.
- `db.ancestors(mime) -> Vec<String>` — önce `mime`, sonra genişlik öncelikli olarak onun çeşidi olduğu her tür; her `text/*` bir `text/plain`'dir, `inode/*` dışındakilerin hepsi `application/octet-stream` ile biter.
- `db.comment(mime, dil) -> Option<String>` — türün sözle adı ("Rust kaynak kodu"); veri klasörlerindeki `mime/<tür>.xml` dosyasının `<comment>` satırlarından, önce kişininki. Önce takma ad izlenir; `dil`'deki yorum (`pt-BR`, `tr_TR.UTF-8`) kazanır, sonra dilin kökü (`pt`), sonra dilsiz olan. XML karakter göndermeleri çözülür. Türü hiçbir klasör anlatmıyorsa `None`.
- `db.comment_with_diagnostics(mime, dil, &mut Vec<Diagnostic>) -> Option<String>` — aynısı; UTF-8 olmayan, kapanmayan bir `<comment>` taşıyan ya da düzenli dosya olmayan her dosya için bir tanı ekler. Böyle dosya atlanır, daha az önemli bir klasör yine cevap verebilir.
- `db.diagnostics() -> &[Diagnostic]`.

## Programlar

- `Apps::load(&XdgDirs, dil, path_var) -> Apps` — her `applications` klasörünün altındaki masaüstü girdileri ve her `mimeapps.list`; adlar, yorumlar ve anahtar sözcükler `dil`'de (`tr_TR.UTF-8`); `TryExec` `path_var`'da (bir `PATH` değeri) aranır.
- `Apps::load_including(&XdgDirs, dil, path_var, tut) -> Apps` — aynı okuma, `tut`'un istediğini saklar: `Include::MISSING`, `TryExec` programı kurulu olmayan bir programı; `Include::HIDDEN`, `Hidden` ile işaretli bir girdiyi; `Include::ALL` ikisini, `Include::NONE` hiçbirini. `Openers::load`'ın böyle bir biçimi yoktur; `Openers { mime: MimeDb::load(&klasörler), apps }` olarak kurulur.
- `EntryDetails { comment, generic_name, categories, keywords, folder, no_display, hidden, try_exec, installed }` — alanlar açık, `all()` içindeki her program için bir tane, aynı sırada. `comment` ve `generic_name` kişinin dilindedir, anahtar yoksa ya da boşsa `None`'dur; `keywords` önce o dildekileri sonra düz olanları, her biri bir kez ve girdinin yazdığı sırayla; `try_exec` yazıldığı gibi `TryExec` programıdır, `installed` bulunup bulunmadığını söyler.
- `apps.details(kimlik) -> Option<&EntryDetails>` — o masaüstü dosyası kimliğinin girdisinin söyledikleri, program kuruluysa.
- `shell_words(satır) -> Option<Vec<String>>` — `$EDITOR` gibi bir komut satırı, POSIX kabuğunun böldüğü gibi bölünmüş; tırnaklara ve ters bölülere uyulur, hiçbir şey genişletilmez; kapanmamış tırnakta `None`. Tırnak içinde boşluklu bir yol tek sözcük kalır.
- `apps.for_mime(&db, mime) -> Vec<&DesktopApp>` — tür için, sonra onun çeşidi olduğu her tür için: her dosyanın varsayılanı, eklenen programlar, sonra o türü sayan programlar. Çıkarılan asla, başlatılamayan da asla: kişinin sildiği ya da kendi programı kurulu olmayan bir program.
- `apps.default_for(&db, mime) -> Option<&DesktopApp>` — en yakın türün varsayılanı, yoksa `for_mime`'ın ilk programı.
- `apps.all()`, `apps.get(kimlik)`, `apps.diagnostics()`.
- `DesktopApp { id, name, exec, terminal, mime_types, path, icon }` — alanlar açık.
- `app.launch_command() -> Option<Vec<OsString>>` — dosyasız aynı satır: `%f %F %u %U` hiçbir şeyi temsil etmez ve yalnız kendilerinin durduğu argüman onlarla birlikte gider, `%c` ad, `%k` girdi ve `%i` `--icon <ikon>` `command` ile aynıdır, `%%` yüzde işareti, sona hiçbir şey eklenmez. Komut vermeyen satır için `None`.
- `app.command(&Path) -> Option<Vec<OsString>>` — doldurulmuş `Exec` satırı: `%f %F %u %U` dosya, `%c` ad, `%k` girdi, `%i` `--icon <ikon>`, `%%` yüzde işareti; eskimiş kodlar atılır; dosya kodu yoksa dosya sona eklenir. Komut vermeyen satır için `None`.

## Tek dosya

- `Openers::load(&XdgDirs, dil, path_var) -> Openers` — `mime` ve `apps` birlikte okunur; `openers.diagnostics()`.
- `openers.for_file(&Path) -> Choices` — `Choices { mime, apps, default }`; `default`, `apps` içinde bir sıra numarasıdır, yalnızca hiç program yoksa `None`.

## Açmak

- `graphical_session(arama) -> bool` — `DISPLAY` ya da `WAYLAND_DISPLAY` dolu.
- `app.can_start(grafik) -> bool` — terminal programı her zaman, grafik program yalnızca grafik oturumda açılır.
- `app.launch(&Path, grafik, bitince) -> Result<Command<Msg>, LaunchError>` — terminal programı için `Handoff`, grafik program için `Open::program`; ikisi de dosyanın klasöründe çalışır, test koşumu bunu `dir` olarak kaydeder.
- `Launched::Returned { code }`, `Launched::Started`, `Launched::Failed(sebep)`.
- `LaunchError::NoGraphicalSession`, `LaunchError::NoCommand`.

## Varsayılan program

- `set_default(&XdgDirs, mime, kimlik) -> Result<Change, SetDefaultError>` — `<mime>=<kimlik>;` yazısını `$XDG_CONFIG_HOME/mimeapps.list` dosyasındaki `[Default Applications]` bölümünde o türün kendi satırına yazar ve o satırın yeni olup olmadığını söyler. Kişinin dosyası ya da klasörü yoksa ikisi de yapılır; var olan bir dosya okunur, tek satırı değiştirilir ve bütünüyle yenilenir. Dosya aynı `XdgDirs` ile `Apps::load` ile yeniden okunduğunda `kimlik`, o türün varsayılanı olur.
- `Change::Added`, `Change::Replaced`; enum `#[non_exhaustive]`.
- `SetDefaultError::Io(io::Error)`, `SetDefaultError::NoConfigHome`, `SetDefaultError::NotText`, `SetDefaultError::ReadOnly`, `SetDefaultError::InTheWay { line }`, `SetDefaultError::Unstorable`; enum `#[non_exhaustive]`. Bunların hepsinde dosya olduğu gibi bırakılır ve `mime` için yazılmış bir satır saydığı programı korur.

## Davranış

- Eksik dosya olağandır. Okunamayan satır, dosyası, satırı ve — hata satırın tamamında değil de bir anahtardaysa — anahtarın yazıldığı sütunla bir uyarıyla atlanır; `Name`'i, `Exec`'i olmayan ya da `Exec`'i komut vermeyen bir uygulama girdisi bir uyarıyla atlanır. Kimliği alınmış kalır; kişinin bozuk kopyası sistemin kopyasını geri getirmez.
- `Type=Application` olmayan ve `Hidden=true` girdiler kimliklerini alır ama program sunmaz; `Include::HIDDEN`, gizli olanları `hidden == true` ile saklar, böylece uygulama kişinin sildiklerini görebilir. `NoDisplay` programlar kalır: dosya açmaya devam ederler, `details` de bunu söyler.
- `TryExec` programı bulunamayan bir girdi düşürülür; `Include::MISSING` onu `installed == false` ile saklar, böylece bir başlatıcı listeleyebilir, durumunu söyleyebilir ve paketini önerebilir. Silinmiş bir program da kurulu olmayan bir program da `for_mime` ya da `default_for` tarafından hiçbir zaman önerilmez: hiçbir şey başlatamazlar.
- Bir dosyanın kendi `[Removed Associations]`'ı kendi eklemelerini geri almaz; daha az önemli dosyaların eklediğini ve girdilerin saydığını çıkarır.
- Yalnızca 16 MiB'a kadar düzenli dosyalar okunur; veritabanı adını taşıyan bir boru hiçbir şeyi bekletmez.
- İkili `magic` kuralları okunmaz; XML açıklamalarından yalnızca `<comment>` satırları, satır satır ve XML ayrıştırıcısı olmadan okunur.
- Hiçbir zaman yazılan tek dosya kişinin kendi `$XDG_CONFIG_HOME/mimeapps.list` dosyasıdır ve onu yazan da yalnızca `set_default`'tır. `$XDG_CONFIG_DIRS` altındaki dosyalar, çalışan bir masaüstünün `<masaüstü>-mimeapps.list` dosyası ve her `.desktop` girdisi okunur, hiçbir zaman yazılmaz.
- `set_default` dosyanın kalanını bayt bayt geri koyar: başka bölümler, yorumlar, boş satırlar ve anlamadığı satırlar hep yerinde kalır; aynı tür için `[Removed Associations]` altındaki satır da, ve `[Default Applications]` içinde aynı türün ikinci satırı da, bir okuyucunun aldığı ilk satır olduğu için. Dosyada hiçbölüm yoksa kendi satırı olmayan tür, dosyanın sonunda kendi bölümünü alır.
