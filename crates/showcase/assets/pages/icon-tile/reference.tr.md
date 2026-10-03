## IconTile

- `IconTile::new(glif, ad)` — `glif` ile çizilen, `ad` ile adlandırılan karo. `glif` ikon setinin
  bir anahtarıdır, `"folder"` gibi, ya da uygulamanın kendi karakteri için bir `Glyph::literal`.
- `.selected(bool)` — karo seçilenlerdendir: `active` zeminini ve bütün yüksekliği boyunca vurgu
  çubuğunu alır, adı da metin rengini alır.
- `.cursor(bool)` — klavyedir karoda: yalnızca vurgu çubuğu, böylece seçimle imleci ayrı tutan bir
  yüzey hangisinin hangisi olduğunu söyleyebilir.
- `.backed(bool)` — karo, glifinin ve adının altında temanın `surface` tonundan bir karo üstünde
  durur; altında resim olan bir zemin için.
- `.color(jeton)` — glif, tema rengi `jeton` ile çizilir, `"accent"` ya da `"series-3"` gibi. Ad
  kendi tonunu korur.
- `.faint(bool)` — karo `faint` rolünün tonunda çizilir: kesilmiş bir girdi ya da bir yere
  götürülmekte olan bir girdi. Seçili soluk karo seçili satırın renklerini korur.
- `IconTile::shown_name(ad) -> Cow<str>` — karonun gösterdiği ad: sığdığında tamamı, değilse üç nokta
  ile kesilmiş hâli. Tamamını karonun yanındaki bir `Tooltip` içinde verin.
- `IconTile::WIDTH` (10), `IconTile::HEIGHT` (3), `IconTile::SIZE` ve `IconTile::PILLAR` (1) — bir
  karonun aldığı ölçü ve çubuk için ayrılmış sütun.

## Girdi

Karo çizer ve hiçbir şeye cevap vermez: ne tuş, ne basış, ne odak. İşaretçiyi karoları dizen yüzey
alır; bu yüzden onlardan bir ızgara, `.bare_cards(true)` verilmiş ve kendi `.on_select` ile
`.on_activate` bağlantıları kurulmuş bir `CardGrid` olur, bir masaüstünün kendi zemini ise fareyi
kendi ızgarasına karşı okur. Böyle bir ızgara, durduğu hücrenin fareini ve odağını karoya ödünç
verir, böylece karo tek başındaymış gibi yanar ve nefes alır.

## Yerleşim

- Karo, verildiği yerde kendini `SIZE` olarak ölçer ve sığanı çizer: çubuğun yanındaki sütunlarda
  ortalanmış glif, altında ad, gerçekten kalan yere göre bir kez daha kesilmiş.
- Üç satır: glif, ad ve altlarında boş bir satır, böylece iki karo hiçbir zaman değmez.
- Karoların satırı her satır gibi yerleşir; `.card_width(IconTile::WIDTH, IconTile::WIDTH)`,
  `.card_height(IconTile::HEIGHT)` ve `.gap(1, 1)` verilmiş bir `CardGrid` aralarına bir hücre boşluk
  koyar.

## Tema anahtarları

- `icon-tile` — `fg` (glifin rengi), `name` (adın rengi), `bg` (seçili karonun aldığı zemin),
  `pillar` (ilk sütununun vurgusu).
- `icon-tile`; `hover`, `selected` ve `focus` durumlarıyla. `selected`tan sonra yazılan bir `focus`
  kuralı ikisi de olan bir kartoda kazanır, böylece seçili kartonun çubuğu da nefes alır.
- `icon-tile.faint` — uygulamanın işaretlediği bir `[variant]` girdisi için biçim.
- Çubuğun nefes alması `pulse()`, zeminin fare altında yükselmesi `mix()` ile olur; çıplak kart için
  `card` ızgarasının kendi `bg` ve `pillar` değerleri çizilmez.
