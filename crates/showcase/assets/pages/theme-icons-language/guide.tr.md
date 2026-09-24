## Üç dosya, sıfır kod

Bir uygulama quvyta-framework'e **tema dosyaları**, **ikon dosyaları** ve **dil dosyaları** verir. Framework her birinin eksiksiz bir varsayılanıyla gelir; uygulama bunlar olmadan da çalışır, senin dosyaların varsayılanları ezer ya da genişletir. Düz dosya oldukları için yeni tema ya da dil eklemek kopyala, adlandır, düzenle demektir.

## Metin olarak verilen dosyalar

İçeri girmenin tek yolu yol vermek değil. `Runtime::theme_source(dosya, metin)`, `icon_source`, `keymap_source` ve `locale_source` doğrudan TOML metnini alır; bu metin genelde deponuzdaki bir dosyanın `include_str!`'ıdır. Kurulan ikili böylece kendi görünümünü ve tuşlarını taşır ve yanında hiç dosya olmadan açılır; `CARGO_MANIFEST_DIR`'den kurulan bir yol bunu yapamaz. Dosya adı yalnızca tanılamaları etiketler; tema ve ikon setlerinde ise adın kökü, klasördeki gibi, id olur.

Metin ve yol bir arada yaşar: metin en son yüklenir, yani kazanır, ve ayrıca verilen yol artık zorunlu olmaz. O yol okunamadığında metin onun yerine geçer ve sebep, programı durdurmak yerine bir tanılamaya dönüşür. Yerine geçecek bir metin yoksa okunamayan yol yine hatadır; çünkü o zaman dosyanın yerini tutacak bir şey yoktur.

## Temalar

Bir temanın iki katmanı vardır.

1. `[colors]` içindeki **renk değişkenleri**: en alttan en üste zeminler için `canvas`, `surface`, `raised`, `active`, `overlay`; tek vurgu rengi ve nefes eşi için `accent` ve `accent-2`; yazı için `text`, `dim`, `muted`; vurgu üstündeki yazı için `ink`; anlam için `success`, `warning`, `danger`, `info`.
2. `[style."bileşen.varyant:durum"]` içindeki **stil kuralları**: her bileşenin her durumda nasıl göründüğü, değişkenlerle yazılır.

Tema yalnızca farklı olanı yazar. `extends = "monochrome"` geri kalan her şeyi devralır; yeni bir tema on satırlık renkten ibaret olabilir.

```toml
[meta]
name = "Aurora"
extends = "monochrome"

[colors]
accent = "#7DD3FC"
```

## Stil kuralları nasıl seçilir

- Seçici dilbilgisi `bileşen.varyant:durum:durum` şeklindedir. Bileşen ve varyant adları küçük harf, rakam ve tire kullanır.
- Daha özel kural kazanır: `button` < `button.primary` < `button:hover` < `button.primary:hover`.
- Eşit özellikte sonra gelen kural kazanır; devralınan temanın kuralları önce gelmiş sayılır.
- Değerler `$değişken`, `#RRGGBB`, `mix(a, b, 30%)` ya da iki renk arasında nefes alan `pulse(a, b)` olabilir.

## Okunabilirlik denetlenir

Tema yüklenirken framework yazı ile zeminler arasındaki kontrastı ve vurgu ile dört durum rengi arasındaki mesafeyi ölçer. Uyarı rengi vurguya benziyorsa ya da yazı zor okunuyorsa sayılarıyla birlikte bir uyarı üretilir. Bozuk değerler uygulamayı çökertmez: girdi atlanır, dosya, satır ve sütun bildirilir. Demodaki durum satırı yüklenen dosyalarda sorun olup olmadığını gösterir.

## Az renkli terminaller

Framework terminale kaç renk gösterdiğini sorar (`ColorDepth::detect`) ve temanın renklerini 24 bit, 256 renklik palet ya da on altı standart renkle çizer. Tema aynı dosyadır; değişen yalnızca çizimdir.

