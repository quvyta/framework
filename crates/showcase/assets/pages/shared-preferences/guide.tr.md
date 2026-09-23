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
- **Aileye dönüş, tek başına.** `Family::follow(uygulama, anahtar)` o uygulamanın dosyasına ailenin kimliğini yazar, başka bir şey yapmaz: ortak dosya ne okunur ne yazılır; bir üye, bütün ailenin çizdiği değeri değiştirmeden yeniden izlemeye döner. `Family::set(.., Scope::Family)` ortak değeri de yazar; bu, uygulamanın kendi ayar sayfasında istediğin şeydir, başka bir üyenin satırında değil.
- **Ortak satırın altındaki kutu kapsamdır.** İşaretliyse değişiklik `quvyta.conf`'a gider, uygulamanın dosyası `"quvyta"` der; aileyi izleyen her uygulama onunla değişir. Boşsa değişiklik uygulamanın kendi dosyasında kalır. Kendi değerini seçmiş bir uygulama başka bir uygulamadan asla değişmez.
- **İki yazan da değişikliğini korur.** Her dosya yazılmadan hemen önce okunur ve yalnızca değişen anahtar yazılır; Unix'te klasör bu sırada bir danışma kilidiyle tutulur.
- **Bozuk satır hiçbir şeyi durdurmaz.** O anahtar algılanan değere düşer; nedeni dosya, satır ve sütunuyla `Preferences::diagnostics` içindedir.
- **Hareketi azalt ve vurgu çubuğu uygulamanındır.** Aynı bölümde durur ve uygulamanın dosyasına kaydedilir. `QUVYTA_REDUCED_MOTION` karar veriyorsa satır pasiftir ve bunu söyler.
- **Değerin nereden geldiği.** `Resolved::source` `App`, `Family` ya da `Detected` olur.


## Güncelleme çıkınca haber vermek

Ailenin güncelleme bildirimi bütün uygulamalar için tek bir anahtardır. Güncellemelerini soran bir uygulama onu bölümün hemen ardından `self.appearance.updates(list, Msg::Appearance)` ile gösterir; hiç sormayan onu eklemez. Sormak için framework'ün `updates` özelliğini aç (`quvyta-framework = { version = "…", features = ["updates"] }`) ve açılışta sor:

```rust
fn init(&mut self) -> Command<Msg> {
    let check = UpdateCheck::new(Family::QUVYTA, "code", "quvyta-code", env!("CARGO_PKG_VERSION"), Msg::NewVersion);
    Command::check_for_update(check)
}
// update içinde:
Msg::NewVersion(update) => Command::toast(update.toast()),
```

- **Günde en fazla bir kez, hiç yolunu kesmeden.** Soru kendi iş parçacığında sorulur; ilk kare onu hiç beklemez. En son ne zaman sorulduğu uygulamanın durum klasöründe tutulur.
- **Soramazsa sessiz.** Ağ yoksa, on saniyede cevap gelmezse ya da cevap okunamazsa hiçbir şey gösterilmez; ertesi gün yeniden sorulur.
- **Giden yalnızca ad.** İstek paketin adını söyler; `User-Agent`'ı paketin adı ve sürümüdür. Kimlik, makine bilgisi ya da kullanım verisi gitmez.
- **Bütün aile için kapatılır.** Anahtar kapalıyken (`quvyta.conf` içinde `update-notice = false`) hiçbir şey sorulmaz ve yazılmaz.
- **Yalnızca gerçekten yeni bir sürüm.** Geri çekilen sürümler sayılmaz, kayıttakinden yeni bir derleme hiçbir şey söylemez. Sürümler semver sırasıyla dizilir: `0.1.0-alpha.9` < `0.1.0-alpha.10` < `0.1.0-beta` < `0.1.0`. Kararlı sürümdeki kişiye ön sürüm hiç önerilmez; ön sürümdeki kişi ondan sonraki en yeni sürümü, bir sonraki alfayı ya da kararlı sürümü duyar, böylece kimse eski bir alfada kalmaz.
- **Testler ağa hiç çıkmaz.** Bir harness soruyu kaydeder (`Harness::update_checks`) ve `Harness::set_latest_version(Some("0.2.0"))` ile cevaplar; denetimin klasörlerine dokunmaz.

## Sık yapılan hatalar

- **`language`'ı sabit bir listeyle tanımlayıp kendini onarmayı açmak.** `self_heal`'den önce `Settings::member_of(&Family::QUVYTA)` çağır ya da `load_member` ile yükle; böylece `"quvyta"` korunur.
- **Bütün ayar dosyasını eski bir kopyadan kaydetmek.** Bellekteki ayarlarını `Appearance::update`'e ver; değişikliği onlar da alır.
- **Başka bir uygulamayı izler yapmak için `Scope::Family` kullanmak.** O, `quvyta.conf`'u verdiğin değerle yeniden yazar; her üyeyi listeleyen bir ayar tablosu böylece ailenin temasını bir üyenin satırından değiştirir. Orada `follow` kullan.
- **Testlerde kullanıcının gerçek dosyalarına yazmak.** Geçici bir klasörle `preferences_in`, `set_in` ve `Appearance::in_folder` kullan.
