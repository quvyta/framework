# Quvyta Framework — Tasarım

Bu belge framework'ün bugünkü halini anlatır. Katalogdaki bütün öğeler tamamlanmıştır (`done`).

---

## 1. Amaç

**quvyta-framework**, Rust ile terminal tabanlı uygulamalar yazmak için bir **framework**'tür.
Quvyta'nın kendi uygulamaları için başlatıldı ve açık kaynaktır. Hedef yalnızca bir bileşen
kütüphanesi değildir: **bununla eksiksiz ve özenle tasarlanmış terminal uygulamaları
yazılabilmelidir.**

İki parça:

1. **`quvyta-framework`**: kütüphane. Kodda `qframe` adıyla kullanılır.
2. **`showcase`**: her bileşenin canlı demosunu, kodunu, rehberini ve referansını gösteren
   uygulama. Yayımlanmaz (`publish = false`).

Bir uygulama framework'e **üç şey** verir:

- **Dil dosyaları** (bir veya daha fazla)
- **Tema dosyaları** (bir veya daha fazla; ikon seti seçimi dahil) ve **ikon dosyaları**
- **Koddan kurulan bileşenler**

Gömülü bir varsayılan tema (Monochrome), ikon seti ve diller (İngilizce ve Türkçe) vardır;
hiç dosya verilmese de uygulama çalışır.

## 2. Temel kararlar

| Konu | Karar |
|---|---|
| Dil / toolchain | Rust, `rust-toolchain.toml` ile 1.95.0 sabit, edition 2024 |
| Çizim motoru | `ratatui-core` + `ratatui-crossterm`. **`ratatui-widgets` hiç kullanılmaz**; tüm görünüm bizim |
| Uygulama modeli | Elm modeli: `view` + `update` + tipli mesajlar + `Command` |
| Durum | Anlamlı veri uygulamada; hover/odak/imleç/kaydırma/animasyon durumu motorda |
| Tema | TOML; token katmanı + CSS benzeri `bileşen.varyant:durum` stil kuralları; `extends` |
| İkon | TOML; her ikon `nerd` / `unicode` / `ascii` biçimli, otomatik mod algılama |
| Dil | TOML; sistem dili algılama, `fallback`, çoğul biçimleri, çalışırken değişim |
| Kısayollar | TOML keymap; eylem adına bağlanır, ipucu çubuğu ve yardım katmanı oradan dolar |
| Ayarlar | `Settings`: TOML ayar dosyası, şema, isteğe bağlı kendini onarma |
| Varsayılan tema | Monochrome; gömülü diğerleri Iris, Nordic, Amber |
| Showcase dili | İngilizce + Türkçe, çalışırken değiştirilebilir |
| Kalite kapıları | clippy deny, pre-commit fmt + clippy + test + doc |

## 3. Proje yapısı

```
quvyta/framework/
├── Cargo.toml                    workspace ve ortak lint ayarları
├── rust-toolchain.toml, rustfmt.toml
├── README.md                     kısa tanıtım ve başlangıç
├── CATALOG.toml                  tüm bileşen ve sistemlerin tek listesi (bkz. §8)
├── showcase.sh                   showcase'i başlatır
├── .githooks/pre-commit
├── docs/                         tasarım belgesi
└── crates/
    ├── quvyta-framework/
    │   ├── src/
    │   │   ├── lib.rs, prelude.rs
    │   │   ├── runtime/          App, Command, Runtime, Harness, motor, katman ve seçim kuralları,
    │   │   │                     pano, arka plan görevleri, onay penceresi, F12 katmanı
    │   │   ├── widget/           Widget, View, NodeMut, çizim ve olay bağlamları, bileşen hafızası
    │   │   ├── theme/            ayrıştırma, token, seçici çözümleme, boya değerleri, doğrulama
    │   │   ├── icons/  i18n/  keymap/  storage/
    │   │   ├── widgets/          her bileşen kendi dosyasında (büyükler klasörde: table/, tabs/)
    │   │   └── color.rs, date.rs, diagnostics.rs, env.rs, event.rs, geometry.rs,
    │   │       motion.rs, router.rs, style.rs, text.rs
    │   ├── assets/{themes,icons,locales,keymaps}/
    │   └── tests/builtin_assets.rs
    └── showcase/
        ├── src/{main.rs, app.rs, catalog.rs, layers.rs, log.rs, regions.rs, tests.rs, pages/}
        └── assets/{pages/<sayfa>/{guide,reference}.{en,tr}.md, locales/, keymap.toml}
```

Katmanlar yalnızca altlarındakini bilir:

```
showcase / uygulamalar
widgets
widget · theme · icons · i18n · keymap · storage
runtime
ratatui-core + ratatui-crossterm
```

Framework'te tek bir uygulamaya özgü kod bulunmaz. Hiçbir dosya her şeyi yapan dev bir dosyaya
dönüşmez; büyüyen dosya sorumluluklarına göre bölünür.

## 4. Çalışma motoru (runtime)

### 4.1 Uygulama arayüzü

