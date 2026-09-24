## Metotlar

- `Terminal::new(&oturum)` — bir oturumu çizer; `pty` özelliğiyle gelir.
- `.pass_through(kapsam, eylem)` — o tuş haritası eylemine bağlı tuşlar programa gitmez; üst bileşenlere, tuş dinleyicilerine ve `App::action`'a ilerler; her eylem için bir kez çağrılır. Tuş geldiği anda geçerli olan tuş haritası karar verir, tuşlar yeniden bağlanınca bu da değişir. Karakter yazan tuşlar (en fazla `shift` ile bir karakter ya da boşluk) her zaman programa ulaşır. Varsayılan: `shift tab` ve `ctrl q` dışında hiçbir şey geçmez.
- `.read_only()` — yalnızca gösteren terminal: oturuma hiç yazmaz (tuş, yapıştırma, fare bildirimi, alternatif ekranda tekerlek ve boyut isteği, hiçbiri), Tab ile de tıklamayla da odak almaz ve soluk çizilir. Renkler, geniş karakterler, programın bıraktığı imleç, çıktıyı seçmek ve geri kaydırmak aynen kalır. Programı bittikten sonra açık kalan bir pencere için.
- Terminalin düğümünde `NodeMut::on_action(kapsam, eylem, mesaj)` — geçirilen eylem odak terminaldeyken `mesaj`'ı gönderir, başka yerdeyken `App::action`'a gider; odağı tek tuşla aç-kapa yapmanın yolu.
- `TerminalSession::shell(klasör)`, `TerminalSession::spawn(program, argümanlar, klasör)` — bir programı sözde terminalde başlatır.
- `TerminalSession::builder(program)` — bir `TerminalBuilder`: `.args(argümanlar)`, `.folder(klasör)` (varsayılan: ev klasörü), `.env(ad, değer)`, `.env_remove(ad)` (program değişkeni hiç görmez, boş olarak bile; bir ad için son `env` ya da `env_remove` geçerlidir), `.size(sütun, satır)` (varsayılan 80 × 24), `.scrollback(satır)` (varsayılan 5000), `.coalesce(aralık)` (varsayılan sıfır), sonra `.spawn()`. `spawn` bu varsayılanlarla kurulan builder'dır.
- `.watch()` — bir `TerminalWatch`; `.write(baytlar)`, `.kill()`, `.exit()`. `write` kişinin girdisi sayılır, çünkü bir tuş vuruşunun geçtiği yol odur; programın bittiği bilindikten sonra hata verir.
- `.paste(metin)` — `metin`'i yapıştırma olarak gönderir: program köşeli yapıştırmayı açtıysa `\x1b[200~` ile `\x1b[201~` arasında, açmadıysa düz. İki işaret de `metin`'den önce çıkarılır. Programın bittiği bilindikten sonra hata verir. `last_input`'u ilerletmez: bu, uygulamanın yazmasıdır. `Terminal` bileşeni kişinin yapıştırmasını da aynı çağrıyla gönderir; o `last_input`'u ilerletir.
- `.last_output()` — bir `Instant`: programın en son ne zaman bir şey yazdığı; okuyan iş parçacığı baytlar geldiği anda işaretler. Hiç yazmamışsa oturumun başlangıcı.
- `.last_input()` — bir `Instant`: programa en son ne zaman tuş yazıldığı (`write` ile ya da bileşenle). Hiç yazılmamışsa oturumun başlangıcı. Kimsenin istemediği bir metni yazmadan önce `last_output` ile birlikte sor.
- `.pid()` — programın süreç kimliği; süreç grubunun kimliği de odur.
- `.terminate(süre)` — süreç grubuna SIGHUP, `süre` dolduğunda hâlâ çalışıyorsa SIGKILL; hemen döner. Süre boyunca oturumu elinde tut: son tutamağı bırakmak programı hemen bitirir.
- `TerminalWatch::next()` — `TerminalEvent::Output` ya da `TerminalEvent::Exited(kod)` gelene kadar bekler; `Command::perform` içinde çalıştır.
- `TerminalWatch::next_change()` — bir `TerminalChange` gelene kadar bekler: `Output`, `Title(metin)`, `WorkingFolder(yol)`, `Bell`, `Notify { title, body }` ya da `Exited(kod)`. Enum genişletilebilir (non-exhaustive).
- `TerminalWatch::next_change_within(süre)` — aynı bekleme, sınırlı: `Some(değişiklik)` ya da `süre` içinde bildirilecek bir şey olmadıysa `None`. Testler için: testler `Command::perform`'u olduğu yerde çalıştırır, sessiz bir programı sınırsız beklemekten hiç dönmez. Sonrasında oturum kullanılmaya devam eder. Çalışan uygulama `next_change`'i kullanmayı sürdürür.

## Davranış