On altı standart rengin iki koyu grisi var, siyah ve parlak siyah; koyu bir tema ise biçimini birkaç adım arayla dört beş zemin tonundan kurar. En yakın renge yuvarlansalar hepsi siyaha iner, sekme şeridi, yükseltilmiş panel ya da diyalog ekrana karışırdı. Bu yüzden on altı renkli bir kare önce tam renkle çizilir, bitince iki kuralla indirilir:

- **Gözün ayırdığı zemin ayrı kalır.** `canvas`'tan OKLab'de en az 0.05 uzak olup yine de canvas'ın rengine düşecek bir ton, ondan uzaklaşan yöndeki bir sonraki griye geçer. Koyu temada `canvas` ve `surface` siyah kalır, `raised`, `active` ve `overlay` parlak siyah olur; açık temada parlak beyazdan beyaza iner.
- **Yazı zeminine gömülmez.** Arkasına karşı 1.6:1'in altında kalacak yazı, 3:1'i tutan en sessiz griyi alır: parlak siyah bir yüzeydeki soluk ya da vurgu renkli yazı beyaz olur, açık bir diyaloğun arkasındaki sayfa kaybolmaz, siyah üstünde parlak siyah olarak soluk kalır.

256 renkli bir kare de önce tam renkle çizilir, bitince indirilir; böylece diyaloğun arkasındaki sayfanın karartılması, bir menünün panelden ayrılması ve sayfa geçişi gerçek renkteki gibi karışır. Bu palette her zemine yetecek kadar gri var, bu yüzden zeminler en yakın girdiyi alır. Yazı en yakın girdisini korur; o girdi arkasına karşı 1.6:1'in altına düşüyorsa kendi rengine en yakın ve 1.6:1'i tutan girdiyi alır. Böylece diyaloğun arkasındaki sayfanın en soluk yazıları kaybolmaz, soluk kalır.

`Rgb::to_ansi16_on`, `Rgb::to_ansi16_text`, `Rgb::to_ansi256` ve `Rgb::to_ansi256_text` karenin verdiği cevabın aynısını verir, böylece bir test ekranı daha az renkte çizmeden denetleyebilir; `Harness::set_depth(ColorDepth::Ansi16)` ya da `ColorDepth::Ansi256` ise öyle çizer.

## İkonlar

Her ikonun üç biçimi vardır: Nerd Font için `nerd`, `unicode` ve `ascii`. `auto` modunda framework terminale ve kurulu yazı tiplerine bakarak seçer; kullanıcı bir mod seçebilir, `QUVYTA_ICONS=ascii` ise bir modu zorlar. ASCII biçimleri şekil taklit etmek için asla parantez kullanmaz.

Bazı adlar tek bir bileşenin işareti değil, ortak bir anlamdır; ekosistemdeki her uygulama onlar için aynı şekli çizer: ana menü için `project`, `profile`, `settings`, `power`; işin durduğu yer için `workspace` ve tuşların ne yaptığını açan düğme için `help`; pencere başlığı için `window-minimize`, `window-maximize`, `window-restore`; başlatıcının girdileri grupladığı program türleri için `category-system`, `category-development`, `category-network`, `category-office`, `category-media`, `category-files`; Quvyta'nın kendi `❖` işareti için `ecosystem`; ve bir durum şeridinin makineden gösterdikleri için `cpu`, `memory`, `battery-full`, `battery-half`, `battery-empty`, `battery-charging`, `terminal`, `session` (bir terminal çoklayıcısının oturumu), `network-down` ve `network-up`. Bunların Unicode biçimi, terminallerin ve eşit aralıklı yazı tiplerinin tek hücre olarak çizdiği Geometric Shapes içinde kalır; böylece dock'taki bir sıra ya da üç hücrelik bir başlık işareti, başka bir yazı tipine düşen bir glifle aralanmaz. `ecosystem` tek istisnadır, çünkü o işaretin kendisidir. İkonu olmayan bir başlatıcı girdisi kategorisinin ikonunu kullanır.

