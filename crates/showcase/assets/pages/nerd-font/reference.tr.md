## Fonksiyonlar ve sabitler

- `nerd_font::installed() -> bool` — bu sistemin yazı tipi klasörlerinde, en fazla üç klasör derinde, adında "nerd" geçen bir dosya var mı.
- `nerd_font::installed_in(&[PathBuf]) -> bool` — aynı arama, verilen klasörlerde.
- `nerd_font::target_dir() -> Option<PathBuf>` — yazı tipinin gideceği yer; sistem bir klasör göstermiyorsa `None`. Sormak klasörü oluşturmaz.
- `nerd_font::install(|Progress| mesaj) -> Command<Msg>` — `Install::new()` ile arka planda kurar.
- `nerd_font::after_install_text() -> String` — etkin dilde dürüst sonraki adım.
- `nerd_font::status_text(kurulu) -> String` — "Bu makinede bir Nerd Font kurulu" ya da "Bu makinede Nerd Font bulunamadı".
- `RELEASE` (`"v3.5.1"`), `FAMILY` (`"Symbols Nerd Font Mono"`), `FONT_FILE` (`"SymbolsNerdFontMono-Regular.ttf"`).

## Install ve Archive

- `Install::new()` — sürüm arşivi, `target_dir()`'e, sisteme kaydedilerek.
- `.archive(Archive)`, `.target(klasör)`, `.register(bool)` — başka bir arşiv, başka bir klasör ve `fc-cache` ya da `reg`'in çalışıp çalışmayacağı.
- `.target_dir() -> Option<&Path>`.
- `.run(&iptal, &mut ilerleme) -> Result<PathBuf, InstallError>` — çağıran iş parçacığında kurulum.
- `.task(|Progress| mesaj) -> Task<Msg>` — arka plan görevi olarak kurulum; `update` içinde kurun.
- `Archive::release()` — Linux ve macOS'ta `.tar.xz`, Windows'ta `.zip`; `Archive::new(adres, sha256)`; `.url()`, `.sha256()`.

## Progress ve InstallError

- `Progress::Downloading { fraction: Option<f32> }`, `Verifying`, `Installing`, `Done { path }`, `Failed(InstallError)`; `.text()` etkin dilde.
- `InstallError::NoFolder`, `MissingTool(ad)`, `Download(ileti)`, `Verify(ileti)`, `Checksum { expected, actual }`, `Extract(ileti)`, `Copy(ileti)`, `Register(ileti)`, `Cancelled`; `.text()`.

## GlyphSample

- `GlyphSample::new(GlyphMode)` — `folder`, `check`, `search` ve `settings` ikonları o kipte, iki hücre arayla, uygulama hangi kipte çizerse çizsin.
- `.keys(anahtarlar)` — başka ikon anahtarları; ikon setinde olmayan anahtar atlanır.
- `GlyphSample::KEYS` — varsayılan anahtarlar.
- `Icons::glyphs(anahtar) -> Option<&IconGlyphs>` — bir ikonun bütün glifleri; örnek bunları okur.

## Davranış

- Adımların sırası: `Downloading { fraction: None }`, ardından `curl`'ün bildirdiği her yeni payla `Downloading`, `Verifying`, `Installing`, sonra `Done` ya da `Failed`. İptal edilen kurulum `Failed` olmadan biter.
- İndirme, sistemin geçici klasöründe kendine ait bir klasöre iner; sonuç ne olursa olsun bu klasör silinir. Özeti tutmayan indirme, başka hiçbir şey yapılmadan silinir.
- Arşivden yalnızca `FONT_FILE` alınır. Geçici bir adla kopyalanıp yerine taşınır; klasörde hiçbir zaman yarım yazı tipi durmaz.
- Özet programları sırayla denenir: Linux'ta `sha256sum`, sonra `shasum -a 256`, macOS'ta tersi, Windows'ta `certutil -hashfile … SHA256`. Hiçbiri yoksa `MissingTool` ilkini söyler.
- Linux: kopyadan sonra `fc-cache -f <klasör>`; fontconfig yoksa atlanır. Windows: `reg add HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts /v "Symbols Nerd Font Mono Regular (TrueType)" /d <dosya>`. macOS: bir şey yapılmaz.
- `Done { path }` Linux'ta klasör, macOS ve Windows'ta yazı tipi dosyasıdır; onu silmek kurulumu geri alır. Windows'ta kayıt defteri satırı kalır ve olmayan bir dosyayı gösterir; Windows bunu yok sayar.
- Görevin etiketi, notları ve hata sebebi `task` çağrılınca çevrilir; `Progress::text` çağrıldığı yerde, görünümde çevirir.

## Dil anahtarları

- `quvyta.nerd-font.task`, `downloading`, `downloading-share` (`{percent}`), `verifying`, `installing`, `done` (`{path}`), `cancelled`.
- `quvyta.nerd-font.error-folder`, `error-tool`, `error-download`, `error-verify`, `error-checksum`, `error-extract`, `error-copy`, `error-register` (`{detail}`).
- `quvyta.nerd-font.after-install`, `found`, `missing`.

## Tema ve ikonlar

- `GlyphSample` kendine ait bir zemin olmadan `text` rengini ve etkin ikon setinin gliflerini kullanır.
- Bu sayfadaki durum ve sonuç bir `Badge` ile bir `Text`, ilerleme bir `ProgressBar`'dır.