- Odaktayken tuşlar xterm gibi kodlanır: kontrol harfleri, kaçış öneki olarak `alt`, değiştirici kodlarıyla normal ya da uygulama modunda oklar, işlev tuşları; `shift tab` hariç (odağı taşır).
- `ctrl q` uygulamaya bırakılır.
- Program o modu açtıysa yapıştırmalar köşeli yapıştırma işaretleriyle sarılır; bunu `paste` yapar ve bileşen kişinin yapıştırması için de onu kullanır. Metnin içindeki `\x1b[200~` ya da `\x1b[201~` çıkarılır; böylece başka yerden gelen bir metin yapıştırmayı erken bitirip gerisini tuş gibi okutamaz.
- Programın bittiği kaydedildikten sonra `write` ve `paste` reddedilir: sözde terminal program gittikten sonra da baytları alır ve onları kimse okumaz.
- Tekerlek geri kaydırma sınırına kadar (varsayılan 5000 satır) geri kaydırır ya da alternatif ekranda üç ok tuşu gönderir.
- Program fare bildirimini açtığında (9, 1000, 1002 ve 1003 modları; varsayılan, UTF-8 ya da SGR kodlamasıyla) basmalar, bırakmalar, sürüklemeler, gezinmeler ve tekerlek (64 ve 65 düğmeleri) xterm'in gönderdiği gibi, terminalin köşesinden sayılan hücrelerle gönderilir. Programın aldığı bir basma, imleci bırakılana kadar tutar; kenarı aşan sürükleme en yakın hücreyi bildirir. `shift` ile basma ya da tekerlek bunun yerine seçer ve geri kaydırır.
- İmleç, program gizlemedikçe canlı ekranda `terminal-cursor` ile çizilir. `read_only()` terminalde ekranla birlikte solar.
- `read_only()` terminal her rengi, imlecinkini de, zemine %45 oranında karıştırır; bu, programın kendi soluk işaretlediği hücrenin çizildiği oranın aynısıdır. Çıkış ve geri kaydırma notları kendi renklerini korur: onlar bileşenin sözleridir, programın ekranı değil.
- Program çıktıktan sonra bir not çıkış kodunu gösterir; tuşlar artık gönderilmez. Yapıştırma ve alternatif ekranda tekerlek adımı da hiçliğe yazılmak yerine geri verilir: yapıştırma `ClipboardEvent::Pasted` olarak `App::clipboard`'a ulaşır, böylece uygulama metnin hiçbir yere gitmediğini söyleyebilir; tekerlek ise terminali tutan şeye ulaşır. Bilerek düşürülen tek yazma fare bildirimidir: biten bir programın imlecin nerede olduğundan öğreneceği bir şey yoktur ve geri verilen bir basma terminalin arkasındaki şeye etki ederdi.
- Program `TERM=xterm-256color` ve `COLORTERM=truecolor` ile, ardından builder'ın değişkenleriyle başlar; bunlar varsayılanların yerine geçebilir.
- Bildirimler: OSC 0 ve 2 başlığı, OSC 7 `file://makine/yol` klasörü koyar (yüzde kodlaması çözülür, makine adına bakılmaz), kaçış dizisi dışındaki BEL zili çalar, OSC 9 `metin` ve OSC 777 `notify;başlık;metin` bildirimdir; sayıyla başlayan OSC 9 (ConEmu'nun ilerleme göstergesi gibi) bildirim sayılmaz. Okumalar arasında bölünen diziler bütün olarak duyulur. Okunmamış başlık ve klasörün en yenisi kalır, okunmamış ziller bir sayılır, en fazla sekiz bildirim bekler. Bildirimler, birlikte geldikleri çıktıdan önce gelir.
- Her tuş, bir öncekinin ne kadar ardından gelirse gelsin programa ulaşır: terminal yazı alır, bu yüzden Enter ve Boşluk hiçbir zaman basılı tutulan tuş sayılmaz, terminalin bildirdiği tekrarlar da gönderilir.
- `CSI satır;sütun f` (HVP), anlamı olan `CSI satır;sütun H` (CUP) gibi çizilir; `f`'sinden önce özel işaret, alt parametre ya da ara bayt taşıyan bir diziye dokunulmaz. DEC çizgi kümesi G0'da ve G1'de, Shift Out ve Shift In ile izlenir: kullanımdayken `_` ile `~` arasındaki baytlar karşılık geldikleri karakterlerle görünür (`lqqkx`, `┌──┐│` olur); `ESC ( B` ve tam sıfırlama (`ESC c`) ASCII'ye döndürür. İkisi de okumalar arasında izlenir. Satır sonu kaydırma kipi (`CSI ?7 l`) izlenmez: son sütuna varan satır her zaman alta kayar.
- 4096 bayttan uzun bir OSC dizisi orada kesilir; dizisini hiç bitirmeyen bir program belleği büyütemez.
- `.coalesce(aralık)` ile çıktı en fazla aralıkta bir bildirilir ve geldikten sonra bir aralıktan fazla bekletilmez; bildirimler ve bitiş bekletilmez.
- `next_change_within(süre)` süre içinde döner, program bir şey söylemediyse `None` verir ve bekleyen boyut değişikliklerini sınırsız beklemeler gibi uygular. `.coalesce(aralık)` varken aralıktan kısa bir sınır, çıktı gelmiş olsa da `None` verebilir: aralık yine beklenir.
- Ekran bir metin seçim bölgesidir; geri kaydırma ve çıkış notları temiz kopya için süstür. Seçim varken `ctrl c` programa gitmez, seçimi kopyalar.

## Tema anahtarları

- `terminal` — `bg`, `fg`: varsayılan renkler.
- `terminal-cursor` — `bg`, `fg`; `focus` ile.
- `terminal-note` — geri kaydırma ve çıkış notlarının `bg`, `fg` renkleri.
- Klasik renkler `raised`, `danger`, `success`, `warning`, `info`, `dim`, `muted` ve `text` tokenlarından gelir.
- Metinler: `quvyta.terminal.scrolled`, `quvyta.terminal.exited`.
