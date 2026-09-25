## Ne zaman kullanılır

Her Quvyta uygulaması, kullanıcı birinde başka bir şey seçmediyse aynı dilde konuşur, aynı tema ve aynı ikonlarla çizer ve kişinin istediği kadar hareket eder. Bu dördünü açılışta `Ecosystem::preferences` ile çöz ve ayar sayfana dil, tema, ikon, hareketi azalt ve vurgu çubuğu satırlarını kendin kurmak yerine hazır `Appearance` satırlarını koy.

## Adım adım

1. `Ecosystem::adopt`'tan sonra çalışma motorunu ekosistemin bir üyesi olarak başlat: `Runtime::new(app).member(Ecosystem::QUVYTA, "code")`. Ayarlarını ve ortak tercihleri yükler, ilk kareden önce uygular ve uygulama çalışırken iki dosyayı da izler. İlk açılan uygulama `quvyta.conf`'u makinede algılanan değerlerle oluşturur.
2. Kendi durumun için de tercihleri çöz, `let prefs = Ecosystem::QUVYTA.preferences("code", &i18n);`, ve `App::preferences`'e yenilerini `Appearance::refresh`'e veren bir mesajla cevap ver. (`.settings(&ayarlar).preferences(&prefs)` ikilisi hâlâ çalışır; `member` ikisini birden yapar ve izler.)
3. Durumunda bir `Appearance` tut: `Appearance::new(Ecosystem::QUVYTA, "code", prefs)`.
4. Onu bir ayar listesine yerleştir: `SettingsList::show(ui, |list| self.appearance.section(list, Msg::Appearance))`. Kurulum sihirbazının ilk adımı başlıksız `rows`'u kullanır.
5. Her değişikliği geri ver: `Msg::Appearance(change) => self.appearance.update(change, &mut self.settings)`. Değişikliği kaydeder ve onu hemen gösteren komutu döndürür.

## Nasıl çalışır

- **Her anahtar kendi başına.** Dil, tema, ikonlar ve hareketi azalt için: uygulamanın dosyası bir değer söylüyorsa o, yoksa `quvyta.conf`, o da yoksa makinenin önerdiği. Eksik anahtar da `"quvyta"` değeri de "ekosistemi izle" demektir.
- **Ekosisteme dönüş, tek başına.** `Ecosystem::follow(uygulama, anahtar)` o uygulamanın dosyasına ekosistemin kimliğini yazar, başka bir şey yapmaz: ortak dosya ne okunur ne yazılır; bir üye, bütün ekosistemin çizdiği değeri değiştirmeden yeniden izlemeye döner. `Ecosystem::set(.., Scope::Ecosystem)` ortak değeri de yazar; bu, uygulamanın kendi ayar sayfasında istediğin şeydir, başka bir üyenin satırında değil.
- **Ortak satırın altındaki kutu kapsamdır.** İşaretliyse değişiklik `quvyta.conf`'a gider, uygulamanın dosyası `"quvyta"` der; ekosistemi izleyen her uygulama onunla değişir. Boşsa değişiklik uygulamanın kendi dosyasında kalır. Kendi değerini seçmiş bir uygulama başka bir uygulamadan asla değişmez.
- **İki yazan da değişikliğini korur.** Her dosya yazılmadan hemen önce okunur ve yalnızca değişen anahtar yazılır; Unix'te klasör bu sırada bir danışma kilidiyle tutulur.
- **Bozuk satır hiçbir şeyi durdurmaz.** O anahtar algılanan değere düşer; nedeni dosya, satır ve sütunuyla `Preferences::diagnostics` içindedir.
- **Hareketi azalt ortaktır, vurgu çubuğu uygulamanındır.** Hareketi azaltmak bir uygulamanın görünüşü değil, kişinin ihtiyacıdır: birinde açan hepsinde ister, bu yüzden dil gibi aynı kutuyu alır. Ortak olmadan önce yazılmış bir ortak dosya onu tutmaz ve hareket var diye okunur. Vurgu çubuğu aynı bölümde durur ve uygulamanın dosyasına kaydedilir. `QUVYTA_REDUCED_MOTION` karar veriyorsa hareketi azalt satırı ve kutusu pasiftir, satır bunu söyler.
- **Değerin nereden geldiği.** `Resolved::source` `App`, `Ecosystem` ya da `Detected` olur.
- **Canlı, çalışırken.** Bir üye ekosistemin klasörünü sistemin kendi olaylarıyla, bir şey değişene kadar uyuyan bir iş parçacığında izler. `quvyta.conf` ya da kendi dosyası değişince tercihler hiçbir şey yazılmadan yeniden çözülür ve yalnızca farklı olan uygulanır: dil, tema, ikonlar ve hareketi azalt; kendi dosyası değiştirdiyse vurgu çubuğu. Kendi temasını seçmiş bir uygulama onu korur. `App::preferences` yeni değerleri duyar; aynı değerlerle yeniden yazılan bir dosya kimseye ulaşmaz, bu yüzden kendi değişikliğini kaydetmek asla döngüye girmez. Klasör izlenemiyorsa uygulama başladığı değerlerle sürer.
- **Eski dosyalara bir kez bakılır.** Bütün üyeler için seçileni yalnızca kendi dosyasına yazmış bir uygulama açılışta, ayarlarını yüklemeden önce bir kez `Ecosystem::settle(uygulama)` çağırır. Dosyasının `quvyta.conf`'takiyle aynı değere sabitlediği her ortak anahtar `"quvyta"`'ya döner; farklı bir değer kalır, çünkü onu kişi seçmiştir. Dosya sonra `shared-checked = true` tutar; böylece ortak değerin sonradan yalnızca bu uygulama için seçilmesi asla geri alınmaz. Olmayan dosya yalnızca bu işaretle oluşturulur.
- **Bütün üyeler tek listede.** `Ecosystem::QUVYTA.members()` her üyenin kimliğini, ayar dosyasının kimliğini (qdesk'te `desktop`, quvyta'da `launcher`), adını, paketini ve komutunu verir; üyeleri listeleyen bir ekran dosya adını hiç tahmin etmez.


