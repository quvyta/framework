## Ne zaman kullanılır

Şeklini bildiğin içerik yüklenirken iskelet kullan: container listesi, grafikli bir kart, ayrıntı paneli. Ekran yerleşimini korur; veri geldiğinde hiçbir şey zıplamaz. Beklenecek içerik olmayan işler için, örneğin bir dağıtımda, ilerleme çubuğu ya da spinner kullan.

## Adım adım

1. Yükleniyor görünümünü, yüklenmiş görünümle aynı yerleşimle kur.
2. İçeriğin geleceği yerlere iskelet koy: ikon için `Skeleton::avatar()`, bir isim ve bir ayrıntı için `Skeleton::lines(2)`, grafik ya da görsel için `Skeleton::block()`.
3. Onlara gerçek içerik kadar boyut ver: satırlar için `.width(Length::Cells(36))`, bloklar için `.height(..)`.
4. Veri gelince gerçek satırları aynı yere çiz.
5. Yükleme başarısız olursa iskeletlerin yerine bir hata mesajı koy; süpürmeye devam etmelerine izin verme.

## Nasıl çalışır

- **Sakin tonda şekiller.** Yer tutucular yükseltilmiş tonda çizilir. Metin satırları her satırın üst yarısını kullanır; böylece alt alta satırlar ayrı okunur. Genişlikler değişir, son satır kısadır; gerçek bir paragraf gibi.
- **İmza süpürme.** Her `motion.shimmer` süresinde şekillerin üstünden bir ışık bandı geçer, hücre hücre karışarak. Bant ekran sütununa göre konumlanır; ekrandaki tüm iskeletler tek bir süpürmenin parçasıdır, her biri kendi başına titremez.
- **Hareketi azaltma.** Şekiller dinlenme tonunda durur.
- **ASCII modu.** Yarım bloklar kullanılmaz; satırlar tam hücreleri renkle doldurur.

## Sık yapılan hatalar

- **Yüklenirken başka yerleşim.** İskelet yüklenmiş görünüme uymazsa ekran yine zıplar.
- **Bitmeyen iskelet.** Boş sonuç boş durum ister, hata bir hata mesajı. İskelet içerik sözü verir.
- **Çok fazla şekil.** Üç yer tutucu satır "bir liste geliyor" der; elli satır yalnızca gürültüdür.