Set, bir uygulamanın ana menüsünün gösterdiği anlamlara da karşılık verir; böylece ekosistemdeki her uygulama aynı şekilleri çizer: `project`, `profile`, `settings` ve `power`, yanlarında `workspace` ve `help`. Proje üzerinde çalışılan şey, çalışma alanı ise onu tutan düzenli yerdir; bu yüzden tek şeklin iki adı değil, iki ayrı anahtardırlar. ASCII'de ikisi de `#` çizer, orada satırın kendi sözcükleri onları ayırır. `help` üç modda da soru işareti çizer, çünkü anlam ekosistemin okunduğu her yerde böyle okunur. Nerd Font şeyin kendisini çizer; Unicode sütunu, terminallerin ve eşit aralıklı yazı tiplerinin tek hücre olarak çizdiği Geometric Shapes içinde kalır, böylece hiçbir satır başka bir yazı tipine düşüp hücreden taşmaz.

## Uygulamanın kendi ikonları

Uygulama kendi ikonlarını bir ikon setini verdiği gibi verir: `Runtime::icon_source("app.toml", include_str!("../assets/icons/app.toml"))`. Gömülü sette olmayan anahtarlar, örneğin `category.internet` ya da `source.aur`, bundan sonra ikon anahtarı alan her bileşende ve `env.icons().glyph(..)` ile çizilir; tema hangi seti seçerse seçsin. Glif kipi değişince bu ikonlar da kendi Nerd, Unicode ya da ASCII sütununa geçer. Demodaki “uygulama ikonu” satırı böyle bir anahtardır ve varsayılan temada çizilir.

- **Uygulamanın anahtarları seçili setin altında durur.** Bir temanın seti ya da temanın tek bir `[icons]` girdisi `category.internet`'i yine de yeniden biçimlendirebilir; kullanıcı her şeyi değiştirebildiği gibi bunu da değiştirir.
- **Framework'ün anahtarları temanındır.** Gömülü sette zaten olan bir anahtar, örneğin `check`, her bileşenin çizdiği bir ikonun yeniden biçimidir; yalnızca bir tema setinizi seçtiğinde (`[meta] icon-set`) geçerli olur. Bir uygulama seti, kullanıcının seçtiği setin ikonlarını hiçbir zaman değiştirmez. Kendi anlamlarınıza kendi adlarını verin, tercihen bir önekle.
- **İki setiniz aynı anahtarı verirse** sonra eklenen kazanır; sonra verilen metnin klasörü ezmesi gibi.
- **Eksik sütun raporlanır, programı durdurmaz.** `nerd` yoksa Unicode glifi yerine geçer, çünkü Nerd Font Unicode'u da çizer; `unicode` yoksa ASCII glifi geçer. İkisi de dosya, satır ve sütunla verilen birer uyarıdır. `ascii` yoksa yerine geçecek daha sade bir şey olmadığından ikon bir hatayla atlanır ve ekranda onun yerinde `⟦anahtar⟧` görünür.

## Diller

Dil dosyaları bölümler halinde gruplanmış anahtarları tutar. `t!("files.count", n = 3)` anahtarı önce etkin dilde, sonra onun `fallback` dilinde, en son İngilizcede arar. Çoğul biçimleri `{ one = "{n} file", other = "{n} files" }` gibi tablolardır; biçimi dilin kendi kuralı seçer, böylece Türkçe, Rusça ya da Arapça kendi biçimini alır. Eksik anahtar fark edilsin diye `⟦files.count⟧` olarak görünür; `missing_keys` ile bir test her dili eksiksiz tutar.

## Bir anahtarın çevrildiğini denetlemek

`i18n.has("tr", "files.count")` bir dilin anahtarı kendi dosyalarında taşıyıp taşımadığını sorar. Dili her zaman adıyla alır, bu yüzden etkin dil cevabı değiştirmez; yedek dillere de düşmez: Türkçenin İngilizceden ödünç alacağı bir anahtar, ekranda İngilizce metin görünse bile `"tr"` için `false` olur. Çoğul anahtar var sayılır. Uygulamanın kullandığı anahtarları sayan bir test, böylece her birini her dilde isteyebilir:

