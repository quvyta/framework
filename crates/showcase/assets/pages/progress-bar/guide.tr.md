## Ne zaman kullanılır

İşin ne kadarının bittiğini biliyorsan ilerleme çubuğu kullan: yükleme, taşıma, test çalıştırması. Bir şeyin ne kadar dolu olduğunu da gösterir: disk ya da kota gibi. İşin büyüklüğü bilinmiyorsa belirsiz çubuğu ya da spinner kullan.

## Adım adım

1. 0 ile 1 arasında bir değerle çubuk ekle: `ui.add(ProgressBar::new(0.45)).width(Length::Fill(1))`.
2. İş ilerleme bildirdikçe değeri durumundan güncelle; çubuk yeni değerle çizilir.
3. Hikâyeyi anlatan bir ton ver: bittiğinde `.variant("success")`, disk dolmak üzereyken `"warning"`, kota bittiğinde `"danger"`.
4. Yüzdeyi başka bir etiket zaten söylüyorsa gizle: `.percent(false)`.
5. İlerlemeyi ölçemiyorsan `ProgressBar::indeterminate()` kullan.

## Nasıl çalışır

- **Sekizde bir hücre hassasiyeti.** Tam hücreler renkle doldurulur, son hücre kısmi blok kullanır; 40 hücrelik bir çubukta 320 görünür adım vardır. ASCII modunda grafiklerdeki gibi en yakın tam hücreye yuvarlanır.
- **Çerçeve yok, parantez yok.** İz sakin bir yüzey, dolgu bir renktir; çevrelerinde hiçbir şey yoktur.
- **Belirsiz süpürme.** Her `motion.shimmer` süresinde iz boyunca bir ışık bandı geçer; her hücre banda uzaklığına göre karışır.
- **Hareketi azaltma.** Belirsiz çubuk silik, düz bir tonla durur.

## Sık yapılan hatalar

- **Geri gitmek.** Küçülen ilerleme güveni kırar; tahmin değişirse belirsize geç.
- **%100'de kalmak.** Biten çubuğu sonuçla değiştir ya da başarı tonuna çevir.
- **Çok kısa çubuklar.** Yaklaşık on hücrenin altında adımlar zor okunur; yer ver.
