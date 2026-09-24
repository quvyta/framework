## Quvyta ve quvyta-framework

quvyta-framework, Rust ile terminal uygulamaları yazmak için bir framework'tür. Quvyta'nın kendi uygulamaları için başlatıldı ve açık kaynaktır. Çalışma motorunu, bileşenleri ve tema, ikon ve dil dosyalarını sağlar; kodda kütüphanenin adı `qframe`'dir.

## Uygulama nedir

quvyta-framework ile yazılmış bir uygulama üç şeydir: **veri**, veriyi **çizen** bir fonksiyon ve veriyi **değiştiren** bir fonksiyon. Çalışma motoru bunları birbirine bağlar: ekranı çizer, tuşları ve tıklamaları mesaja çevirir, her mesajı güncelleme fonksiyonuna verir ve yeniden çizer.

Elle çizim yapmazsın, fareyi takip etmezsin, ne zaman yeniden çizileceğine karar vermezsin. Ekranı tarif eder, mesajlara tepki verirsin.

## Adım adım

1. Ekranın bağlı olduğu her şeyi tutan bir struct yaz. Burada sayacı ve altındaki klasör ekranını taşıyan `State`.
2. Olabilecek her şeyi bir enum'da topla: `Msg`. Her buton, alan ve liste bunlardan birini gönderir.
3. `update` fonksiyonunu yaz: mesajı eşle, durumu değiştir, motorun senin için bir şey yapması gerekiyorsa bir `Command` döndür.
4. `view` fonksiyonunu yaz: bileşenleri `ui.add` ile ekle, `ui.row` ve `ui.column` ile grupla, her bileşene göndereceği mesajı ver.
5. `Runtime::new(app).run()` ile başlat.

## Nasıl çalışır

- **View her değişiklikten sonra çalışır.** Bu ucuzdur: bileşenler sade değerlerdir ve terminale yalnızca değişen hücreler gönderilir.
- **Hiçbir şey değişmezse çizim yapılmaz.** Girdi ya da animasyon yokken döngü bekler ve hiçbir şey çizmez; boştaki uygulama işlemciyi neredeyse hiç kullanmaz.
- **Senin için anlamı olmayan durum motorda yaşar.** Hover, odak, imleç konumu ve kaydırma her bileşen için hatırlanır; senin durumun yalnızca uygulamanın anlamını tutar.
- **Yavaş iş bir komuttur.** `Command::perform` bir kapanışı arka plan iş parçacığında çalıştırır ve bittiğinde mesajını teslim eder. Yukarıdaki sayma butonu diski bu şekilde okur; iş sürerken spinner dönmeye devam eder.

## Kendi mesajları olan ekranlar

Birden çok ekranı olan bir uygulama her ekrana kendi `Msg`, `update` ve `view` fonksiyonlarını verir; ekran tek başınaymış gibi yazılır. Yukarıdaki klasör paneli böyle bir ekrandır: `folder::Msg::Count` gönderir, `update` fonksiyonu da bir `Command<folder::Msg>` döndürür.

Uygulama onu bir varyant ve iki dönüşümle bağlar:

- **Görünüm:** `ui.map(|m| send(Msg::Folder(m)), |ui| state.folder.view(ui))` ekranın mesajlarıyla kurulan bir sütun ekler. İçindeki her şey dönüştürülmüş gelir: butonlar ve alanlar, iç içe satırlar ve paneller, ekranın açtığı pencereler, odak istekleri.
- **Komutlar:** `state.folder.update(m).map(|m| send(Msg::Folder(m)))` ekranın motordan istediklerini dönüştürür. Arka plan işinin sonradan gönderdiği mesaj da, buradaki `Counted` gibi, aynı dönüşümden geçer.

Dönüşüm `Msg::Folder` gibi bir varyant ya da ihtiyacını yakalayan bir kapanış olabilir; örneğin ekranın durduğu sekme. Komutun dönüşümü arka plan iş parçacıklarında çalışır, bu yüzden `Send` ve `Sync` olmalıdır.

## Yaşam döngüsü: açılış, ölçü ve çıkış

`App`'in dört isteğe bağlı metodu uygulamaya ömrü boyunca eşlik eder. Her birinin bir varsayılanı vardır; uygulama yalnızca gerekenleri yazar. Yukarıdaki Yaşam döngüsü paneli dördünü de gösterir; showcase'in kendisi onları uygular.