```rust
for key in ["app.save", "app.files"] {
    assert!(i18n.has("en", key) && i18n.has("tr", key), "{key} çevrilmemiş");
}
```

`translate(key)` sonucunu anahtarın kendisiyle karşılaştırmak bu işi görmez: eksik anahtar `⟦key⟧` olarak çevrilir, bu da anahtarın kendisi değildir; böyle bir test çeviri eksikken de geçer.

## Bir dilin âdetleri

Bir dil sözcüklerinden ibaret değildir. Bir sayının ondalıklarını nereye koyduğu, tarihi hangi sırayla yazdığı, sayının yanındaki zaman biriminin ne kadar kısa olduğu: bunlardan biri yanlışsa, bütün sözcükler doğru olsa bile ekran yanlış okunur.

- **Ondalıklar.** `quvyta.number.decimal`, bir dilin sayının tam kısmıyla ondalıkları arasına ne yazdığını söyler: İngilizce, Japonca ve Çincede nokta; Almanca, İspanyolca, Fransızca, Portekizce, Rusça ve Türkçede virgül. Framework'ün çizdiği her sayı buradan geçer: grafik etiketleri, kaydırıcının değeri, alandaki sayı, dosya boyutu. Kendi sayılarını `qframe::i18n::number(değer, basamak)` ile yaz; o zaman tek bir ekran iki yazımı karıştırmaz.
- **Sayı alanı yazdığını okur.** `NumberInput`, dilin ayracını yazıldığında kabul eder; noktayı da kabul eder, çünkü sayı tuş takımında dil ne olursa olsun nokta vardır. Alan ekrandayken dil değişirse kendini yeniden yazar.
- **Tarihler.** `Date::written()` bir tarih alanının gösterdiği uzun biçimdir (`September 18, 2026`, `18 Eylül 2026`, `2026年9月18日`), `Date::written_short()` kısa aylısı, `Date::day_and_month()` yılsız gün ve ay (başlıklar için), `Date::day_and_month_short()` en dar olanı. Günle ayı kendin yan yana koymak yerine bunları kullan: bir başlığın `18 Eylül`, alanın `Eylül 18` demesi tam olarak böyle olur.
- **Sayının yanındaki birim.** Bir dil, birimi tek başına dururkenkinden daha kısa yazar: Çincede sayının ardından `分` gelir, sözcüğün kendisi ise `分钟`'dır. `quvyta.duration.*` ve `quvyta.time.*` anahtarları kısa biçimi taşır; biri bir süre yazdığında **okunan** sözcükler ikisini de tutar.

## Çalışırken değiştirmek

`Command::set_theme`, `Command::set_locale` ve `Command::set_icon_mode` her şeyi yeniden başlatmadan bir anda değiştirir. Ayarlar ekranının listeleri `env.themes()` ve `env.i18n().list()`'ten gelir. Seçimi kaydetmek uygulamanın işidir.

Görünüm etkin dili `ui.env().i18n().active()` ile okur. `Env` olmayan `update`, `init` ve öteki `App` metotlarında `qframe::i18n::active_code()` aynı kodu verir (`tr`, `pt-BR`), bir `Command::set_locale`'in hemen ardından da: dil dosyası olmayan bir kaynaktan metin seçmek için kullan, örneğin bir masaüstü veritabanının birçok dilde taşıdığı açıklamalar.

## Sık yapılan hatalar

- **Bileşenlerde renk sabitlemek.** Değişken kullan; koddaki bir hex değeri temayı takip etmez.
- **Durum için vurgu rengini kullanmak.** Anlamı `success`, `warning`, `danger`, `info` içinde tut.
- **Tek sinyal olarak renk.** Durum renklerini bir ikon ya da kelimeyle birlikte kullan.
