## Ne zaman kullanılır

Durmadan gelen ve alttan okunan çıktılar için log görünümü kullan: bir dağıtım, bir derleme, bir container'ın logu, arka planda çalışan bir iş. Bitmiş bir belge için kod görünümü, insanların karşılaştırdığı satırlar için tablo daha uygundur.

## Adım adım

1. Durumunda bir `LogBuffer::new(50_000)` tut. Dolunca en eski satırlar düşer.
2. Satırları `update` içinde ekle: `buffer.push(LogLine::new(LogLevel::Warn, metin).time(zaman))`.
3. Göster: `LogView::new(&self.buffer)`. Tamponu klonlamak ucuzdur; her karede bir maliyeti yoktur.
4. `.min_level(LogLevel::Warn)` ile süz, `.search(&self.sorgu)` ile ara.
5. Boş logun ne demek olduğunu `.empty_text(..)` ile söyle, kopyalamaları `.on_copy(|satır| ..)` ile duy.
6. Dışarıdan arka plan komutlarıyla besle: bir sonraki parçayı bekleyen `Command::perform` bir mesaj döndürür, `update` satırları ekler ve bir sonrakini ister.

## Nasıl çalışır

- **Sen başka yere bakana kadar takip eder.** En alttayken yeni satırlar görünüme kayar. Yukarı kaydırınca görünüm olduğun yerde kalır; sessiz bir not aşağıdaki satırları sayar. Tekrar aşağı in, End'e bas ya da nota tıkla; takip yeniden başlar.
- **Seviyeler renkli kelimelerdir.** Her satır seviyesini, seviyenin durum renginde bir kelime olarak taşır; anlam renk olmadan da kaybolmaz. Zaman damgaları siliktir.
- **Arama daraltır ve vurgular.** Yalnızca sorguyu içeren satırlar kalır, eşleşme parlar. Küçük harfli sorgu büyük-küçük harfe bakmaz; tam eşleşme için bir büyük harf ekle.
- **Büyük ve hızlı.** Satırlar paylaşılan parçalarda tutulur ve süzme artımlıdır: yeni satırlara bir kez bakılır, her karede bütün tampona değil. Yalnızca ekrandaki satırlar çizilir.
- **İhtiyacın olanı kopyala.** ↑/↓ satır imlecini koyar, shift genişletir, tıklama ya da sürükleme seçer; `c` seçili satırların zamanını, seviyesini ve mesajını kopyalar.
- **Kopyalamak için sağ tık.** Seçili satırlara, ya da yalnızca onu almak için herhangi bir satıra sağ tıklamak Kopyala ve Ham kopyala menüsünü açar. Kopyala, `c` gibi, satırları tek boşlukla `saat seviye mesaj` olarak yazar; Ham kopyala sütunları ekrandaki gibi hizalı tutar. Shift+F10 ve menü tuşu menüyü geçerli seçim için açar.
- **Program çıktısı terminalde göründüğü gibi.** Derleme araçları bir ilerleme satırını satır başı dönüşüyle yeniden çizer, kelimeleri kaçış dizileriyle renklendirir. Bir satır yalnızca terminalin ekranda bırakacağı şeyi tutar; başka bir programın çıktısı geldiği gibi eklenebilir.

## Sık yapılan hatalar

- **Logu `view` içinde yeniden kurmak.** Satırları `update` içinde ekle; `view` yalnızca tamponu gösterir.
- **Sınırsız vektörler.** Sonsuza kadar büyüyen bir `Vec<String>` belleği yer; sınır tamponun kapasitesidir.
- **Bir süreci `view` içinde okumak.** Çıktıyı arka plan komutunda oku ve mesaj olarak ilet.