- **`init(&mut self) -> Command<Msg>`** bir kez, ilk karenin başında, görünümü kurulmadan önce çalışır. İlk işi döndür: ilk tuş listeye ulaşsın diye `Command::focus("menu")`, başlayacak bir tik, açılacak bir diyalog. Odak o kare çizilir çizilmez, hiçbir girdi okunmadan yerine oturur ve kare, bileşen odaklı halde yeniden çizilir. Showcase'in menüsü klavyeyi bu yolla alır: ilk ↓ onun içinde ilerler.
- **`resized(&self, ölçü: Size) -> Option<Msg>`** terminalin ölçüsünü uygulama açılırken (`init`'ten hemen önce) ve her yeniden boyutlanmada duyar. Mesajı `update`'ten geçer; ölçüye ihtiyaç duyan iş, örneğin `Process::pty(sütun, satır)`, orada başlar. `view` içinde `ui.size()`'ın bildirdiği ölçünün aynısıdır ve o kare kurulmadan önce uygulanır, bu yüzden ikisi hiç ayrışmaz.
- **`before_quit(&self) -> Option<Msg>`** çalışma motoru kullanıcı adına çıkmak üzereyken her seferinde sorulur: `ctrl q` bağı ve komut paletinden çalıştırılan çıkış eylemi. `None` çıkar. Bir mesaj uygulamayı açık tutar ve onun yerine teslim edilir; örneğin "bitir ve çık, çalışır bırak ya da vazgeç" diye sormak için. Uygulama kararını verince `Command::quit()` döndürür; bu kendi kararıdır ve yeniden sorulmaz. Yukarıdaki "Çıkmadan önce sor"u aç ve `ctrl q`'ya bas.
- **`terminating(&self, sebep: Termination) -> Option<Msg>`** uygulamayı kullanıcının değil sistemin kapattığını duyar. Kaydetmek için tek şanstır. `None` hemen çıkar; bir mesaj uygulamayı açık tutar, `update` kaydeder ve `Command::quit()` döndürür. Varsayılan, `Termination::Terminate`'e `before_quit` ile cevap verir ve `Termination::Hangup`'ta hemen çıkar; iki kancayı da yazmayan uygulama yine temiz çıkar.

Yalnızca bir şey bildiren kancalar (`resized`, `before_quit`, `terminating`, `action`, `clipboard`) durumu okur ve bir mesajla cevap verir; iş başlatan kanca (`init`) `update` gibi bir komut döndürür. Test düzeneği hepsini terminalin çalıştırdığı yerde çalıştırır: `Harness::new(app, 120, 30)` 120 × 30'u bildirir ve `init`'i çalıştırır, `resize(60, 20)` 60 × 20'yi bildirir, `press("ctrl+q")` `before_quit`'e sorar, `terminate(Termination::Hangup)` `terminating`'e haber verir.

## Uygulamayı sistem kapatınca

Unix'te çalışma motoru, çalıştığı sürece üç sinyali yakalar ve her biri bir `Termination` olur:

- **`SIGTERM`** (`kill`, bir servis yöneticisi, kapanan makine) ve dışarıdan gönderilen **`SIGINT`** `Terminate` olur. Terminal hâlâ yerindedir; uygulama kaydedip çıkabilir, hatta sorabilir. Uygulamanın içinde `ctrl c` bu sinyal değil, bir tuştur.
- **`SIGHUP`** `Hangup` olur: SSH bağlantısı koptu, pencere ya da `tmux` bölmesi kapandı. Artık kimse bir soruya cevap veremez ve bundan sonra hiçbir şey çizilmez: sormadan kaydet. Kabuğun ilettiği kopuş ile kabuk kapanınca sistemin gönderdiği aynı kopuştur, bir kez bildirilir.

Her çıkış sınırlı sürede biter. `Termination::grace` dolunca (terminate'ten sonra beş, kopuştan sonra üç saniye) motor uygulamayı beklemeden çıkar. İkinci bir `SIGTERM` ya da `SIGINT` hemen çıkarır. Döngünün kendisi takılmışsa, hiç dönmeyen bir `update` içinde, süreç bir saniye sonra yine de sinyalin kendisiyle sona erer. Terminal var olduğu sürece her durumda eski haline döner: ham kip kapanır, normal ekran ve imleç geri gelir. Kopuştan sonra ona hiçbir şey yazılmaz.

Döngü sinyali geldiği anda duyar; bir tuşu ya da bir süreyi beklerken bile. `Command::handoff` sırasında terminal programındır; sinyal ona iletilir, program kabuğun bir işi olsaydı nasıl biterse öyle biter, sonra uygulama terminali geri alır ve sinyali kendisi duyar.

Showcase'i başlat, "Çıkmadan önce sor"u aç ve başka bir terminalden `kill <pid>` gönder: soru `App::terminating`'ten gelir ve olay günlüğü bunu gösterir.

```rust
fn terminating(&self, _sebep: Termination) -> Option<Msg> {
    // Çalışan sayaç, onu ister bir kişi ister sistem kapatsın, önce kaydedilir.
    self.running.then_some(Msg::SaveAndQuit)
}
```

## Kareler ne sıklıkta çizilir

Kareler saate göre çizilmez: çalışma motoru bir şey değiştiğinde çizer, kalan zamanda klavyeyi bekler. `App::frame_limit` saniyede en fazla kaç kare çizileceğini söyler, çünkü "bir şey değişti" kimsenin okuyamayacağı hızda olabilir — satır satır akan gömülü bir terminal, üst üste haber veren arka plan işi.

Varsayılan, makinenin başında saniyede 60, uzak bağlantıda 20 kare çizer. İkisini `Env::remote` ayırır: `SSH_CONNECTION` ya da `SSH_TTY` doluysa doğrudur, böylece SSH sunucusunun başlattığı bir oturum uygulama hiçbir şey sormadan tanınır. Ağ üzerinden her kare, bağlantıdan geçen bütün bir ekrandır; durmadan yazan bir program, kimsenin ayırt edemediği karelerle bağlantıyı harcar.

```rust
fn frame_limit(&self) -> FrameLimit {
    // Makinede 60, yavaş bağlantıda 10.
    FrameLimit::per_second(60).remote(10)
}
```

Sınır, girdiye verilen cevabı hiç geciktirmez. Bir tuşun, tıklamanın, imleç hareketinin ya da yapıştırmanın ardından gelen kare, sınır ne kadar düşük olursa olsun hemen çizilir: insan bütün programı yazdığı karakterin yankısından yargılar. Yalnızca uygulamanın kendi işinden doğan kareler birleştirilir ve bekletilen kare aradaki süre dolar dolmaz çizilir, yani sınırın kendisi gecikme eklemez. `FrameLimit::none()` istenen her kareyi çizer; `FrameLimit::per_second(n)` her bağlantıda tek sayıyı kullanır.

Uygulama `Env::remote`'u kendi kararları için de okur: yavaş bağlantıda daha az animasyon, daha küçük resim, daha sade bir ilk ekran.

## Terminalin gösterebildiği resimler

`Env::graphics` bu terminalde bir resmin nasıl çizilebileceğini söyler: gerçek pikseller için `Graphics::Kitty` ve `Graphics::Sixel`, her hücrede iki renkli piksel için `Graphics::HalfBlock` (256 renkli her terminal bunu her bağlantıda gösterir) ve hiç resim çizilmemesi gereken yerde `Graphics::None`.

Runtime terminali devraldığı anda ona bir kez sorar: yalnızca soran, hiçbir şey saklamayan bir kitty grafik sorgusu ve ardından terminalin aygıt özelliklerini isteyen bir soru. Her terminal ikincisine sırasıyla cevap verir, bu yüzden onun cevabı sonu gösterir: cevap veren bir terminal birkaç milisaniyede biter, vermeyene en çok 150 ms tanınır. Cevaplar girdi ayrıştırıcısı başlamadan doğrudan terminalden okunur, böylece hiçbiri tuş olarak gelmez; bundan geç gelen bir cevap da girdiden ayıklanır.

Kitty'den `OK` gelirse kitty, `4`'ü sayan aygıt özellikleri sixel, geri kalan her şey (sessizlik dahil) yarım blok demektir. Sonra ortam söz alır. 16 renk ya da ASCII glifler `Graphics::None` verir. tmux ya da GNU screen içinde (`TMUX` ya da `STY` dolu) kitty ve sixel yarım bloğa döner, çünkü çoklayıcı onları geçirmez; orada terminale sorulmaz bile. `kitty`, `sixel`, `halfblock` ya da `none` değerini alan `QUVYTA_GRAPHICS` ortam değişkeni, sorunun yanlış tanıdığı bir terminal için hepsinin üstünde karar verir; başka bir değer yok sayılır ve `Env::diagnostics`'e yazılır.

```rust
match ui.env().graphics() {
    Graphics::None => { /* resmin ne olduğunu söyle */ }
    _ => { /* çiz */ }
}
```

Testler hiçbir terminale sormaz: `Env::builtin` yarım blok verir, `Harness::set_graphics` aynı kurallarla başka bir terminal gibi cevap verir.

## Terminal olmadan test

`Harness` aynı uygulamayı bellekteki bir ekranda, sahte bir saatle çalıştırır. Tuşa bas, yaz, metnin üstüne tıkla ve ekranı düz satırlar olarak geri oku:

```rust
let mut app = Harness::new(Counter::default(), 40, 6);
app.press("tab").press("enter");
assert!(app.screen().contains("Count  1"));
```

`Harness` içinde `Command::perform` hemen çalışır, işler sahte saati izler; testler her seferinde aynı sonucu verir.

## Sık yapılan hatalar

- **`view` içinde I/O yapmak.** Bir şey animasyon yaparken view saniyede defalarca çalışır. Dosya okumayı ve ağ çağrılarını `Command::perform` ile yap.
- **Hover ya da odağı kendi durumunda tutmak.** Motor bunu zaten biliyor; bileşenlerin kendilerini buna göre çizmesine izin ver.
- **Yeri değişen bileşenlere `.id(...)` vermemek.** Bileşenler konumlarıyla tanınır; beliren, kaybolan ya da sırası değişen bileşenlere isim ver ki durumları onları takip etsin.
