## Ne zaman kullanılır

Seviye sütunları bir şeyin şu anda ne kadar yüksek olduğunu göstermek içindir: bir müzik
çalıcının spektrumu, bir karışımın kanalları, her kanalın yükü. "Hangi bant yüksek, ne kadar
yüksek?" sorusunu bir bakışta yanıtlar. Zaman içindeki bir değer için mini grafik, adıyla karşılaştırılan
değerler için çubuk grafik, tek sayı ve bir sınır için ölçer kullanın.

## Adım adım

1. Seviyeleri kendiniz hesaplayın, her sütun için sıfırdan bire kadar; bileşen yalnızca sütunları
   çizer, değer bir frekans bandından, bir ses kanalından ya da bir kuyruk derinliğinden gelebilir.
2. Çizin: `ui.add(LevelBars::new(levels)).width(Length::Fill(1)).height(Length::Cells(8))`.
3. Düğümün istediğiniz çözünürlükte yüksekliğini verin: her satır sekiz seviyedir, sekiz satır bir
   bant için altmış dört adım gösterir.
4. Bir bandın ulaştığı yüksek işareti tutmak istiyorsanız o seviyeyi yanında tutup verin:
   `.peaks(peaks)`. Sütunun üstünde olmayan bir şapka çizilmez.
5. Biçimi işine yaradığında açın: daha dolgun bir çubuk için `.bar_width(2)`, havasız bir bant
   grafiği için `.gap(0)`, sütunların orta satırdan aşağı ve yukarı büyümesi için `.mirror(true)`,
   sütunun temadan vurguya doğru derinleşmesi için `.gradient(true)`.
6. Okunmasını istediğiniz sayıları başka yere yazın: bir sütundan değer okunmaz, yalnızca seviye
   okunur.

## Nasıl çalışır

- **Hücrenin sekizde biriyle sütunlar.** Her değer, alanın dibinden başlayarak `▁▂▃▄▅▆▇█` ile
  ölçülen bir sütundur; üç satır bile yirmi dört seviye verir.
- **Ne kadar dar olursa olsun tüm spektrum.** Sığmayan değerler komşularıyla ortalanarak birleşir,
  böylece kırk sütunluk bir alan ilk kırk bandı değil iki yüz bandı gösterir. Artan yer ise
  sütunları genişletir; paylaştırılamayan tek hücre en dıştaki sütunlara gider, iki uç eşitlenir.
- **Çizgi değil, şapka.** Tepe, seviyeye yetişen blokla çizilir; seviye bir hücrenin tepesi olduğunda
  ince `▔` çizgisiyle. Şapka temanın tepe tonunda çizilir, böylece sütunun devamı değil bir işaret
  olarak okunur.
- **Tek vurgu.** Sütunlar mini grafikteki gibi vurgudan bir adım aşağıda bir ton alır, böylece şapka
  ve geçişin tepesi vurgunun kendisiyle öne çıkar. Geçiş hücre başına bir adım atar ve vurguda
  biter.
- **Sayı olmayan değer yoktur.** Sıfırın altındaki seviye sıfıra kırpılır, sayı olmayan seviye
  sessizliktir; böylece ölçülmemiş bir bant tam sütun yerine hiç çizilmez.
- **On altı renk ve ASCII.** Yalnızca on altı standart renk varsa geçiş çizilmez ve her sütun tek
  tonda kalır; çünkü orada bir karışım hücre başına bir palet girdisine yuvarlanır ve beneklenir.
  Blok glifi yoksa tam hücreler seviyenin tonunu alır, sütunun bittiği hücre ise dolduğu pay kadar
  o tonu taşır; böylece en sessiz bant bile bir hücreyi boyar.

## Sık yapılan hatalar

- **Anlamsız bir şapka.** `.peaks` bandın gerçekten ulaştığı seviyeyi ister. Yine mevcut seviyeyi
  vermek her sütuna bir şapka çizer, bu da şapkasız olmakla aynıdır.
- **Tek satır ve ince ölçek.** Tek satır sekiz seviyedir; az hareket eden bir spektrum donmuş görünür.
  Dört satır ya da daha fazlası verin.
- **Seviye olmayan değerler.** Sıfırdan yüze yüzdeler seviye değildir: bire kırpılır ve her bant aynı
  görünür. Önce tam ölçeğe bölün.
- **Sütundan sayı okumak.** Blok seviyeyi söyler, değeri değil. Birinin okuması gerekiyorsa dB
  rakamını satırın yanına yazın.
- **Saatsiz hareket.** Bileşen bir değeri kendi başına hareket ettirmez; uygulama yeni seviyeleri
  gönderir ve hareket azaltılmışken çalışmaya devam eden bir görselleştirici göndermeyi bırakmalıdır.
