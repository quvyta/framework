## Ne zaman kullanılır

Quvyta ailesinin her uygulaması, kullanıcı birinde başka bir şey seçmediyse aynı dilde konuşur, aynı tema ve aynı ikonlarla çizer. Bu üçünü açılışta `Family::preferences` ile çöz ve ayar sayfana dil, tema, ikon, hareketi azalt ve vurgu çubuğu satırlarını kendin kurmak yerine hazır `Appearance` satırlarını koy.

## Adım adım

1. Açılışta, `Family::adopt` ve `Settings::load_member`'dan sonra ortak tercihleri çöz: `let prefs = Family::QUVYTA.preferences("code", &i18n);`. İlk açılan uygulama `quvyta.conf`'u makinede algılanan değerlerle oluşturur.
2. Çalışma motorunu kendi ayarlarından sonra bunlarla başlat: `Runtime::new(app).settings(&settings).preferences(&prefs)`.
3. Durumunda bir `Appearance` tut: `Appearance::new(Family::QUVYTA, "code", prefs)`.
4. Onu bir ayar listesine yerleştir: `SettingsList::show(ui, |list| self.appearance.section(list, Msg::Appearance))`. Kurulum sihirbazının ilk adımı başlıksız `rows`'u kullanır.
5. Her değişikliği geri ver: `Msg::Appearance(change) => self.appearance.update(change, &mut self.settings)`. Değişikliği kaydeder ve onu hemen gösteren komutu döndürür.

## Nasıl çalışır

- **Her anahtar kendi başına.** Dil, tema ve ikonlar için: uygulamanın dosyası bir değer söylüyorsa o, yoksa `quvyta.conf`, o da yoksa makinenin önerdiği. Eksik anahtar da `"quvyta"` değeri de "aileyi izle" demektir.
- **Ortak satırın altındaki kutu kapsamdır.** İşaretliyse değişiklik `quvyta.conf`'a gider, uygulamanın dosyası `"quvyta"` der; aileyi izleyen her uygulama onunla değişir. Boşsa değişiklik uygulamanın kendi dosyasında kalır. Kendi değerini seçmiş bir uygulama başka bir uygulamadan asla değişmez.
- **İki yazan da değişikliğini korur.** Her dosya yazılmadan hemen önce okunur ve yalnızca değişen anahtar yazılır; Unix'te klasör bu sırada bir danışma kilidiyle tutulur.
- **Bozuk satır hiçbir şeyi durdurmaz.** O anahtar algılanan değere düşer; nedeni dosya, satır ve sütunuyla `Preferences::diagnostics` içindedir.
- **Hareketi azalt ve vurgu çubuğu uygulamanındır.** Aynı bölümde durur ve uygulamanın dosyasına kaydedilir. `QUVYTA_REDUCED_MOTION` karar veriyorsa satır pasiftir ve bunu söyler.
- **Değerin nereden geldiği.** `Resolved::source` `App`, `Family` ya da `Detected` olur.

## Sık yapılan hatalar

- **`language`'ı sabit bir listeyle tanımlayıp kendini onarmayı açmak.** `self_heal`'den önce `Settings::member_of(&Family::QUVYTA)` çağır ya da `load_member` ile yükle; böylece `"quvyta"` korunur.
- **Bütün ayar dosyasını eski bir kopyadan kaydetmek.** Bellekteki ayarlarını `Appearance::update`'e ver; değişikliği onlar da alır.
- **Testlerde kullanıcının gerçek dosyalarına yazmak.** Geçici bir klasörle `preferences_in`, `set_in` ve `Appearance::in_folder` kullan.
