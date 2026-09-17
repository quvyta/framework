## Ne zaman kullanılır

Ayırıcı, aynı yüzey içindeki iki grup arasında bir duraklamadır: çalışan ve durdurulan servisler, bugünkü ve eski dağıtımlar, yan yana iki değer. Gruplar farklı şeylerse her birine kendi panelini ver; yüzeyler, aralarına çizilen her şeyden daha iyi ayırır.

## Adım adım

1. Bir sütunda iki grubun arasına sade bir ayırıcı koy: `ui.add(Divider::new())`. Tek bir boş satırdır.
2. Ardından gelen grubu bir başlıkla adlandır: `Divider::new().label("DURDURULANLAR")`. Başlıkları panel başlıkları gibi yaz.
3. Gruplar büyükse daha çok nefes ver: `.space(2)`.
4. Yalnızca boşluk yetmiyorsa, örneğin kalabalık bir ekranda, boya: `.band()`.
5. Yan yana şeylerin arasında `Divider::new().vertical().band()` kullan ve satır boyunca uzansın diye `.fill_height()` ekle.

## Nasıl çalışır

- **Neden çizgi yok.** Ekranı boydan boya geçen bir çizgi ekrandaki en gürültülü şeydir ve ayırdığı şeyler hakkında hiçbir şey söylemez. Quvyta'da gruplama boşlukla ve tonla yapılır; paneller ve seçili satırlar da böyle şekillenir.
- **Önce boşluk.** Sade ayırıcı hiçbir şey çizmez. Gözün iki grubu görmesi için bir boş satır yeter ve içerikle asla yarışmaz.
- **Başlık aşağıya aittir.** Başlık, boşluğun ardından kendi satırında, adlandırdığı grubun hemen üstünde durur; böylece önceki grubun sonu değil, yeni grubun başlangıcı olarak okunur.
- **Bant bir tondur.** Bant, ayırıcıyı yüzey ile yükseltilmiş yüzey arasındaki bir renkle doldurur: bir çizgi değil, daha sakin tonda bir oluk. Hem zeminde hem panellerin içinde çalışır.

## Sık yapılan hatalar

- **Her şeyin arasına ayırıcı.** Her satır ayrılırsa hiçbir şey gruplanmaz. Yalnızca grupların arasında kullan.
- **Panel kenarının yanında ayırıcı.** Yüzeyler zaten ayırır; panelin yanındaki bir bant yalnızca kenarı kalınlaştırır.
- **Dikey ayırıcıda `fill_height` unutmak.** O olmadan bant bir satır boyunda kalır.
