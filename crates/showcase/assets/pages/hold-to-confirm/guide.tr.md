## Ne zaman kullanılır

Kazara olmaması gereken ama pencereyle sormanın can sıkacağı kadar sık yapılan işlerde basılı tutarak onayı kullan: çalışan işler varken çıkmak, bir birimi silmek, önbelleği temizlemek. Onay basılı tutmanın kendisidir; hiçbir şey açılmaz, cevaplanacak bir şey yoktur. Açıklama isteyen seyrek ve ağır kararlar için `Command::confirm` ile sor.

## Adım adım

1. Kontrolü işin olduğu yere ekle: `HoldToConfirm::new(t!("delete-volume"))`.
2. Mesajı ver: `.on_confirm(Msg::DeleteVolume)`. Barlar dolunca bir kez gelir.
3. Bir tuş birleşiminin her yerden çalışması için `.key("ctrl+q")` ekle.
4. Kontrol yalnızca basılı tutulurken görünsün istiyorsan `.floating(true)` ekle; o zaman sol üst köşede küçük bir kart çıkar.
5. Süreyi yalnızca gerekçeyle değiştir: `.duration(Duration::from_millis(2000))`.
6. Barlara işe uyan bir renk vermek için bir tema rengi adı ver: silmek için `.color("$danger")`, yayımlamak için `.color("$success")`. Vermezsen temanın uyarı rengini alırlar.

## Nasıl çalışır

- **Üç bar sırayla, her biri bir bütün olarak dolar.** Basılı tutma süresi üç eşit parçaya bölünür. İlk parçada birinci barın tamamı iz renginden temanın hedef rengine (yerleşik temalarda uyarı rengi) geçer; sonra ikinci bar, sonra üçüncü. Üçüncü bar tam renge ulaşınca eylem olur. Toplam süre senin verdiğin süredir.
- **Renkler temadan gelir.** Barın başladığı renk `track`, vardığı renk `to`. `.color(...)` bu `to` rengini tema dosyasındaki yazımın aynısıyla değiştirir: bir token (`"$danger"`, `"$success"`, `"$accent"`), bir karışım (`"mix($accent, $danger, 50%)"`) ya da sabit bir `"#RRGGBB"`. Hedef tema rengidir, çünkü temayla birlikte değişir; sabit renk de olur ama her temada aynı kalır.
- **Hatalı renk kontrolü bozmaz.** Temanın tek bir renge çeviremediği ifade (yazım hatası, bilinmeyen token, `pulse()`) yok sayılır ve barlar temanın `to` rengine dolar. Neden reddedildiğini `Theme::solid(ifade)` söyler.
- **Bırakınca barlar boşalır.** Süre dolmadan bırakılırsa barlar `motion.enter` süresinde, son dolan bardan başlayarak hızla geri boşalır.
- **Hover ve odakta vurgu çubuğu** buton gibi kontrolün ilk hücresinde çıkar.
- **Bir kez gönderir.** Tamamlandıktan sonra tuş bırakılıp yeniden basılana kadar başka bir şey olmaz.
- **Basılı tuş nasıl anlaşılır.** Kitty klavye protokolünü destekleyen terminaller tekrarları ve bırakmayı bildirir; diğerleri klavyenin tekrar gecikmesinden sonra basışı birkaç on milisaniyede bir yineler. Kontrol, bırakma olayı gelince ya da basıştan sonra 650 ms, son tekrardan sonra 350 ms içinde tekrar gelmezse tuşu bırakılmış sayar.
- **Fare olaysız da çalışır.** Kıpırdamadan basılı tutulan düğme bir şey göndermez; çalışma motoru kontrole sessiz bir tekrar iletir. Kontrolün dışına çıkmak iptal eder.
- **Hareket azaltılmışsa** bir bar, basılı tutmanın kendi üçte biri bitince bir anda tam renge geçer; bırakınca barlar bir anda boşalır. Sürenin kendisi asla kısalmaz.

## Sık yapılan hatalar

- **Çok uzun süreler.** İki saniyeyi aşınca bozuk gibi gelir; kazaları durdurmak için 1,2 sn yeter.
- **Bir işin tek yolunu yüzen bir kısayola saklamak.** İnsan göremediği tuşu keşfedemez; tuşu kısayol çubuğunda ya da işin yanında göster.
- **Seçim için kullanmak.** Basılı tutmak "evet, gerçekten" der; hangisi olduğunu söyleyemez.
- **Renk kodunu sabit yazmak.** `"#E5484D"` bir temada doğru, ötekilerde yanlış durur; `"$danger"` her temaya uyar.
