## Nerede kullanılır

Bir klasör uygulamanın konusunun parçasıysa dosya yöneticisi kullanılır: düzenleyicinin yanındaki proje dosyaları, masaüstündeki Dosyalar penceresi, programdan çıkmadan dosya oluşturup ad değiştirip taşıyıp silmeye yarayan bir bölme. Tek bir yol seçip çıkmak için dosya ve klasör seçici vardır: seçici bir soruyu cevaplar, yönetici çalışılan bir yerdir.

- **Ekrandaki her yönetici için bir durum.** `FileManagerState` her ekran durumu gibi senin durumunun içinde yaşar ve her `FileManagerMsg`'i ona verirsin.
- **Açmak senin işin.** Yönetici yalnızca "şu yol açılmak istendi" der. Sekme, pencere, önizleme ya da bir diyaloğun cevabı olması senin kararın.
- **Klasörün ne olduğunu söyle.** Çıkılmaması gereken bir klasör için `confined()`; onsuz yönetici kendisine verilen kökü gösterir, bütün dosya sistemi de olabilir.
- **Biçimi seç.** `view(FileView::Tree | List | Icons)`: klasör içinde klasör; tek klasör, boyut, tarih ve izinleriyle satır satır; ya da tek klasör, simge ızgarası olarak. Hiçbir şey söylemezsen ağaç gelir.

## Adım adım

1. Durumu tut: `FileManagerState::new(kök)`, işlemler kökten çıkmayacaksa `.confined()` ile.
2. Yönetici ekrana gelince kökü oku: `self.manager.load(Msg::Files)`, `init`'ten ya da ekrana girildiğinde. İlk seferinde kökü, sonrasında açık her klasörü yeniden okur.
3. Her mesajı ona ver: `Msg::Files(m) => self.manager.update(m, Msg::Files)`. Okumalar ve dosya işlemleri arka planda çalışır; çizim onları beklemez.
4. Çiz: `FileManager::new(&self.manager, Msg::Files).on_open(|yol| Msg::Open(yol.to_path_buf())).show(ui).fill()`.
5. Yalnızca senin bildiğini ekle: "burada terminal aç" için `on_open_terminal`, satırın menüsündeki kendi öğelerin için `menu_items(|key, targets| …)`, bir girdinin senin için ne anlama geldiği için `row_mark(|key| …)`.
6. Silinen girdinin nereye gideceğini söyle: kişinin kendi çöp kutusu için `.trashing()`, kalıcı silme için hiçbir şey. Nokta dosyalarını gösteren bir yönetici için `.showing_hidden(true)`.
7. Başka programları takip etmek için durumda `.following(true)`, ya da yönetici ekrandayken `set_following(true)`, ekrandan çıkınca `false`.

## Nasıl çalışır

