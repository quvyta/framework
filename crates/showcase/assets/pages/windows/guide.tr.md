## Ne zaman kullanılır

Bir yüzeyin nerede duracağına ve ne kadar büyük olacağına kullanıcı karar veriyorsa pencere kullan: programlardan bir masaüstü, kenara çekilen bir araç kutusu, karşılaştırmak için yan yana tutulan iki belge. Yerleşim uygulamanın kararıysa satır, sütun ya da bölücü doğru olandır; pencere kullanıcıya yerleştirme işi yükler.

## Adım adım

1. Pencereleri kendi durumunda, üst üste binme sırasıyla, en alttaki önce olacak şekilde tut: ad, dikdörtgen ve gövdenin göstereceği şey.
2. Masaüstüne bir stack ver ve her pencereyi yerleştir: `ui.place(win.rect, |ui| ..).id(win.name)`. Dikdörtgen stack'in sol üst köşesinden sayılır; her yerleştirilmiş pencereye ad ver, böylece sıra değişince durumu ve süren sürüklemesi onunla gelir.
3. Pencereyi içinde kur: `ui.add_with(Window::new(ad).subtitle(..).icon(..).focused(..), |ui| ..)` ve gövdenin bileşenlerini kapanışın içine koy.
4. Hareket edebilir yap: `.on_event(move |olay| Msg::Window(ad, olay))`. Bu olmadan pencere yalnızca bir yüzeydir: işaret de, tutamak da yoktur.
5. Olayları `update` içinde uygula: farkları dikdörtgene ekle, `Focus` gelince pencereyi öne al, `Close` gelince kaldır.
6. Sınırları uygularken kendin koy: en küçük boyut, masaüstünde kalmak, büyütmenin ne demek olduğu. Pencere fareyi bildirir; kurallar senin.
7. Bırakmaya karşılık ver: taşıma ya da boyutlandırmadan sonra tuş kalkınca `WindowEvent::Dropped` gelir. Kenara yapıştırma, hayaletin yerine oturması ve boyutu kaydetmek oraya aittir, her farka değil.

## Nasıl çalışır

- **Kutu değil, yüzey.** Bir satır başlık, sonra gövde. Hiçbir yerde çizgi yok: başlık şeridi gövdeden bir ton açıktır, odaktaki pencere ötekilerin bir ton üstüne çıkar, adı parlak ve kalındır, sol kenarında `▌` vurgu çubuğu durur.
- **Üç işaret.** Küçült, büyüt (büyükken eski boyuta dön) ve kapat başlığın sağ ucunda, her biri üç hücre; ekosistemdeki bütün kapatma işaretleri gibi imleç üstüne gelince üçü birlikte aydınlanır. Başka bir yerde bırakılan tıklama hiçbir şey yapmaz.
- **Taşıma ve boyutlandırma.** Başlık pencereyi sürükler, çift tık büyütür. Gövdenin sağ sütunu, alt satırı ve köşesi tutamaktır: imleç gelene kadar görünmez, sonra bir ton aydınlanır, sürüklenirken tam olarak bölücü sınırı gibi vurgu rengini alır. Sol kenar vurgu çubuğunun sütunu, üst kenar başlık olduğu için başka işe ayrılmıştır: alt tuşunu basılı tutup sol tuşla sürüklemek her yerden taşır, sağ tuşla sürüklemek imlece en yakın kenardan ya da köşeden boyutlandırır, sol ve üst dahil.
- **Sürükleme pencereye aittir.** Pencerede bir tuş bir kez indi mi, her sürükleme ve bırakma ona ulaşır, ekranın çok dışında bile; imleç taşıdığı pencereyi kaybetmez.
- **Gövdedeki terminal faresini korur.** Bir [`Terminal`](terminal) içindeki program fareyi okurken gövdedeki sade tıklamalar onundur. Başlık, işaretler, tutamaklar ve bütün alt sürüklemeleri yine pencerenindir; htop'un etrafındaki pencere taşınabilir kalır.
- **Farklar son mesajdan bu yana geçen hücrelerdir.** `Move { dx, dy }` ve `Resize { edge, dx, dy }` imlecin ne kadar gittiğini söyler, pencerenin nerede olması gerektiğini değil. Hangi kenarların hareket ettiğini `edge.left()`, `right()`, `top()` ve `bottom()` söyler.
- **Her durum çizilir.** Odakta ve odak dışı, imleç altındaki işaretler ve tutamaklar, daralan başlıkta önce alt başlığın sonra adın `…` ile kısalması (işaretler hep kalır), ekran dışına taşıp kırpılan pencere ve boş masaüstü — orayı uygulama doldurur (örnek pencereleri yeniden açmayı önerir). `on_event` verilmemiş pencere pasif durumdur: taşınamayan, işaretsiz bir yüzey.
- **Yavaş hat için hayalet.** SSH'de çizilen her kare bayt demektir; bu yüzden pencere yerinde kalabilir, imleci yalnızca bir `Ghost` izler, bırakılınca pencere tek karede oraya geçer. Aynı yüzey pencerenin yapışacağı alanı da gösterir: altında ne varsa ona vurgu rengi karışmış bir dikdörtgen, çizgi yok, tıklanacak bir şey yok. Pencerelerden sonra aynı stack'e yerleştir; gücünü sürüklenen hayalet için `.mix(0.25)`, yapıştırma önizlemesi için `.mix(0.20)` ile seç. Oyun alanındaki Hayalet sürükleme anahtarı ikisini birlikte gösterir.
- **İsteğe bağlı gölge.** `.shadow(true)` pencerenin sağında bir sütunu, altında bir satırı koyultur; pencere zeminin üstünde yüzer. Hareketi azalt açıkken ve 16 renkte çizilmez.
- **On altı renk.** Orada bütün yüzey tonları siyaha düştüğü için başlık şeridi odaktaki pencerede vurgu rengini, ötekilerde griyi alır; ad ikisinde de koyudur.

## Sık yapılan hatalar

- **Kararı pencereye bırakmak.** Pencere kendi kendine hiç hareket etmez. `update` olayı uygulayana kadar hiçbir şey olmaz; kenara yapıştırma, döşeme ve en küçük boyut ancak bu yüzden mümkündür.
- **Adı unutmak.** Adsız yerleştirilmiş pencere, sıra değişince hafızasını yitirir ve süren sürükleme biter.
- **İçeriğinden küçük dikdörtgen.** Gövde vurgu çubuğunun sütununu, ondan sonraki bir hücreyi, sağ sütunu ve alt satırı boş tutar; 20 × 5'in altında okunacak bir şey kalmaz. Örnekteki gibi kendi en küçük boyutunu koru.
- **Sabit yerleşim için pencere.** Ekranı her zaman paylaşan iki bölme bölücüye aittir; kullanıcı onları yerleştirmek zorunda kalmamalı.
