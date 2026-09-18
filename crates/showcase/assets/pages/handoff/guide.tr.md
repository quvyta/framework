## Ne zaman kullanılır

Başka bir program kullanıcıyla kendisi konuşmak zorunda olduğunda teslim kullanılır. Ham kipte, alternatif ekranda çalışan bir uygulama başkasının istemini gösteremez: `sudo` parola sorusunu kontrol eden TTY'ye yazar, bizim ekranımız onu yutar, üstelik iki program aynı tuşları okur. Birkaç saniye kenara çekilmek dürüst çözümdür; lazygit, git ve bütün paket yöneticisi arayüzleri bunu yapar.

- **`Command::handoff`** — yetki bileti için `sudo -v`, "Proceed to review?" diye soran `paru`, `$EDITOR`, bir sayfalayıcı.
- **`Task`** — ilerleme bildiren kendi işin; o sırada ekran çizilmeye devam eder.
- **İstemi asla taklit etme.** Kendi çizdiğin parola uygulamanın belleğinden geçer; kullanıcının yerine yanıtladığın inceleme sorusu ise onun güvenlik kararını elinden alır.

## Adım adım

1. Kur: `Handoff::new("sudo", Msg::Authorized).arg("-v")`. Mesaj, uygulama ekranı geri aldıktan sonra gelir.
2. Ekranın neden bırakıldığını söyle: `.notice("Kurulum için yetki alınıyor")`. Bu satır, program başlamadan önce temizlenmiş ekrana yazılır.
3. Son satırları okunmaya değer programlar için beklemeyi aç: `.pause(true)` uygulama ekranı geri almadan önce bir tuş bekler. Varsayılan kapalıdır.
4. `update`'ten döndür: `Command::handoff(handoff)`.
5. Sonucu yanıtla: `HandoffOutcome::Finished { code }`, başarı için `Some(0)`, ret için başka bir sayı, programı bir sinyal bitirdiyse `None`; program hiç başlayamadıysa `HandoffOutcome::Failed(reason)`.
6. Testlerde `harness.handoffs()` isteği okur, `harness.set_handoff_outcome(...)` yanıtı koyar: testte hiçbir zaman parola yazılmaz.

## Nasıl çalışır

