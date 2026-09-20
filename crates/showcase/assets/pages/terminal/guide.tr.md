## Ne zaman kullanılır

Kullanıcının uygulamanın içinde gerçek bir kabuğa ya da gerçek, etkileşimli bir programa ihtiyacı olduğunda terminal kullan: bir container'ın yanında konsol, bir REPL, bir veritabanı istemcisi. Yalnızca gösterdiğin çıktılar için log görünümü daha hafiftir ve aranabilir.

## Adım adım

1. `Cargo.toml` içinde framework'ün `pty` özelliğini aç.
2. Bir program başlat: `TerminalSession::shell(&klasör)` ya da `TerminalSession::spawn(program, &argümanlar, &klasör)`; oturumu durumunda tut.
3. İzle: `let watch = session.watch();` ile `Command::perform(move || Msg::Changed(watch.next()))` döndür.
4. `TerminalEvent::Output` gelince izlemeyi yeniden başlat; görünüm kendiliğinden yeniden çizilir. `TerminalEvent::Exited(kod)` gelince dur.
5. Göster: `Terminal::new(&session)`; her düğüm gibi boyutlandırılır. Programı `session.terminate(süre)`, `session.kill()` ile ya da oturumu bırakarak durdur.

Daha fazla denetim için başlatmayı `TerminalSession::builder(program)` ile tarif et: `.args(..)`, `.folder(..)`, `.env(ad, değer)`, `.size(sütun, satır)`, `.scrollback(satır)` ve `.coalesce(aralık)`, sonra `.spawn()`. Programın başlığını, klasörünü, zilini ve bildirimlerini de duymak için `watch.next_change()` ile izle.

## Nasıl çalışır

