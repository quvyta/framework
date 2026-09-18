## Metotlar

- `DurationInput::new(süre)` — bir `Duration`'ı saat ve dakika olarak gösteren alan. Saniyenin kesirleri gösterilmez, ilk değişiklikte düşer.
- `.seconds(bool)` — saniyeyi de gösterir ve düzenletir.
- `.invalid(bool)` — süreyi kendi kuralına uymuyor diye işaretler.
- `.disabled(bool)` — alanı soluklaştırır; odak alamaz, değiştirilemez.
- `.on_change(|süre| mesaj)` — her değişiklikte yeni süreyle mesaj. Verilmezse alan görünür ama odak alamaz.
- `.on_reject(|hata| mesaj)` — okunamayan bir yapıştırmanın `DurationError`'ıyla mesaj. Verilmezse böyle bir yapıştırma, saat girişindeki gibi yok sayılır ve iletilir.
- `parse_duration(metin, i18n)` — yazılmış bir süreyi yapıştırmanın okuduğu gibi okur: `Ok(Duration)` ya da okunamayan ilk parçanın `DurationError`'ı.
- `DurationError::message(i18n)` — sebep, etkin dilde. Türleri `Empty`, `Character`, `UnknownUnit`, `BadNumber`, `MissingNumber`, `MissingUnit`, `RepeatedUnit(DurationUnit)`, `BadClock` ve `TooLarge`.
- `DurationUnit` — `Hours`, `Minutes`, `Seconds`.

## Okuma

- Birimli sayılar, herhangi bir sırayla, her birim bir kez: `1 sa 30 dk`, `1h30m`, `2 saat`. Büyük küçük harf fark etmez; `İ`, `I` ve `ı` aynı okunur.
- Birim sözcükleri her dilin `quvyta.duration.hour-words`, `minute-words` ve `second-words` anahtarlarıdır; önce etkin dil.
- Bir birimden sonra kendi birimi olmayan sayı bir küçük birimi alır: `1 sa 30`, 1 sa 30 dk'dır. Tek başına sayı dakikadır.
- Ondalık nokta ya da virgül bir birimi böler, saniyeye yuvarlanır: `1,5 sa`, `1.5 h`.
- `s:dd` ya da `s:dd:sn` saat yüzü saat ve dakikadır; dakika ve saniye 60'ın altında bir ya da iki hanedir.
- 59'u geçen dakika ve saniye taşar; en uzun süre 99 sa 59 dk 59 sn'dir.

## Davranış

- Her bölüm için bir hücre boşluk, iki rakam ve bir hücre boşluk, artı birim sözcükleri ölçer: Türkçe ve İngilizcede 12 hücre, saniyeyle 17. 99'dan büyük verilen saatler yanlış sayı göstermek yerine bölümlerini genişletir.
- Bundan dar alanda alan aynı bölümlerle `ss : dd` saat biçimini gösterir; daha da darsa kenarda kesilir.
- ← → etkin bölümü değiştirir ve uçlarda durur; ↑ ↓ süreyi etkin bölümün bir birimi kadar değiştirir, birimler arasında taşar, sıfırda ve en uzun sürede durur.
- Rakamlar etkin bölümü ikişer doldurur, sonra sonraki bölüm etkin olur. `:` ve boşluk sonrakine geçer, Backspace etkin bölümü sıfırlar.
- Tıklanan bölüm etkin olur; bir birim sözcüğü önündeki bölüme aittir. Alan odağı kaybedince yeniden ilk bölüm etkin olur.
- Tekerlek farenin altındaki bölümü her çentikte bir birim değiştirir; odağı ve etkin bölümü kıpırdatmaz. Bir birim sözcüğünün ya da iki noktanın üzerinde, fare alana girdiğinden beri en son üzerinden geçtiği bölümü, yoksa etkin bölümü değiştirir. Pasif ya da `on_change` verilmemiş alan tekerleği geçirir, sayfa kayar.
- Ctrl+A sürenin tamamını seçer; Ctrl+C onu etkin dilde, sıfır olan parçaları atlayarak kopyalar (`1 sa 30 dk`, `45 dk`, `0 dk`); Ctrl+X kopyalar ve sıfırlar.
- Okunan bir yapıştırma süreyi ayarlar; saniye ekranda değilse yapıştırılan saniye düşer, süre kendi saniyesini korur.
- Sağ tık, Shift+F10 ya da menü tuşu düzenleme menüsünü açar; Kes ve Kopyala sürenin tamamının seçili olmasını ister.
- Sıfır süre, alanın üzerine gelinene ya da odaklanılana kadar rakamlarını `time-separator` ile çizer: boş durum.

## Tema anahtarları

Saat girişiyle ortaktır:

- `time-input` — `bg`, `fg`; durumlar `hover`, `focus`, `invalid`, `disabled`.
- `time-segment` — `bg`, `fg`, `bold`; etkin bölüm için `selected`, ilk rakam ikincisini beklerken `active`.
- `time-separator` — birim sözcüklerinin, iki noktaların ve dinlenen sıfır sürenin `fg`'si.

## Dil anahtarları

- `quvyta.duration.hours`, `minutes`, `seconds` — sayılardan sonra gösterilen sözcükler.
- `quvyta.duration.hour-words`, `minute-words`, `second-words` — birim olarak okunan her sözcük, virgülle ayrılmış. Diller arasında çakışmamalıdır.
- `quvyta.duration.hour-name`, `minute-name`, `second-name` ve `empty`, `character`, `unit`, `number`, `no-number`, `no-unit`, `repeated`, `clock`, `too-large` — sebepler.
