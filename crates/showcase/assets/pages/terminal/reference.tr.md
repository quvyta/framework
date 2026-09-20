## Metotlar

- `Terminal::new(&oturum)` — bir oturumu çizer; `pty` özelliğiyle gelir.
- `.pass_through(kapsam, eylem)` — o tuş haritası eylemine bağlı tuşlar programa gitmez; üst bileşenlere, tuş dinleyicilerine ve `App::action`'a ilerler; her eylem için bir kez çağrılır. Tuş geldiği anda geçerli olan tuş haritası karar verir, tuşlar yeniden bağlanınca bu da değişir. Karakter yazan tuşlar (en fazla `shift` ile bir karakter ya da boşluk) her zaman programa ulaşır. Varsayılan: `shift tab` ve `ctrl q` dışında hiçbir şey geçmez.
- Terminalin düğümünde `NodeMut::on_action(kapsam, eylem, mesaj)` — geçirilen eylem odak terminaldeyken `mesaj`'ı gönderir, başka yerdeyken `App::action`'a gider; odağı tek tuşla aç-kapa yapmanın yolu.
- `TerminalSession::shell(klasör)`, `TerminalSession::spawn(program, argümanlar, klasör)` — bir programı sözde terminalde başlatır.
- `TerminalSession::builder(program)` — bir `TerminalBuilder`: `.args(argümanlar)`, `.folder(klasör)` (varsayılan: ev klasörü), `.env(ad, değer)`, `.size(sütun, satır)` (varsayılan 80 × 24), `.scrollback(satır)` (varsayılan 5000), `.coalesce(aralık)` (varsayılan sıfır), sonra `.spawn()`. `spawn` bu varsayılanlarla kurulan builder'dır.
- `.watch()` — bir `TerminalWatch`; `.write(baytlar)`, `.kill()`, `.exit()`.
- `.pid()` — programın süreç kimliği; süreç grubunun kimliği de odur.
- `.terminate(süre)` — süreç grubuna SIGHUP, `süre` dolduğunda hâlâ çalışıyorsa SIGKILL; hemen döner. Süre boyunca oturumu elinde tut: son tutamağı bırakmak programı hemen bitirir.
- `TerminalWatch::next()` — `TerminalEvent::Output` ya da `TerminalEvent::Exited(kod)` gelene kadar bekler; `Command::perform` içinde çalıştır.
- `TerminalWatch::next_change()` — bir `TerminalChange` gelene kadar bekler: `Output`, `Title(metin)`, `WorkingFolder(yol)`, `Bell`, `Notify { title, body }` ya da `Exited(kod)`. Enum genişletilebilir (non-exhaustive).
- `TerminalWatch::next_change_within(süre)` — aynı bekleme, sınırlı: `Some(değişiklik)` ya da `süre` içinde bildirilecek bir şey olmadıysa `None`. Testler için: testler `Command::perform`'u olduğu yerde çalıştırır, sessiz bir programı sınırsız beklemekten hiç dönmez. Sonrasında oturum kullanılmaya devam eder. Çalışan uygulama `next_change`'i kullanmayı sürdürür.

## Davranış

- Odaktayken tuşlar xterm gibi kodlanır: kontrol harfleri, kaçış öneki olarak `alt`, değiştirici kodlarıyla normal ya da uygulama modunda oklar, işlev tuşları; `shift tab` hariç (odağı taşır).
- `ctrl q` uygulamaya bırakılır.
- Program o modu açtıysa yapıştırmalar köşeli yapıştırma işaretleriyle sarılır.
- Tekerlek geri kaydırma sınırına kadar (varsayılan 5000 satır) geri kaydırır ya da alternatif ekranda üç ok tuşu gönderir.
- Program fare bildirimini açtığında (9, 1000, 1002 ve 1003 modları; varsayılan, UTF-8 ya da SGR kodlamasıyla) basmalar, bırakmalar, sürüklemeler, gezinmeler ve tekerlek (64 ve 65 düğmeleri) xterm'in gönderdiği gibi, terminalin köşesinden sayılan hücrelerle gönderilir. Programın aldığı bir basma, imleci bırakılana kadar tutar; kenarı aşan sürükleme en yakın hücreyi bildirir. `shift` ile basma ya da tekerlek bunun yerine seçer ve geri kaydırır.
- İmleç, program gizlemedikçe canlı ekranda `terminal-cursor` ile çizilir.
- Program çıktıktan sonra bir not çıkış kodunu gösterir; tuşlar artık gönderilmez.
- Program `TERM=xterm-256color` ve `COLORTERM=truecolor` ile, ardından builder'ın değişkenleriyle başlar; bunlar varsayılanların yerine geçebilir.
- Bildirimler: OSC 0 ve 2 başlığı, OSC 7 `file://makine/yol` klasörü koyar (yüzde kodlaması çözülür, makine adına bakılmaz), kaçış dizisi dışındaki BEL zili çalar, OSC 9 `metin` ve OSC 777 `notify;başlık;metin` bildirimdir; sayıyla başlayan OSC 9 (ConEmu'nun ilerleme göstergesi gibi) bildirim sayılmaz. Okumalar arasında bölünen diziler bütün olarak duyulur. Okunmamış başlık ve klasörün en yenisi kalır, okunmamış ziller bir sayılır, en fazla sekiz bildirim bekler. Bildirimler, birlikte geldikleri çıktıdan önce gelir.
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
