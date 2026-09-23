## Metotlar

- `Ecosystem::preferences(uygulama, &i18n)` — uygulamanın dilini, temasını ve ikonlarını çözer; ortak dosya yoksa algılanan değerlerle oluşturur.
- `Ecosystem::preferences_in(klasör, uygulama, &i18n)` — aynısı, platformun ekosistem klasörü yerine `klasör`de.
- `Ecosystem::set(uygulama, Shared::Theme, "nordic", Scope::Ecosystem)` — tek bir ortak anahtarı yazar; `Scope::App` yalnızca uygulamanın dosyasına yazar. `set_in(klasör, …)` başka bir klasörde.
- `Ecosystem::follow(uygulama, Shared::Theme)` — uygulamayı ekosistemin değerine döndürür: yalnızca `<uygulama>.conf` `theme = "quvyta"` alır, ortak dosyaya dokunulmaz. `follow_in(klasör, …)` başka bir klasörde.
- `Preferences::language()`, `theme()`, `icons()` — her biri bir `Resolved { value, source }`; tek anahtar için `source(anahtar)`.
- `Preferences::apply()` — çalışan uygulamayı çözülen değerlere geçiren komutlar.
- `Preferences::diagnostics()` — ortak dosyanın yeri gösterilen sorunları.
- `Runtime::preferences(&prefs)` — çözülen değerlerle başlar; `Runtime::settings`'teki aynı anahtarlara üstün gelir.
- `Settings::member_of(&ekosistem)` — ekosistemin kimliği ortak anahtarların geçerli bir değeridir ve "burada ayarlanmadı" diye okunur.
- `Appearance::new(ekosistem, uygulama, prefs)` — satırların durumu; `.in_folder(klasör)` başka yere kaydeder.
- `Appearance::section(liste, mesaj)` — bir başlık ve satırlar; `rows(liste, mesaj)` başlıksız.
- `Appearance::update(değişiklik, &mut ayarlar)` — bir `AppearanceChange`'i kaydeder ve onu gösteren komutu döndürür.
- `SettingRow::nested(true)` — üstteki satıra ait bir satır: metni iki hücre içeriden başlar.
- `Ecosystem::update_notice()`, `update_notice_in(klasör)` — ekosistemin güncelleme çıkınca haber verip vermediği; `quvyta.conf` `update-notice = false` demedikçe açık. `set_update_notice(açık)`, `set_update_notice_in(klasör, açık)` yazar; `Preferences::update_notice()` ötekilerle birlikte okur. `Appearance::updates(list, msg)` onun satırıdır, `section`'ın hemen ardından.
- `UpdateCheck::new(ekosistem, uygulama, paket, sürüm, yeni_sürümde)` — crates.io'ya sorulan soru, `updates` özelliğiyle; test için `.in_folders(ayar, durum)`, bir ayna ya da test sunucusu için `.registry(adres)`. `Command::check_for_update(soru)` sorar. `sürüm` bir ön sürüm olabilir (`0.1.0-alpha.1`): o zaman sonraki ön sürümler de bildirilir; kararlı sürümden yalnızca kararlı sürümler.
- `Update::latest()`, `current()`, `package()`, `toast()` — cevap ve her üyenin aynı biçimde gösterdiği bildirim; `Update::new(ekosistem, paket, sürüm, en_yeni)` elle bir tane kurar.
- `Harness::update_checks()`, `set_latest_version(Some("0.2.0"))` — testin sorudan gördüğü: kaydedilir, verilen sürümle cevaplanır, ağ üzerinden hiç sorulmaz.

## Dosyalar

- Satırın altındaki kutu işaretli: `quvyta.conf` `theme = "nordic"`, `code.conf` `theme = "quvyta"` alır.
- Kutu boş: `quvyta.conf` değişmez, `code.conf` `theme = "nordic"` alır.
- `follow`: `quvyta.conf` değişmez, okunmaz bile; `code.conf` `theme = "quvyta"` alır; olmayan dosya yalnızca o anahtarla oluşur.

## Davranış

- Her anahtar için sıra: uygulamanın değeri (ekosistemin kimliği dışında her şey), ortak dosya, algılama (dil `I18n::detect` ile, tema her zaman `monochrome`, ikonlar `detect_glyph_mode` ile).
- Uygulamanın dosyasında olmayan anahtar ekosistemi izler.
- Ortak dosya kendini izleyemez; içindeki `"quvyta"` bildirilir ve algılanan değer kullanılır.
- Zaten ekosistemi izleyen bir anahtarda `follow` hiçbir şeyi değiştirmez, hata da vermez; okunamayan bir dosyada `InvalidData` ile başarısız olur ve dosyayı olduğu gibi bırakır.
- Kaydedilemeyen bir değişiklik yine uygulanır; satırı bir sonraki değişikliğe kadar nedenini söyler.
- `QUVYTA_REDUCED_MOTION` karar verirken hareketi azalt satırı pasiftir.
