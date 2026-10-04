## Ne zaman kullanılır

Bir ekranın asıl konusu olan tek değer için büyük metin kullan: durum panosunda saat, bir servisin çalışma süresi, geri sayım, açılış ekranının başlığı. Dikkati güçlü çeker; bir ekranda bir tane olmalı, nadiren iki.

## Adım adım

1. Ekle: `ui.add(BigText::new("14:32"))`.
2. Üstüne değerin ne olduğunu söyleyen küçük, silik bir başlık koy.
3. En önemli değere vurgu rengini ver: `.variant("accent")`.
4. Metni durumundan güncelle; her metin gibi yeniden çizilir.
5. Logo ya da açılış başlığı için harfleri ikinci bir tema rengine doğru karıştır: `.gradient("info", Gradient::Columns)`, aşağı doğru karışım için `Gradient::Rows`.
6. Yer bırak: metin üç satır boyundadır (ASCII modunda beş) ve karakter başına yaklaşık dört hücre tutar.
7. Metin dışarıdan geliyorsa — bir parçanın adı, bir dosya adı — `BigText::fits(metin)` ile sor: bu yazı tipinin çizemediği bir karakter varsa metnin tamamı düz metin olarak çizilir.

## Nasıl çalışır

- **Küçük bir piksel yazı tipi.** Her glif beş piksel boyunda, birkaç sütun genişliğinde bir bit eşlemdir (`:` `.` ve `'` için bir, boşluk ve `,` için iki, `N` ve `Ñ` için dört, `M` `W` `Æ` ve `Œ` için beş). Rakamları, `:`, `.`, `%`, `-` işaretlerini, `'`, `!`, `?`, `&`, `(`, `)`, `,`, `/` noktalama işaretlerini ve bütün Latin harflerini çizer: A'dan Z'ye harfler, Türkçe (`Ç Ğ İ Ö Ş Ü`) ve Batı Avrupa (`À Á Â Ä Å È É Ê Ë Ì Í Î Ï Ñ Ò Ó Ô Ö Ø Ù Ú Û Ü Ý Ÿ ß Æ Œ`) biçimleriyle.
- **Küçük harf büyük harf olarak çizilir.** Küçük harf, karakterinin Unicode büyük harf biçimiyle çizilir; Türkçe kuralı geçerlidir: `ı` her dilde `I`, `i` ise etkin dil Türkçe ya da Azerbaycanca olduğunda `İ`, başka her dilde `I` olur. Böylece "içinde" Türkçe ekranda İÇİNDE, İngilizce ekranda IÇINDE okunur.
- **Beş satır bir büyük harfin üstüne yer bırakmaz.** Aksan kendi satırında bir işaret olur, harf onun altında çizilir; cedilla alt satırı alır. `Ü` ve `Ÿ` üst satırını çift noktalara verir, harf bir satır aşağıda durur. `Ä` ve `Ö` düz harflerinden bir piksel geniştir: üç sütuna sıkıştırıldıklarında `H` ve `V` gibi okunurlardı.
- **Yarım bloklar.** Unicode ve Nerd Font gliflerinde iki piksel `▀`, `▄` ve `█` ile tek hücreyi paylaşır; beş piksel üç satıra sığar. ASCII modunda yarım blok yoktur; her piksel bir hücreyi arka plan rengiyle boyar.
- **Geçiş, parıltı değil.** Gradient harflerin varacağı tema rengini adlandırır, kendine ait bir renk almaz; böylece logo temayı izler. Bir kez boyanır ve hiç oynamaz; parıldayan harfler için `ShimmerText` vardır. Karışım hücre hücre ilerler: sütun yönünde metin kaç hücre genişse o kadar, satır yönünde Unicode ve Nerd Font modunda üç, ASCII modunda beş adım olur.
- **Geçiş ne zaman düşer.** Terminalde yalnızca on altı standart renk varsa ya da tema istenen rengi tanımıyorsa harfler düz rengini korur. Her hücreyi o palete tek tek yuvarlamak harfleri benek benek yapardı; düz renk her terminalde okunur. 256 renkli palet geçişi korur ve her adımı en yakın girdiye yuvarlar.
- **Sığmazsa sade çizim.** Bu yazı tipinin çizemediği bir karakteri olan bir metin — Kiril, Japon, bir emoji — o karakterin yerinde delik bırakmak yerine baştan sona kalın düz metin olarak çizilir; alana sığmayan metin de böyle çizilir. Ölçtüğü de çizdiği tek satırdır.

## Sık yapılan hatalar

- **Cümleler.** Büyük metin bir değer ya da kelime içindir; uzun metin geniş olur ve zor okunur.
- **Birden çok büyük değer.** Her şey büyük olunca hiçbir şey öne çıkmaz. Önemli olanı seç.
- **Başlıksız değer.** "%99,98" üstünde "Çalışma süresi" yazmadıkça bir şey anlatmaz.
- **Başka bir yazı sistemi.** Kiril, Çin ya da bir emoji için büyük biçim yoktur; metin kalın düz metin olarak okunur, her karakter korunur ama boyu kaybolur.