- **Okuma hiçbir zaman çizim sırasında olmaz.** Bir klasör bir kez, tek cevapta okunur ve görünüm elde olandan kurulur. On bin girdi tek parça gelir ve bir kez çizilir; parça parça akıtılmaz, çünkü her yeniden çizim uzak bağlantıda yeni bir ekran demektir.
- **Düz görünüm tek klasör gösterir.** Liste ve simgeler `state.folder()`'ın söylediği klasörü gösterir; en üstte o klasörün kendi satırı durur: menüsü oradadır ve o satır klasörden çıkmanın yoludur, bir klasör satırı ise içine girer. Tuşlar, menüler ve bütün işlemler üç biçimde de aynıdır.
- **Liste bir sayfa okur, klasörü değil.** Boyut, tarih ve izin, o tek girdi için sisteme bir çağrı daha demektir; bu yüzden liste imlecin çevresindeki iki yüz girdilik bir sayfayı ister ve geleni saklar. Ağaç ile simgeler hiçbir şey istemez. Hangi satırları çizdiğini tam olarak bilen bir uygulama onları `state.detail(anahtarlar, sar)` ile ister.
- **Yalnızca görünen satırlar boyanır.** On bin girdilik klasör iki yüz girdilik klasör kadar tutar: ekrandaki satırlar, o kadar. Ağaç satırı boyut, tarih ya da izin diye hiçbir şey okumaz.
- **Yükleniyor göstergesinin kuralı var.** 300 ms'den kısa okuma hiçbir şey göstermez; daha yavaşı klasörün kendi satırına küçük bir spinner koyar ve o yaklaşık 500 ms kalır. Hızlı okuma ekranda yanıp sönmez, yavaş okuma da titremez.
- **Yol değil, anahtar.** Her girdinin bir anahtarı vardır: köke göre yolu, her platformda `/` ile yazılmış. İşlemler anahtar alır ve yolu kendileri kurar; bu yüzden kökten çıkan bir anahtar (`..`, boş parça, NUL) nereden gelirse gelsin reddedilir — oturum dosyası dahil.
- **Kısıtlı yönetici bağı reddeder.** Yolun üzerindeki sembolik bağ olan bir klasör reddedilir, çünkü bağ her yeri gösterebilir. Bağın kendisi yine bir girdidir: adını değiştirmek, taşımak ya da silmek bağa işler, gösterdiği şeye dokunmaz.
- **İşaret, yöneticinin bilemediğini söyleme yoludur.** `row_mark(|key| RowMark::new().sign("warning", "warning").faint(true))` bir satıra tonlu bir işaret ve solgun bir ad verir: yedeğe girmeyen bir girdi, sürüm denetiminin yok saydığı bir dosya, kaydedilmemiş bir şey. İşaret ile ton bilerek birlikte gelir, böylece renkler kapalıyken de satır ayırt edilir. İşaret satırı yöneticinin kendi hâllerinden daha gür yapamaz: kesilmiş girdi ve pasif yönetici solgun kalır.
- **Çöpe atmak bir yeniden adlandırmadır, bu yüzden bir sınırı var.** Girdi çöpün içine yeniden adlandırılarak gider ve bu yalnızca çöpün bulunduğu dosya sisteminde çalışır. Çalışmadığında yönetici girdiyi sessizce silmez ve olmuş gibi de yapmaz: adı ve sebebiyle, tehlike renginde, kalıcı silmeyi sorar. Çöp kutulu bir yöneticinin kalıcı sildiği tek yer o sorudur.
- **Kopyalama beklemek değil, bir iştir.** Framework'ün iş düzeneğinde çalışır: yönetici ne yaptığını, nereye geldiğini ve bir Durdur düğmesini bir satırda gösterir, satırlar altında okunur kalır. Durdurmak yarı yazılmış girdiyi geri alır — yarım dosya, hiç dosyadan kötüdür — ve kopyalanmış olan kalır. Aynı işi kendi `Tasks` listene koymak için `work()`'ü oku.
- **Kopyalamak taşımak değildir.** `Copy` girdileri `Cut` gibi kenara koyar, `Paste` onları kopyalar; kopyalanan yerinde kalır, bu yüzden onda solgun bir şey olmaz ve bağ izlenmek yerine bağ olarak kopyalanır.
- **Gizli girdiler okunur, sonradan getirilmez.** Her okumayla gelirler; onları göstermek satırlarla ilgili bir karardır, diske ikinci bir bakış değil.
- **Reddedilen söylenir.** Tek girdi sebebini söyler; birkaç girdi her biri için bir satır verir ve yapılabilen yapılır. Yapıştırılamayan kesilmiş girdi kesik kalır, başka yerde denenmek üzere.
- **Dışarıdan gelen değişiklikler birleştirilir.** `following(true)` ile yönetici tam olarak ekrandaki klasörleri izler ve değişiklik hangisinde olduysa onu yeniden okur. Yalnızca içerik değişmişse hiçbir şey okunmaz: satırlar addır. Taşan bir izleme ya da kaybolan bir klasör ekrandaki her şeyi yeniden okutur.

## Tuzaklar

- **Zamanlayıcıyla yeniden okuma.** Kendi işlemin bitince, yönetici ekrana dönünce ve bir izleme değişiklik dediğinde yenile.
- **Dosyanın ne olduğuna karar verme.** Uzantılar, görüntüleyiciler ve programlar uygulamanın işidir; yönetici yalnızca yolu verir.
- **Girdi çöpe gitmediği hâlde gitti deme.** `Trash`'ı ver ve gerisini yöneticinin sorusuna bırak; yeniden adlandırmanın olup olmadığını o bilir.
- **Kaybolmuş olabilecek bir anahtarı saklama.** Bir değişiklikten sonra yönetici klasörde artık olmayanı unutur, senin sakladığın anahtar hiçbir şeyi adlandırmıyor olabilir. Duruma sor.
- **Takip etmek bir bekleyen iş parçacığı demektir.** Kimsenin bakmadığı yöneticide kapat ve bunun bir Linux izlemesi olduğunu unutma: başka yerde satırları dürüst tutan şey senin kendi yeniden okumalarındır.
