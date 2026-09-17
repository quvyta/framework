## Ne zaman kullanılır

Günün bir saati için saat girişi kullan: bakım aralığının ne zaman başladığı, gece yedeğinin ne zaman alındığı, saat, dakika ve saniyeyle yazılan bir zaman aşımı. Her parça ayrı ayrı yazılabilir ya da değiştirilebilir, değer her zaman geçerli bir saattir.

## Adım adım

1. Saati uygulamanda tut: `starts: TimeOfDay`.
2. Çiz: `TimeInput::new(state.starts)`. Seçeneksiz alan saat ve dakikayı gösterir.
3. Değişiklikleri işle: `.on_change(|time| Msg::Starts(time))`, saati `update` içinde sakla.
4. Saniye önemliyse `.seconds(true)` ekle.
5. Başka alanlarla ilgili kuralları uygulamanda denetle; `.invalid(true)` ve bir mesajla göster, örneğin başlangıçtan önce biten bir aralık.

## Nasıl çalışır

- **Tek yüzeyde bölümler.** Saat, dakika ve saniye ikişer haneli bölümlerdir; aralarındaki iki nokta soluktur. Alan odaklıyken bir bölüm etkindir ve vurgu renginden bir dokunuşla yükselir.
- **Her zaman 24 saat.** Alan her dilde `14:30` yazar; bir dilin 12 saatlik düzenine uymaz. Kullanıcıların ÖÖ/ÖS bekliyorsa bunu alanın yanında belirt.
- **Klavye:** ← ve → bölümler arasında gezer, ↑ ve ↓ etkin bölümü değiştirir ve başa sarar (23'ten sonra 00, 00'dan önce 59 gelir). Yazılan rakamlar etkin bölümü doldurur: `0930` 09:30 olur. İki haneli bir değeri başlatamayacak rakam, örneğin saatte 7, bölümü hemen tamamlar. `:` sonraki bölüme geçer, Backspace bölümü sıfırlar.
- **Fare:** tıklanan bölüm etkin olur.
- **Değer her zaman geçerlidir.** Yarım yazılmış bir durum yoktur; her değişiklik eksiksiz bir `TimeOfDay` gönderir.
- **Tekerlek farenin üzerindeki bölümü değiştirir**, her çentikte bir; ↑ ↓ gibi başa sarar. Önce tıklamak gerekmez; etkin bölüm ve odak yerinde kalır. Fare iki noktanın üzerindeyse en son üzerinden geçtiği bölüm değişir; fare doğrudan iki noktaya geldiyse etkin bölüm.
- **Saati kopyalamak.** Ctrl+A saatin tamamını seçer; Ctrl+C onu `09:30` (saniyeyle `09:30:15`) olarak kopyalar, Ctrl+X kopyalayıp sıfırlar. `9:30` ya da `09:30:15` yapıştırmak saati ayarlar; başka metin yok sayılır. Sağ tık aynı eylemleri menü olarak açar.

## Sık yapılan hatalar

- **Bir günden uzun süreler için saat girişi.** Saat 23'te durur; birimli bir sayı girişi kullan.
- **Alanın içinde doğrulamak.** Tek başına bir saat her zaman geçerlidir; saatler arasındaki ilişki uygulamanın işidir.
- **Alanın yanında 12 saatlik açıklamalar.** Alan her zaman 24 saatliktir; açıklamaları da öyle yaz.
