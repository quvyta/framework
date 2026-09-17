## Ne zaman kullanılır

Bir ekranın asıl konusu olan tek değer için büyük metin kullan: durum panosunda saat, bir servisin çalışma süresi, geri sayım, açılış ekranının başlığı. Dikkati güçlü çeker; bir ekranda bir tane olmalı, nadiren iki.

## Adım adım

1. Ekle: `ui.add(BigText::new("14:32"))`.
2. Üstüne değerin ne olduğunu söyleyen küçük, silik bir başlık koy.
3. En önemli değere vurgu rengini ver: `.variant("accent")`.
4. Metni durumundan güncelle; her metin gibi yeniden çizilir.
5. Yer bırak: metin üç satır boyundadır (ASCII modunda beş) ve karakter başına yaklaşık dört hücre tutar.

## Nasıl çalışır

- **Küçük bir piksel yazı tipi.** Her glif beş piksel boyunda, üç piksel genişliğinde bir bit eşlemdir (`:` ve `.` için bir, `N` için dört, `M` ve `W` için beş). Rakamlar, `:`, `.`, `%`, `-` ve A'dan Z'ye harfler vardır; küçük harfler büyük biçimleri kullanır; diğer karakterler boşluk olur.
- **Yarım bloklar.** Unicode ve Nerd Font gliflerinde iki piksel `▀`, `▄` ve `█` ile tek hücreyi paylaşır; beş piksel üç satıra sığar. ASCII modunda yarım blok yoktur; her piksel bir hücreyi arka plan rengiyle boyar.
- **Sığmazsa sade çizim.** Alan çok dar ya da kısaysa metin kesilmek yerine normal boyda ve kalın çizilir.

## Sık yapılan hatalar

- **Cümleler.** Büyük metin bir değer ya da kelime içindir; uzun metin geniş olur ve zor okunur.
- **Birden çok büyük değer.** Her şey büyük olunca hiçbir şey öne çıkmaz. Önemli olanı seç.
- **Başlıksız değer.** "%99,98" üstünde "Çalışma süresi" yazmadıkça bir şey anlatmaz.
