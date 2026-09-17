## Metotlar

- `MenuItem::new(anahtar, etiket)` — bir hedef; `.icon(anahtar, Option<renk>)`, `.badge(metin)`.
- `MenuGroup::new(anahtar, öğeler)` — bir grup; `.title(metin)` silik başlığı ekler.
- `Menu::new(gruplar)` — menü.
- `.selected(Option<&str>)` — açık öğenin anahtarı.
- `.on_select(|anahtar| msg)` — açık olandan farklı bir öğe açılınca gönderilir.
- `.collapsible(|grup, açık| msg)` — başlıklı gruplar katlanır; `.collapsed(anahtarlar)` kapalı olanları listeler.

## Davranış

- Gruplar arasında boş bir satır olur. Katlanan grubun başlığı kalır.
- Klavye imleci açık öğeden başlar; menü odaktayken hover gibi çizilir. Böyle tek bir vurgu vardır: fareyi bir satıra götürmek imleci oraya taşır.
- Katlanabilen başlık bir öğe gibi çubukla yükselir ve başlığını kaydırır; oku sağda sabit kalır.
- Harfle atlama yalnızca görünen öğelere bakar ve başa sarar.
- Açık öğeyi yeniden açmak onu parlatır, mesaj göndermez.

## Tuşlar

- `up` `down` gezer, `home` `end` uçlara gider, bir harf o harfle başlayan sonraki öğeye atlar, `enter` ya da `space` açar.
- `collapsible` ile: başlıkta `enter` açıp kapatır, `right` açar, `left` kapatır, bir öğede `left` başlığına gider.

## Fare

- Açmak için öğeye, katlamak için başlığa tıkla; kaydırmak için tekerleği çevir ya da kaydırma çubuğunu sürükle.

## Tema anahtarları

- `menu-item` — `bg`, `fg`, `bold`, `pillar`; `hover`, `selected`, `focus`, `pressed` durumları.
- `menu-badge` — `fg`; `selected`.
- `menu-heading` — `fg`; katlanan menülerde `bg` ve `pillar` ile `hover`.
- `scrollbar`.

## İkonlar

- `pillar`, katlanan başlıklarda `chevron-down` ve `chevron-right`, ve verdiğin ikonlar.
