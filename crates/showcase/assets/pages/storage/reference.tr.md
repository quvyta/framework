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

## Ayar klasörü

- Linux ve diğer Unix: mutlak yolsa `$XDG_CONFIG_HOME`, değilse `$HOME/.config`.
- macOS: `$HOME/Library/Application Support`.
- Windows: `%APPDATA%`.
- Bulunamazsa: ayarlar bir uyarı tanısıyla bellekte kalır.

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
- Kaydetme klasörü oluşturur, `settings.toml.tmp-<pid>` yazar, diske işler ve yeniden adlandırır; sorunlu yüklenen bir dosya önce `settings.toml.bak` olarak kopyalanır.
- `get::<f64>` ile okunan ondalıklı sayı tam sayıyı da kabul eder.