- **Gerçek bir sözde terminal.** Program bir PTY içinde çalışır; kabuklar, editörler ve tam ekran araçlar her terminaldeki gibi davranır. Çıktı arka planda, 5000 satırlık ya da `.scrollback(satır)` ne derse o kadar geçmişi olan bir ekrana ayrıştırılır; oyun alanı bunu bir sonraki kabuk için ayarlar.
- **Ham ANSI değil, tema renkleri.** On altı klasik renk temadan gelir: kırmızı temanın tehlike tonu, yeşil başarı tonu, zemin yüzey rengidir. 256 renk ve 24 bit renkler istendiği gibi çizilir.
- **Tuşlar programa gider.** Odaktayken Tab ve Esc dahil her tuş gönderilir; `shift tab` odaktan çıkar, `ctrl q` yine uygulamadan çıkar. Varsayılan olarak uygulamaya başka hiçbir tuş ulaşmaz. Program destekliyorsa yapıştırma, köşeli yapıştırma moduyla gönderilir.
- **Seçtiğin eylemler geçebilir.** `.pass_through(Scope::Global, "help")` ile o tuş haritası eyleminin tuşları programa gitmez, her zamanki yoldan ilerler: terminali saran yan panele, tuş dinleyicilerine ve `App::action`'a. Her eylem için bir kez çağır. Karakter yazan bir tuş, yani en fazla `shift` ile basılan bir karakter ya da boşluk, her zaman programa gider: yardım hem `?` hem `f1` üzerindeyse `?` kabuğa yazılır, `f1` yardımı açar. Bu sayfa yardımı geçirir; kabuk odaktayken de `f1` çalışır.
- **Tek tuşla çık, tek tuşla dön.** Geçirilen bir eylem terminalin içinde "çık", dışında "geri dön" anlamına gelebilir. Eylemi geçir, terminalin düğümünde `.on_action(Scope::App, "terminal-focus", Msg::Leave)` ile cevapla ve `App::action`'da `Msg::Enter`'e çevir; iki mesaj da `Command::focus` ile öbür yere odaklanır. Hangi mesajın geleceğine odak karar verir, odak nasıl gelmiş olursa olsun; kabuğa tıklayıp girmek de doğru ayrılır. Bu sayfada `ctrl alt boşluk` kabuktan başlat düğmesine geçer ve geri döner.
- **Başlatma seçenekleri.** `TerminalSession::spawn`, her şeyi varsayılan olan builder'dır: 80 × 24 ekran, 5000 satır geri kaydırma, her okuma bildirilir. Builder'a bileşenin alacağı boyutu ver ki tam ekran bir program ilk ekranını doğru boyutta çizsin. Ek değişkenler programa `TERM=xterm-256color` ve `COLORTERM=truecolor`'ın üstüne ulaşır, istersen onların yerine geçer.
- **Programın kendisi hakkında söyledikleri.** `TerminalWatch::next_change` bir `TerminalChange` döndürür: `Output`, `Title` (OSC 0 ve 2), `WorkingFolder` (OSC 7, yüzde kodlaması çözülmüş), `Bell`, `Notify { title, body }` (OSC 9 ve OSC 777) ve `Exited(kod)`. Başlık ve klasör okunmamış öncekinin yerine geçer, okunmamış ziller bir sayılır, en fazla sekiz bildirim bekler; bir sel belleği hiç büyütmez. `next` eskisi gibi yalnızca çıktıyı ve bitişi bildirir. Bu sayfada kabuğun üstündeki şerit son başlığı, klasörü, bildirimi ve zilin kaç kez çaldığını gösterir.
- **Düzenli hızda çıktı.** `.coalesce(Duration::from_millis(16))` çıktıyı en fazla aralıkta bir bildirir; `yes` ya da hızlı bir log her okumada değil her aralıkta bir kare ister, son baytlar da bir aralıktan fazla bekletilmez. Bu sayfa bir kare kullanır.
- **Testler için sınırlı bekleme.** Ekran testi `Command::perform`'u olduğu yerde çalıştırır; canlı ama sessiz bir programı sınırsız beklemek hiç geri dönmez. `watch.next_change_within(Duration::from_secs(2))` ya `Some(değişiklik)` ya da program o süre içinde bir şey söylemediyse `None` verir; oturum kullanılmaya devam eder: yine sor ya da programa yaz ve yine sor. Çalışan uygulamanın izleyicisi kendi iş parçacığındadır, `next_change`'i kullanmayı sürdürür.
- **Kibar bitiş.** `session.terminate(süre)` programın süreç grubuna, kapanan bir terminal penceresi gibi SIGHUP gönderir; `süre` içinde bitmezse SIGKILL. Hemen döner; bitişi izleyici bildirir. Süre boyunca oturumu elinde tut: son tutamağı bırakmak programı hemen bitirir, bu yüzden bu sayfadaki Durdur oturumu yerinde bırakır. `session.pid()` süreç kimliğini verir. Bu sayfadaki Durdur kabuğu böyle bitirir.
- **Boyut bileşeni izler.** Bileşen çizerken boyutunu ister; izleyici bunu arka plan iş parçacığında uygular, çizim hiçbir zaman sürece dokunmaz.
- **Tekerlekle geçmiş.** Tekerlek önceki çıktılara geri kaydırır, sessiz bir not ne kadar geride olduğunu söyler; yazmak canlı ekrana döndürür. Tam ekran programlara bunun yerine ok tuşları gönderilir; fareyi isteyen programlara tekerleğin kendisi gider.
- **Çıktı seçilebilir.** Ekranda sürüklemek programın çıktısını seçer; `ctrl c` kopyalar (metin seçiliyken `ctrl c` programı kesmek yerine kopyalar), sağ tık Kopyala ve Ham kopyala sunar. Geri kaydırma ve çıkış notları temiz kopyaya girmez.
- **Fareyi isteyen program alır.** Bir program fareyi istediğinde tıklamalar, sürüklemeler ve tekerlek ona gider: `htop`'u ya da `:set mouse=a` yazdıktan sonra `vim`'i dene. `shift` ile sürüklemek yine çıktıyı seçer, `shift` ile tekerlek yine geçmişe kaydırır.

## Sık yapılan hatalar

- **İzlemeyi yeniden başlatmamak.** Çalışan bir izleyici yoksa ekran güncellenmez, boyut değişiklikleri bekler.
- **Göstermediğin oturumları tutmak.** Oturum programını canlı tutar; bırak ya da `terminate` veya `kill` çağır.
- **`TerminalChange`'i her şeyi yakalayan kolsuz eşlemek.** İleride yeni bildirim türleri gelebilir; eşlemeyi `_ => {}` ile bitir ve izlemeyi orada da yeniden başlat.
- **Testlerde kabuğu çalışma motoru üzerinden başlatmak.** Testler arka plan işlerini sırayla çalıştırır ve sınırsız izleyici bekler; `TerminalWatch::next`'i kendin sür ya da `next_change_within` ile sınırlı bir değişiklik iste.
