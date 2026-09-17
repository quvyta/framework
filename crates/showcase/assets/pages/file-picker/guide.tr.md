## Ne zaman kullanılır

Kullanıcının bir dosyayı ya da klasörü göstermesi gerektiğinde seçiciyi kullan: açılacak bir proje, içe aktarılacak bir ayar dosyası, dışa aktarılacak bir klasör. Seçim bilinen birkaç öğe arasındaysa bir açılır liste ya da liste daha naziktir.

## Adım adım

1. Durumunda bir `FileBrowser::new(başlangıç, PickMode::Files)` tut; yalnızca bazı dosyaları göstermek için `.extensions(["toml"])` ekle.
2. Okumayı `self.browser.open(başlangıç, Msg::Picker)` ile başlat ve verdiği komutu döndür.
3. Göster: `FilePicker::new(&self.browser, Msg::Picker).show(ui)`; her düğüm gibi boyutlandırılır.
4. `update` içinde `FilePickerMsg::Chosen(yol)` mesajını kendin yakala, diğer bütün seçici mesajlarını `self.browser.update(mesaj, Msg::Picker)` ile ilet ve komutunu döndür.
5. Klasör seçmek için `PickMode::Folders` kullan: dosyalar silikleşir, düğme seçili klasörü ya da açık olanı seçer.

## Nasıl çalışır

- **Okuma çizimi asla bekletmez.** Klasörler arka plan komutunda okunur. Kullanıcının artık beklemediği bir klasörün cevabı yok sayılır.
- **Klasör okunurken hiçbir şey yanıp sönmez.** Ekrandaki klasör olduğu gibi, kullanılabilir halde kalır; yenisi okununca tek karede onun yerine geçer. Okumaların çoğu birkaç milisaniye sürer; bunlar için "yükleniyor" göstermek ekranı yalnızca kırpıştırır.
- **Gösterge önce bekler, sonra kalır.** Yalnızca 300 ms'den uzun süren bir okuma yolun hemen ardında küçük bir spinner gösterir; göründükten sonra en az 500 ms kalır, böylece o da yanıp sönmez. Bu süreler yaygın arayüz önerilerine dayanır: yaklaşık 300 ms'nin altındaki beklemeyi insanlar fark etmez, yarım saniyeden kısa görünen bir şey de hata gibi okunur. İki yolu da görmek için oyun alanında "Yavaş disk"i aç.
- **Hatalar beklemez.** Okunamayan bir klasör cevabı gelir gelmez mesajını gösterir, spinner da hemen kaybolur.
- **Hatalar çökme değil, durumdur.** Okuma iznin olmayan, kaybolmuş ya da klasör olmayan bir yol; tehlike işareti, bir cümle ve yolun kendisiyle gösterilir.
- **Yol, geri dönüş yoludur.** Açık klasörün üstündeki her parça tıklanabilir ve hover'da yükselir; uzun yollar baştaki parçalarını `…` ile kısaltır. Listenin ilk satırı da üst klasöre götürür, klavye de yukarı çıkabilir.
- **Önce klasörler.** Girdiler önce klasörler, sonra büyük-küçük harfe bakmadan ada göre sıralanır. Gizli girdiler, anahtar açılana kadar gizli kalır.
- **Süzme mümkünse seçimi korur.** Süzgece yazmak listeyi daraltır; seçili girdi kaybolursa ilk eşleşme seçilir.

## Sık yapılan hatalar

- **`read_folder`'ı `view` içinde çağırmak.** Diske dokunur; yalnızca bir komutun içinde kullan.
- **`update`'ten komutu döndürmeyi unutmak.** Klasör o zaman hiç değişmez (300 ms sonra da yol satırı dönmeye başlar).
- **Yüklenirken kendi görünümünü temizlemek.** Benzer bir ekranı kendin kuruyorsan, son cevabı yükleniyor durumuyla değiştirme; yenisi gelene kadar ekranda tut.
- **`Chosen`'ı sıradan bir güncelleme sanmak.** O sonuçtur; diyaloğu orada kapat ya da dosyayı orada aç.
