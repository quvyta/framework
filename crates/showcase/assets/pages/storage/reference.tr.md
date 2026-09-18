## Metotlar

- `Settings::load(uygulama)` — `<ayar klasörü>/<uygulama>/settings.toml` dosyasını okur; dosya yoksa ayarlar boş başlar.
- `Settings::open(yol)` — belirli bir dosyayı okur.
- `Settings::in_memory()` — hiç kaydedilmez; testler için.
- `Settings::parse_str(dosya, metin)` — TOML metnini okur, sorunları `dosya` adıyla bildirir; kaydetme bir şey yapmaz.
- `.schema(sema)` — yüklenen her anahtarı `sema` ile denetler: bilinmeyen anahtarlar ve geçersiz değerler yeri gösterilen uyarılara dönüşür.
- `.self_heal(bool)` — varsayılan kapalı; şema varken bilinmeyen anahtarları siler, geçersiz değerlerin yerine varsayılanı yazar, geçersiz isteğe bağlı değerleri siler, açık öneklere ve eksik anahtarlara dokunmaz, dosyanın yedeğini alır ve bir kez kaydeder.
- `.get::<T>(anahtar)`, `.get_or(anahtar, varsayılan)`, `.set(anahtar, değer) -> bool` (değişti mi), `.remove(anahtar) -> bool`, `.value(anahtar)`, `.keys()`.
- `.theme()`, `.language()`, `.icon_mode()`, `.reduced_motion()`, `.pillar_style()`, `.slide()` — bilinen anahtarlar `Settings::THEME`, `LANGUAGE`, `ICONS`, `REDUCED_MOTION`, `PILLAR`, `SLIDE`.
- `.apply::<Msg>()` — kayıtlı tema, dil, ikon, hareketi azaltma, çubuk ve kayma değerlerine geçen komutlar.
- `.save()` — bölünmeden yazar; `.save_command(|Result<(), String>| msg)` — bir kopyayı arka plan iş parçacığında kaydeder.
- `.to_toml()`, `.path()`, `.diagnostics()`.
- `Runtime::settings(&settings)` — kayıtlı görünümü ilk kareden önce uygular.
- `Setting` — `bool`, `String`, tam sayı tipleri, `f32`, `f64`, `Vec<String>` için hazır; `SettingValue` saklanan biçimdir.

## Şema

- `Schema::builtin()` — `theme` ve `language` (herhangi bir metin; `monochrome`, `en`), `icons` (`auto`, `nerd`, `unicode`, `ascii`; `auto`), `reduced-motion` (`false`), `pillar` (`thick`, `thin`; `thick`), `slide` (`true`).
- `Schema::default()` — hiç anahtar yok.
- `.flag(anahtar, varsayılan)` — `true` ya da `false`.
- `.text(anahtar, varsayılan)` — herhangi bir metin.
- `.choice(anahtar, seçenekler, varsayılan)` — `seçenekler` içinden bir metin; ör. yüklü tema kimlikleri.
- `.check(anahtar, varsayılan, |değer: &T| bool)` — `T` olarak okunur ve kapanıştan geçer; ör. bir aralıktaki sayı.
- `.optional(anahtar, tür)` — varsayılanı olmayan anahtar; `tür` `SettingKind::flag()`, `SettingKind::text()`, `SettingKind::choice(seçenekler)` ya da `SettingKind::check(|değer: &T| bool)` olur.
- `.open(önek)` — noktalı `önek` tablosunun altındaki her anahtarı (ör. `plugins`: `[plugins]` ve altı) olduğu gibi tutar; sondaki nokta yok sayılır, `""` hiçbir şey açmaz, aynı öneki iki kez açmak bir kez açmakla aynıdır.
- Bir anahtarı yeniden tanımlamak eski kuralının yerine geçer; varsayılanlı bir anahtarla isteğe bağlı bir anahtar arasında da.

## Klasörler

