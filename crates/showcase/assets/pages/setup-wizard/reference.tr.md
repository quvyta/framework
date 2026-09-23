## Setup

Uygulamanın tuttuğu durum; `Msg` uygulamanın kendi mesaj türü.

- `Setup::new(ecosystem, app, &i18n, wrap) -> Setup<Msg>` — `app`'ın kurulumu, ilk adımında, ilk adımın her mesajı `wrap` ile sarılı. Ortak ayarları hiçbir şey yazmadan çözer.
- `Setup::new_in(config_dir, ecosystem, app, &i18n, wrap)` — aynısı başka bir ekosistem klasörüyle, test ya da deneme için.
- `.on_finish(msg)` — sihirbaz ortak anahtarları yazıp dosyayı oluşturunca uygulamaya gönderilen mesaj.
- `.install(Install)` — `Install::new()` yerine başka bir kurulum, örneğin geçici bir klasöre.
- `.font_dirs(Vec<PathBuf>)` — Nerd Font'un aranacağı başka klasörler.
- `.needed() -> bool` — sihirbaz hâlâ gösterilecek mi: uygulamanın kendi dosyası yok ve sihirbaz bitmedi.
- `.step() -> usize` — bulunduğu adım; görünüm adımı 0.
- `.preferences() -> &Preferences` — ilk adımın şu andaki ortak değerleri, henüz hiçbir şey yazılmadan.
- `.update(SetupMsg, &mut Settings) -> Command<Msg>` — mesajı uygular ve onu gösteren komutu döndürür. `Finish`'te yazar.

## SetupMsg

İlk adımın ve butonların gönderdiği her şey; hepsi `Setup::update`'e gider.

- `Appearance(AppearanceChange)` — ilk adımın bir satırı değişti.
- `Install`, `Installing(Progress)` — yazı tipi kurulumu istendi ve adımları.
- `Back`, `Next`, `Step(usize)` — bir önceki adım, bir sonraki adım ve üstteki adımlardan seçilen bitmiş bir adım.
- `Finish` — sihirbaz bitti; "Varsayılanlarla başla" bunu ilk adımdan gönderir.

## SetupWizard

`view` içinde `Setup`'tan kurulur.

- `SetupWizard::new(&setup)` — yalnızca görünüm adımıyla sihirbaz.
- `.step(başlık, |ui| …)` — uygulamanın kendi adımı, eklendikleri sırayla.
- `.on_cancel(msg)` — Vazgeç butonu ve sihirbazın içinde Esc; kapatmak hiçbir şey yazmaz.
- `.page_height(satır)` — her adıma aynı yükseklik, böylece butonlar yerinde kalır.
- `.show(ui) -> NodeMut` — ekler. Butonlar `Wizard`'ın butonları: `wizard-cancel`, `wizard-back`, `wizard-next`.

## Neleri kullanır

- `Appearance::rows(list, message)` — ekosistemin paylaştığı üç satır, başlıksız ve uygulamanın kendi satırları olmadan; `Appearance::without_saving()` değişikliği dosyaya yazmadan uygular.
- `Ecosystem::preferences_without_saving(app, &i18n)` ve `..._in(config_dir, app, &i18n)` — olmayan ortak dosyayı olmayan bırakan çözüm, sihirbazı olan uygulama için.
- `Ecosystem::set(app, key, value, scope)` — Bitir'in her ortak anahtar için bir kez yazdığı yer.
- `GlyphSample::new(GlyphMode)` — glif kipi adlarının yanındaki örnekler.
- `nerd_font::installed_in`, `nerd_font::status_text`, `Install::task`, `Progress`, `nerd_font::after_install_text` — ilk adımın yazı tipi kurulumu.

## Metinler

Framework'ün dil dosyalarında, dokuz dilde: `quvyta.setup.sample-hint`, `install`, `defaults`, `not-saved`; adımın adı ve satırları `quvyta.appearance.*`, butonlar `quvyta.wizard.*`.
