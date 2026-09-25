## Ne zaman kullanılır

Bir Quvyta uygulaması ilk kez açıldığında. Uygulamanın kendi ayar dosyası yokken açılır, her Quvyta uygulamasının paylaştığı üç şeyi sorar, uygulamanın kendi sorularını sormasına izin verir ve sonunda iki dosyayı bir kerede yazar.

- **Her uygulamaya bir sihirbaz değil, tek sihirbaz.** Görünüm adımı framework'ün: her yerde aynı satırlar, aynı kutular, dokuz dilde aynı sözler. Uygulama yalnızca kendine ait olanı ekler.
- **Bütün ekosistem için bir kez sorulur.** Ortak dosya bir dili, bir temayı ve ikonları zaten tutuyorsa onları başka bir üye sormuştur: görünüm adımı atlanır, sihirbaz uygulamanın kendi ilk adımıyla açılır ve Bitir uygulamayı ortak değerleri izler hale getirir. Kendi adımı olmayan bir uygulamaya o zaman hiç sihirbaz gerekmez. Ortak dosya eksikse adım ilk sırada gelir, ne varsa onunla dolu ve her kutusu işaretli.
- **Ya hep ya hiç.** Sihirbaz yarıda kapatılırsa hiçbir şey yazılmaz, ortak dosya bile; bir sonraki açılışta yine gelir. "Varsayılanlarla başla" ise doldurulanı koruyarak çıkma yolu.

## Adım adım

1. Durumu tut: `Setup::new(Ecosystem::QUVYTA, "code", &i18n, Msg::Setup).on_finish(Msg::Ready)`. Eski bir ayar dosyasını ondan önce `Ecosystem::adopt` ile taşı; taşınan uygulamaya sorulmaz.
2. Açılışta ayarları da yazmadan çöz: `ecosystem.preferences_without_saving("code", &i18n)` değerleri `Runtime::preferences`'a verir ve klasöre dokunmaz. `Setup` kendi satırları için bunu zaten yapar.
3. İstendiği sürece çiz: `if self.setup.needed() { SetupWizard::new(&self.setup).step(başlık, sayfa).show(ui) }`, değilse uygulamanın kendi ekranı.
4. Uygulamanın kendi adımlarını `step(başlık, |ui| …)` ile, her adım için bir kez, sırayla ekle. İçerikleri ve mesajları uygulamanın kendisidir; framework onlara hiç bakmaz.
5. Her `Msg::Setup(mesaj)`'a `self.setup.update(mesaj, &mut self.settings)` ile cevap ver. Başka bir şey gerekmez: Geri, İleri, satırlar, yazı tipi kurulumu ve Bitir hepsi içindedir.
6. Uygulamanın kendi anahtarlarını `on_finish` mesajında yaz: sihirbaz üç ortak anahtarı yazmış ve dosyayı oluşturmuştur, `settings` de onları tutar; `settings.set(..)` ve `settings.save()` onları korur.
7. Kendi adımı olmayan bir uygulama `Setup`'a `.appearance_only()` ekler: ortak dosya görünüm adımını cevaplıyorsa uygulamanın dosyası orada, ekosistemi izleyerek yazılır ve `needed()` yanlış döner.
8. Adımlara aynı yüksekliği `page_height(satır)` ile ver, böylece butonlar yer değiştirmez; sihirbaz kapatılabilir olacaksa `on_cancel(Msg::Quit)` ekle — kapatmak hiçbir şey yazmaz.

## Nasıl çalışır

- **`Wizard`'ın üstünde kurulu.** Üstteki adımlar, tek sayfa, Geri, İleri ve Bitir başka her akışın kullandığı bileşenin kendisi; kurulum sihirbazı yalnızca ilk sayfayı doldurur ve butonların ne demek olduğunu söyler.
- **Görünüm adımı `Appearance::rows`.** Ayar sayfasının gösterdiği aynı üç satır, her birinin "Tüm Quvyta uygulamalarında" kutusuyla, `without_saving()` bir `Appearance` üstünde: değişiklik hemen uygulanır, yani tema seçmek sihirbazı o temayla yeniden çizer, gideceği dosya ise bekler.
- **İkonları göz seçer.** Satırların altında aynı dört ikon, uygulamanın hangi kiple çizdiğine bakılmadan, üç glif kipinde birden çizilir. Makinede Nerd Font yoksa adım simgeleri kurmayı önerir, nereye geldiğini gösterir ve sonrasında glifler hâlâ kutucuksa ne yapılacağını söyler.
- **Bitir üç anahtar yazar.** Her ortak anahtar `Ecosystem::set`'ten geçer: kutu işaretliyken ekosistemin dosyasına ve uygulamanın dosyasına ekosistemin kimliği, kutu boşken yalnızca uygulamanın dosyasına. Burada tutulan bir değer ortak dosyaya hiç gitmez. Uygulamanın dosyasını oluşturmak sihirbazı temelli bitiren şeydir.
- **Görünüm adımı gizlenmez, çıkarılır.** Ortak dosya tamsa üstteki adımlar uygulamanın kendi adımıyla başlar, Geri ondan öteye gitmez ve `step()` görünüm adımını yine 0 sayar; yani uygulamanın ilk adımı her durumda 1'dir. Hangisi olduğunu `asks_appearance()` söyler.
- **Yerleşme işareti kurulum sayılmaz.** Yalnızca `Ecosystem::settle`'ın bıraktığı `shared-checked` anahtarını tutan bir dosya sihirbazı yine açar.
- **Yazma başarısızsa sihirbaz kalır.** Sebep Bitir'e basılan adımda görünür, uygulamaya bittiği söylenmez ve hiçbir şey yarım yazılmaz.

## Sık yapılan hatalar

- **Açılışta `Ecosystem::preferences` çağırmak.** Ortak dosyayı oluşturur; yarıda kapatılan bir sihirbaz arkasında bir şey bırakmış olurdu. Sihirbazı olan uygulamada `preferences_without_saving` kullan.
- **Uygulamanın kendi anahtarlarını `on_finish`'ten önce yazmak.** Yarım kalan sihirbazın bırakmaması gereken bir dosyada görünürler.
- **Dili, temayı ya da ikonları kendi adımında sormak.** Onlar ekosistemin; ilk adım onları her uygulama için bir kerede zaten sorar.
- **Testi ya da denemeyi gerçek klasörlere yöneltmek.** `Setup::new_in(klasör, ..)`, `install(..)` ve `font_dirs(..)` testi `~/.config/quvyta`'dan ve kullanıcının yazı tiplerinden uzak tutar.
