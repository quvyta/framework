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
- **Kaydırma çubuğu sessizdir.** Kaydırma çubuğu sütunu yalnızca içerik taşınca, temanın seçtiği stilde belirir; başparmak hover ya da sürüklemede parlar. İçerik sütunu ona yer bırakır; çubuğun altına hiçbir şey çizilmez.
- **Konum hatırlanır.** Router sayfası içinde sayfaya geri dönmek kullanıcının kaldığı yeri geri getirir.

## Sık yapılan hatalar

- **Kaydırma alanı içinde kaydırma alanı.** İçteki sabit yükseklikli değilse iç içe koyma.
- **İçine liste koymak.** Listeler sanal kaydırır ve kendileri kayar; onun yerine yükseklik ver.
