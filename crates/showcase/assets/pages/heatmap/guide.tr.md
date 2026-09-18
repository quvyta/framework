## Ne zaman kullanılır

Soru "uzun bir dönemde, gün gün ne kadar" olduğunda ısı haritası kullan: odak günleri, commit'ler, antrenmanlar, arızalar. Bir yıl yedi satır ve elli iki sütuna sığar; okuyan, serileri ve boşlukları bir bakışta görür. Birkaç günün kesin değeri için çubuk grafik, son dakikaların şekli için mini grafik kullan; ısı haritası sayıyı değil deseni yanıtlar.

Ton bir kategoriyi temsil ettiği her yerde gösterge kullan: yan yana birkaç ısı haritası ya da payları yalnızca renkle ayrılan bir grafik. Temada beş seri tonu vardır; altıncı serinin bilmece olmamasını sağlayan şey göstergedir.

## Adım adım

1. Durumunda gün başına bir değer tut, en eskisi önde; hiçbir şey olmayan gün için sıfır.
2. Çiz: `ui.add(Heatmap::new(değerler)).height(Length::Cells(7))`.
3. Aralık ilk satırda başlamıyorsa ilk değere gününü ver: `.starts_at(2)` iki hücreyi boş bırakır; boş hücre aralığın dışındaki bir gündür, boş bir gün değil.
4. Ölçeği bir hedefe sabitle: `.max(120.0)`; yoksa aralığın en yoğun günü en üst kademe olur ve sakin bir yıl, yoğun bir yıl kadar yoğun görünür.
5. Okuyanın tek bir günü okumasına izin ver: `.selected(state.seçilen)` ile birlikte `.on_select(|gün| Msg::Seç(gün))`, o günün değerini ızgaranın yanına kendin yaz. Ton sayı olarak okunmaz.
6. Birkaç kategoriden biri için `.series(n)` temanın n'inci seri tonunu alır; `Legend::new(["Rust", "Belgeler", "İnceleme"])` aynı sırayla adlarını verir; `.tones([0, 1, 2])` her adı ısı haritasına verilen tona bağlar, böylece kategorilerin yalnızca bir kısmını gösteren gösterge de doğru renkleri adlandırır.

## Nasıl çalışır

- **Dört kademe, sürekli geçiş değil.** Sıfır ve altındaki gün boş tonunu alır; üstündeki her değer tam tona doğru dört kademeden birini alır ve yalnızca en büyük gün (ya da `max`) en üste çıkar. Kademe adlandırılır ve karşılaştırılır; yumuşak bir geçiş yalnızca kesin görünür.
- **Renkten yapılır, karakterden değil.** Her hücre dolu bir hücredir; ızgara Nerd Font'ta, Unicode'da ve ASCII'de aynıdır.
- **Düşük renk derinliği.** 16 renkli terminalde aynı terminal rengine düşecek kademeler atılır ve seviyeler kalan tonlara yayılır: daha az kademe, ama iki farklı seviye asla tek tonda çizilmez ve bir şey taşıyan gün, hiçbir şey taşımayan gün gibi görünmez.
- **Dar alan en yeni haftaları tutar.** Sütunlar soldan tam olarak düşer, sıkıştırılmaz; `columns(genişlik)` kaçının kaldığını söyler, böylece "son 12 hafta" yazabilirsin. Kısa alan üstten sığan satırları tutar.
- **Fare ve klavye eşittir.** İşaretçinin altındaki hücre aydınlanır; ok tuşları bir imleci gezdirir, ← ve → birer sütun, ↑ ve ↓ birer gün, Home ve End uçlara gider. Tıklama ve Enter aynı şeyi yapar: hücreyi bildirir. `on_select` yoksa ısı haritası bir resimdir: odak da mesaj da yok.
- **Her hal çizilir.** Hiç değer yoksa hiçbir şey çizilmez ve hiç yer ölçülmez; boş hal için yer kalır. Sıfırlardan oluşan bir yıl ise boş tonunda bir ızgaradır: "hiçbir şey olmadı" der, "burada hiçbir şey yok" demez.

## Sık yapılan hatalar

- **Tek cevap olarak ton.** Seçilen günün değerini metin olarak göster; kimse bir tondan dakika okumaz.
- **Hedefte otomatik ölçek.** En iyi günü on dakika olan bir hafta en üst kademeyi doldurur. `.max()`'a hedefi ver.
- **Boşlukları sıfır saymak.** Aralığın kapsamadığı gün sakin bir gün değildir. Başlangıç için `starts_at` kullan ve aralığı verinin bittiği yerde bitir.
- **Göstergeyi atlamak.** Seri tonları beşten sonra tekrar eder; adlar olmadan iki kategori tek renk olur ve onları ayıran hiçbir şey kalmaz.
