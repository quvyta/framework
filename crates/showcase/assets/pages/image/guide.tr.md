## Ne zaman kullanılır

Kişinin seçtiği ya da görmesi gereken bir resim için: masaüstü arka planı, bir dosyanın önizlemesi, bir profil resmi, başka yerde çizilmiş bir grafik. 256 ve daha çok renkli her terminalde, SSH'de de çizilir, çünkü renkli hücrelerden başka bir şey istemez; kitty grafik protokolünü konuşan bir terminal ise onu gerçek piksellerle kendisi çizer.

## Adım adım

1. `Cargo.toml`'da `quvyta-framework`'ün `image` özelliğini açın: `features = ["image"]`.
2. Dosyayı arka planda çözün, çünkü büyük bir fotoğrafı çözmek biraz sürer: `Command::perform(move || ImageData::decode_file(&path, (width, height * 2)), Msg::Decoded)`. Alanın hücre genişliğini ve yüksekliğinin iki katını isteyin; böylece yalnızca ekranın gösterebileceği kadar piksel saklanır. `env.graphics()` `Graphics::Kitty` olduğunda terminal gönderilen her pikseli gösterir, o yüzden daha çoğunu, genişliğin yaklaşık on, yüksekliğin yirmi katını isteyin; resim saklandığı boyutta gönderilir, asla büyütülmez.
3. Çözülürken bir `Spinner` gösterin; olmazsa hatayla bir `EmptyState` gösterin, hata bir cümle olarak okunur: `error.to_string()`.
4. Çizin: `ui.add(Image::new(&data).fit(Fit::Cover)).fill()`.
5. Elinizde zaten piksel varsa `ImageData::from_rgb(width, height, &bytes)` kullanın.

## Nasıl çalışır

- **Hücre başına iki piksel.** Her hücre `▀` olarak çizilir: üstteki piksel glifin rengi, alttaki arkasındaki zemindir. Bir hücre eninin yaklaşık iki katı boyundadır, bu yüzden iki yarı kareye yakındır ve resim şeklini korur.
- **Üç yerleşim.** `Contain` resmin tamamını sığdığı kadar büyük gösterir, yanında zemin kalır. `Cover` bütün alanı doldurur, taşanı iki yandan eşit keser; duvar kâğıdı için olan budur. `Center` resmi kendi boyutunda, yarım hücre başına bir piksel gösterir, sığmayanı keser.
- **Seçilmez, ortalanır.** Küçültmede her yeni piksel kapladığı bütün piksellerin ortalamasıdır, böylece fotoğraf karıncalanmaz, tonlarını korur.
- **Bir kez hesaplanır.** Hücreler bileşenin belleğinde tutulur, yalnızca alanın boyutu, yerleşim ya da resim değişince yeniden hesaplanır. Aynı resmi yeniden çizen kare yalnızca onları kopyalar.
- **Üstüne çizilebilen bir zemin.** Resmin ulaşmadığı hücreler altındakini korur; resimden sonra çizilen her şey onun üstüne düşer, yani pencereler ve simgeler bir duvar kâğıdının üstünde durabilir.
- **Terminal yapabiliyorsa gerçek piksel.** `Env::graphics` `Kitty` dediğinde bileşen yarım blok çizmez: hücreleri düz zemin alır, resmi terminal onların üstüne çizer. Pikseller hatta bir kez gider ve bir numarayla saklanır; sonraki kare yalnızca nerede gösterileceğini söyler, hiçbir şeyin kıpırdamadığı kare ise hiçbir şey yazmaz. Artık gösterilmeyen resim terminalin belleğinden silinir.
- **Menüler üstte kalır.** Resim metnin altında, hücrelerin zemininin üstünde durur. Üstüne çizilen her şey kazanır: bir kenar boyunca açılan menü ya da panel resmi görünen kısma kırpar; ortasına düşen bir şey (pencere, iletişim kutusu) ya da bir iletişim kutusunun karartılmış arka planı, o çekilene kadar resmi yarım blokla çizdirir.
- **Her terminal.** 256 renkte her hücrenin iki yarısı da en yakın palet rengini alır. Yalnızca on altı standart renkte ya da ASCII gliflerde pikseller asla karakterle çizilmez: resmin adı, biçimi ve boyutu, bu terminalin resim gösteremediğini söyleyen bir cümleyle gösterilir.

## Sık yapılan hatalar

- **`view` içinde çözmek.** Büyük bir fotoğrafı çözmek hissedilir bir süre alır; bir kez, `Command::perform` içinde yapın ve `ImageData`'yı durumunuzda tutun.
- **Resmin tamamını istemek.** Olduğu gibi saklanan 4000'e 3000'lik bir fotoğraf onlarca megabayttır. `max` olarak ekranın gösterebileceği boyutu verin; yalnızca o saklanır.
- **Her karede yeni bir `ImageData`.** Her çözme ya da `from_rgb` yeni bir resimdir ve yeni resim yeniden hesaplanır. Durumunuzdakini klonlayın; klon aynı resimdir, bir referans sayısına mal olur.
