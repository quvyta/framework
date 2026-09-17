## Ne zaman kullanılır

Yardımcı olan ama ekranı kullanmak için şart olmayan kısa bir ipucu için kullan: kısa etiketli bir araç çubuğu butonunun ne yaptığı, `…` ile kısaltılmış bir adın tamamı, bir durum işaretinin anlamı. Tek satırda tut. Kullanıcının başarmak için okuması gereken her şey ekranın kendisinde, etkileşimli her şey bir açılır kutuda olmalı.

## Adım adım

1. Bileşeni sar: `ui.add_with(Tooltip::new(t!("restart-tip")), |ui| { ui.add(Button::new(t!("restart"))); })`.
2. İpucu klavye kullananlar için de önemliyse `.on_focus(true)` ekle; odak içerideyken görünür.
3. Alt taraf kalabalıksa `.placement(Placement::Above)` ile başka bir taraf seç.

## Nasıl çalışır

- **Bekler.** İpucu, imleç bileşenin üstünde temanın `motion.hover-delay` süresi kadar (gömülü temalarda 450 ms) durduktan sonra belirir ve imleç ayrılır ayrılmaz kaybolur.
- **Yola çıkmayan bir katmandır.** Katman yüzeyinde, çerçevesiz tek satır metin; her şeyin üstüne çizilir. Tıklama yakalamaz: açıkladığı bileşene ya da başka bir yere tıklamak, ipucu yokmuş gibi çalışır.
- **İmleci örtmeden yer bulur.** Varsayılan olarak altta; yer yoksa öbür tarafa geçer, imlecin altındaki hücreyi örtecekse kenara çekilir.
- **Yavaşça belirir**: `motion.enter` süresince metnin rengi yüzeyden tam parlaklığa karışır. Hareket azaltılmışsa anında oradadır.
- **Klavye odağı isteğe bağlıdır.** `.on_focus(true)` ile odak içerideyken ipucu hemen görünür; klavye kullananlar da aynı ipucunu alır.
- **Düz içerik de olur.** Fare davranışı olmayan bir metin de üzerine gelinmeyi algılar, çünkü ipucu sardığı bütün alanı dinler.

## Sık yapılan hatalar

- **Gerekli bilgiyi saklamak.** Dokunmatik ve klavye kullananlar onu hiç görmeyebilir; gerekli metni ekrana koy.
- **Uzun cümleler.** İpucu tek satırdır; ekran darsa `…` ile kısaltılır.
- **Her şeye ipucu.** Açık bir etiketin ipucuna ihtiyacı yoktur; her kontroldeki ipucu imleci bekletmeyi gürültüye çevirir.