```rust
use qframe::prelude::*;
use qframe::widgets::TextInput;

struct Counter { value: i32, name: String }

#[derive(Clone)]
enum Msg { Increment, NameChanged(String) }

impl App for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.value += 1,
            Msg::NameChanged(name) => self.name = name,
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            ui.add(Text::new(t!("counter.value", n = self.value)));
            ui.add(TextInput::new(&self.name).on_change(Msg::NameChanged));
            ui.add(Button::new(t!("counter.inc")).variant("primary").on_press(Msg::Increment));
        });
    }
}

fn main() -> std::io::Result<()> {
    Runtime::new(Counter { value: 0, name: String::new() })
        .theme_dir("themes")
        .locale_dir("locales")
        .run()
}
```

İsteğe bağlı iki metot vardır: `fn action(&self, name: &str) -> Option<Msg>` kısayol eylemlerini
mesaja çevirir (`[app]` eylemleri ve motorun uygulamaya bıraktığı `help`, `palette` gibi global
eylemler); `fn clipboard(&self, event: &ClipboardEvent) -> Option<Msg>` bileşenlerin ve fare
seçiminin kopyalarını, hiçbir bileşenin almadığı yapıştırmaları duyar. Zamanlayıcı ya da kanal
aboneliği yoktur: süren işler `Command::perform` ve `Command::task` ile yürür.

`Runtime` başlangıçta `.theme_dir`, `.icon_dir`, `.locale_dir`, `.keymap_file`, `.theme(id)` ve
kaydedilmiş görünümü ilk karede uygulayan `.settings(&settings)` alır. `.locale_source(dosya, metin)`
programa `include_str!` ile gömülen dil dosyalarını yükler: `cargo install` yalnızca çalıştırılabilir
dosyayı kurduğu için yayınlanan uygulamalar metinlerini böyle taşır.

### 4.2 Durum ayrımı

| Sahibi | İçerik |
|---|---|
| Uygulama | metin içeriği, işaretli mi, sürgü değeri, seçili sekme, açık katmanlar, router yığını |
| Motor (kimliğe göre) | hover, odak, imleç ve seçim aralığı, geri al geçmişi, kaydırma konumu, animasyon başlangıç anları, açılır listenin açık olması |

Kimlik otomatik üretilir (görünüm ağacındaki konumdan); sırası değişebilen yerlerde
`.id(...)` ile açıkça verilir. Bir karede çizilmeyen kimliklerin motor durumu atılır;
`ui.page(anahtar, ...)` içindekiler sayfa gizliyken de korunur.

### 4.3 Döngü

- Olay kaynakları: tuş, fare, yapıştırma, boyut değişimi, arka plan işi sonucu, pano okuması.
- Olay → mesaj → `update` → değişiklik varsa `view` + çizim.
- **Değişiklik yoksa çizim yok.** Girdi ya da animasyon yokken döngü bekler; boşta işlemci
  neredeyse hiç harcanmaz.
- Animasyon olan bileşen ekrandaysa motor yalnızca bir sonraki karenin zamanı gelince çizer,
  hareket bitince beklemeye döner. Animasyonlar **zamana** bağlıdır, sayaca değil.
- Destekleyen terminalde senkron çıktı (titreme önleme).
- Çizim sırasında dosya/süreç erişimi yapılmaz; bunlar `Command::perform` ya da
  `Command::task` işidir.

### 4.4 Command

`none`, `batch`, `quit`, `focus(id)`, `set_theme`, `set_locale`, `set_icon_mode`,
`set_reduced_motion`, `set_pillar`, `set_slide`, `copy`, `read_clipboard`, `perform` (arka plan
işi, bitince mesaj), `task` ve `cancel_task` (ilerlemesi görünen, iptal edilebilen iş), `confirm`
(motorun çizdiği onay penceresi), `toast`, `dismiss_toast`, `toast_corner`.
Arka plan işleri çizimi asla bloklamaz; yürütücü thread tabanlıdır. Thread açılamazsa
`perform` işi yerinde çalıştırır, uygulama çökmez. Router bir komut değil, uygulama durumunda
tutulan `Router<P>` değeridir.

### 4.5 Klavye

- Tab / Shift+Tab görünüm sırasıyla odak gezdirir; grup kontrolleri (radyo, segment, liste,
  ayar listesi) tek odak durağıdır ve içlerinde ok tuşları çalışır.
- Modal katman açıkken odak içeride kalır, uygulama kısayolları durur; kapanınca odak eski yerine döner.
- Keymap: eylem adı → tuş. `KeyHints` ve yardım katmanı (`?`) buradan dolar; etiketler
  `quvyta.keys.<eylem>` ve `keys.<eylem>` dil anahtarlarından gelir. Keymap dosyayla ezilebilir;
  aynı tabloda aynı tuşa bağlı iki eylem uyarı üretir.
- Global eylemler: `quit` (ctrl+q), `focus-next`/`focus-prev`, `debug` (f12), `copy` (ctrl+c),
  `paste` (ctrl+v), `toggle-panel` (alt+b); `help` (?) ve `palette` (ctrl+p) uygulamaya iletilir.
