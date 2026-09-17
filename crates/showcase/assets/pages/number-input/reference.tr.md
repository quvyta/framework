## Metotlar

- `NumberInput::new(değer)` — `değer`i gösteren, sınırsız, birer birer değişen alan.
- `.range(en_küçük, en_büyük)` — izin verilen en küçük ve en büyük değer; ters verilen aralık sıraya sokulur.
- `.step(adım)` — ↑ ve ↓'nin ne kadar değiştireceği; ondalıkları yazılışı ve `.` yazılıp yazılamayacağını belirler. Pozitif olmayan adım 1 olur.
- `.steppers(bool)` — sağda tıklanabilir eksi ve artı bölümleri.
- `.placeholder(metin)` — alan boşken görünen soluk metin.
- `.invalid(bool)` — değeri kendi kuralına uymuyor diye işaretler.
- `.disabled(bool)` — salt okunur ve odak alamaz.
- `.on_change(|değer| mesaj)` — aralıktaki her yeni geçerli sayıyla mesaj.

## Davranış

- İstem işaretini, iç boşluğu, en küçük, en büyük ve güncel değerden en genişini (aralık yoksa sekiz hücre) ve adım bölümleri varsa altı hücreyi ölçer.
- Yazarken rakamları, en küçük değer sıfırın altındaysa `-` işaretini, adımın ondalığı varsa `.` karakterini kabul eder; geri kalan her şey `TextInput` gibi düzenlenir.
- Sayıya çevrilemeyen ya da aralık dışındaki metin `invalid` durumuyla gösterilir ve gönderilmez; boş alan ne işaretlenir ne gönderilir.
- ↑ ↓ bir adım, Page Up / Page Down on adım; yazılan metin sayıysa ondan başlar, sonuç aralığa sıkıştırılır.
- Adım bölümüne tıklamak bir adım değiştirir ve bölümü parlatır; sınırdaki bölüm `disabled` durumunu gösterir.
- Uygulamanın değeri değişince metin adımın ondalıklarıyla yeniden yazılır.
- Alanın üzerindeki tekerlek her çentikte aralık içinde bir adım ilerletir ve çevresindeki kaydırma alanına geçmez.
- Sağ tık, Shift+F10 ya da menü tuşu düzenleme menüsünü açar (Kes, Kopyala, Yapıştır, Tümünü seç); menü açıkken ↑ ↓ değeri değil menüyü gezer.

## Tema anahtarları

- `text-input`, `text-input-prompt`, `text-input-placeholder`, `text-input-selection`, `text-input-cursor` — alanın kendisi, `TextInput` ile aynı.
- `number-input-stepper` — `bg`, `fg`; durumlar `hover`, `focus` (alan odaklıyken), `pressed`, `disabled`.
- `[icons]` — `prompt`, `stepper-minus`, `stepper-plus`.
