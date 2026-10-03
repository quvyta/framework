## Metotlar

- `LevelBars::new(values)` — her değer için bir sütun, her biri sıfırdan bire kadar bir seviye,
  en alçak bant önce.
- `.gap(cells)` — sütunlar arasındaki boş hücreler; varsayılan bir.
- `.bar_width(cells)` — bir sütunun çizildiği hücre sayısı; varsayılan bir, değerler yer bıraktığında
  genişler, alandan dar kalırsa alana kırpılır.
- `.peaks(peaks)` — her sütunun verilen seviyesinde bir şapka, her değer için bir değer; hiç şapka
  istenmiyorsa `peaks([])`.
- `.mirror(on)` — sütunları orta satırdan aşağı ve yukarı büyütür.
- `.gradient(on)` — sütunu derinleştikçe temanın taban tonundan vurguya doğru karıştırır.

## Davranış

- Her değer için bir sütun, hücrenin sekizde biriyle ölçer; düğümü doldurulacak bir genişlik ve
  istediğiniz çözünürlük için bir yükseklik verin, tek satır sekiz seviyedir.
- Sıfırın altındaki seviye sıfıra kırpılır, sayı olmayan seviye sessizliktir.
- Sığmayan değerler komşularıyla ortalanarak birleşir, böylece dar alanda da tüm spektrum görünür;
  birleşen bir sütunun şapkası, kapsadığı şapkaların ortalamasıdır.
- Artan yer her sütunu aynı pay kadar genişletir; paylaştırılamayan hücreler en dıştaki sütunlara
  gider, böylece sıranın iki ucu eşitlenir.
- `.mirror(true)` iki satır ister: tek satırlık alanda iki yarı da bir hücreden ince olurdu, bu yüzden
  sütun sade bir sütun gibi çizilir. Eşit paylaşılamayan seviye önce yukarı büyür.
- Şapka, seviyesi kendi sütununun seviyesinin üstünde ve kendine ait bir hücresi varsa çizilir: o
  seviyeye yetişen blokla, ya da seviye bir hücrenin tepesi olduğunda ince `▔` çizgisiyle.
- Alanın dışında hiçbir şey çizilmez ve değer listesi boşsa hiçbir şey çizilmez.
- ASCII gliflerde tam hücreler seviyenin tonunu iz üstünde alır, sütunun bittiği hücre dolduğu pay
  kadar o tonu taşır; böylece en sessiz seviye bile bir hücreyi boyar.
- On altı standart renkte geçiş çizilmez ve her sütun tek tonda kalır.
- Seviye sütunu ne tuşa ne tıklama cevap vermez: odağ almaz, mesaj göndermez.

## Tema anahtarları

- `level-bars` — `base` (sütunlar ve bir geçişin dibi), `peak` (şapka ve bir geçişin tepesi),
  `track` (ASCII kipindeki zemin).