- `config_dir(app)` — `app` uygulamasının ayarlarının yeri: Linux ve diğer Unix'te mutlak yolsa `$XDG_CONFIG_HOME/<app>`, değilse `HOME` mutlak yolsa `$HOME/.config/<app>`; macOS'ta `$HOME/Library/Application Support/<app>`; Windows'ta dolaşan klasör olan `%APPDATA%\<app>`.
- `data_dir(app)` — `app` uygulamasının kendi kayıtlarının yeri: Linux ve diğer Unix'te mutlak yolsa `$XDG_DATA_HOME/<app>`, değilse `$HOME/.local/share/<app>`; macOS'ta ayarlarla aynı klasör olan `$HOME/Library/Application Support/<app>`, çünkü macOS bir komut satırı uygulaması için ayrı bir veri klasörü tanımaz; Windows'ta yerel klasör olan `%LOCALAPPDATA%\<app>`, böylece kayıtlar makineler arasında kopyalanmaz. Mutlak yol olmayan bir `HOME`, göreli bir XDG değişkeni gibi ikisinde de yok sayılır; hiçbir şey çalışma klasörünün altına düşmez.
- Boş bir değişken tanımsız sayılır; göreli bir `XDG_*` yolu geçersizdir ve yok sayılır.
- Platformun değişkenleri bir şey söylemiyorsa `None`. `Settings::load` o zaman her şeyi bir uyarı tanısıyla bellekte tutar.
- Sormak klasörü oluşturmaz, klasörün var olması da gerekmez.

## Aile

- `Family::QUVYTA`, `Family::new(kimlik, başlık)` — bir uygulama ailesi: klasör ve dosya adları için küçük harfli bir `kimlik`, kullanıcının ad olarak okuduğu yerler için bir `başlık`. `.id()`, `.title()`.
- `.config_dir()` — ailenin klasörü: Linux ve diğer Unix sistemlerinde mutlaksa `$XDG_CONFIG_HOME/<kimlik>`, değilse `$HOME/.config/<kimlik>`; macOS'ta `$HOME/Library/Application Support/<başlık>`; Windows'ta `%APPDATA%\<başlık>`. Ev klasörü yoksa `None`.
- `.shared_file()` — `<config_dir>/<kimlik>.conf`, bütün üyelerin paylaştığı ayarlar.
- `.app_file(uygulama)` — `<config_dir>/<uygulama>.conf`. Kimliği ailenin kimliğiyle aynı olan bir uygulama ortak dosyayı alır.
- `.app_dir(uygulama)` — `<config_dir>/<uygulama>`, uygulamanın diğer yapılandırma dosyaları için.
- `.workspace_dir(uygulama_başlığı)` — `<documents_dir>/<başlık>/<uygulama_başlığı>`, kullanıcının uygulamayla yaptığı işin yeri.
- `Settings::load_member(&aile, uygulama)` — `Settings::open(aile.app_file(uygulama))`; ev klasörü yoksa `Settings::load` ile aynı uyarıyla bellekte ayarlar.
- `Settings::with_diagnostics(tanılar)` — yüklemenin çevresinde bulunan tanıları dosyanın kendi tanılarının önüne koyar; sonraki şema denetimlerinde de kalırlar.
- Sormak hiçbir şey oluşturmaz.

## Eski ayarları taşımak

