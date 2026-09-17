## Ne zaman kullanılır

Kullanıcının bir dahaki açılışta aynen bulmayı beklediği seçimler için ayar saklamayı kullan: tema, dil, ikonlar, vurgu çubuğu, seçim kayması, hareketi azaltma ve varsayılan dağıtım bölgesi gibi kendi tercihlerin. Bu bir veritabanı değildir; belgeleri, geçmişi ve önbellekleri kendi dosyalarında tut.

## Adım adım

1. Başlangıçta yükle: `let settings = Settings::load("uygulamam");`. Linux'ta bu `~/.config/uygulamam/settings.toml` dosyasını (ya da `$XDG_CONFIG_HOME` altını) okur.
2. Anahtarlarını bir şemayla tarif et: `Schema::builtin().choice("deploy.region", ["eu-west", "us-east"], "eu-west").flag("deploy.confirm", true)`. `theme` ve `language` anahtarlarını yüklü kimliklerle yeniden tanımla; bunları ancak çalışırken bilirsin. Anlamlı bir varsayılanı olmayan anahtar isteğe bağlıdır: `.optional("deploy.note", SettingKind::text())`. Başka kodun doldurduğu bir tablo, ör. eklentilerin ayarları, açıktır: `.open("plugins")`.
3. Dosyayı şemayla denetle, istersen kendini onarsın: `Settings::load("uygulamam").schema(sema).self_heal(true)`.
4. Kaydedilmiş görünümü daha ilk karede göster: `Runtime::new(Uygulama::new(settings.clone())).settings(&settings).run()`.
5. `view` ya da `update` içinde varsayılanla oku: `settings.get_or("deploy.region", "eu-west".to_owned())`.
6. `update` içinde değiştir ve kaydet: `if settings.set("deploy.region", bolge) { return settings.save_command(Msg::Kaydedildi); }`. Dosya arka plan iş parçacığında yazılır.
7. `settings.diagnostics()` çıktısını sakin bir yerde (ayarlar ekranı, log) göster; kullanıcı elle düzenlediği bir değerin neden yok sayıldığını ya da onarıldığını öğrensin.
8. Testlerde `Settings::in_memory()` ya da `Settings::parse_str` kullan; böylece gerçek ayar klasörüne dokunulmaz.

## Nasıl çalışır

- **Tipli değerler.** Değerler boolean, tam sayı, ondalıklı sayı, metin ve listedir. Kayıtlı tip farklıysa ya da sayı sığmıyorsa `get::<T>` `None` döndürür; yanlış bir değer senin varsayılanına düşer.
- **Noktalı anahtarlar tablodur.** `deploy.region`, `[deploy]` altında `region` olarak yazılır; düz anahtarlar önce gelir.
- **Neyin geçerli olduğunu şema söyler.** Her anahtarın bir türü (`flag`, `text`, listeden `choice` ya da kendi kapanışınla `check`) ve bir varsayılanı vardır. İsteğe bağlı bir anahtarın türü (`SettingKind::flag()`, `text()`, `choice(..)`, `check(..)`) vardır, varsayılanı yoktur. Şema verilmezse yalnızca yerleşik anahtarlar denetlenir: `theme`, `language`, `icons`, `reduced-motion`, `pillar` ve `slide`.
- **Denetim yalnızca uyarır.** Şema varken, bilmediği bir anahtar da kabul etmediği bir değer de dosya, satır ve sütun içeren bir uyarı olur. Dosyaya dokunulmaz.
- **Kendini onarma, anahtar anahtar düzeltir.** `self_heal(true)` ile her anahtar tek başına değerlendirilir: geçerli anahtarlar kalır, bilinmeyenler silinir, geçersiz değerlerin yerine varsayılan yazılır. Anahtarların sırası hiçbir zaman sorun sayılmaz ve olduğu gibi kalır. Bir şey değiştiyse dosyanın eski hali `settings.toml.bak` olarak saklanır ve onarılmış dosya bir kez kaydedilir. Her onarım tanılarda bir uyarıdır; kullanıcı ne olduğunu görebilir. Onarma varsayılan olarak kapalıdır ve senin şemana ihtiyaç duyar: uygulamanın bütün anahtarlarını yalnızca sen bilirsin.
- **Eksik anahtar varsayılanını kullanır ve dosyaya yazılmaz.** Dosyada olmayan bir anahtar onarırken de dosyaya eklenmez: `get_or` senin varsayılanını verir, dosyada yalnızca kullanıcının ya da kodunun sakladığı şeyler durur.
- **İsteğe bağlı anahtar yalnızca geçerliyse kalır.** Geçerli değer kalır; geçersiz değer bir uyarıyla silinir, çünkü yerine yazılacak bir varsayılan yoktur; eksikse eklenmez ve `None` olarak okunur.
- **Açık önekler olduğu gibi kalır.** `.open("plugins")` altındaki her anahtar, yani `[plugins]` ve altındaki her tablo, hiç denetlenmez, bildirilmez ve silinmez. Önekin altında tanımladığın bir anahtar yine kendi kuralına uyar. Tek başına `plugins = …` tablonun altında sayılmaz, her anahtar gibi denetlenir.
- **Bölünmeyen kayıt.** Yeni dosya eskisinin yanına yazılır, diske işlenir ve eskisinin üstüne taşınır; çökme olursa geriye eski ya da yeni dosya kalır, asla yarısı kalmaz.
- **Bozuk dosya uygulamayı durdurmaz.** Söz dizimi hataları ve kullanılamayan girdiler tanılara dönüşür; okunabilen her şey yine kullanılır. Bozuk bir dosyanın üstüne ilk kez yazılmadan önce `settings.toml.bak` olarak kopyası alınır.
- **Ek bağımlılık yok.** Ayar klasörü doğrudan `XDG_CONFIG_HOME`, `HOME` ya da `APPDATA` değişkeninden bulunur; dosyayı framework'ün küçük TOML yazıcısı yazar.