- Basılı tutulan Enter/Space tekrar basmaz (100 ms içindeki basışlar tek sayılır); destekleyen
  terminalde kitty klavye protokolü (bırakma/tekrar, menü tuşu); onayda parlama.

### 4.6 Fare

- Her karede tıklanabilir alanlar kaydedilir; hover, tıklama, sürükleme, tekerlek, orta tık, sağ tık.
- **Fareyle yapılabilen her şeyin klavye karşılığı vardır; klavyeyle yapılabilen her şey fareyle de
  yapılır** (açılır listelerin, menülerin, paletin ve yardım katmanının kaydırma çubukları
  sürüklenir; tablo başlığındaki oklar tıklanır).
- Tekerlek her yerde aynı adımdır (üç satır). Tekerleği kullanan kontrol (sürgü, sayı ve saat
  alanı) olayı tüketir; çevredeki kaydırma alanı kaymaz.
- Sürükle-bırak (sekme sıralama, bileşen yuvası, bölücü, sürgü, kaydırma çubuğu) motorun ortak
  işaretçi yakalama altyapısını kullanır.

### 4.7 Dayanıklılık

- Panic dahil her çıkışta terminal eski haline döner.
- Boyut değişiminde yeniden yerleşim; her bileşen dar ve çok küçük alanda düzgün küçülür
  (showcase testi her sayfayı çok küçük terminallerde çizer).
- Renk derinliği algılanır (`COLORTERM`, `TERM_PROGRAM`, `TERM`); 24-bit yoksa renkler 256 ya da
  16 renge yaklaştırılır.
- Dosya yükleyiciler panik yapmaz: her sorun dosya:satır:sütun içeren bir `Diagnostic` olur,
  bozuk girdi atlanır, gömülü varsayılanlar her zaman çalışır.

### 4.8 Test sürücüsü

`Harness` gerçek terminal olmadan uygulamayı çalıştırır: `press`, `type_text`, `paste`, `click`,
`click_text`, `drag`, `hover`, `mouse`, `advance` (sahte saat), `screen`, `fg`, `bg`, `is_bold`,
`html`. `Command::perform` harness içinde hemen çalışır, görevler sahte saati izler; sistem
panosu `set_system_clipboard` ile taklit edilir. Testler ekranı tam metin olarak ve renkleri
hücre hücre karşılaştırır.

### 4.9 F12 hata ayıklama katmanı

Framework'e dahil, her uygulamada açılır: tıklanabilir alanlar boyanır, odaklanabilen her
bileşen odak sırasındaki numarasını gösterir, bir panel kare sayısını, çizim süresini, tıklama
alanı sayısını, odak sırasının uzunluğunu ve odaklı bileşeni yazar.

## 5. Tema, ikon, dil, kısayol ve ayar dosyaları

Ortak mantık: gömülü varsayılanlar → geliştiricinin klasörü aynı adı ezer, yeni adı listeye
ekler → çalışırken değiştirilebilir. Ayarlar ekranı için listeler `env.themes()` ve
`env.i18n().list()`'ten gelir. Seçimi kalıcı kaydetmek uygulamanın işidir; bunun için `Settings`
vardır (§5.5).

### 5.1 Tema

```toml
[meta]
name    = "Nordic"
extends = "monochrome"

[colors]
canvas   = "#0B1118"
surface  = "#111A26"
raised   = "#192637"
active   = "#23344B"
overlay  = "#142030"
accent   = "#38BDF8"
accent-2 = "#A5E4FD"
text     = "#F0F9FF"
dim      = "#93C5E0"
muted    = "#4F7A96"
ink      = "#0B1118"
success  = "#7DDC9B"
warning  = "#F4C06A"
danger   = "#F2777A"
info     = "#B4A7F5"

[motion]
pulse-period = "1400ms"
flash        = "90ms"
cursor-blink = "530ms"
step         = "60ms"      # hücre hücre animasyonlarda kare arası
slide        = true        # seçimde kayma
enter        = "140ms"     # katman girişleri
spinner      = "80ms"
shimmer      = "1600ms"
page         = "260ms"     # sayfa geçişleri; yoksa enter × 2
hover-delay  = "450ms"     # ipucu gecikmesi

[typography]
title     = { fg = "$text", bold = true }
body      = { fg = "$text" }
secondary = { fg = "$dim" }
faint     = { fg = "$muted" }

[style.button]
bg      = "$raised"
fg      = "$dim"
bold    = true
padding = [0, 2]

[style."button:focus"]
bg     = "$active"
fg     = "$text"
pillar = "pulse($accent, $accent-2)"
```

- **Seçici dilbilgisi:** `bileşen ( "." varyant )? ( ":" durum )*`. Bileşen ve varyant adları
  `a-z0-9-` karakterlerinden oluşur; alt parçalar tire ile adlandırılır (`list-item`).
- **Seçici önceliği:** önce durum sayısı, sonra varyant: `bileşen` < `bileşen.varyant` <
  `bileşen:durum` < `bileşen.varyant:durum`; eşitlikte dosyada sonra gelen kazanır; `extends`
  ile gelen üst temanın kuralları daha önce sayılır.