## Güncelleme çıkınca haber vermek

Ekosistemin güncelleme bildirimi bütün uygulamalar için tek bir anahtardır. Güncellemelerini soran bir uygulama onu bölümün hemen ardından `self.appearance.updates(list, Msg::Appearance)` ile gösterir; hiç sormayan onu eklemez. Sormak için framework'ün `updates` özelliğini aç (`quvyta-framework = { version = "…", features = ["updates"] }`) ve açılışta sor:

```rust
fn init(&mut self) -> Command<Msg> {
    let check = UpdateCheck::new(Ecosystem::QUVYTA, "code", "quvyta-code", env!("CARGO_PKG_VERSION"), Msg::NewVersion);
    Command::check_for_update(check)
}
// update içinde:
Msg::NewVersion(update) => Command::toast(update.toast()),
```

- **Günde en fazla bir kez, hiç yolunu kesmeden.** Soru kendi iş parçacığında sorulur; ilk kare onu hiç beklemez. En son ne zaman sorulduğu uygulamanın durum klasöründe tutulur.
- **Soramazsa sessiz.** Ağ yoksa, on saniyede cevap gelmezse ya da cevap okunamazsa hiçbir şey gösterilmez; ertesi gün yeniden sorulur.
- **Giden yalnızca ad.** İstek paketin adını söyler; `User-Agent`'ı paketin adı ve sürümüdür. Kimlik, makine bilgisi ya da kullanım verisi gitmez.
- **Bütün ekosistem için kapatılır.** Anahtar kapalıyken (`quvyta.conf` içinde `update-notice = false`) hiçbir şey sorulmaz ve yazılmaz.
- **Yalnızca gerçekten yeni bir sürüm.** Geri çekilen sürümler sayılmaz, kayıttakinden yeni bir derleme hiçbir şey söylemez. Sürümler semver sırasıyla dizilir: `0.1.0-alpha.9` < `0.1.0-alpha.10` < `0.1.0-beta` < `0.1.0`. Kararlı sürümdeki kişiye ön sürüm hiç önerilmez; ön sürümdeki kişi ondan sonraki en yeni sürümü, bir sonraki alfayı ya da kararlı sürümü duyar, böylece kimse eski bir alfada kalmaz.
- **Testler ağa hiç çıkmaz.** Bir harness soruyu kaydeder (`Harness::update_checks`) ve `Harness::set_latest_version(Some("0.2.0"))` ile cevaplar; denetimin klasörlerine dokunmaz.

## Sık yapılan hatalar

- **`language`'ı sabit bir listeyle tanımlayıp kendini onarmayı açmak.** `self_heal`'den önce `Settings::member_of(&Ecosystem::QUVYTA)` çağır ya da `load_member` ile yükle; böylece `"quvyta"` korunur.
- **Bütün ayar dosyasını eski bir kopyadan kaydetmek.** Bellekteki ayarlarını `Appearance::update`'e ver; değişikliği onlar da alır.
- **Başka bir uygulamayı izler yapmak için `Scope::Ecosystem` kullanmak.** O, `quvyta.conf`'u verdiğin değerle yeniden yazar; her üyeyi listeleyen bir ayar tablosu böylece ekosistemin temasını bir üyenin satırından değiştirir. Orada `follow` kullan.
- **Testlerde kullanıcının gerçek dosyalarına yazmak.** Geçici bir klasörle `preferences_in`, `set_in` ve `Appearance::in_folder` kullan; canlı izlemeyi sınamak için `Harness::member_in` ile `poll_preferences`.
