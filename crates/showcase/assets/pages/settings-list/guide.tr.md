## Ne zaman kullanılır

Tek tek ve hemen uygulanan tercihler için ayar listesi kullan: tema, animasyonlar, container motoru. Her satır bir etiket ve bir kontroldür. Birkaç değer bir şey olmadan önce birlikte denetleniyorsa form kullan.

## Adım adım

1. Her değeri durumunda tut: `animations: bool`, `engine: usize`.
2. Listeyi kur: `SettingsList::show(ui, |list| { … })`.
3. Satırları başlıklar altında topla: `list.heading(t!("appearance"))`.
4. Her ayar için tam olarak bir kontrollü satır ekle: `list.row(SettingRow::new(t!("animations")), |ui| { ui.add(Switch::new(state.animations).on_toggle(Msg::Animations)); })`.
5. Yetenekleri işe yaradığı yerde aç: silik ikinci satır için `.description(…)`, kilitli ayar için `.disabled(true)` (kontrolü de pasif yap), bir şey açan satır için `.on_activate(mesaj)`; örneğin `2.4 GB` gösteren bir değer satırı.

## Nasıl çalışır

- **Satırlar dokunulana kadar sadedir.** Fare altındaki satır yüzeyini yumuşak bir çubukla yükseltir; klavyenin satırı nefes alan çubukla daha da yükselir. Odaktaki liste hiçbir zaman iki satırı birden yükseltmez: fareyi bir satıra götürmek onu klavyenin satırı yapar.
- **Yalnızca etiket kayar.** Hover ve seçimde etiket ve açıklama bir hücre sağa geçer; çubuk ve kontrol olduğu yerde kalır. Etiket sütunu bunun için bir hücre pay bırakır, uzun etiketleri `…` ile keser.
- **Liste tek bir kontrol gibi odak alır.** Tab listeye bir kez gelir, içindeki her anahtara değil. Yukarı ve Aşağı pasif satırları atlayarak etkin satırlar arasında gezer; diğer her tuş seçili satırın kontrolüne gider: Boşluk anahtarı değiştirir, Enter açılır listeyi açar, Sağ ve Sol segment seçicinin seçimini değiştirir.
- **Kullanılmayan tuşlar satırı çalıştırır.** Kontrol Enter ya da Boşluk'u kullanmazsa ve satırın `on_activate` mesajı varsa o mesaj gönderilir.
- **Fare doğrudan kontrollere gider.** Anahtara tıklamak onu değiştirir; etikete tıklamak satırı seçer, çalıştırılabiliyorsa çalıştırır.
- **Değerler uygulamanın, klavyenin satırı motorundur.**

## Sık yapılan hatalar

- **Kaydet'i bekleyen ayarlar.** Ayar listesi hemen uygular; bir grup değer için form kullan.
- **Bir satırda iki kontrol.** Tuşlar tek kontrole ulaşır; ayarı ikiye böl.
- **Belge gibi açıklamalar.** Tek kısa satır; uzun yardım rehbere aittir.
- **Pasif satırın kontrolünü pasif yapmamak.** Satır soluklaşır ve atlanır, ama kontrol kendi durumunu korur.
