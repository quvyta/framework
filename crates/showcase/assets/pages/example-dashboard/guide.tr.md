## Ne zaman kullanılır

Bu sayfa bir bileşen değil, örnek bir uygulamadır: yalnızca framework parçalarıyla kurulmuş bir container sunucusunun durum ekranı. Bir izleme ya da genel bakış ekranı kurarken rozetlerin, büyük metnin, sparkline'ların, göstergelerin, çubuk grafiklerin, boş durumların ve iskeletlerin birlikte nasıl çalıştığını görmek için oku.

## Adım adım

1. **Başlık.** Başlık olarak sunucu adı, altında durum rozetleri, sağda büyük bir saat ve yenile butonu olan bir panel. Durum tonunu yalnızca sunucunun genel durumu kullanır; container sayısı nötr kalır.
2. **Eğilim ve kapasite yan yana.** İki panel `Length::Fill(1)` ile bir satırı paylaşır: güncel değeri metin olarak yanında duran üç satırlık bir sparkline ile işlemci geçmişi ve ölçerleri hizalansın diye aynı `label_width` değerini kullanan göstergelerle kaynaklar.
3. **Karşılaştırma ve sorunlar.** Servisler en fazla %100 sabitli bir çubuk grafiktir; yalnızca sınırı aşan servisler tehlike tonunu ve işaretini alır. Yanında uyarılar paneli durur.
4. **Boş hal de tasarlanmıştır.** Ters giden bir şey yoksa uyarılar paneli boş bir yüzey yerine bunu söyleyen bir boş durum gösterir.
5. **Yükleniyor hali de tasarlanmıştır.** Veri yoldayken her panel yerleşimini korur ve gelecek içeriğin şeklinde iskeletler gösterir; saat ve başlık yerinde kalır.
6. Yükleniyor, uyarılar ve normal çalışma arasında geçmek için oyun alanını kullan; yeni bir örnek almak için Yenile'ye bas.

## Nasıl çalışır

- **Tek vurgu.** Vurgu rengindeki tek değer saattir; grafikler bir kademe altını kullanır ki vurgulanan uçlar öne çıkabilsin. Durum renkleri yalnızca bir şey iyi, uyarı ya da kötü anlamına geldiğinde ve her zaman nokta, `▲` ya da `✕` ile görünür.
- **Hiçbir yerde çizgi yok.** Paneller yüzey tonu ve boşlukla ayrılır; ekranda tek bir çerçeve ya da ayırıcı karakteri yoktur.
- **Durum uygulamada yaşar.** Sayfa bir tik, bir yükleniyor bayrağı ve bir uyarılar bayrağı tutar. Her bileşen `view` içinde bu durumdan yeniden kurulur; yenilemek yalnızca tiki değiştirmektir.
- **Tekrarlanabilir örnekler.** Sayılar küçük, belirlenimci bir üreteçten gelir; testler ve ekran görüntüleri her seferinde aynı gösterge panelini gösterir.

## Sık yapılan hatalar

- **Her şey renkli.** Her gösterge, çubuk ve rozet bir ton taşıyınca tek gerçek sorun kaybolur. Çoğu şeyi nötr tut.
- **Boş paneller.** İçinde hiçbir şey olmayan bir uyarılar paneli bozuk görünür. "Uyarı yok" de.
- **Zıplayan yerleşim.** Yüklenirken üç satırlık bir grafiğin yerinde spinner olursa veri gelince bütün ekran kayar. Aynı boyutta iskeletler kullan.
