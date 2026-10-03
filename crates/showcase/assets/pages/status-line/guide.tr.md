## Ne zaman kullanılır

Durum satırını insanın "durum nasıl" diye baktığı yerde kullan: bir container listesinin altında, bir formun üstünde, bir grafiğin altında. Durum sürdüğü kadar kalır ve uygulama onu kendi koyduğu yere koyar. Kendiliğinden kalkan haberler için bildirim, bir sayı için rozet, içeriği olmayan alan için boş durum kullan.

## Adım adım

1. Cümleyi yaz: `StatusLine::new("3 container'ın 3'ü sağlıklı")`.
2. Bildirdiği durumu söyle: `.tone(ToastKind::Success)`. İşaret de renk de bu türden gelir.
3. Bir çıkış yolu varsa sun: `.action(Button::new("Yeniden dene").on_press(Msg::Retry))`.
4. Gereken yeri ver. Sütundaki bir satır istediği genişliği alır; `.fill_width()` bütün satırı verir ve cümlenin sarılmasına izin verir.

## Nasıl çalışır

- **Yer varsa tek satır.** İşaret, bir boşluk, cümle, sonra iki hücrelik boşluktan sonra buton.
- **Dar alan sarar, asla kesmez.** Cümle kendi altına sarılır ve sütununu korur; işaret söylediği kelimelerin yanında durmayı sürdürür. Buton kesilmek yerine kendi satırına iner, böylece hiçbir şey `…` ile bitmez.
- **Anlam asla yalnızca renk değildir.** İşaret, aynı türden bir bildirimin gösterdiği glifin aynısıdır, aynı renkte: başarıda `✓`, uyarıda `▲`, tehlikede `✕`, bilgide `ℹ`; ASCII ve Nerd Font'ta da her biri tek karakter. Bir sat ile aynı türden bir bildirim bir aile gibi görünür.
- **Cümle uygulamanın kendi metnidir,** durumun renginde; bir başlık gibi değil, bir durum gibi okunur. Cümlenin sıradan metin gibi okunmasını isteyen tema o tür için `status-line` adını verir; işaret yine tonu korur.
- **Buton gerçek bir butondur.** Tab ile odak alır, Enter, Space ve fareyle kendi mesajını gönderir ve ekrandaki bütün butonlar gibi görünür.
- **Yalnızca buton girdi alır.** Cümle ve işaret metindir; üstlerine basmak hiçbir şey yapmaz, böylece rastgele bir tıklama kullanıcının niyet etmediği bir mesajı gönderemez.

## Sık yapılan hatalar

- **Durum satırını habere kullanmak.** Oldu, cevap gerektirmiyor ve kendiliğinden kalkan bir şey bildirimdir; satır şu an nasıl olduğunu söyler.
- **Birden çok buton.** Satır tek satırdır. İkinci eylem yukarıdaki içeriğe ya da bir pencereye aittir.
- **İşaretsiz renk.** Bir cümleyi elle durum renginde yazmak tam da bu bileşenin yerine geçtiği şeydir: işaret ve renk ayrılamaz, çünkü ikisi de türden gelir.
- **Bitmeyen cümle.** Durum satırı kısa bir cümleledir, bir günlük değil. Uzun haberler bildirime, bir panele ya da günlük görünümüne gider.
