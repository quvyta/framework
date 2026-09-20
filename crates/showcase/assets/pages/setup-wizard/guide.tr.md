## Ne zaman kullanılır

Ailenin bir uygulaması ilk kez açıldığında. Uygulamanın kendi ayar dosyası yokken açılır, her Quvyta uygulamasının paylaştığı üç şeyi sorar, uygulamanın kendi sorularını sormasına izin verir ve sonunda iki dosyayı bir kerede yazar.

- **Her uygulamaya bir sihirbaz değil, tek sihirbaz.** Görünüm adımı framework'ün: her yerde aynı satırlar, aynı kutular, dokuz dilde aynı sözler. Uygulama yalnızca kendine ait olanı ekler.
- **İlk adım her zaman gösterilir.** Aile bir dili, bir temayı ve ikonları zaten paylaşıyorsa da adım gelir: ortak dosyadan dolu ve her kutusu işaretli, böylece tek bir İleri hepsini kabul eder.
- **Ya hep ya hiç.** Sihirbaz yarıda kapatılırsa hiçbir şey yazılmaz, ortak dosya bile; bir sonraki açılışta yine gelir. "Varsayılanlarla başla" ise doldurulanı koruyarak çıkma yolu.

## Adım adım

1. Durumu tut: `Setup::new(Family::QUVYTA, "code", &i18n, Msg::Setup).on_finish(Msg::Ready)`. Eski bir ayar dosyasını ondan önce `Family::adopt` ile taşı; taşınan uygulamaya sorulmaz.
2. Açılışta ayarları da yazmadan çöz: `family.preferences_without_saving("code", &i18n)` değerleri `Runtime::preferences`'a verir ve klasöre dokunmaz. `Setup` kendi satırları için bunu zaten yapar.
3. İstendiği sürece çiz: `if self.setup.needed() { SetupWizard::new(&self.setup).step(başlık, sayfa).show(ui) }`, değilse uygulamanın kendi ekranı.
4. Uygulamanın kendi adımlarını `step(başlık, |ui| …)` ile, her adım için bir kez, sırayla ekle. İçerikleri ve mesajları uygulamanın kendisidir; framework onlara hiç bakmaz.
5. Her `Msg::Setup(mesaj)`'a `self.setup.update(mesaj, &mut self.settings)` ile cevap ver. Başka bir şey gerekmez: Geri, İleri, satırlar, yazı tipi kurulumu ve Bitir hepsi içindedir.
6. Uygulamanın kendi anahtarlarını `on_finish` mesajında yaz: sihirbaz üç ortak anahtarı yazmış ve dosyayı oluşturmuştur, `settings` de onları tutar; `settings.set(..)` ve `settings.save()` onları korur.
7. Adımlara aynı yüksekliği `page_height(satır)` ile ver, böylece butonlar yer değiştirmez; sihirbaz kapatılabilir olacaksa `on_cancel(Msg::Quit)` ekle — kapatmak hiçbir şey yazmaz.

## Nasıl çalışır

- **`Wizard`'ın üstünde kurulu.** Üstteki adımlar, tek sayfa, Geri, İleri ve Bitir başka her akışın kullandığı bileşenin kendisi; kurulum sihirbazı yalnızca ilk sayfayı doldurur ve butonların ne demek olduğunu söyler.
- **Görünüm adımı `Appearance::rows`.** Ayar sayfasının gösterdiği aynı üç satır, her birinin "Tüm Quvyta uygulamalarında" kutusuyla, `without_saving()` bir `Appearance` üstünde: değişiklik hemen uygulanır, yani tema seçmek sihirbazı o temayla yeniden çizer, gideceği dosya ise bekler.
- **İkonları göz seçer.** Satırların altında aynı dört ikon, uygulamanın hangi kiple çizdiğine bakılmadan, üç glif kipinde birden çizilir. Makinede Nerd Font yoksa adım simgeleri kurmayı önerir, nereye geldiğini gösterir ve sonrasında glifler hâlâ kutucuksa ne yapılacağını söyler.
- **Bitir üç anahtar yazar.** Her ortak anahtar `Family::set`'ten geçer: kutu işaretliyken ailenin dosyasına ve uygulamanın dosyasına ailenin kimliği, kutu boşken yalnızca uygulamanın dosyasına. Burada tutulan bir değer ortak dosyaya hiç gitmez. Uygulamanın dosyasını oluşturmak sihirbazı temelli bitiren şeydir.
- **Yazma başarısızsa sihirbaz kalır.** Sebep ilk adımda görünür, uygulamaya bittiği söylenmez ve hiçbir şey yarım yazılmaz.

## Sık yapılan hatalar

- **Açılışta `Family::preferences` çağırmak.** Ortak dosyayı oluşturur; yarıda kapatılan bir sihirbaz arkasında bir şey bırakmış olurdu. Sihirbazı olan uygulamada `preferences_without_saving` kullan.
- **Uygulamanın kendi anahtarlarını `on_finish`'ten önce yazmak.** Yarım kalan sihirbazın bırakmaması gereken bir dosyada görünürler.
- **Dili, temayı ya da ikonları kendi adımında sormak.** Onlar ailenin; ilk adım onları her uygulama için bir kerede zaten sorar.
- **Testi ya da denemeyi gerçek klasörlere yöneltmek.** `Setup::new_in(klasör, ..)`, `install(..)` ve `font_dirs(..)` testi `~/.config/quvyta`'dan ve kullanıcının yazı tiplerinden uzak tutar.
