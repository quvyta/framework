## Metotlar

- `Terminal::new(&oturum)` — bir oturumu çizer; `pty` özelliğiyle gelir.
- `TerminalSession::shell(klasör)`, `TerminalSession::spawn(program, argümanlar, klasör)` — bir programı sözde terminalde başlatır.
- `.watch()` — bir `TerminalWatch`; `.write(baytlar)`, `.kill()`, `.exit()`.
- `TerminalWatch::next()` — `TerminalEvent::Output` ya da `TerminalEvent::Exited(kod)` gelene kadar bekler; `Command::perform` içinde çalıştır.

## Davranış

- Odaktayken tuşlar xterm gibi kodlanır: kontrol harfleri, kaçış öneki olarak `alt`, değiştirici kodlarıyla normal ya da uygulama modunda oklar, işlev tuşları; `shift tab` hariç (odağı taşır).
- `ctrl q` uygulamaya bırakılır.
- Program o modu açtıysa yapıştırmalar köşeli yapıştırma işaretleriyle sarılır.
- Tekerlek 5000 satıra kadar geri kaydırır ya da alternatif ekranda üç ok tuşu gönderir.
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
