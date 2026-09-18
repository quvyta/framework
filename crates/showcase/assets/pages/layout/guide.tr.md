## Satırlar, sütunlar ve katmanlar

Her ekran bir kapsayıcı ağacıdır. `ui.column` çocukları yukarıdan aşağıya dizer, `ui.row` soldan sağa yerleştirir, `ui.stack` üst üste çizer. Kapsayıcılar serbestçe iç içe geçer.

## Boyutlar

Her çocuğun bir genişliği ve yüksekliği vardır, her biri üç türden biridir:

- `Length::Auto` — bileşen ne kadar ölçerse o kadar. Metin kelimelerini, buton etiketini ve iç boşluğunu ölçer.
- `Length::Cells(n)` — tam `n` sütun ya da satır.
- `Length::Fill(ağırlık)` — `Auto` ve `Cells` kardeşler paylarını aldıktan sonra kalanın bir payı. `Fill(2)`, `Fill(1)`'in iki katını alır.

`.fill()`, `.fill_width()` ve `.fill_height()` `Fill(1)` için kısayollardır.

## Boşluk ve hizalama

- `.gap(n)` satır ya da sütun çocukları arasında `n` hücre bırakır.
- `.padding(Padding::symmetric(dikey, yatay))` herhangi bir düğümün içinde boş alan tutar.
- `.justify(Align::Center)` yer kaldığında çocukları ana eksende yerleştirir; `.align(..)` onları çapraz eksende yerleştirir.

Boşluk çizgilerden değil, boş hücrelerden ve yüzey renginden gelir. Tutarlı bir ritim kullan: gruplar arasında bir satır, kardeşler arasında iki üç sütun.

## Yerleşimi alana göre seçmek

`ui.size()`, uygulamanın çizdiği alandır: sütun ve satır olarak terminalin tamamı. Uygulama bunu `view` içinde okuyup dizilişini seçer; "48 sütunun altında üç sütunu katla" tek bir `if` olur:

- Geniş: sütunlar bir `row` içinde yan yana.
- Dar: aynı içerik bir `column` içinde; sığmayan kısım kendi sayfasına geçer.

Sayı her iç içe kurucuda aynıdır — bir `column`, bir `Panel`, bir `SidePanel` parçası ya da bir `Modal` içinde de yine terminaldir — çünkü görünüm, yerleşim ekranı paylaştırmadan önce kurulur. Bu kararı basit tutar: karar kullanıcının elindeki terminalle ilgilidir. Okumak G/Ç değildir; boyut değişince sonraki kare yeni boyutu görür, testlerde `Harness::resize` da aynısını yapar; böylece dar yerleşim her ekran gibi test edilir.

Bu, uygulamanın kendi dizilişi içindir. Kendi dikdörtgeni içinde uyum sağlayan bir bileşen, örneğin etiketlerini kısaltan bir liste sütunu, bunu kendisi yapar.

## Çerçeve yerine yüzey

İlgili içeriği bir `Panel` içinde grupla: arka planından bir kademe yüksek bir yüzey. Panelin içindeki iç panel bir kademe daha yüksektir. Göz gruplamayı yalnızca tondan okur.

## Sayfalar ve router

`Router<P>`, kullanıcının açtığı sayfaları bir yığın olarak tutar. `push` bir sayfa açar, `back` geri döner, `replace` geçmiş eklemeden mevcut sayfayı değiştirir. Mevcut sayfayı `ui.page(anahtar, ...)` içinde çiz: her sayfa gizliyken odağını ve kaydırma konumunu korur, geri dönen kullanıcı kaldığı yere düşer.

## Uygulama iskeleti

`AppShell` alışılmış yerleşimi kurar: yüzey rengiyle ayrılmış üst çubuk, yan menü, gövde ve alt çubuk. `collapse_below` sütunun altında yan menü gizlenir; `sidebar_open(true)` ile gövdenin üstünde bir katman olarak belirir. Bu showcase onunla kuruldu.

## Sık yapılan hatalar

- **Auto bir ebeveyn içinde Fill.** Boyunu içeriğine göre belirleyen ebeveynin paylaşacak alanı yoktur; ebeveyne bir boyut ver.
- **Her yerde sabit genişlik.** Ekranlar terminale uyum sağlasın diye `Fill` tercih et.
- **Boyutu durumda tutmak.** `ui.size()` `view` içinde her zaman günceldir; uygulamanın durumunda tutulan bir kopya ilk boyut değişikliğinde eskir.
- **Ayırıcı çizmek.** Karakterlerden bir çizgi yerine boşluk ya da yüzey değişimi kullan.
