## Ne zaman kullanılır

Bir kişi şeyleri adıyla değil resimleriyle tanıyorsa ikon karosu kullanın: masaüstünün
simgeleri, bir dosya yöneticisinin ızgarasındaki girdiler. Bir karo, on sütun ve üç satır
lık hücrede simgenin üstünde ortalanmış adıdır; kaç tane olursa olsun ve adları ne kadar uzun olursa
olsun bir klasör onları hizalar. Boyut, tarih ve izinleri olan bir liste için tablo kullanın; tek
bir öğe tıklama alıyorsa düğme.

## Adım adım

1. Karo ne gösteriyor deyin: `IconTile::new("folder", "Photos")`. İlk argüman ikon setinin bir
   anahtarıdır, glif kipine uyar; kendi karakteriniz bir `Glyph::literal` olur.
2. Karo kendini `IconTile::WIDTH` sütun ve `IconTile::HEIGHT` satır olarak ölçer, daha dar bir
   yerde daha azını, ve oraya sığanı çizer.
3. Hangi kartoların seçili olduğunu `.selected(bool)`, klavyenin hangisinin üstünde olduğunu
   `.cursor(bool)` ile söyleyin.
4. Bir resmin üstündeyse `.backed(true)` ile söyleyin: karo o zaman temanın `surface` tonundan bir
   karo üstünde durur, böylece adı resmin ne olduğuna bakmadan okunur.
5. Rengiyle söylenen bir tür için `.color("series-3")` deyin. Ad kendi tonunu korur, böylece
   renkli simgelerden bir ızgara yine tek olarak okunur.
6. Kesilmiş bir girdi ya da bir yere götürülmekte olan bir girdi için `.faint(bool)` deyin.
7. Karo çizer ve başka hiçbir şey yapmaz: karoları dizen yüzey basışları alır. Kartoları
   `.bare_cards(true)` ile bir [`CardGrid`](card-grid) içine koyup ızgaranın `.on_select` ve
   `.on_activate` bağlantılarını kurun ya da bir masaüstü gibi kendi zemininizin üstünde çizin.

## Nasıl çalışır

- **Çubuğun sütunu hep vardır.** Seçili ya da imleç üstündeki karo, seçili olsun olmasın, ilk
  sütununu boyunca vurgu çubuğunu `▌` taşır; böylece simge ve ad her durumda aynı hücrelerde
  durur ve hiçbir şey kaymaz.
- **Seçili karo seçili zemini alır.** Adı metin rengini alır; seçili olmayan karo adını `dim`
  tonunda tutar. İmleç üstündeki karo yalnızca çubuğu alır, böylece ikisini ayrı tutan bir yüzey
  hangisinin hangisi olduğunu söyler.
- **Fare zemini bir adım yükseltir**, basılabilir bir panel gibi, ve ad onunla birlikte
  parlar. Bir resmin üstündeki karo yalnızca kendi yüzey karosunu yükseltir; resmin rengi karoya
  ait değildir.
- **Ad üç nokta ile kesilir.** `IconTile::shown_name`, karonun gösterdiği addır; tamamını karonun
  yanındaki bir `Tooltip` içinde verin.
- **Klavyede çubuk nefes alır**, ızgara klavyeyle odaklandığında; azaltılmış hareket onu hareketsiz
  tutar.
- **ASCII de uyur.** ASCII kipinde ikon setinin ASCII sütunu kullanılır ve üç nokta gösterilemeyen
  bir terminalde kesilen ad `~` ile biter.

## Temayla biçimlendirme

```toml
[style.icon-tile]
fg     = "$text"
name   = "$dim"

[style."icon-tile:hover"]
name = "$text"

[style."icon-tile:selected"]
bg     = "$active"
name   = "$text"
pillar = "$accent"

[style."icon-tile:focus"]
pillar = "pulse($accent, $accent-2)"

[style."icon-tile.faint"]
fg   = "$muted"
name = "$muted"
```

## Sık yapılan hatalar

- **Basışa cevap veren bir karo.** Yalnızca çizer; karoları dizen yüzey işaretçiyi alır ve cevap
  veren bir karo onu çevresindeki zeminden alırdı.
- **Seçili kartoda bir sütun eksik.** Yanındaki simgeleri kaydırmaması için her kartoda
  `IconTile::PILLAR` bırakın.
- **Kendi yüzeyleri olan kartolarla bir karo ızgarası.** Izgaraya `.bare_cards(true)` verin: çıplak
  kart, ızgaranın verdiği zemin üstünde durur ve hücresinin tamamını alır, böylece yalnızca seçili
  kartonun kendi yüzeyi olur.
