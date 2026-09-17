## Metotlar

- `TimeOfDay::new(saat, dakika, saniye)` — 24 saatlik düzende bir saat; her parça en büyük değerinde durdurulur. `hour`, `minute`, `second` alanları herkese açıktır; saatler saat sırasına göre karşılaştırılır.
- `TimeInput::new(saat)` — saat ve dakikayı gösteren alan.
- `.seconds(bool)` — saniyeyi de gösterir ve düzenletir.
- `.invalid(bool)` — saati kendi kuralına uymuyor diye işaretler.
- `.disabled(bool)` — alanı soluklaştırır; odak alamaz, değiştirilemez.
- `.on_change(|saat| mesaj)` — her değişiklikte yeni saatle mesaj. Verilmezse alan görünür ama odak alamaz.

## Davranış

- Her bölüm için dört, her iki nokta için bir hücre ölçer: 9 hücre, saniyeyle 14.
- ← → etkin bölümü değiştirir ve uçlarda durur; ↑ ↓ bölümü bir artırır ya da azaltır, başa sarar.
- Rakamlar etkin bölümü doldurur; bölüm iki rakamdan sonra ya da aralıkta bir değer başlatamayacak bir rakamdan sonra tamamlanır ve sonraki bölüm etkin olur. Aralığın dışına taşıracak ikinci rakam bölümü yeniden başlatır.
- `:` sonraki bölüme geçer, Backspace etkin bölümü sıfırlar.
- Tıklanan bölüm etkin olur. Alan odağı kaybedince yeniden ilk bölüm etkin olur.
- Her dilde 24 saatlik biçimde yazılır.
- Tekerlek farenin altındaki bölümü her çentikte bir değiştirir ve başa sarar; odağı ve etkin bölümü kıpırdatmaz. İki noktanın üzerinde, fare alana girdiğinden beri en son üzerinden geçtiği bölümü, yoksa etkin bölümü değiştirir; alandan çıkınca bu unutulur. Pasif ya da `on_change` verilmemiş alan tekerleği geçirir, sayfa kayar.
- Ctrl+A saatin tamamını seçer (her bölüm `text-input-selection` alır); başka bir tuş ya da tıklama seçimi kaldırır. Ctrl+C `ss:dd` ya da `ss:dd:sn` kopyalar; Ctrl+X kopyalar ve `00:00` yapar.
- Her parçası aralıkta olan yapıştırılmış bir `s:dd` ya da `s:dd:sn` saati ayarlar; başka her şey yok sayılır ve iletilir.
- Sağ tık, Shift+F10 ya da menü tuşu düzenleme menüsünü açar; Kes ve Kopyala saatin tamamının seçili olmasını ister. Dil anahtarları `quvyta.edit.*`.

## Tema anahtarları

- `time-input` — `bg`, `fg`; durumlar `hover`, `focus`, `invalid`, `disabled`.
- `time-segment` — `bg`, `fg`, `bold`; etkin bölüm için `selected`, ilk rakam ikincisini beklerken `active`.
- `time-separator` — `fg`.
