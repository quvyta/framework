## Metotlar

- `Modal::new()` — boş bir pencere; içeriğini `ui.add_with(modal, |ui| …)` ile ekle.
- `.title(metin)` — ilk satırda kalın bir başlık.
- `.variant(ad)` — tema varyantı; `"danger"` sol kenardaki çubuğu tehlike rengine boyar.
- `.width(hücre)` — iç boşluk dahil genişlik. Varsayılan: 56; dar ekranda küçülür.
- `.on_close(mesaj)` — pencere kapatıldığında gönderilir: Esc ya da × işaretine tıklama. Yoksa ikisi de yoktur.
- `.dismissable(bool)` — Esc, × işareti ve dışarı tıklama pencereyi birlikte kapatır mı. Varsayılan: `true`. `false` işareti de gizler.
- `.close_on_click_outside(bool)` — pencere kapatılabilirken karartılmış ekrana tıklanınca da kapatma mesajını gönderir. Varsayılan: `false`.
- `.action(buton)` — sağ alttaki eylem satırına, eklendiği sırayla bir buton.

## Tuşlar

- `tab` / `shift tab` yalnızca penceredeki bileşenler arasında döner.
- `esc` pencere kapatılabilirken kapatma mesajını gönderir; üst üste binen pencerelerde yalnızca üstteki duyar.
- Diğer her tuş içerideki odaklı bileşene gider; alttaki bileşenlere asla ulaşmaz, pencere açıkken uygulama kısayolları durur.

## Fare

- İçerideki bileşenler her zamanki gibi çalışır.
- × işareti yüzeyin sağ üst köşesinde, içeriğin üstündeki iç boşluk satırında durur: fare üstüne gelince birlikte aydınlanan üç hücre; tıklamak kapatma mesajını gönderir. Yalnızca pencere kapatılabilirken çizilir.
- Karartılmış ekrandaki tıklama, tekerlek ve sürükleme yutulur; `close_on_click_outside` ile pencereyi kapatır. Pencerenin altındaki hiçbir şey tepki vermez.

## Davranış

- Eklendiği yerde yer kaplamaz; tüm görünümden sonra, ortada, zemine doğru karıştırılmış ekranın üstünde çizilir.
- Yüzeyin her satırında ilk iç boşluk sütununu bir `▌` çubuğu doldurur ve yüzeyle birlikte gelir; ASCII modunda renkli bir hücredir.
- Kapatılabilir pencerenin üstte en az bir satır, sağda en az üç hücre iç boşluğu vardır; × işareti içeriğin üstüne binmez.
- `motion.enter` süresinde gelir: yüzey her yandan iki sütun ve bir satır büyür ve solarak belirir; hareket azaltılmışsa anında.
- Açılınca içerideki ilk odaklanabilir bileşen odağı alır; kapanınca odak önceki bileşene döner.
- Gövde içeriği kadar uzundur, ekrana sığmayan kısım kesilir; uzun içeriği bir kaydırma alanına koy.
- İpucu satırı pencere kapatılabilirken `esc kapat`, birden fazla bileşen odak alabiliyorsa `tab geç` gösterir; eylemlerin yanına sığmayan ipuçları düşer.

## Tema anahtarları

- `modal` — `bg`, `padding` (varsayılan `[1, 3]`), `pillar` (vurgunun sönük hali); `modal.danger` — `pillar`.
- `modal-title` — `fg`, `bold`.
- `close-mark` — `fg`, `bg`, `bold`; `active` ve `hover` ile.
- `layer-backdrop` — `scrim` (ekranın karıştırıldığı renk), `strength` (yüzde).
- `layer-hint-key`, `layer-hint-label` — `fg`, `bold`.
- `[motion]` — `enter`.
- `[icons]` — `pillar`, `close`.
- Dil — `quvyta.layer.close`, `quvyta.layer.switch`.
