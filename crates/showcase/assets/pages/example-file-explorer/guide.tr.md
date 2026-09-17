## Ne zaman kullanılır

İç içe bir şeyi gezen ve seçilenin ayrıntısını gösteren bir araç yaparken bu örneği oku: dosya yöneticisi, container tarayıcısı, veritabanı gezgini. Yalnızca framework bileşenlerinden oluşmuş, küçük ama bütün bir uygulamadır.

## Adım adım

1. Okuduğun veriyi durumda tut: klasörden listesine giden bir harita (yükleniyor, hazır ya da hatalı), açık klasörler, bulunduğun klasör, sıralı girdileri ve önizleme.
2. Solda klasörlerden bir `Tree` çiz. Klasörler okunmadan önce `expandable` olur; birini açmak onu okuyan bir komut döndürür ve bu sırada onu `.loading(true)` ile işaretler; ağaç spinner'ı yalnızca okuma yavaşsa gösterir.
3. Ortada bulunduğun klasörün `Table`'ını çiz; ad sıralanabilir, boyut sağa hizalı. Sıralama mesajı gelince girdileri `update` içinde sırala.
4. Bir satır seçilince dosyanın başını arka plan komutunda oku ve bir `ScrollView` içinde `CodeView` ya da `Markdown` ile göster.
5. Yolu üste, durum satırını alta `Text` parçaları olarak koy: ikincil yerler silik, bulunduğun yer kalın.

## Nasıl çalışır

- **Hiçbir şey bekletmez.** Örnek hazır açılsın diye başlangıçta bir kez okunan kök klasör dışında her klasör ve dosya `Command::perform` ile okunur; görünüm yalnızca durumda olanı çizer, büyük bir klasör okunurken de arayüz cevap vermeye devam eder.
- **Üç bölme, sıfır çizgi.** Ağaç, tablo ve önizleme iki hücrelik boşluk ve kendi satır yüzeyleriyle ayrılır; aralarına hiçbir şey çizilmez.
- **Gösterilen, cevap gelene kadar kalır.** Bir klasöre gitmek ya da bir dosya seçmek tabloyu veya önizlemeyi temizlemez; yeni içerik tek karede eskisinin yerine geçer. 300 ms'den uzun süren bir klasör okuması ağaçtaki satırında en az 500 ms döner, böylece hızlı okumalar ekranı kırpıştırmaz.
- **Her durumun bir yüzü var.** Yavaş okunan klasör döner, okunamayan klasör bunu tehlike renginde söyler, boş klasörün bir cümlesi vardır, ikili dosya basılmak yerine öyle olduğu söylenir.
- **Tablo klasör açar.** Bir klasör satırını açmak oraya gider ve ağaçta üst klasörlerini de açar; iki bölme hep aynı şeyi söyler.

## Sık yapılan hatalar

- **Dosyaları `view` içinde okumak.** Küçük bir `read_to_string` bile `view` içinde her karede çalışır.
- **Bütün dosyayı önizlemek.** Sınırlı bir baş kısmı oku; loglar ve derlemeler gigabaytlarca olabilir.
- **Geç gelen cevapları karıştırmak.** Bir önizleme cevabını göstermeden önce hâlâ seçili dosyaya ait olduğunu kontrol et.
