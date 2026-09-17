## Ne zaman kullanılır

Bir şey çalışırken ve ne kadar ilerlediğini söyleyemiyorken spinner göster: imaj indirmek, sunucuyu beklemek, container başlatmak. İlerlemeyi biliyorsan onun yerine ilerleme çubuğu kullan.

## Adım adım

1. İşin gösterildiği yere ekle: `ui.add(Spinner::new())`.
2. Ne olduğunu söyle: `.label(t!("pulling"))`. Tek başına spinner bir şeyin çalıştığını *gösterir*, etiket *ne* olduğunu anlatır.
3. Varsayılan `Arc` çoğu yere uyar. Yer başka bir şey istiyorsa stil seç: arka planda sakin sağlık durumu için `.style(SpinnerStyle::Pulse)`, yoğun iş için `Dots`, dolan bir pasta için `Slices`.
4. Durum önemliyse ton ver: yeniden denerken `.variant("warning")`, bağlantı koptuğunda `"danger"`.
5. İş başarıyla bitince spinner'ı yerinde bitir: `.done(true)` ver ve etiketi sonuca çevir ("deploy-api imajı derlendi"). Önce ve sonra aynı bileşen olsun diye spinner'a bir `.id(..)` ver. İş başarısız olursa spinner'ı kaldır ve neyin ters gittiğini söyle.

## Nasıl çalışır

- **Tek hücre.** Spinner çevresindeki yerleşimi hiç değiştirmez; etiket iki hücre sonra başlar. Her stilin ve bitişin her karesi tam bir hücre genişliğindedir; `Quarters`, terminallerin kendisinin çizdiği çeyrek blokları `▖▘▝▗` döndürür, bu yüzden hiçbir yazı tipinde taşmaz.
- **Stiller alfabetik sırada.** `SpinnerStyle::ALL` sırasıyla Arc, Dots, Orbit, Pop, Pulse, Quarters ve Slices'ı listeler; Arc en baştadır ve varsayılandır.
- **Her stil bir hücre animasyonudur.** Karelerinin Nerd Font, Unicode ve ASCII karakterleri vardır; her terminalde çalışır. Bir tema birini `[animations.spinner-arc]` ile değiştirir; Animasyon stüdyosu sayfası hepsini her karakter modunda gösterir ve kopyalarını düzenletir. `Spinner::animation(ad)` başka herhangi bir animasyonu oynatır.
- **Nerd Font karakterleri.** Nerd Font modunda `Arc`, `nf-extra-progress_spinner_1..6` (U+EE06 ile U+EE0B arası) karelerini döndürür; `Slices`, `nf-md-circle_slice_1..8` (U+F0A9E ile U+F0AA5 arası) ile bir pastayı dilim dilim doldurur. İkisi de Nerd Font v3 ister. Unicode modunda `Arc` `◜◠◝◞◡◟` ile süpürür, `Slices` yarım daireler `◐◓◑◒` ile döner; ASCII modunda `-\|/` ve `.oO@` kullanılır.
- **Hız temadan gelir.** Kareler her `motion.spinner` süresinde değişir; nabız stili `motion.pulse-period` boyunca silik renkten tona karışarak nefes alır.
- **Bitiş.** `done` açılınca spinner dönmeyi bırakır, `spinner-done` animasyonunun beş karesini her biri bir `motion.step` sürecek şekilde bir kez oynatır ve sonuncuda kalır. Onay işareti her adımda büyür; rengi spinner'ın renginden (nabız, o anda hangi renkteyse oradan) `$success` rengine, yani temanın başarı rengine eşit adımlarla karışır: ortadaki kare tam ara renktir. `done` kapanınca yeniden döner.
- **Bitiş karakterleri.** Nerd Font: `nf-md-progress_check`, `nf-fa-check_circle_o`, `nf-oct-check_circle`, `nf-md-check_circle_outline`, `nf-md-checkbox_marked_circle` (U+F0995, U+F05D, U+F49E, U+F05E1, U+F0133); Nerd Font v3 gerekir. Unicode: `·∙✓✔✔`. ASCII: `..vvv`. İki kare aynı karakteri paylaştığında da renk ilerlemeye devam eder.
- **Hareketi azaltma.** Dönen spinner ilk karesinde sabit durur; bitmiş spinner son işareti son renginde hemen gösterir. İlk göründüğünde zaten bitmiş olan spinner da oynatmadan işarette durur.

## Sık yapılan hatalar

- **Hiç bitmeyen spinner.** İş başarısız olursa spinner'ı durdur ve bunu söyle.
- **Başarısızlık için onay işareti.** `done` işin başarıyla bittiği anlamına gelir; işaret başarı rengindedir. Hata varsa hatayı göster.
- **Aynı anda çok spinner.** Hareketle dolu bir ekran gürültüdür; işleri tek spinner altında bir sayıyla topla.
- **Bilinen ilerleme için spinner.** İnsanlar ne kadar süreceğini bilmek ister; `ProgressBar::new(değer)` kullan.
