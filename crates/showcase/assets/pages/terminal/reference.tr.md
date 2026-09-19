## Metotlar

- `Terminal::new(&oturum)` — bir oturumu çizer; `pty` özelliğiyle gelir.
- `.pass_through(kapsam, eylem)` — o tuş haritası eylemine bağlı tuşlar programa gitmez; üst bileşenlere, tuş dinleyicilerine ve `App::action`'a ilerler; her eylem için bir kez çağrılır. Tuş geldiği anda geçerli olan tuş haritası karar verir, tuşlar yeniden bağlanınca bu da değişir. Karakter yazan tuşlar (en fazla `shift` ile bir karakter ya da boşluk) her zaman programa ulaşır. Varsayılan: `shift tab` ve `ctrl q` dışında hiçbir şey geçmez.
- Terminalin düğümünde `NodeMut::on_action(kapsam, eylem, mesaj)` — geçirilen eylem odak terminaldeyken `mesaj`'ı gönderir, başka yerdeyken `App::action`'a gider; odağı tek tuşla aç-kapa yapmanın yolu.
- `TerminalSession::shell(klasör)`, `TerminalSession::spawn(program, argümanlar, klasör)` — bir programı sözde terminalde başlatır.
- `.watch()` — bir `TerminalWatch`; `.write(baytlar)`, `.kill()`, `.exit()`.
- `TerminalWatch::next()` — `TerminalEvent::Output` ya da `TerminalEvent::Exited(kod)` gelene kadar bekler; `Command::perform` içinde çalıştır.

## Davranış

- Odaktayken tuşlar xterm gibi kodlanır: kontrol harfleri, kaçış öneki olarak `alt`, değiştirici kodlarıyla normal ya da uygulama modunda oklar, işlev tuşları; `shift tab` hariç (odağı taşır).
- `ctrl q` uygulamaya bırakılır.
- Program o modu açtıysa yapıştırmalar köşeli yapıştırma işaretleriyle sarılır.
- Tekerlek 5000 satıra kadar geri kaydırır ya da alternatif ekranda üç ok tuşu gönderir.
- Program fare bildirimini açtığında (9, 1000, 1002 ve 1003 modları; varsayılan, UTF-8 ya da SGR kodlamasıyla) basmalar, bırakmalar, sürüklemeler, gezinmeler ve tekerlek (64 ve 65 düğmeleri) xterm'in gönderdiği gibi, terminalin köşesinden sayılan hücrelerle gönderilir. Programın aldığı bir basma, imleci bırakılana kadar tutar; kenarı aşan sürükleme en yakın hücreyi bildirir. `shift` ile basma ya da tekerlek bunun yerine seçer ve geri kaydırır.
- İmleç, program gizlemedikçe canlı ekranda `terminal-cursor` ile çizilir.
- Program çıktıktan sonra bir not çıkış kodunu gösterir; tuşlar artık gönderilmez.
- Program `TERM=xterm-256color` ve `COLORTERM=truecolor` ile başlar.
- Ekran bir metin seçim bölgesidir; geri kaydırma ve çıkış notları temiz kopya için süstür. Seçim varken `ctrl c` programa gitmez, seçimi kopyalar.

## Tema anahtarları

- `terminal` — `bg`, `fg`: varsayılan renkler.
- `terminal-cursor` — `bg`, `fg`; `focus` ile.
- `terminal-note` — geri kaydırma ve çıkış notlarının `bg`, `fg` renkleri.
- Klasik renkler `raised`, `danger`, `success`, `warning`, `info`, `dim`, `muted` ve `text` tokenlarından gelir.
- Metinler: `quvyta.terminal.scrolled`, `quvyta.terminal.exited`.
