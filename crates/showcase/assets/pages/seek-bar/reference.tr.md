## Metotlar

- `SeekBar::new(kesir)` — `kesir`de bir çubuk, 0..1 aralığına sıkıştırılmış; sayı olmayan bir değer başta durur.
- `.percent(göster)` — çubuğun ardındaki yüzdeyi gösterir ya da gizler; ilerleme çubuğundaki gibi varsayılan olarak gösterilir.
- `.variant(isim)` — `"success"` gibi tema varyantı, tam olarak ilerleme çubuğundaki gibi.
- `.on_seek(|kesir| msg)` — bir basışın, bir sürüklemenin ya da bir ok tuşunun gittiği kesirle mesaj. O olmadan çubuk çizilir ama odak, üzerine gelme ve işaretçi almaz.
- `.hover_label(|kesir| metin)` — işaretçinin üstünde gösterilen etiketi yazar, örneğin bir parçanın zamanı.

## Davranış

- Aldığı tüm genişliği ve bir satırı ölçer; `ProgressBar::new(kesir)` ile aynı hücreleri ve tonları çizer.
- Yüzde, `.percent(false)` gizlemedikçe sağda beş hücre kaplar; çubuk kalanını kullanır, bir basış ya da üzerine gelme yalnızca bu hücreleri sayar.
- Dolgu hücrenin sekizde biri kadar, ASCII modunda en yakın tam hücreye yuvarlanarak dolar.
- Bir basış, düştüğü hücrenin merkezine gider: yirmi hücrelik bir çubukta dördüncü hücre `4.5 / 20` olur.
- Sürükleme, bırakılana kadar işaretçiyi yakalar ve işaretçinin geçtiği her hücreye gider; iki uçtan öteye ya da çubuğun satırının dışına çıkınca kesir 0 ya da 1'de tutar.
- ← → çubuğun yirmide birini taşır, Home / End uçlara gider. Uçta, taşınacak bir yer kalmadığında, çubuk hiçbir şey göndermez.
- İşaretçinin altında çubuk bir ton açılır ve işaretçinin altındaki hücre vurgu rengini alır.
- `on_seek` olmadan çubuk üzerine gelinmez ve Tab ile ulaşılmaz.
- Bir üst etiket, kütüphanenin ipucu olarak işaretçinin kendi hücresinin üstüne çizilir ve üzerine gelme gecikmesi olmadan, hemen görünür.

## Tema anahtarları

- `seek-bar`, `seek-bar.<varyant>` — `track`, `fill`; `hover` ve `focus` durumları.
- `progress-label`, `progress-label.<varyant>` — `fg`, `bold`; yüzde, ilerleme çubuğunun yazdığı yüzdenin aynısıdır.
- `[motion]` — etiketin belirmesi için `enter`, odaktayken dolgunun nefes alması için `pulse-period`.