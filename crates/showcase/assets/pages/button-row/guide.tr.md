## Ne zaman kullanılır

Buton sırası, bir yerin eylemleri içindir: küçük bir düzenleyicinin araç çubuğu, bir formun kendi düğmeleri, bir ekranın sonundaki seçenekler. Satır sığmayanı kendisi karar verir, böylece terminal daraldığında hangi eylemi gizleyeceğini uygulama seçmez ve her eylem okuma sırasındaki yerini korur. Sekme şeridi değildir (her düğme birden etki eder) ve `Modal` ya da `EmptyState` içindeki bir iletişim kutusunun eylem satırı da değildir; onlar kendi düğmelerini kendileri yerleştirir.

## Adım adım

1. Sırayı kur ve düğmeleri tek tek ekle: `ButtonRow::new().button(...)`.
2. Her düğmeye sıradaki gibi kendi mesajını ver: `Button::new("Save").on_press(Msg::Save)`.
3. Satıra kullanabileceği genişliği ver: bir panelde `.fill_width()`, tüm satırı almasın istiyorsan `.width(Length::Cells(60))`.
4. `update` içinde mesajın söylediğini yap. Menüden seçilen bir düğme, satırda basılanla aynı mesajı gönderir, ayırt edilecek bir şey yoktur.

## Nasıl çalışır

- **Düğmeler sıradan düğmelerdir.** Her biri etiketini, ikonunu, kısayolunu, çeşidini ve pasif durumunu korur ve bir iletişim kutusunun eylem satırı gibi öncekinin iki hücre sonrasında durur.
- **Dar satır sonunu menüye taşır.** Her düğme sığdığında satırda başka denetim yoktur. Sığmadığında sondakiler, sondan başlayarak, ilklerinin duracağı yerde duran bir `Diğerleri` denetimine geçer. Bir girdi seçmek o düğmenin mesajını gönderir; tek bir düğmeye bile yer olmayan satır denetimi yalnız, kesilmiş olarak gösterir.
- **Menü framework'ün kendi menüsüdür.** Sekme şeridinin gizli sekmeleri ve konum yolunun gizli seviyeleri için kullandığı açılır parçanın aynısı, böylece ↑ ↓ Home End, harf yazmak, Enter ve Esc her yerde nasılsa öyle davranır.
- **Denetim her buton gibidir.** `button` ile aynı yüzey, iç boşluk ve etiket; üstüne gelince ve odaktayken ilk hücresinde çubuk, basışta bir ton parlar. Sözcükler `quvyta.button-row.more` dil anahtarından gelir, yani okuyanın dilindedir.
- **Klavye.** Tab düğmeleri soldan başlayarak birer birer ve sonra denetimi bulur. Enter ya da Space menüyü açar, menü bir seçim yapılana ya da Esc ile kapanana kadar klavyeyi tutar. Menü açıkken bir düğmenin yanına basmak menüyü kapatır ve yine o düğmeye basar, böylece menü kapatmak için bir tıklama harcanmaz.

## Sık yapılan hatalar

- **Denetimi ekranın sonunda beklemek.** O, sığmayan ilk düğmenin duracağı yerde durur, bu yüzden düğmelerinden geniş bir satır sağında boşluk bırakır.
- **Düğmeleri uygulamada da gizlemek.** Bırakın satır yapsın: menüdeki bir düğme, satırdakiyle aynı mesajı gönderir, böylece her eylemin tek bir yolu olur.
- **Tek bir düğmeye ad vermeyi beklemek.** Satırdaki düğme kendi düğümü değildir, bu yüzden `Command::focus("save")` ona ulaşamaz; satıra odaklanıp Tab'ın gezmesini bırakın.
