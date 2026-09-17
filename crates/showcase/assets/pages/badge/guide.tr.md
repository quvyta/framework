## Ne zaman kullanılır

Rozet, başka bir şeye ait kısa bir durumu gösterir: çalışan bir container, başarısız bir dağıtım, sabitlenmiş bir dal. İlgi bekleyen şeyleri, örneğin uyarıları ya da güncellemeleri saymak için de kullanılır. Bir cümle ya da eylem gerekiyorsa metin veya buton kullan.

## Adım adım

1. Nötr bir rozet ekle: `ui.add(Badge::new("Duraklatıldı"))`.
2. Durumun ne anlama geldiğini söyleyen bir ton ver: `.variant("success")`, `"warning"`, `"danger"`, `"info"` ya da `"accent"`.
3. Rozeti anlattığı şeyin yanına koy; genellikle satırın sonuna: önce isim, sonra durum.
4. Sayı önemliyse ekle: `Badge::new("Uyarılar").variant("danger").count(3)`.
5. Etiketi bir iki kelimede tut; dar yerlerde rozete bir genişlik ver, etiketi `…` ile keser.

## Nasıl çalışır

- **Şekli renk verir.** Hap, iki yanında birer hücre boşluk olan boyanmış bir yüzeydir. Durum renginin yaklaşık %16'sı yüzeyin üstüne karışır; ton belli olur ama hap sakin kalır.
- **Renk asla tek başına kalmaz.** Her rozet, kelimesinden önce bir `●` işareti taşır (ASCII modunda `*`); durum renk olmadan da okunur.
- **Sayılar.** Sayı, etiketin hemen ardından daha güçlü bir bölümde durur; 99'dan büyükler `99+` görünür, hap büyümez.
- **Nötr.** Varyant yoksa rozet yükseltilmiş yüzeyde soluk yazıyla durur; iyi ya da kötü haber olmayan durumlar içindir.

## Sık yapılan hatalar

- **Gökkuşağı.** Tonların anlamı vardır. Her rozetin başka renkte olduğu bir liste hiçbir şey anlatmaz; durumların çoğu nötr olmalı.
- **Hapın içinde cümle.** "Sağlık denetiminin geçmesi bekleniyor" metne aittir; rozet "Başlıyor" der.
- **Durum için vurgu rengi.** Vurgu tonu senin kendi vurgun içindir, sabitlenmiş ya da yeni gibi; başarı ya da hata için değil.
