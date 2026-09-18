## Ne zaman kullanılır

Bir süre için süre girişi kullan: odak oturumu, mola, günlük hedef, zaman aşımı, hatırlatmaya kalan süre. Saat girişi gibi kurulur — saat ve dakika, istenirse saniye; her parça ayrı ayrı yazılır ya da değiştirilir — ama değer bir `std::time::Duration`'dır, birim sözcükleri dile uyar ve süre asla başa sarmaz.

## Adım adım

1. Süreyi uygulamanda tut: `session: Duration`.
2. Çiz: `DurationInput::new(state.session)`. Seçeneksiz alan saat ve dakikayı gösterir: `0 sa 50 dk`, İngilizcede `0 h 50 min`.
3. Değişiklikleri işle: `.on_change(|length| Msg::Session(length))`, süreyi `update` içinde sakla.
4. Saniye önemliyse, örneğin bir zaman aşımında, `.seconds(true)` ekle.
5. Başka alanlarla ilgili kuralları uygulamanda denetle; `.invalid(true)` ve bir mesajla göster, örneğin oturumdan uzun bir mola.
6. Yapıştırılan metnin neden okunmadığını söylemek için `.on_reject(|error| Msg::Rejected(error))` ekle ve alanın altında `error.message(ui.env().i18n())` göster.

## Nasıl çalışır

- **Birim sözcükleriyle bölümler.** Sayılar tek bir yüzeyde durur; arkalarındaki birim sözcükleri soluktur ve dil dosyalarından gelir. Alan Türkçede `1 sa 30 dk`, İngilizcede `1 h 30 min` yazar.
- **Klavye:** ← ve → bölümler arasında gezer, ↑ ve ↓ etkin bölümü kendi biriminden bir değiştirir. Yazılan rakamlar etkin bölümü ikişer ikişer doldurur: `0130` 1 sa 30 dk olur. `:` ya da boşluk tek rakamdan sonra da sonraki bölüme geçer, Backspace bölümü sıfırlar.
- **Süre başa sarmaz, taşar.** 0 sa 59 dk bir artınca 1 sa 00 dk olur; aşağı sıfırda, yukarı 99 sa 59 dk 59 sn'de durur. 59'u geçen dakikalar da taşar: dakikaya `90` yazmak 1 sa 30 dk olur.
- **Tekerlek farenin altındaki bölümü değiştirir**, her çentikte bir birim; tıklamak gerekmez, odak kıpırdamaz. Bir birim sözcüğünün üzerinde, farenin en son üzerinden geçtiği bölümü; fare doğrudan sözcüğe geldiyse etkin bölümü değiştirir. Saat girişi de tam böyle davranır.
- **Kopyalamak ve yapıştırmak.** Ctrl+A sürenin tamamını seçer; Ctrl+C onu etkin dilde yazıldığı gibi (`1 sa 30 dk`) kopyalar, Ctrl+X kopyalayıp sıfırlar. Yapıştırma `90 dk`, `90 min`, `1 sa 30 dk`, `1 hour 30 minutes`, `1h30m`, `1,5 sa` ya da `2:15` saat yüzünü okur; hangi dil etkin olursa olsun uygulamanın bildiği her dilde. Çıplak sayı dakikadır. Sağ tık aynı eylemleri menü olarak açar.
- **Boş ve dar.** Sıfır süre boş durumdur: fare ya da odak gelene kadar rakamları birim sözcükleri kadar soluk durur. Sözcükler sığmadığında alan aynı bölümlerle `1 : 30` saat biçimini gösterir.

## Sık yapılan hatalar

- **Süre için saat girişi.** Günün saati gece yarısında başa sarar ve birimi yoktur; 90 dakikalık bir süre sabahın 01:30'u değildir.
- **Birim sözcüklerini koda yazmak.** Sözcükler `quvyta.duration.*` anahtarlarından gelir; yeni bir dil ekleyen uygulama sözcüklerini oraya ekler, alan onları okur ve yazar.
- **`2:15`'i dakika ve saniye sanmak.** Saat yüzü her zaman saat ve dakikadır; dakika ve saniye için `2 dk 15 sn` yaz.
