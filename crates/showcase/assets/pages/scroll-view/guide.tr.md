## Ne zaman kullanılır

Aldığı alandan uzun olabilecek içeriği kaydırma alanına koy: belgeler, uzun formlar, ayar sayfaları. Listeler kendileri kayar, bir kaydırma alanına ihtiyaçları yoktur.

## Adım adım

1. İçeriğiyle ekle: `ui.add_with(ScrollView::new(), |ui| { ... })`.
2. Bir yükseklik ver; genelde `.fill()` ya da `.height(Length::Cells(n))`.
3. Aynı yerde birden fazla kaydırma alanı görünebiliyorsa `.id(...)` ile adlandır; her biri kendi konumunu korur.

## Nasıl çalışır

- **İçerik tam yüksekliğinde yerleşir** ve bir pencereden çizilir. Pencere dışındaki her şey, yarısı görünen bileşenler dahil, kırpılır.
- **Üç kaydırma yolu.** Fare tekerleği üç satır kaydırır, kaydırma çubuğuna tıklanıp sürüklenebilir, alan odaktayken ↑ ↓ PgUp PgDn Home End hareket ettirir.
- **Odak alanı çeker.** Tab görünmeyen bir iç bileşene odaklanınca alan onu gösterecek kadar kayar.
- **Sonu izlemek tek bir seçenek.** `.follow_end(true)`, kişi bakarken büyüyen içerik içindir: bir sohbet, bir derlemenin çıktısı. Alan sonunda açılır ve satırlar geldikçe orada kalır; her yeni sona kayarak gider, hareketi azalt açıksa atlar. Okumak için yukarı kaydırmak görünümü kişinin bıraktığı yerde tutar, altta soluk bir not aşağıdaki satırları sayar; End, en alta geri kaydırmak ya da nota tıklamak izlemeyi yeniden başlatır.
- **Odak ve izleme çatışmaz.** Odaklanan bir bileşeni göstermek için yapılan hareket sondan ayrılıyorsa, yukarı kaydırmak gibi izlemeyi durdurur; böylece bileşen görünür kalır. Sondaki odaklı bir bileşen, örneğin sohbetin altındaki yazma alanı, izlemeyi sürdürür ve içerik büyüdükçe görünür kalır.
- **Kaydırma çubuğu sessizdir.** Kaydırma çubuğu sütunu yalnızca içerik taşınca, temanın seçtiği stilde belirir; başparmak hover ya da sürüklemede parlar. İçerik sütunu ona yer bırakır; çubuğun altına hiçbir şey çizilmez.
- **Konum hatırlanır.** Router sayfası içinde sayfaya geri dönmek kullanıcının kaldığı yeri geri getirir.

## Sık yapılan hatalar

- **Sona kaydırmayı uygulamadan yapmaya çalışmak.** Büyüyen içerik için uygulamanın konumu takip etmesi gerekmez; `.follow_end(true)` bunu yapar ve kişinin yukarı kaydırmasına saygı duyar.

- **Kaydırma alanı içinde kaydırma alanı.** İçteki sabit yükseklikli değilse iç içe koyma.
- **İçine liste koymak.** Listeler sanal kaydırır ve kendileri kayar; onun yerine yükseklik ver.
