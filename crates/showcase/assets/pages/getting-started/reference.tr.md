## App

- `type Msg` — olabilecek her şey; `Send` olmalı.
- `fn update(&mut self, msg) -> Command<Msg>` — bir mesajı uygular.
- `fn view(&self, ui: &mut View<Msg>)` — ekranı tarif eder; I/O yapmaz.
- `fn action(&self, name) -> Option<Msg>` — bir kısayol eylemini mesaja çevirir: kendi `[app]` eylemlerini ve motorun sana bıraktığı `help`, `palette` gibi global eylemleri. İsteğe bağlı.
- `fn clipboard(&self, &ClipboardEvent) -> Option<Msg>` — bileşenlerin ve fare seçiminin kopyalarını, hiçbir bileşenin almadığı yapıştırmaları duyar. İsteğe bağlı.

## Command

- `Command::none()` — yapılacak bir şey yok.
- `Command::batch([..])` — sırayla birkaç komut.
- `Command::perform(|| msg)` — işi arka plan iş parçacığında çalıştırır, sonra mesajını teslim eder.
- `Command::quit()` — uygulamadan çıkar.
- `Command::focus("isim")` — `.id("isim")` ile adlandırılmış bileşene odaklanır.
- `Command::set_theme(id)`, `Command::set_locale(kod)`, `Command::set_icon_mode(mod)` — çalışırken görünümü ve dili değiştirir.
- `Command::set_reduced_motion(bool)`, `Command::set_pillar(stil)`, `Command::set_slide(bool)` — çalışırken hareketi, çubuğu ve seçimdeki kaymayı değiştirir.
- `Command::copy(metin)` — panoya kopyalar, SSH üzerinden de çalışır; `Command::read_clipboard(|metin| msg)` panoyu okur.
- `Command::confirm(onay)` — bir pencerede soru sorar ve cevabı teslim eder.
- `Command::toast(bildirim)`, `Command::dismiss_toast(anahtar)`, `Command::toast_corner(köşe)` — bildirimler.
- `Command::task(iş)`, `Command::cancel_task(id)` — ilerlemesi görünen, iptal edilebilen arka plan işi.

## Runtime

- `Runtime::new(app)` — gömülü dosyalarla bir çalışma motoru.
- `.theme_dir(yol)`, `.icon_dir(yol)`, `.locale_dir(yol)`, `.keymap_file(yol)` — kendi dosyalarını gömülülerin üstüne yükler.
- `.theme(id)` — Monochrome yerine başka bir temayla başlar.
- `.settings(&ayarlar)` — kullanıcının kaydettiği görünümle başlar; kayıtlı değerler `.theme` seçiminden güçlüdür.
- `.run()` — uygulama kapanana kadar terminali yönetir; çıkışta ve panikte terminali eski haline getirir.

## Harness

- `Harness::new(app, genişlik, yükseklik)` — gömülü ortam, hemen çizilmiş halde; kendi ortamın için `Harness::with_env(app, ortam, genişlik, yükseklik)`.
- `.press("ctrl+s")`, `.key(olay)`, `.type_text("merhaba")`, `.paste(metin)` — klavye girdisi.
- `.click(x, y)`, `.click_text("Kaydet")`, `.hover(x, y)`, `.drag(nereden, nereye)`, `.mouse(tür, x, y)` — fare girdisi.
- `.send(msg)` — bir mesajı bileşen göndermiş gibi teslim eder.
- `.advance(süre)` — sahte saati ilerletir; animasyonlar ve parlamalar onu takip eder. `.render()` yeniden çizer.
- `.set_theme(id)`, `.set_locale(kod)`, `.set_glyph_mode(mod)`, `.set_reduced_motion(bool)`, `.set_system_clipboard(Some(metin))` — ortamı değiştirir.
- `.screen()`, `.find(metin)`, `.fg(x, y)`, `.bg(x, y)`, `.is_bold(x, y)`, `.buffer()`, `.html(başlık)` — çizileni okur.
- `.app()`, `.env()`, `.is_focused("isim")`, `.copied()`, `.clipboard()`, `.quit_requested()` — uygulamanın ve motorun durumuna bakar.

## Tuşlar

- `tab` ve `shift tab` odağı gezdirir; `ctrl q` çıkar; `f12` hata ayıklama katmanını açar.
