## Ne zaman kullanılır

İki alanla yan yana çalışılıyorsa ve hangisinin ne kadar yer alacağına kullanıcı karar vermek istiyorsa bölücü kullan: editörün yanında dosya listesi, derleme logunun üstünde kaynak görünümü gibi. Yerleşim sabitse bir satır ya da sütun yeter.

## Adım adım

1. İlk bölmenin boyutunu uygulamanda tut: `files: u16`.
2. Yönü seç: yan yana bölmeler için `Splitter::columns(state.files)`, üst üste bölmeler için `Splitter::rows(state.log)`.
3. Bölmeleri doldur: `.first(|ui| ...)` ve `.second(|ui| ...)`, sonra `.show(ui)`.
4. Boyutlanabilir yap: `.on_resize(|boyut| Msg::Files(boyut))` ve boyutu `update` içinde kaydet.
5. Hiçbir bölme işe yaramaz hale gelmesin diye sınır koy: bir aralık için `.limits(16, 60)`, yalnızca en küçük boyut için `.limits(16, None)`.
6. Bölücüye yer ver: aldığı alanı doldurur, bu yüzden boyutu belli bir sütuna ya da doldurulan bir alana koy.

## Nasıl çalışır

- **Asla çizgi yok.** Sınır, çevresindeki zeminle aynı görünen tek boş hücredir. İmleç üstüne gelince bir kademe aydınlanır, sürüklenirken vurgu rengini alır, odaktayken hafif bir vurgu tonu taşır.
- **Sürükle ya da klavye.** Tab sınıra ulaşır; ← → (üst üste bölmelerde ↑ ↓) onu bir hücre taşır, Shift beş hücre taşır, Home ve End sınırlara atlar.
- **Karar senin durumunda.** Sürüklemek yalnızca sınırlar içinde kalmış boyutları gönderir; bölmeler sen kaydedince yer değiştirir.
- **Sınırlar senin.** Varsayılan olarak ilk bölme en az 1 hücredir ve üst sınırı yoktur. `.limits(min, max)` `max` için bir sayı ya da `None` alır, yan paneldeki gibi; oyun alanı dosya bölmesini sınırsız, 16–40 ve 24–48 arasında değiştirir.
- **İkinci bölme her zaman en az bir hücre korur;** alan ne kadar daralırsa daralsın ilk bölme sınırları aşmaz.
- **Bölücüler iç içe geçer.** Bir bölme, kendi boyutu olan başka bir bölücü taşıyabilir.

## Sık yapılan hatalar

- **Durumda oran tutmak.** Boyut hücre cinsindendir; pencere çok değişirse `update` içinde sınırla ya da yeniden hesapla.
- **Kaydırma alanında boyutsuz bölücü.** Bütün kaydırma yüksekliğini almaya çalışır; kapsayıcısına bir yükseklik ver.
- **Bölmenin içine ayırıcı çizmek.** Sınır gerektiğinde kendini zaten gösterir; çizilmiş bir çizgi görünümü bozar.
