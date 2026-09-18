## Ne zaman kullanılır

Her öğe kendine ait küçük bir yüzeyi hak ediyorsa ve öğeler yan yana okunuyorsa kart ızgarası kullan: bir mağazanın uygulamaları, bir başlatıcının programları, arasından seçilecek profiller. Ayrıntılı satırlardan oluşan bir sütun için liste ya da tablo kullan.

## Adım adım

1. Kaç kart olduğunu söyle: `CardGrid::new(uygulamalar.len())`.
2. Kart ölçülerini ver: `.card_width(24, 32)` (bir kartın en az ve en fazla genişliği), `.card_height(3)` satır içerik, sütunlar ve satırlar arasında `.gap(2, 1)`. Varsayılanlar da bunlardır.
3. Durumunda tuttuğun seçimi `.selected(self.selected)` ile göster; hareketi `.on_select(Msg::Select)`, açmayı `.on_activate(Msg::Open)` ile al.
4. Kartın ne göstereceğini `.card(|ui, sıra| { .. })` ile kur. Kapatma okuduğu şeyin sahibidir: durumundaki `Rc<[Uygulama]>`'yı kopyalayıp içine taşı. Yazıları `Text::no_wrap()` ile kur; dar bir kart onları `…` ile keser.
5. Aynı anda birden çok seçim için `.checked(bool_listesi)` ve `.on_toggle(Msg::Toggle)` ekle.
6. Boş ızgaranın ne anlama geldiğini `.empty(EmptyState::new(..).action(..))` ile söyle.

## Nasıl çalışır

- **Sütunlar genişliği izler.** En az genişlikte sığan kadar kart yeri paylaşır, her biri en fazla genişliğe kadar; artan yer sağda boş kalır. 24 ile 32 hücre ve 2 hücre boşlukla 96 hücrelik alanda üç, 140 hücrelik alanda beş sütun olur. Tek karttan dar bir alan, alan kadar geniş tek bir sütun gösterir.
- **Yalnızca ekrandaki kartlar kurulur.** Kapatma ızgara çizilirken yalnızca görünen kartlar için çalışır; on bin kart bir ekranlık kart kadar tutar.
- **Oklar ızgara gibi gezer.** Sol ve sağ satır boyunca gider, satırın ucunda durur; yukarı ve aşağı sütunu korur, kısa son satıra inen aşağı ok son kartı verir. Home ve End ilk ve son karta, PgUp ve PgDn bir ekranlık satır atlar. Seçili kart hep görünür kalır.
- **Aynı anda tek kart yanar.** Farenin altındaki kart bir ton yükselir ve sol kenarı boyunca soluk bir çubuk alır. Fare ızgaranın üstünde hareket ettikçe vurguyu o taşır, seçili kart dinlenir; sonraki tuş vurguyu geri alır ve farenin durduğu karttan devam eder.
- **Kart bir yüzeydir, satır değil.** Seçili kart seçili zemini alır; ızgaraya klavyeyle gelindiyse çubuğu nefes alır. Hiçbir şey kaymaz.
- **İşaret köşede durur.** İşaretli kart sağ üst köşesinde vurgu renginde bir işaret taşır. Yanan kart orada soluk bir işaret gösterir: tıklarsan kart açılmadan işaretlenir. Boşluk seçili kartı işaretler.
- **Fare, tuşların yaptığını yapar.** Tıklamak kartı seçer ve açar, tekerlek bir sıra kart kaydırır, kaydırma çubuğuna basılıp sürüklenebilir.

## Temayla biçimlendirme

```toml
[style.card]
bg      = "$raised"
padding = [0, 2]

[style."card:hover"]
bg     = "mix($text, $raised, 8%)"
pillar = "mix($accent, $active, 45%)"

[style."card:selected:focus"]
pillar = "pulse($accent, $accent-2)"
```

## Sık yapılan hatalar

- **Kapatmada durumu ödünç almak.** Kart kapatması bileşen kadar yaşar; içine bir referans yerine `Rc` ya da `Arc` kopyası taşı.
- **Kartın içine buton koymak.** Karttaki bileşenler çizilir ama girdi almaz: basılan yüzey kartın tamamıdır. Eylemleri kartın açtığı sayfaya koy.
- **Kartta yazıyı sarmak.** Kartın yüksekliği sabittir; sarılan yazı altta kesilir. `…` ile bitsin diye `no_wrap` kullan.
