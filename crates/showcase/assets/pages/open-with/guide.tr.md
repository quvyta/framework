## Ne zaman kullanılır

Uygulamanız kişinin bir dosyayı başka bir programda açmasına izin veriyorsa kullanın: dosya gezgininde Enter, bir "Birlikte aç" menüsü, bir düzenleyicinin dosya ağacı. Cevabı masaüstü zaten biliyor. Ortak MIME veritabanı dosyanın türünü söyler, kurulu her program `.desktop` dosyasında hangi türleri açtığını yazar, `mimeapps.list` de kişinin kendi varsayılanlarını tutar. Bunları okumak, kişinin dosya yöneticisinin verdiği cevabın aynısını verir.

- **Kendi tablonuzu tutmayın.** Kodunuzdaki bir uzantı listesi, bir program kurulduğu gün masaüstüyle ayrışır.
- **Her şeyi `$EDITOR` ile açmayın.** Resim bir resim göstericisine aittir; Rust dosyası da kişinin metin için seçtiği düzenleyiciye.
- **Kabuk satırı kurmayın.** `DesktopApp::command` bir argüman listesi döndürür; `notlarım "v2"; rm -rf ~.txt` adlı bir dosya tek bir argümandır, başka hiçbir şey değil.

## Adım adım

1. Klasörleri başlangıçta bir kez okuyun: `let dirs = XdgDirs::from_env(|ad| std::env::var(ad).ok());`.
2. Türleri ve programları yükleyin: `let openers = Openers::load(&dirs, &dil, std::env::var_os("PATH").as_deref());`; `dil`, `LANG`'deki gibi kişinin dilidir.
3. Tek bir dosyayı sorun: `let secenekler = openers.for_file(&yol);` türünü, programlarını ve hangisinin varsayılan olduğunu verir.
4. Pencere açılıp açılamayacağını bir kez sorun: `let grafik = graphical_session(|ad| std::env::var(ad).ok());`.
5. Açın: `program.launch(&yol, grafik, Msg::Acildi)`, `update`'ten döndürülecek komutu ya da `NoGraphicalSession` verir.
6. Menüde açılamayanı sönük gösterin: `program.can_start(grafik)`.

## Nasıl çalışır

- **Tür önce addan gelir.** `globs2` desenlerinin ağırlıkları vardır: eşleşenlerin en ağırı kazanır, sonra büyük-küçük harfe duyarlı olan duyarsızı geçer, sonra en uzunu; `yedek.tar.gz` yalnızca gzip değil, sıkıştırılmış bir tar'dır.
- **Kimsenin tanımadığı ad okunur.** İlk 4 KiB karar verir: boş ya da NUL içermeyen UTF-8 ise `text/plain`, değilse `application/octet-stream`. Klasör `inode/directory`'dir.
- **Türlerin ebeveynleri vardır.** `subclasses`, `text/x-rust`'ın bir `text/plain` olduğunu söyler, zaten her `text/*` türü öyledir; kişinin metin için seçtiği düzenleyici Rust kaynağını da açar.
- **Türlerin sözle adları vardır.** `openers.mime.comment(&mime, &dil)`, `text/x-rust` için "Rust kaynak kodu" verir; veritabanında varsa kişinin dilinde. "Birlikte aç" penceresinin başlığı ya da bir özellikler penceresi için.
- **Programlar sırayla bulunur.** Kullanıcının `applications` klasörü sisteminkinden önce gelir; aynı kimlikte ilki kazanır, `Hidden=true` bir dosya kimliği sonraki klasörlerden gizler. Alt klasörler kimliğe `-` ile katılır. `PATH`'te bulunamayan bir `TryExec` programı düşürür.
- **Kişinin seçimleri sayılır.** `mimeapps.list` dosyaları en çok tercih edilenden başlayarak okunur: `[Default Applications]`, `[Added Associations]` ve bir programı bir türden daha az önemli her dosyada ve o türü sayan programlar arasından çıkaran, ama kendi dosyasının eklediğini geri almayan `[Removed Associations]`. Bir masaüstünün kendi `<masaüstü>-mimeapps.list` dosyası, aynı klasördeki ortak dosyadan önce gelir.
- **Açmak doğru yoldan gider.** Terminal programına (`Terminal=true`) terminal devredilir, program bitince ekran geri gelir. Grafik program uygulamanın yanında başlar. `DISPLAY` ya da `WAYLAND_DISPLAY` yoksa grafik program açılamaz; `launch` denemek yerine bunu söyler. İki türlü program da masaüstü dosya yöneticilerindeki gibi dosyanın klasöründe çalışır; göreli yollar ve "Farklı kaydet" dosyanın yanından başlar.
- **Testlerde hiçbir şey çalışmaz.** Test düzeneği devri ya da açmayı kaydeder; `handoffs()` ya da `opens()` ile okunur.

Bu sayfadaki her dosya, program ve seçim bu çalıştırmaya ait bir klasörde durur. Türünü ve programlarını görmek için bir dosya seçin. Düz metnin, dolayısıyla Rust'ın da varsayılanı sayfalayıcı; Rust için düzenleyici eklenmiş ve önde duruyor; resim gösterici de PNG'den çıkarılmış, bu yüzden diyagramın programı yok. Grafik oturumu kapatın: pencereli program açılamayan bir programa dönüşür.

## Sık yapılan hatalar

- **Testte sahibin klasörlerini okumak.** Geçici bir klasör üzerine bir `XdgDirs` kurun, `TryExec` için kendi `PATH`'inizi verin.
- **İlk programı varsayılan sanmak.** Tam o tür için bir program, üst türün varsayılanından önce gelir; Enter'ın hangisini açacağını `Choices::default` söyler.
- **Birden çok dosyayı tek komutla açmak.** `command` tek dosya alır; her dosya için ayrı isteyin.
- **`NoDisplay` programları "Birlikte aç"tan saklamak.** Yalnızca başlatıcılardan uzak tutulurlar; dosya açmaya devam ederler.
- **`mimeapps.list`'e yazmak.** Varsayılanlar kişinin masaüstü ayarıdır. Onları değiştirmek ayrı bir karardır.
