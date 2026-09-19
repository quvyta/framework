## App

- `type Msg` — olabilecek her şey; `Send` olmalı.
- `fn update(&mut self, msg) -> Command<Msg>` — bir mesajı uygular.
- `fn view(&self, ui: &mut View<Msg>)` — ekranı tarif eder; I/O yapmaz.
- `ui.map(f, |ui| ekran.view(ui))` — bir ekranın kendi mesajlarıyla kurulan, her mesajı `f` ile dönüştürülen bir sütun ekler; içindeki işleyiciler, iç içe kaplar, katmanlar ve açılır parçalar hep dönüştürülmüş gelir.
- `fn action(&self, name) -> Option<Msg>` — bir kısayol eylemini mesaja çevirir: kendi `[app]` eylemlerini ve motorun sana bıraktığı `help`, `palette` gibi global eylemleri. İsteğe bağlı.
- `fn clipboard(&self, &ClipboardEvent) -> Option<Msg>` — bileşenlerin ve fare seçiminin kopyalarını, hiçbir bileşenin almadığı yapıştırmaları duyar. İsteğe bağlı.
- `fn init(&mut self) -> Command<Msg>` — ilk karenin başında, görünümü kurulmadan önce bir kez çalışır; döndürdüğü `Command::focus` ilk tuştan önce yerindedir. İsteğe bağlı.
- `fn resized(&self, Size) -> Option<Msg>` — terminal ölçüsü, açılışta (`init`'ten önce) ve her yeniden boyutlanmada; `ui.size()`'ın bildirdiğiyle aynıdır, mesajı `update`'ten geçer. İsteğe bağlı.
- `fn before_quit(&self) -> Option<Msg>` — çalışma motoru kullanıcı adına çıkmadan önce sorulur (çıkış bağı, komut paletinden çıkış eylemi); bir mesaj uygulamayı açık tutar ve onun yerine teslim edilir. `Command::quit()` hiç sorulmaz. İsteğe bağlı.
- `fn terminating(&self, Termination) -> Option<Msg>` — uygulamayı sistemin kapattığını duyar: bir `SIGTERM` ya da dışarıdan gelen `SIGINT` için `Termination::Terminate`, bir `SIGHUP` için `Termination::Hangup`. `None` hemen çıkar; bir mesaj kaydedip çıkması için açık tutar. Varsayılan olarak terminate'e `before_quit` cevap verir, kopuş çıkar. İsteğe bağlı.

## Command

- `Command::none()` — yapılacak bir şey yok.
- `Command::batch([..])` — sırayla birkaç komut.
- `Command::perform(|| msg)` — işi arka plan iş parçacığında çalıştırır, sonra mesajını teslim eder.
- `Command::quit()` — uygulamadan çıkar; uygulamanın kendi kararıdır, bu yüzden `before_quit` sorulmaz.
- `Command::focus("isim")` — `.id("isim")` ile adlandırılmış bileşene odaklanır.
- `Command::set_theme(id)`, `Command::set_locale(kod)`, `Command::set_icon_mode(mod)` — çalışırken görünümü ve dili değiştirir.
- `Command::set_reduced_motion(bool)`, `Command::set_pillar(stil)`, `Command::set_slide(bool)` — çalışırken hareketi, çubuğu ve seçimdeki kaymayı değiştirir.
- `Command::copy(metin)` — panoya kopyalar, SSH üzerinden de çalışır; `Command::read_clipboard(|metin| msg)` panoyu okur.
- `Command::confirm(onay)` — bir pencerede soru sorar ve cevabı teslim eder.
- `Command::toast(bildirim)`, `Command::dismiss_toast(anahtar)`, `Command::toast_corner(köşe)` — bildirimler.
- `Command::task(iş)`, `Command::cancel_task(id)` — ilerlemesi görünen, iptal edilebilen arka plan işi.
- `komut.map(f)` — aynı işi `f(mesaj)` teslim ederek yapar: bir ekranın komutları için, uygulamanın `update` fonksiyonunda. Perform ve task işlerinin sonradan gönderdiği mesajlar da `f`'den geçer; `f` `Send + Sync` olmalıdır.

## Runtime

- `Runtime::new(app)` — gömülü dosyalarla bir çalışma motoru.
- `.theme_dir(yol)`, `.icon_dir(yol)`, `.locale_dir(yol)`, `.keymap_file(yol)` — kendi dosyalarını gömülülerin üstüne yükler.
- `.theme_source(dosya, metin)`, `.icon_source(dosya, metin)`, `.locale_source(dosya, metin)`, `.keymap_source(dosya, metin)` — aynı dosyaları metin olarak yükler, ör. `include_str!("../locales/tr.toml")`; kurulan programın yanında dosya taşıması gerekmez. Metin, eşleşen yoldan sonra gelir ve kazanır; tema ve ikon setlerinde dosya adının kökü id olur. Ayrıca verilen yol artık zorunlu değildir: okunamadığında metin onun yerine geçer ve sebep, programı durdurmak yerine bir tanılamaya dönüşür.
- `.theme(id)` — Monochrome yerine başka bir temayla başlar.
- `.settings(&ayarlar)` — kullanıcının kaydettiği görünümle başlar; kayıtlı değerler `.theme` seçiminden güçlüdür.
- `.run()` — uygulama kapanana kadar terminali yönetir; çıkışta ve panikte terminali eski haline getirir. Unix'te `SIGTERM`, `SIGINT` ve `SIGHUP`'ı yakalar, `App::terminating`'e bildirir ve her zaman sınırlı sürede biter: süre dolunca, ikinci bir `SIGTERM` ya da `SIGINT`'te hemen, döngü takılmışsa bir saniye sonra sinyalin kendisiyle. Devir sırasında gelen sinyal önce devredilen programa ulaşır.

## Termination

- `Termination::Terminate` — bir `SIGTERM` ya da dışarıdan gelen `SIGINT`; terminal hâlâ yerinde.
- `Termination::Hangup` — bir `SIGHUP`: terminal gitti ve bundan sonra hiçbir şey çizilmez; tekrarlanan kopuş bir kez bildirilir.
- `sebep.grace()` — uygulamanın kendi kendine çıkma süresi: terminate'ten sonra beş, kopuştan sonra üç saniye.

## Harness

- `Harness::new(app, genişlik, yükseklik)` — gömülü ortam, hemen çizilmiş halde; ölçüyü `resized`'a bildirir ve `init`'i çalıştırır. Kendi ortamın için `Harness::with_env(app, ortam, genişlik, yükseklik)`.
- `.resize(genişlik, yükseklik)` — ekranı yeniden boyutlar, yeni ölçüyü `resized`'a bildirir ve baştan çizer.
- `.press("ctrl+s")`, `.key(olay)`, `.type_text("merhaba")`, `.paste(metin)` — klavye girdisi.
- `.click(x, y)`, `.click_text("Kaydet")`, `.hover(x, y)`, `.drag(nereden, nereye)`, `.mouse(tür, x, y)` — fare girdisi.
- `.send(msg)` — bir mesajı bileşen göndermiş gibi teslim eder.
- `.advance(süre)` — sahte saati ilerletir; animasyonlar ve parlamalar onu takip eder, süresi dolan bir kapanış çıkar. `.render()` yeniden çizer.
- `.terminate(Termination::Terminate)`, `.terminate(Termination::Hangup)` — bir `SIGTERM` ya da `SIGHUP`'ı taklit eder: `terminating` onu terminaldeki gibi duyar, ikinci terminate çıkar, tekrarlanan kopuş hiçbir şeyi değiştirmez.
- `.set_theme(id)`, `.set_locale(kod)`, `.set_glyph_mode(mod)`, `.set_reduced_motion(bool)`, `.set_system_clipboard(Some(metin))` — ortamı değiştirir.
- `.screen()`, `.find(metin)`, `.fg(x, y)`, `.bg(x, y)`, `.is_bold(x, y)`, `.buffer()`, `.html(başlık)` — çizileni okur. Çift genişlikli karakter, kapladığı hücre olmadan bir kez okunur: `screen().contains("防火墙")` tutar, `find` çizildiği sütunu verir.
- `.app()`, `.env()`, `.is_focused("isim")`, `.copied()`, `.clipboard()`, `.quit_requested()` — uygulamanın ve motorun durumuna bakar.

## Tuşlar

- `tab` ve `shift tab` odağı gezdirir; `ctrl q` `before_quit`'e sorduktan sonra çıkar; `f12` hata ayıklama katmanını açar.
