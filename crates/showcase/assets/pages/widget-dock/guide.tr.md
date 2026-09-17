## Ne zaman kullanılır

Birbirinden bağımsız araçlardan oluşan bir yan alan için bileşen yuvası kullan; bir IDE'nin yan paneli gibi: kaynak kontrolü, container'lar, portlar, etkinlik. İnsanlar ilgilendikleri araçları açar, kalanları kapatır, kendi sıralarına dizer; uygulama da bunu hatırlar.

## Adım adım

1. Yerleşimi kaydedilebilsin diye uygulamanda tut: gösterim sırasına göre bileşen kimlikleri ve hangilerinin açık olduğu.
2. Başlıkları bu sırayla kur: `WidgetDock::new(order.iter().map(|id| başlık(id)))`.
3. Her bileşen için aynı sırayla bir çocuk ekle; çoğunlukla `.fill()` sütunlar ya da listeler.
4. Açık durumunu gösterim sırasıyla ver, değişiklikleri işle: `.open(open).on_toggle(|konum, açık| Msg::Toggle(konum, açık))`.
5. Sıralamaya izin vermek için `.on_move(|from, to| Msg::Move(from, to))` ekle; `update` içinde `from` konumundaki kimliği çıkar, `to` konumuna koy.
6. Yuvaya sabit bir yükseklik ver, örneğin yan panelin içinde `.fill()`.

## Nasıl çalışır

- **Açık bileşenler yüksekliği paylaşır.** Payından azına ihtiyacı olan bileşen doğal yüksekliğini korur; listeler gibi uzun olanlar kalanı eşit böler ve içlerinde kayar.
- **Akordeonla aynı bölümler.** Başlık satırları, hover ve odak, oklar, ikonlar ve ayrıntılar aynı görünür ve aynı davranır; yalnızca yükseklik kuralı farklıdır.
- **Bir başlığı sürüklemek** bileşeni kaldırır: başlığı vurgu çubuğuyla katman tonunda bir hayalet olarak imleci izler, diğer bileşenler aralarını kapatır, renkli bir satır bırakılacak yeri gösterir. Bırakınca tek bir taşıma mesajı gider. Kıpırdamayan bir tıklama ise açıp kapatır.
- **Klavye:** `ctrl shift ↑↓` odaktaki bileşeni bir sıra taşır.
- **Açılıp kapanma** yuvanın sabit yüksekliği içinde hareketlidir; hareket azaltılmışsa bir anda olur.

## Sık yapılan hatalar

- **Sırayı bileşenin tutacağını sanmak.** Yuvanın kendi sırası yoktur; taşıma mesajını uygulamazsan hiçbir şey yer değiştirmez.
- **Taşımadan sonra konumla kimliği karıştırmak.** Mesajlar gösterim konumunu taşır; kimliğe sıra listesi üzerinden çevir.
- **Yükseklik vermemek.** Yalnızca içeriğini ölçen bir sütunda yuva paylaştıracak bir şey bulamaz; `.fill()` ya da hücre sayısı ver.