Bu sayfada "Bozuk bir dosya göster", elle bozulmuş bir dosyayı showcase'in şemasıyla yükler; bu şema `deploy.note` ve `deploy.retries` anahtarlarını isteğe bağlı tanımlar ve `plugins` tablosunu açar. Karşılaştırmak için "Kendini onar" anahtarını aç kapa: kapalıyken uyarılar listelenir ve dosya olduğu gibi kalır; açıkken onarımlar listelenir, onarılmış dosya görünür ve her onarım olay günlüğüne yazılır. Onarılmış dosyada not kalır, `retries = "twice"` gider, `[plugins]` olduğu gibi durur; dosyada hiç olmayan `deploy.confirm` da eklenmez. Showcase açılırken kendi ayar dosyasını da böyle onarır.

## Sık yapılan hatalar

- **Eksik bir şemayla onarmak.** Tanımlamayı unuttuğun bir anahtar bilinmeyen sayılır ve onarma onu siler. Onarmayı açmadan önce sakladığın her anahtarı tanımla; yedek eski dosyayı korur ama kullanıcının seçimi uygulamadan gitmiş olur.
- **Varsayılanı olmayan anahtara varsayılan uydurmak.** `""` gibi uydurma bir varsayılan, geçersiz değerin yerine kullanıcının hiç seçmediği bir şey yazar; anahtarı `optional` ile tanımla.
- **Uyarıları susturmak için önek açmak.** Açık tablo hiç onarılmaz, içindeki yazım hatası da kalır. Yalnızca başka kodun sahip olduğu tabloları aç, kendi okuduğun anahtarları tanımla.
- **Yüklü temaları ya da dilleri koda gömmek.** `theme` ve `language` için `choice` listesini ortamın yüklediklerinden kur; yoksa kullanıcının kendi teması "onarılıp" gider.
- **`view` içinde kaydetmek.** `view` diske dokunmamalı; `update` içinden `save_command` ile kaydet.
- **Testlerde gerçek ayar klasörünü kullanmak.** `Settings::load` çağıran testler geliştiricinin kendi ayarlarını okur ve yazar.
- **Anahtar parçasının içine nokta koymak.** Nokta her zaman tabloları ayırır; isimlerin içinde `-` kullan.
- **Çalışırken ayarları asıl kaynak saymak.** Canlı değeri durumunda tut ve değişince sakla; showcase'in üst çubuğu böyle yapar.