- **Teslim çizimi bilerek durdurur.** Çizim iş parçacığında olur: kullanıcı başka bir programla konuşuyordur, çizilecek bir şey yoktur. Arka plan işleri çalışmaya devam eder, mesajları dönüşte uygulanır.
- **Program terminali devralır** — standart girdi, çıktı ve hata uygulamanın kendisidir, yani tuşlar çekirdekten doğrudan programa gider. Uygulama hiçbir şeyi aktarmaz, parola belleğine hiç girmez.
- **`Ctrl-C` programa ulaşır, uygulamaya asla.** Program kendi süreç grubunda çalışır ve bu grup, program çalıştığı sürece terminalin ön planı yapılır; bir kabuğun bir işi çalıştırması gibi. `sudo` isteminde `Ctrl-C`, bir sayfalayıcıda `Ctrl-\`, bir boyut değişikliği: terminal bunları yalnızca o gruba gönderir; parola istemi iptal olur, uygulama kapanmaz. Uygulamanın kendi sinyal ayarlarında öncesinde, sırasında ya da sonrasında hiçbir şey değişmez.
- **`Ctrl-Z` askıya almaz.** Uygulama, durdurulan bir programa geri dönüş yolu sunabilecek bir kabuk değildir; duran program hemen devam ettirilir, `less` ve editörler kendilerini yeniden çizer.
- **Uygulamanın oturumunda kalır.** Süreç grubu oturum değildir: program kontrol eden terminali ve oturumu korur; `sudo` biletini bunlar için tutar, sıcak bilet sıcak kalır. Kendi oturumuna konan bir programdan parola yine istenir.
- **Dönüş eksiksizdir.** Ham kip ve alternatif ekran geri alınır, klavye geliştirme bayrakları yeniden itilir ve ekran baştan çizilir; çünkü program eski ekranın üzerine yazmıştır.
- **Nasıl biterse bitsin terminal geri alınır.** Başlayamayan program, sinyalle ölen program, çalışma zamanının paniği: koruyucunun düşmesi ve panik kancası terminali her durumda geri verir.
- **Sırada birden fazla teslim varsa** istendikleri sırayla, biri bitince öteki çalışır.

## Çalışmayı sürdüren program

Bazı programlar bir kez sorar, sonra oturumun geri kalanında uygulamaya hizmet eder. `pkexec /usr/lib/app/helper --serve` parolayı terminalde sorar ve istekleri satır satır yanıtlayan yetkili bir yardımcıya dönüşür. Teslim programın bitmesini bekler, bu yüzden ekran hiç geri gelmezdi; `Command::handoff_detached` ise program hazır olduğunu söyler söylemez ekranı geri alır.

1. Teslim gibi kur, sonradan gelen her satır için bir mesajla: `DetachedHandoff::new("pkexec", Msg::Started).args([helper, "--serve"]).notice(...).on_line(Msg::Helper)`.
2. `update`'ten döndür: `Command::handoff_detached(handoff)`.
3. Program hazır olunca standart çıktısına ilk satırını yazar. Terminalin ön planı uygulamaya döner, ekran baştan çizilir ve `DetachedOutcome::Detached { child, first_line }` gelir. `LiveChild`'ı durumunda sakla: `child.write_line(istek)` onun standart girdisine yazar.
4. Sonraki her satır, program çalıştığı sürece `ChildLine::Line(metin)` olarak gelir; sonuncusunun ardından `ChildLine::Ended { code }` gelir.
5. İlk satırını yazmadan biten program — reddedilen ya da iptal edilen paroladan sonra 126 ya da 127 döndüren `pkexec` — teslimdeki gibi `DetachedOutcome::Finished { code }` verir. `.pause(true)` yalnızca bu durumda tuş bekler, böylece sebep okunabilir.
6. Bitirmek için girdisini `child.close_stdin()` ile kapat; yardımcı girdisinin sonunu okur ve biter. Çalışma sonunda uygulamanın durumu bırakılınca girdi kendiliğinden kapanır. `child.kill()` de var, ama root olarak çalışan bir programa sen sinyal gönderemezsin.
7. Testlerde `LiveChild::for_tests()` yerine geçen bir çocuk ve programı oynayan bir `TestChild` verir: `harness.set_detached_outcome(DetachedOutcome::Detached { child, first_line })` teslimi yanıtlar, `program.written()` uygulamanın yazdıklarını gösterir, `program.say(satır)` ve `program.exit(kod)` bir sonraki adımda uygulamaya ulaşır. `harness.detached_handoffs()` istenenleri gösterir.

İlk satıra kadar yukarıdaki her şey geçerlidir: `Ctrl-C` programa gider, `Ctrl-Z` askıya almaz, oturum korunur. Yalnızca standart hata terminalde kalır, çünkü `pkexec` ve `sudo` standart girdiden değil, kontrol eden terminalin kendisinden sorar. Sonrasında program terminalin arka planında çalışır; o andan sonra standart hataya bir şey yazmamalıdır, çünkü yazdığı her şey uygulamanın ekranına düşer.

## Sık yapılan hatalar

- **`sudo`'yu boruyla çalıştırmak.** O zaman istemin gidecek yeri olmaz. Ya teslim ya sahte terminal; arası yok.
- **Çocuğu kendi oturumuna koymak.** `setsid` sıcak `sudo` biletini kaybettirir; qpackages'ın istek dosyası bunu ölçtü.
- **Teslimden sonra her şeyi yeniden çizmemek.** Programın bıraktığı ekran senin değildir; çalışma zamanı onu temizler, yarım bir kareyi asla saklama.
- **Hiçbir şey yazmayan program için beklemek.** `sudo -v` bir saniye sürer, okunacak bir şey bırakmaz; orada tuş beklemek yalnızca yolu tıkar.
