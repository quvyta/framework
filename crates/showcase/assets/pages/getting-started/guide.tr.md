## Quvyta ve quvyta-framework

quvyta-framework, Rust ile terminal uygulamaları yazmak için bir framework'tür. Quvyta'nın kendi uygulamaları için başlatıldı ve açık kaynaktır. Çalışma motorunu, bileşenleri ve tema, ikon ve dil dosyalarını sağlar; kodda kütüphanenin adı `qframe`'dir.

## Uygulama nedir

quvyta-framework ile yazılmış bir uygulama üç şeydir: **veri**, veriyi **çizen** bir fonksiyon ve veriyi **değiştiren** bir fonksiyon. Çalışma motoru bunları birbirine bağlar: ekranı çizer, tuşları ve tıklamaları mesaja çevirir, her mesajı güncelleme fonksiyonuna verir ve yeniden çizer.

Elle çizim yapmazsın, fareyi takip etmezsin, ne zaman yeniden çizileceğine karar vermezsin. Ekranı tarif eder, mesajlara tepki verirsin.

## Adım adım

1. Ekranın bağlı olduğu her şeyi tutan bir struct yaz. Burada sayaç, isteğe bağlı dosya sayısı ve işin sürdüğünü söyleyen bir bayrak taşıyan `State`.
2. Olabilecek her şeyi bir enum'da topla: `Msg`. Her buton, alan ve liste bunlardan birini gönderir.
3. `update` fonksiyonunu yaz: mesajı eşle, durumu değiştir, motorun senin için bir şey yapması gerekiyorsa bir `Command` döndür.
4. `view` fonksiyonunu yaz: bileşenleri `ui.add` ile ekle, `ui.row` ve `ui.column` ile grupla, her bileşene göndereceği mesajı ver.
5. `Runtime::new(app).run()` ile başlat.

## Nasıl çalışır

- **View her değişiklikten sonra çalışır.** Bu ucuzdur: bileşenler sade değerlerdir ve terminale yalnızca değişen hücreler gönderilir.
- **Hiçbir şey değişmezse çizim yapılmaz.** Girdi ya da animasyon yokken döngü bekler ve hiçbir şey çizmez; boştaki uygulama işlemciyi neredeyse hiç kullanmaz.
- **Senin için anlamı olmayan durum motorda yaşar.** Hover, odak, imleç konumu ve kaydırma her bileşen için hatırlanır; senin durumun yalnızca uygulamanın anlamını tutar.
- **Yavaş iş bir komuttur.** `Command::perform` bir kapanışı arka plan iş parçacığında çalıştırır ve bittiğinde mesajını teslim eder. Yukarıdaki sayma butonu diski bu şekilde okur; iş sürerken spinner dönmeye devam eder.

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
