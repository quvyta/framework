## Ne zaman kullanılır

Framework'teki her ikonun bir Nerd Font glifi vardır ve bu glifler onları taşıyan bir yazı tipi ister. Algılama Nerd Font bulamadığında ya da kullanıcı Nerd ikonlarını seçip kutucuk gördüğünde kurulumu önerin: ilk açılış ekranında ikon seçiminin yanında ve uygulamanın ayarlarında bir satır olarak.

- **Yalnızca simgeler.** Framework, harfleri olmayan, yalnızca simgelerden oluşan Symbols Nerd Font Mono'yu kurar. Kullanıcı terminalinin yazıyı çizdiği yazı tipini korur; eksik glifleri başka kurulu yazı tiplerinden alan bir terminal ikonları orada bulur. Linux'taki terminallerin çoğu bunu fontconfig ile yapar.
- **Son sözü göz söyler.** Hiçbir program terminalin hangi yazı tipiyle çizdiğini öğrenemez. Kurulumdan önce ve sonra aynı ikonları Nerd ve Unicode sütunuyla gösteren `GlyphSample`'lar koyun; hangi satırın şekil olarak okunduğunu kullanıcı söylesin.
- **Önerin, dayatmayın.** Kullanıcı düğmeye basmadan hiçbir şey kurulmaz; Unicode ikonlar da gayet iyi bir seçimdir.

## Adım adım

1. Durumu gösterin: `nerd_font::installed()` ve `nerd_font::status_text(installed)`, bir de yazı tipinin gideceği yer, `nerd_font::target_dir()`.
2. Örnekleri yerleştirin: `ui.add(GlyphSample::new(GlyphMode::Nerd))` ve yanında `GlyphSample::new(GlyphMode::Unicode)`, her biri bir etiketin ardından.
3. Kurulumu `update` içinde başlatın: `nerd_font::install(Msg::Font)` komutu döndürür. İptal edilebilen ya da bir `TaskList`'te görünen bir görev için `Install::new().task(Msg::Font).on_event(Msg::Task)` kullanın ve `id()`'sini saklayın.
4. Her adımı gösterin: son `Progress`'i tutun ve `progress.text()`'i çizin; `Progress::Downloading { fraction }` bir `ProgressBar`'ı doldurur.
5. `Progress::Done { path }` geldiğinde örnekleri yeniden ve altlarında `nerd_font::after_install_text()`'i gösterin: Nerd satırı hâlâ kutucuksa metin, JetBrainsMono Nerd Font'u kurup terminalin ayarlarında seçmesini söyler.
6. `Progress::Failed(error)` geldiğinde `progress.text()` sebebi verir, varsa programın kendi iletisiyle.
7. Testlerde gerçek yazı tipi klasörüne asla dokunmayın: `Install::new().archive(Archive::new("file:///…/sahte.tar", sha256)).target(geçici).register(false)`.

## Nasıl çalışır

- **Sabit sürüm, yazılı özet.** Arşiv Nerd Fonts v3.5.1'den gelir ve SHA-256 özeti kaynakta yazılıdır. Tutmayan indirme silinir, hiçbir şey kurulmaz.
- **Sistemin kendi programları.** `curl` indirir ve ilerlemesini bildirir, `sha256sum` ya da `shasum -a 256` (Windows'ta `certutil`) denetler, `tar` yalnızca yazı tipi dosyasını açar. Bunlar Linux, macOS ve Windows 10 sonrasıyla gelir; framework ağ, arşiv ya da özet kodu taşımaz. Windows'a, kendi `tar`'ının açtığı zip arşivi gider ve programları `System32`'den alınır.
- **Kullanıcının kendi klasörü.** Linux'ta `~/.local/share/fonts/QuvytaNerdFont` (`XDG_DATA_HOME`'u izler), macOS'ta `~/Library/Fonts`, Windows'ta `%LOCALAPPDATA%\Microsoft\Windows\Fonts`. Yönetici izni istenmez.
- **Sisteme haber verilir.** Linux'ta fontconfig kuruluysa `fc-cache -f` yeni klasörü okur; Windows'ta yazı tipi `HKCU\…\Fonts` altına yazılır; macOS'ta bir şey gerekmez.
- **Geri alınabilir.** `Progress::Done` kurulumu geri almak için silinecek tek yolu verir: Linux'ta kendi klasörü, klasörün paylaşıldığı yerde yazı tipi dosyası.
- **Arka planda.** İş bir `Task` olarak yürür: ekran çizilmeye devam eder, kullanıcı öteki seçimleri yapabilir. Etiketi ve notları kurulurken, `update` içinde çevrilir, çünkü iş parçacığının dili yoktur.

## Sık yapılan hatalar

- **Diskteki dosyaya güvenmek.** Klasördeki bir yazı tipi terminalin onunla çizdiği anlamına gelmez; kurulumdan sonra örnekleri her zaman yeniden gösterin.
- **Terminalin ayarlarını yazmak.** Terminalin ayar dosyası kullanıcınındır. Neyin seçileceğini söyleyin, dosyayı değiştirmeyin.
- **Varsayılan olarak tam bir aile kurmak.** Tam bir Nerd Font kullanıcının yazı tipinin yerine ancak kullanıcı onu seçerse geçer; yalnızca simgeleri içeren yazı tipi kullanıcının seçtiği hiçbir şeyi değiştirmez.
- **Görevi `update` dışında başlatmak.** Hiçbir dilin etkin olmadığı yerde kurulursa etiketi ve notları anahtar olarak okunur.