- `aile.adopt(uygulama, eski_klasör) -> Migration` — `eski_klasör/settings.toml` `app_file(uygulama)` olur; `eski_klasör` altındaki diğer her dosya, her derinlikte, `app_dir(uygulama)` altında aynı göreli yola gider.
- `aile.adopt_in(config_dir, uygulama, eski_klasör)` — aynı taşıma, ailenin klasörü olarak `config_dir` ile; kullanıcının kendi ayarlarına dokunmaması gereken testler ve demolar için.
- `eski_klasör` `app_dir(uygulama)` ise yalnızca `settings.toml` taşınır ve klasör kalır.
- Her taşıma: yeni ad sahiplenilir (boş oluşturulur, orada bir şey varsa başarısız olur; Unix'te eski dosyanın izinleriyle), `atomic_write` ile doldurulur, eski dosyanın izinlerini alır, geri okunup karşılaştırılır; eski dosya ancak ondan sonra silinir. Başarısız bir adım yeni dosyayı siler, eskisini tutar.
- Dolu bir yeni yer, sembolik bağ (asla izlenmez, asla taşınmaz), düz dosya olmayan her şey ve okunamayan dosya olduğu yerde kalır; her biri yolu anan bir tanıyla. `eski_klasör` sembolik bağ ya da dosyaysa, ya da `app_dir`'in içinde veya onu içeriyorsa hiçbir şey taşınmaz.
- Boşalan eski klasörler en derinden başlayarak silinir; içinde bir şey kalan klasör kalır; `app_dir` olan `eski_klasör` asla silinmez.
- Olmayan bir `eski_klasör` yapılacak iş değildir; ikinci çalıştırma hiçbir şeyi değiştirmez. Ev klasörü yoksa hiçbir şey olmaz ve rapor bunu söyler.
- `Migration` — `(nereden, nereye)` çiftleri olarak `.moved()`, `.diagnostics()` (bilerek bırakılanlar için uyarı, başarısız olanlar için hata), `.is_clean()`.
- Taşımanın ortasında bir çökme, bütün eski dosyanın yanında boş bir yeni dosya bırakabilir; sonraki çalıştırma ikisini bildirir ve ikisini de tutar.

## Belgeler klasörü

- `documents_dir()` — Linux ve diğer Unix sistemleri: ayar kökündeki (mutlaksa `$XDG_CONFIG_HOME`, değilse `$HOME/.config`) `user-dirs.dirs` dosyasının `XDG_DOCUMENTS_DIR` satırı; değer çift tırnak içinde `"$HOME/…"` ya da mutlak bir yoldur, `#` bir yorum satırı başlatır, ters bölü sonraki karakteri kaçırır, başka hiçbir şey genişletilmez ve son geçerli satır kazanır. Eksik, okunamayan ya da bozuk dosya ve ev klasörünün kendisi olan değer (dosya bir klasörü böyle kapatır) `$HOME/Documents` verir.
- macOS: `$HOME/Documents`. Windows: `dirs` crate'i üzerinden Belgeler Bilinen Klasörü, olmazsa `%USERPROFILE%\Documents`.
- Mutlak bir ev klasörü yoksa `None`. Oluşturulmaz.

## Makine adı

- `machine_name() -> Option<String>` — bu makinenin dosya adına hazır adı: Unix'te `uname -n`'nin yazdığı düğüm adı (`rustix` ile okunur; kabuk da dosya da yok), Windows'ta `COMPUTERNAME`, başka yerlerde `None`.
- Üç adımda güvenli hale gelir: çevresindeki boşluk atılır; harf ya da rakam (her yazıdan), `-`, `_` veya `.` olmayan her karakter, UTF-8 olmayan baytlar dahil, `-` olur; iki uçtaki noktalar atılır.
- Platform bir ad vermiyorsa ya da bu adımlardan sonra geriye bir şey kalmıyorsa `None`.
- Her çağrıda yeniden okunur, saklanmaz. Sistemin değiştirilmemiş adı sunulmaz.

## Bölünmeyen yazma

- `atomic_write(path, contents)` — diskte ya önceki dosyayı ya yenisini bırakır, ikisinin yarısını asla. Üst klasörün var olması gerekir. Sembolik bağ olan bir `path` üzerinden yazılır: bağın gösterdiği dosya değişir, izinlerini korur ve bağ bağ olarak kalır; bir dotfiles deposundan bağlanan ayar dosyası o deponun dosyası olmaya devam eder. Döngü kuran bağlar hatadır.
- `atomic_write_reporting(path, contents, |step| …)` — aynı yazma, biten her adımı bildirir.
- `WriteStep` — sırasıyla `Wrote(PathBuf)` (gerçek dosyanın yanındaki geçici dosya), `SyncedFile`, `Renamed`, `SyncedDirectory`.
- Geçici dosya aynı klasörde `<ad>.tmp-<pid>-<n>` olarak durur, çünkü yeniden adlandırma dosya sistemi sınırını geçemez; sayaç aynı yolu yazan iki tarafı ayırır.
- Başarısız bir adım geçici dosyayı siler, böylece başarısız yazma geriye bir şey bırakmaz; hata, başarısız olan ilk adımın hatasıdır.
- Unix'te klasör, yeniden adlandırmadan sonra diske indirilir; yeni adın elektrik kesintisine dayanmasını sağlayan adım budur. Diğer platformlarda, Windows dahil, standart kütüphane bir klasörü açıp diske indiremez, bu yüzden `SyncedDirectory` bildirilmez: yarım yazılmış dosya çıkmaz ama yeniden adlandırmadan hemen sonraki bir kesinti eski dosyayı bırakabilir. Daha iyisi bu framework'ün `unsafe` olmadan yapamadığı çağrıları ister.

## Tek örnek

- `AppLock::acquire(path)` — `Ok(Some(lock))` kilidi aldı, `Ok(None)` kilidi başka bir süreç tutuyor, `Err` denenemedi. Üst klasörün var olması gerekir.
- Kilit, değer yaşadığı sürece yaşar: değeri düşürmek kilidi salar, sürecin nasıl olursa olsun bitmesi de salar. Geriye kalmış bir kilit dosyası kilit değildir.
- Unix'te bu `flock(LOCK_EX | LOCK_NB)`. Bu tür bir kilit sürece değil açık dosyaya aittir, bu yüzden aynı süreç içinde aynı yola yapılan ikinci `acquire` da `None` der.
- Diğer her platformda, Windows dahil, burada danışma kilidi yoktur: `acquire` `io::ErrorKind::Unsupported` döner ve asla `Ok` olmaz; uygulama sessizce kilitsiz çalışmak yerine kilidi olmadığını öğrenir.
- `holder_pid(path)` — kilit dosyasına yazılmış süreç kimliği, kilidi tutanı adıyla anan bir mesaj için. Yalnızca tanı metni: süreç kimlikleri yeniden kullanılır, hiçbir karar buna dayanamaz.

## Davranış

- Söz dizimi hatalarının yeri gösterilir; dosyanın okunabilen kalanı kullanılır.
- Tarih-saat değerleri ve tablo dizileri bir uyarıyla atlanır.
- Tanılar anahtarın kendisini gösterir; dosyada başka yerde geçen aynı kelimeyi değil.
- Şema yokken yerleşik anahtarlar denetlenir: `theme` ve `language` metin, `reduced-motion` ve `slide` boolean, `icons` `auto`, `nerd`, `unicode`, `ascii`'den biri, `pillar` `thick`, `thin`'den biri olmalıdır. Aksi halde yeri gösterilen bir uyarı çıkar ve tipli yardımcılar değeri yok sayar. Başka anahtarlar bildirilmez.
- Şema varken ve onarma kapalıyken bilinmeyen anahtarlar ve geçersiz değerler uyarıdır, dosyaya dokunulmaz.
- Şema varken ve onarma açıkken her anahtar tek başına değerlendirilir; anahtar sırası hiç değiştirilmez ve bildirilmez; değeri saklanamayan bilinen bir anahtar varsayılanını alır; dosyanın eski hali `settings.toml.bak` olarak kopyalanır ve onarılmış dosya bir kez kaydedilir; her onarım bir uyarıdır. Kaydedilemeyen onarım bir hata tanısıdır, onarılmış değerler yine kullanılır.
- Dosyada olmayan bir anahtar, onarma açık da olsa kapalı da olsa eklenmez: tipli okumalar `None`, `get_or` senin varsayılanını verir.
- İsteğe bağlı anahtar: geçerliyse kalır; geçersizse uyarı olur (onarma kapalıyken `it is ignored`, açıkken `removed`); değeri saklanamıyorsa onarırken silinir; eksikse hiçbir şey olmaz.
- Açık önekin altındaki anahtarlar hiç bildirilmez ve silinmez; önekin altında tanımlanan bir kural kendi anahtarını yine denetler ve onarır. Önekin kendisini adlandıran anahtar (`plugins = 1`) altında sayılmaz.
- Onarırken tek dosyada birlikte duramayan anahtarlar (aynı tabloda `git = 1` ile `"git.sign" = true`; ikisi de noktalı anahtar olarak okunur) ayrılır: tanımlı anahtar açık olana üstün gelir, değilse önce yazılan kalır; öbürü bir uyarıyla silinir.
- Açık önekin altındaki tarih-saat değerleri ve tablo dizileri her yerdeki gibi atlanır, bu yüzden onarılmış dosyada yer almazlar; yedek dosya onları korur.
- `.schema` ve `.self_heal` herhangi bir sırayla çağrılabilir; denetim yeniden çalışınca önceki onarımlar tanılarda kalır.
- Kaydetme klasörü oluşturur ve `atomic_write` üzerinden yazar; sorunlu yüklenen bir dosya önce kendi adına `.bak` eklenerek kopyalanır: `settings.toml.bak`, `code.conf.bak`.
- `get::<f64>` ile okunan ondalıklı sayı tam sayıyı da kabul eder.