- **Değerler:** `$token`, `#hex`, `mix(a, b, yüzde)`, `pulse(a, b)` (iki renk arasında nefes).
- **Durumlar:** `:hover :focus :active :pressed :disabled :selected :checked :invalid`.
- **Özellikler:** `fg bg bold italic underline dim pillar padding gap` + bileşene özgü anahtarlar
  (ör. switch: `track track-on knob knob-on`; slider: `fill fill-cell track knob`). Birkaç anahtar
  sabit bir listeden sözcük alır (`[style.scrollbar] style = "block"`); listede olmayan sözcük
  yeriyle birlikte bildirilir. Her bileşenin anahtarları showcase Referans bölümünde listelenir.
- Varyant adları serbesttir; tema bir varyantı tanımladığı anda `.variant("ad")` çalışır.
- **Doğrulama:** geçersiz değer ya da bozuk giriş → dosya:satır:sütun içeren tanılama, uygulama
  çökmez, varsayılana düşer. **Okunabilirlik denetimi** (WCAG kontrast oranı, OKLab mesafesi):
  `text`/`canvas` ≥ 7 · `text`/`surface` ≥ 7 · `dim`/`surface` ≥ 4.5 · `muted`/`surface` ≥ 2.5 ·
  her durum rengi/`surface` ≥ 4.5 · `ink`/`accent` ≥ 4.5 · `accent` ve dört durum renginin her
  ikilisi arası OKLab mesafesi ≥ 0.10. Altında kalan tema sayılarla uyarı üretir.

**Gömülü temalar:** `monochrome` (varsayılan), `iris`, `nordic`, `amber`. Hepsi monochrome'u
genişletir; stiller monochrome'da yaşar, diğerleri renkleri ve gerektiğinde birkaç tonu ezer.
Durum renkleri her temaya göre ayrı ayarlanır:

| Tema | accent | success | warning | danger | info |
|---|---|---|---|---|---|
| monochrome | `#F5F5F7` | `#6EE7A8` | `#F5C66B` | `#F47174` | `#8AB4F8` |
| iris | `#818CF8` | `#6EDBA0` | `#F3C26B` | `#F47C8A` | `#7DD3FC` |
| nordic | `#38BDF8` | `#7DDC9B` | `#F4C06A` | `#F2777A` | `#B4A7F5` |
| amber | `#F59E0B` | `#A6CF7E` | `#F28B82` | `#E5484D` | `#8FB8DE` |

