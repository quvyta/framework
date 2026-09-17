## Ne zaman kullanılır

Kullanıcının uygulamanın içinde gerçek bir kabuğa ya da gerçek, etkileşimli bir programa ihtiyacı olduğunda terminal kullan: bir container'ın yanında konsol, bir REPL, bir veritabanı istemcisi. Yalnızca gösterdiğin çıktılar için log görünümü daha hafiftir ve aranabilir.

## Adım adım

1. `Cargo.toml` içinde framework'ün `pty` özelliğini aç.
2. Bir program başlat: `TerminalSession::shell(&klasör)` ya da `TerminalSession::spawn(program, &argümanlar, &klasör)`; oturumu durumunda tut.
3. İzle: `let watch = session.watch();` ile `Command::perform(move || Msg::Changed(watch.next()))` döndür.
4. `TerminalEvent::Output` gelince izlemeyi yeniden başlat; görünüm kendiliğinden yeniden çizilir. `TerminalEvent::Exited(kod)` gelince dur.
5. Göster: `Terminal::new(&session)`; her düğüm gibi boyutlandırılır. Programı `session.kill()` ile ya da oturumu bırakarak durdur.

## Nasıl çalışır

- **Gerçek bir sözde terminal.** Program bir PTY içinde çalışır; kabuklar, editörler ve tam ekran araçlar her terminaldeki gibi davranır. Çıktı arka planda, 5000 satırlık geçmişi olan bir ekrana ayrıştırılır.
- **Ham ANSI değil, tema renkleri.** On altı klasik renk temadan gelir: kırmızı temanın tehlike tonu, yeşil başarı tonu, zemin yüzey rengidir. 256 renk ve 24 bit renkler istendiği gibi çizilir.
- **Tuşlar programa gider.** Odaktayken Tab ve Esc dahil her tuş gönderilir; `shift tab` odaktan çıkar, `ctrl q` yine uygulamadan çıkar. Program destekliyorsa yapıştırma, köşeli yapıştırma moduyla gönderilir.
- **Boyut bileşeni izler.** Bileşen çizerken boyutunu ister; izleyici bunu arka plan iş parçacığında uygular, çizim hiçbir zaman sürece dokunmaz.
- **Tekerlekle geçmiş.** Tekerlek önceki çıktılara geri kaydırır, sessiz bir not ne kadar geride olduğunu söyler; yazmak canlı ekrana döndürür. Tam ekran programlara bunun yerine ok tuşları gönderilir.
- **Çıktı seçilebilir.** Ekranda sürüklemek programın çıktısını seçer; `ctrl c` kopyalar (metin seçiliyken `ctrl c` programı kesmek yerine kopyalar), sağ tık Kopyala ve Ham kopyala sunar. Geri kaydırma ve çıkış notları temiz kopyaya girmez.

## Sık yapılan hatalar

- **İzlemeyi yeniden başlatmamak.** Çalışan bir izleyici yoksa ekran güncellenmez, boyut değişiklikleri bekler.
- **Göstermediğin oturumları tutmak.** Oturum programını canlı tutar; bırak ya da `kill` çağır.
- **Testlerde kabuğu çalışma motoru üzerinden başlatmak.** Testler arka plan işlerini sırayla çalıştırır ve izleyici bekler; `TerminalWatch::next`'i kendin sür.
