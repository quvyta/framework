## Yöntemler

- `Handoff::new(program, |HandoffOutcome| msg)` — program ve uygulama ekranı geri aldığında gelen mesaj.
- `.arg(arg)`, `.args(args)` — argümanlar, sırayla.
- `.dir(path)` — programın çalışma dizini; verilmezse uygulamanın dizini.
- `.env(key, value)` — bir değişken; ortamın geri kalanı devralınır.
- `.notice(text)` — program başlamadan önce temizlenmiş ekrana yazılan satır.
- `.pause(true)` — program bittikten sonra bir tuş bekler; varsayılan kapalı.
- `Command::handoff(handoff)` — teslimi çalışma zamanından ister.
- `HandoffOutcome::Finished { code: Option<i32> }`, `HandoffOutcome::Failed(String)`.
- `Harness::handoffs() -> &[HandoffRequest]` — istenenler, en eskisi başta; `HandoffRequest` içinde `program`, `args`, `notice` ve `pause`.
- `Harness::set_handoff_outcome(outcome)` — o andan sonraki her teslimin yanıtı; verilmezse `Finished { code: Some(0) }`.
- `DetachedHandoff::new(program, |DetachedOutcome| msg)` — ilk satırına kadar terminalin sahibi olan bir program; teslim gibi `.arg`, `.args`, `.dir`, `.env`, `.notice` ve `.pause` alır (`pause` yalnızca program ilk satırı yazmadan bittiyse bekler).
- `.on_line(|ChildLine| msg)` — programın sonraki satırları mesaj olarak; verilmezse okunup atılır.
- `Command::handoff_detached(handoff)` — çalışma zamanından ister; öteki teslimlerle aynı sıraya girer.
- `DetachedOutcome::Detached { child: LiveChild, first_line: String }`, `DetachedOutcome::Finished { code: Option<i32> }`, `DetachedOutcome::Failed(String)`.
- `ChildLine::Line(String)`, `ChildLine::Ended { code: Option<i32> }`.
- `LiveChild` (kopyalaması ucuz, kopyalar aynı programı paylaşır): `write_line(&str) -> io::Result<()>`, `close_stdin()`, `kill() -> io::Result<()>`, `try_wait() -> io::Result<Option<Option<i32>>>`, `id() -> Option<u32>`.
- `LiveChild::for_tests() -> (LiveChild, TestChild)`; `TestChild`: `say(satır)`, `exit(kod)`, `written() -> Vec<String>`, `stdin_open() -> bool`, `killed() -> bool`.
- `Harness::detached_handoffs() -> &[HandoffRequest]`, `Harness::set_detached_outcome(outcome)` — teslimdeki gibi; verilmezse `Finished { code: Some(0) }`.

## Davranış

- Teslim çizim iş parçacığında olur ve uygulama bekler. Arka plan işleri çalışmaya devam eder; mesajları teslimden sonra uygulanır.
- Sıra: ham kipten ve alternatif ekrandan çık, imleci göster, ekranı temizle, bilgi satırını yaz, programı devralınan akışlarla çalıştır, `pause` açıksa tuş bekle, ham kip ve alternatif ekranı geri al, klavye geliştirme bayraklarını yeniden it, terminali temizle ve ekranın tamamını çiz.
- Program kendi oturumuna konmaz: `sudo` bileti kontrol eden terminal ve oturum için tutulur.
- Unix'te, uygulama kontrol eden terminalinin ön planındaysa program kendi süreç grubunda başlar ve bu grup program bitene kadar terminalin ön planı olur. Tuşların sinyalleri (`Ctrl-C`, `Ctrl-\`, `Ctrl-Z`) ve boyut değişiklikleri yalnızca programa ve onun çocuklarına ulaşır; uygulamanın sinyal ayarları hiç değişmez. Duran program hemen devam ettirilir. Ardından ön plan yeniden uygulamanın grubudur; bu arada terminali okumaya çalıştığı için durmuş bir uygulama çocuğu da devam ettirilir. Kontrol eden terminal yoksa program uygulamanın grubunda çalışır.
- Programı bir sinyal bitirdiyse `code` `None` olur. Başlatılamayan program da, geri verilemeyen ya da geri alınamayan terminal de `Failed(reason)` ile biter.
- `pause` yalnızca program gerçekten çalıştıysa bekler; hiç başlamayan program okunacak bir şey bırakmaz.
- Program öldürülse ya da çalışma zamanı panik yapsa da terminal geri alınır: terminal koruyucusunun düşmesi ve panik kancası uygulama kipinden çıkar.
- Tek bir güncellemede istenen birkaç teslim, en eskisinden başlayarak sırayla çalışır.
- `Harness`'ın terminali yoktur: isteği kaydeder ve testin koyduğu sonuçla yanıtlar; mesaj `Command::perform` işinde olduğu gibi sonraki bir güncellemede gelir.
- Arka planda süren teslim, programın standart çıktıdaki ilk satırına kadar teslim gibi çalışır; standart girdi ve çıktı uygulamaya borudur, standart hata terminaldedir. O satırda terminalin ön planı uygulamaya döner, ekran geri alınıp baştan çizilir, program kendi süreç grubunda, arka planda çalışmayı sürdürür.
- Programın çıktısını başından beri tek bir iş parçacığı okur, bu yüzden ilk satırdan sonraki hiçbir satır kaybolmaz; uygulamanın okuduğundan hızlı yazan program dolu borusunda bekler. Her satır döngüyü hemen uyandırır.
- İlk satırını yazmadan biten program `Finished { code }` ile biter; çıktısını kapatıp çalışmayı sürdüren program, girdisi kapatılarak, teslimin programı gibi beklenir.
- Son `LiveChild` kopyası bırakılınca programın standart girdisi kapanır; uygulamanın durumu çalışma bitince bırakıldığı için girdisinin sonunda biten bir yardımcı uygulamadan fazla yaşamaz.

## Tema ve ikonlar

- Teslim kendine ait bir şey çizmez. Bu sayfadaki sonuç satırı bir `Badge` ve bir `Text`'tir; onların tema anahtarları geçerlidir.

## Dil anahtarları

- `quvyta.handoff.pause` — bekleme tuş beklerken yazılan satır.