Gerekçe: hata için kırmızı, başarı için yeşil evrensel olarak okunur; bu yüzden tona
dokunulmaz ama temaya uydurulur (Amber'de sıcak adaçayı yeşili, derin kızıl). Uyarı
vurgu rengiyle çakışıyorsa (Amber) başka tona kaydırılır (somon). Bilgi rengi vurgu
mavi ise (Nordic) lavantaya kayar. Durum rengi hiçbir zaman tek başına anlam taşımaz;
her zaman bir işaret veya metinle gelir (renk körlüğü).

### 5.2 İkonlar

```toml
[icons]
check   = { nerd = "", unicode = "✓", ascii = "v" }
spinner = { nerd = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏", unicode = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏", ascii = "-\\|/" }
pillar  = "thick"   # ▌; "thin" ▎ ya da tek hücrelik herhangi bir karakter
```

Mod: `auto` (terminal, UTF-8 yerel ayarı ve kurulu Nerd Font yazı tipleri algılanır), `nerd`,
`unicode`, `ascii`; `QUVYTA_ICONS` ortam değişkeni modu zorlar. Tema bir ikon seti seçebilir
(`[meta] icon-set`) veya tek ikonları `[icons]` ile ezebilir. Çok kareli ikonlarda (spinner) her
karakter bir karedir. Genişlikler gerçek hücre genişliğiyle ölçülür; tek hücreden geniş `pillar`
tanılama üretir ve varsayılan kullanılır. ASCII glifleri köşeli ayraçla şekil taklit etmez.

### 5.3 Dil

```toml
[meta]
name = "Türkçe"
code = "tr"
fallback = "en"

[counter]
value = "Değer: {n}"

[files]
count = { one = "{n} dosya", other = "{n} dosya" }
```

`t!("counter.value", n = 5)`. Arama sırası: etkin dil, `fallback` zinciri, İngilizce. Sistem dili
algılanır (ortam değişkenleri, sonra işletim sistemi). Framework metinleri `quvyta.*` altında,
gömülü EN + TR. Eksik anahtar `⟦anahtar⟧` olarak görünür; `I18n::missing_keys` testlerin her dili
tamam tutmasını sağlar.

### 5.4 Kısayollar

```toml
[global]
quit         = "ctrl+q"
focus-next   = "tab"
focus-prev   = "shift+tab"
debug        = "f12"
toggle-panel = "alt+b"
copy         = "ctrl+c"
paste        = "ctrl+v"
help         = "?"
palette      = "ctrl+p"

[app]
save = "ctrl+s"
```

Değer tek tuş ya da tuş listesidir. Shift simgelere katlanır (`?`), harflerde yazılır (`shift+s`).

### 5.5 Ayarlar (`Settings`)

- `Settings::load("uygulama")` işletim sisteminin yapılandırma klasöründeki `settings.toml`'u okur
  (`$XDG_CONFIG_HOME` ya da `~/.config`, macOS'ta `~/Library/Application Support`, Windows'ta
  `%APPDATA%`). Bozuk dosya uygulamayı durdurmaz; okunabilen her şey kullanılır.
- Değerler tiplidir (`get::<T>`, `get_or`, `set`); noktalı anahtarlar TOML tablolarıdır.
  Framework'ün okuduğu anahtarlar: `theme`, `language`, `icons`, `reduced-motion`, `pillar`,
  `slide`; `Runtime::settings` bunları ilk karede uygular.
- **Şema:** `Schema::builtin()` ve uygulamanın eklediği `flag`, `text`, `choice`, `check` kuralları.
  `.schema(şema)` yalnızca uyarır: bilinmeyen anahtar ve geçersiz değer yeri gösterilen birer uyarıdır.
- **Kendini onarma** (`.self_heal(true)`, varsayılan kapalı): her anahtar tek başına değerlendirilir;
  bilinmeyen silinir, geçersiz olan varsayılanını alır, sıra hiç değişmez. Bir şey değiştiyse eski
  dosya `settings.toml.bak` olur ve onarılmış dosya bir kez kaydedilir; her onarım bir uyarıdır.
  Onarma uygulamanın tam şemasını ister.
- Kayıt atomiktir (geçici dosya, diske yazma, yeniden adlandırma); `save_command` arka planda kaydeder.

## 6. Estetik anayasası

Estetik bu projenin pazarlık konusu olmayan önceliğidir.

1. **Şekli renk verir, karakter vermez.** Buton, sekme, chip, seçili satır, onay kutusu:
   zemin rengi + iç boşluk + yükselme.
2. **Yasak dekorasyon (her modda, ASCII dahil):** `[ ]` `( )` `< >` `{ }` ile sarmalama;
   `[x]`, `(o)`, `< Tamam >` kalıpları; tam çerçeve kutular (`┌─┐`, `+--+`); `|` ile ayırma;
   `===`, `---`, `-->`, `>>` gibi metin süsleri; ekranı dolduran çizgiler. Kullanıcının kendi
   içeriği muaftır. İstisnalar yalnızca `▌` (vurgu çubuğu) ve `━` (sürgü ve ray anahtarının rayı).
3. **Ayrım boşlukla ve tonla.** Zemin katmanları: canvas → surface → raised → active → overlay.
   Ayırıcı, bölücü ve panel kenarı asla çizgi değildir: ton farkıdır, hover'da bir kademe aydınlanır,
   sürüklerken vurgu rengini alır.
4. **Hiyerarşi renk ve ağırlıkla** (tema `typography`).
5. **Tek vurgu rengi.** Durum renkleri yalnızca anlam için ve her zaman bir işaretle (nokta, ikon, sözcük).
6. **İkon azdır ve amaçlıdır.**
7. **Hareket fısıldar.** Süreler temadan; hareketi azaltma her şeyi anında yapar.
8. **Her durum tasarlanır:** boş, yükleniyor, hata, dar alan, pasif, odaklı, hover.

### 6.1 Somut davranışlar

- **Basit varsayılan, katman katman yetenek.** Bileşen seçeneksiz en sade haliyle çalışır ve
  isteğe bağlı yeteneklerin hiçbir izini taşımaz; her yetenek bağımsız bir seçenekle açılır; yeni
  bileşen ancak yerleşim temelden farklıysa doğar (yatay `Tabs` ile dikey `TabRail` gibi); ortak
  davranış (seçme, kapatma, sıralama, açma kapama) tek bir iç modelde yaşar.

- **Vurgu çubuğu `▌`.** Her yerde tek `pillar` ikonu kullanılır: satırlar, menüler, sekmeler,
  butonlar, kartlar, katmanlar. Tema `[icons] pillar = "thick" | "thin" | "karakter"` ile, uygulama
  `Command::set_pillar` ve `pillar` ayarıyla seçer. ASCII modunda çubuk renkli bir hücredir ve
  temiz kopyaya girmez. Hover'da soluk, seçimde ve klavye odağında `accent` ↔ `accent-2` arasında
  nefes alır. Butonlar, kartlar, sekmeler ve basılabilen diğer kontroller odağı yalnızca odak
  klavyeyle geldiğinde gösterir; tıklanan kontrol farenin altında sakin kalır.

- **Kayma yalnızca liste yapılarında.** Seçimde kayma (`motion.slide` ya da `slide` ayarı açıksa)
  liste, menü, ağaç, tablo (yalnızca ilk hücre), sekmeler, tab rail, ayar satırları (etiket),
  akordeon ve bileşen yuvası başlıkları, açılır liste, bağlam menüsü ve komut paleti satırlarında
  olur. Başka hiçbir bileşen kaymaz.
  - **Çubuk gösterir, kaymaz:** butonlar (hepsi; kısayol bölümü çubuk hücresiyle başlar:
    `▌ ⏎  Kaydet`), yazı alanları, kopyalama alanı, kapalı açılır liste ve tarih seçici alanı,
    basılı tutarak onay kontrolü, yan panel düğmesi.
  - **Segmented:** çubuk kontrolün en solunda değil, üzerine gelinen (klavye odağında seçili)
    bölümün ilk hücresinde çıkar.
  - **Çubuk yok:** Checkbox, Switch, RadioGroup, Slider; kendi renkleri hover ve odağı gösterir.
  - **Satırın yapısı** tek yerde kurulur: sabit işaretler (girinti, çoklu seçim işareti, açma oku
    ya da yüklenme göstergesi), kayan kısım (ikon ve yazı), sabit sağ kısım (detay, rozet, değer,
    kapatma işareti). Yazı sütunu bir hücre pay ayırır; uzun yazı duruşta da kayarken de aynı
    yerden `…` ile kesilir.

- **Tek vurgu.** Klavye imleci ile farenin altındaki satır aynı anda yanmaz. Fare yalnızca
  hareket edince vurguyu taşır; açılış anında satırın üstünde duran fare bir şey değiştirmez;
  klavye farenin bıraktığı yerden devam eder. Açılır liste, açılır menü, tarih seçici, palet,
  menü ve ayar listesi aynı kuralı kullanır.

- **Katmanlar.**
  - **Soldurucu katmanlar** (Modal, `Command::confirm` penceresi, yardım katmanı, komut paleti)
    ekranı soldurur, odağı içeride tutar, dışarıdaki her basışı yutar ve sol kenarları boyunca
    tam boy bir çubuk taşır (tehlikeli pencerede tehlike renginde).
  - **Soldurmayan katmanlar** (açılır liste, tarih seçici takvimi, popover, bağlam menüsü, sekme
    taşma menüsü) dışarıdaki basışta kapanır ve aynı basış düştüğü yere de ulaşır. Katmanı açan
    bileşene basmak yalnızca kapatır, yeniden açmaz.
  - **Kapatılabilir = Esc ve × birlikte.** `dismissable(bool)` ikisini birden açar ya da kapatır;
    biri ötekisiz olmaz. Modal'da `close_on_click_outside` dışarı tıklamayı da ekler.
  - **Kapatma işareti ×** ortak, üç hücrelik bir parçadır: üç hücre fare altında birlikte aydınlanır.
    Soldurucu katmanlarda yüzeyin ilk satırında, sağ üst köşededir; kapatılabilir katman en az bir
    satır üst ve üç hücre sağ iç boşluk tutar, işaret içeriğin üstüne binmez. Bildirimde başlık
    satırının sonunda, sekmelerde sekmenin sağındadır.

- **Kutu kontrolleri.** Checkbox ve RadioGroup varsayılan olarak iki hücrelik düz renk kutusu
  çizer: boş ton ya da vurgu rengiyle dolu; kısmen işaretli kutu yalnızca sol hücresini doldurur.
  Karakter yoktur, ASCII'de de aynı görünür. Değişim `motion.step` × 3 sürede renk karışımıyla olur.
  Diğer görünümler seçenekle açılır: `CheckboxStyle::Check`, `RadioStyle::Dot`. İki kutu bilerek
  aynı görünür; fark anlamdadır.

- **Switch.** Varsayılan kapsül: 5 hücrelik düz yüzey, 2 hücrelik topuz; topuz her `motion.step`'te
  bir hücre ilerler, renkler adımla karışır. Ton merdiveni: kapalı topuz `mix($muted, $raised, 40%)`,
  açık iz `mix($accent, $raised, 55%)`, açık topuz `mix($accent, $raised, 85%)`; hover iki yarıyı da
  bir kademe aydınlatır. Diğer stiller `Rail` ve `Labeled`.

- **Kaydırma çubuğu.** Tek bileşen, temadan seçilen stil: `block` (varsayılan; yalnızca zemin
  renkleri, hiçbir karakter seçime girmez), `half`, `thin`, `dots`. Yalnızca içerik taşınca görünür,
  çerçeve çizmez, temiz kopyaya girmez, her yerde tıklanıp sürüklenebilir. Açılır listede görünüp
  görünmeyeceğine tam açılmış yüksekliğe göre karar verilir.

- **Metin seçimi.** Varsayılan olarak hiçbir yer seçilemez. Seçim bölgesi bileşenin kendisidir
  (`NodeMut::selectable(true)` ya da çizerken `PaintCx::selectable`); `CodeView`, `Markdown` ve
  `Terminal` kendiliğinden seçilebilir. Bırakmak kopyalamaz; `ctrl c` ve sağ tık menüsündeki
  **Kopyala** süs hücrelerini (`PaintCx::decoration`), yalnız süsten oluşan satırları ve satır sonu
  dolgusunu atarak temiz kopyalar; **Ham kopyala** ekrandaki hücreleri olduğu gibi alır.

- **Hücre adımlı animasyonlarda renk karışımı:** Konum hücre hücre ilerlerken renkler adıma
  orantılı karışır (3 karede orta kare tam ara ton). Switch, sürgü, sekme, bildirim girişi ve sayfa
  geçişi dahil tüm adımlı hareketlerde geçerlidir.

- **İmza efekti — belirsiz ilerleme süpürmesi:** Her hücre ayrı 24-bit renkle boyanır; parlak bant
  hücreden hücreye akar (ease-in-out, bir tur `motion.shimmer`). ProgressBar, ShimmerText ve
  Skeleton kullanır.

- **Onay parlaması:** basışta `motion.flash` süresince bir ton parlama.

- **Hareketi azaltma:** `Command::set_reduced_motion`, `reduced-motion` ayarı ya da
  `QUVYTA_REDUCED_MOTION`. Ortam değişkeni tanımlıysa kararı o verir (`1` azaltır, `0` hareketi
  tutar); kayıtlı ayar ve çalışırken gelen komut bunu değiştiremez.

- **Yan panel ve bölücü:** kenar bir ton farkıdır. Kenara gelince sütun aydınlanır ve ortasında
  iki hücrelik düğme (`▌` ve ok) yükselir; `▌` yalnızca fare düğmenin üstündeyken ya da klavye
  odağında görünür. Sınırlar geliştiricinindir: `limits(min, max)`, `max` için `None` sınırsızdır.

## 7. Bileşenler için ortak kurallar

- Klavye ve fareyle tam kullanılabilir.
- Görünüm tamamen temadan; bileşende sabit renk yok.
- Gerçek hücre genişliği hesabı (Türkçe karakter, birleşik harfler, geniş karakterler, emoji);
  taşan metin `…`.
- Dar alanda düzgün küçülür, asla bozuk çizilmez.
- Boş / yükleniyor / hata / pasif / odaklı / hover halleri tasarlanmıştır.
- Birim testleri `Harness` ile tam ekran metnini, klavye ve fare davranışını, dar genişliği,
  hareketi azaltmayı, ASCII modunu ve anlam taşıyan renkleri doğrular.
- Public API'nin her öğesi İngilizce rustdoc ile belgelenir; stil anahtarları tipin belgesindedir;
  örnekler doctest olarak derlenir.

## 8. Katalog

Her öğe `CATALOG.toml`'da bulunur: `id`, `name`, `group`, `kind`, `priority`, `status`, `page`,
`types` (öğenin bütün public tipleri, fonksiyonları ve sabitleri) ve kararların kısa özeti olan
`notes` / `notes-tr`. Bugün bütün öğeler `done`; öncelikler yapılış sırasını gösterir:

**P0 — Temel:** Runtime · Test sürücüsü · Tema, ikon ve dil sistemleri · View ve yerleşim
(satır, sütun, katman; `Auto`, `Cells`, `Fill` uzunlukları, boşluk, hizalama) · Router ·
AppShell · Keymap · F12 katmanı · Text · Panel · Button · TextInput · Select · List · ScrollView ·
Tabs · KeyHints · Markdown · CodeView.

**P1 — Her uygulamanın ihtiyacı:** Motion · Checkbox · Switch · Segmented · RadioGroup · Slider ·
NumberInput · TextArea · Form + Field · Modal · Onay (`Command::confirm`) · Popover · ContextMenu ·
Toast · Tooltip · Badge · ProgressBar · Spinner · ShimmerText · Divider · EmptyState · Yardım
katmanı · Pano.

**P2 — Zengin uygulamalar:** Table · Tree · Splitter · Menu · Breadcrumb · Accordion · Steps ·
Wizard · SettingsList · CommandPalette · HoldToConfirm · Gelişmiş sekmeler (kapatma, genişlik,
taşma, sıralama) · LogView · Skeleton · FilePicker (dosya ya da klasör) · Ayar saklama · Sayfa
geçişleri · Fareyle metin seçme · Kaydırma çubuğu stilleri · TabRail · WidgetDock · SidePanel.
Örnek uygulamalar: Kurulum sihirbazı, Dosya gezgini.

**P3 — Özel ihtiyaçlar:** Sparkline · Gauge · BarChart · BigText · DatePicker · TimeInput ·
Terminal (PTY, `pty` özelliği) · Arka plan görevleri (`Task`, `TaskList`). Örnek uygulama: Gösterge paneli.

## 9. Showcase

### 9.1 Ekran

`AppShell` ile kurulur: üst çubuk (marka; tema, dil ve ikon seçimleri) · sol menü (arama `/`,
`1.1` gibi numaralı gruplar ve sayfalar) · gövde ·
alt `KeyHints`. `?` yardım katmanını, `ctrl p` komut paletini açar. Dar ekranda menü `ctrl b` ile
açılan bir katmana döner.

### 9.2 Bileşen sayfası

Numaralı sekmelerle dört bölüm (`1`–`4` tuşları):

1. **Demo** — canlı bileşenler; **oyun alanı** (yetenekleri tek tek açan ayarlar); **olay günlüğü**
   (her mesaj; seçilip kopyalanabilir).
2. **Kod** — demonun gerçek kaynağı; `// region: ad` işaretli bölgeler derleme zamanında dosyadan
   alınır, CodeView ile renklendirilir.
3. **Rehber** — EN + TR markdown: ne zaman kullanılır, adım adım, nasıl çalışır, sık yapılan hatalar.
4. **Referans** — öğenin tipleri, metotlar, davranış, tuşlar ve fare, tema, ikon ve dil anahtarları.

Kod, Rehber ve Referans bölümleri seçilebilir.

### 9.3 Ek bölümler

- **Temeller:** Başlangıç, Tema·İkon·Dil, Yerleşim, Odak ve Kısayollar, Hareket, Pano, Ayar
  saklama, Sayfa geçişleri, Fareyle metin seçme, Arka plan görevleri.
- **Örnek uygulamalar:** Kurulum sihirbazı, Dosya gezgini, Gösterge paneli; her biri Kod, Rehber ve Referans ile.

### 9.4 Hayalet bileşen yasağı

`cargo test` şunları doğrular, biri tutmazsa kırılır:

- `widgets` altındaki her public tip bir öğenin `types` listesinde.
- `done` olan her öğenin showcase sayfası var: Kod bölgesi ve EN + TR Rehber ile Referans.
- Showcase'teki her sayfa bir `done` öğeye ait; her `// region:` işareti bir bölge verir.
- `planned` öğelerin tipi ve sayfası olmaz; menüde soluk görünür ve notlarını etkin dilde gösterir.
- Showcase dil dosyaları her dilde aynı anahtarlara sahip.
- Hiçbir sayfa, bölüm, tema ve karakter modunda kutu çizme karakteri yok; demo yüzeylerinde köşeli
  ayraç yok.

Pre-commit bu testleri çalıştırır; sayfası olmayan bileşen commit edilemez.

## 10. Kalite ve test

### 10.1 Kurallar

- **Yer tutucu kod yok:** stub, `todo!()`, `unimplemented!()`, sahte dönüş değeri ya da
  "sonra doldururuz" kodu yazılmaz. Her değişiklik uçtan uca çalışır.
- Tek implementasyonlu trait yazılmaz; soyutlama ikinci implementasyonla doğar.
- `unsafe` yasak (`unsafe_code = "forbid"`).
- Kod yorumları ve public API belgeleri İngilizce; kullanıcıya dönük metinler dil dosyalarında.

### 10.2 Lint

- Workspace: `clippy::all = deny`; seçilmiş kurallar deny: `cast_lossless`,
  `needless_pass_by_value`, `redundant_closure_for_method_calls`, `semicolon_if_nothing_returned`,
  `uninlined_format_args`.
- `missing_docs = deny`, `rustdoc::broken_intra_doc_links = deny`.
- `cargo fmt --check` (`max_width = 120`).

### 10.3 Testler

| Tür | Ne doğrular |
|---|---|
| Birim | yerleşim, seçici çözümleme, renk karışımı, metin düzenleme, çoğul kuralları, keymap ayrıştırma, ayar okuma ve onarma |
| Bileşen | `Harness` ile tam ekran metni, tuş ve fare dizileri, sahte saatle animasyon kareleri, renkler |
| Gömülü dosyalar | temalar tanılamasız çözülür, ikon seti her modda geçerli, gömülü diller tamam, keymap geçerli ve etiketli |
| Katalog ve showcase | §9.4; her sayfa çok küçük terminallerde çizilir |
| Doctest | public API örnekleri derlenir ve çalışır |
| Görsel inceleme | `QUVYTA_REVIEW=1 cargo test -p showcase visual_review` her sayfayı her temada `target/showcase-review.html`'e yazar |

### 10.4 Kapılar

- `.githooks/pre-commit`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`.
  Bir kez: `git config core.hooksPath .githooks`.
- Hook'lar `LC_ALL=C` ile çalışır (Türkçe yerel ayarında `i/I` sorunu).
- `--no-verify` ve benzeri atlatmalar kullanılmaz.

### 10.5 Görsel onay

Bir öğe `done` olmadan önce görsel inceleme dosyası dört temada incelenir ve öğe showcase'te
elle denenir. Testlerin geçmesi tek başına yetmez.

## 11. Bağımlılıklar

Framework: `ratatui-core`, `ratatui-crossterm` (`crossterm`, `osc52`), `unicode-width`,
`unicode-segmentation`, `toml`, `sys-locale`, `pulldown-cmark` (Markdown).
`pty` özelliğiyle: `portable-pty`, `vt100`.
Showcase: `quvyta-framework` (`pty` açık), `toml`.
Yeni bağımlılık ancak onu kullanan öğeyle birlikte ve gerekçesiyle eklenir; ayar klasörü, TOML yazımı,
bulanık arama ve pano araçları framework'ün kendi kodudur.

## 12. Kapsam dışı

Sağdan sola yazım, web/GUI arka ucu, tarih/sayı yerelleştirme biçimleri (DatePicker'ın
ihtiyacı kadarı hariç), eklenti sistemi, async çalışma zamanı entegrasyonu.
