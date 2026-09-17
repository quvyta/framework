## Üç dosya, sıfır kod

Bir uygulama quvyta-framework'e **tema dosyaları**, **ikon dosyaları** ve **dil dosyaları** verir. Framework her birinin eksiksiz bir varsayılanıyla gelir; uygulama bunlar olmadan da çalışır, senin dosyaların varsayılanları ezer ya da genişletir. Düz dosya oldukları için yeni tema ya da dil eklemek kopyala, adlandır, düzenle demektir.

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

## İkonlar

Her ikonun üç biçimi vardır: Nerd Font için `nerd`, `unicode` ve `ascii`. `auto` modunda framework terminale ve kurulu yazı tiplerine bakarak seçer; kullanıcı bir mod seçebilir, `QUVYTA_ICONS=ascii` ise bir modu zorlar. ASCII biçimleri şekil taklit etmek için asla parantez kullanmaz.

## Diller

Dil dosyaları bölümler halinde gruplanmış anahtarları tutar. `t!("files.count", n = 3)` anahtarı önce etkin dilde, sonra onun `fallback` dilinde, en son İngilizcede arar. Çoğul biçimleri `{ one = "{n} file", other = "{n} files" }` gibi tablolardır; biçimi dilin kendi kuralı seçer, böylece Türkçe, Rusça ya da Arapça kendi biçimini alır. Eksik anahtar fark edilsin diye `⟦files.count⟧` olarak görünür; `missing_keys` ile bir test her dili eksiksiz tutar.

## Çalışırken değiştirmek

`Command::set_theme`, `Command::set_locale` ve `Command::set_icon_mode` her şeyi yeniden başlatmadan bir anda değiştirir. Ayarlar ekranının listeleri `env.themes()` ve `env.i18n().list()`'ten gelir. Seçimi kaydetmek uygulamanın işidir.

## Sık yapılan hatalar

- **Bileşenlerde renk sabitlemek.** Değişken kullan; koddaki bir hex değeri temayı takip etmez.
- **Durum için vurgu rengini kullanmak.** Anlamı `success`, `warning`, `danger`, `info` içinde tut.
- **Tek sinyal olarak renk.** Durum renklerini bir ikon ya da kelimeyle birlikte kullan.
