## Ne zaman kullanılır

Kullanıcı bir eylem başlatacaksa buton kullan: kaydet, gönder, sil, bir diyalog aç. Yerler arasında gezinmek için buton değil, liste satırı ya da sekme kullan.

Her ekranda **tek bir birincil buton** olsun. Her şey vurgulanırsa hiçbir şey vurgulanmamış olur.

## Adım adım

1. Bir etiketle oluştur: `Button::new(t!("actions.save"))`. Etiketler koddan değil, dil dosyalarından gelir.
2. Göndereceği mesajı ver: `.on_press(Msg::Save)`. Mesajı olmayan buton çizilir ama odak alamaz ve basılamaz.
3. Önemliyse varyant seç: ana eylem için `.variant("primary")`, geri alınamaz eylemler için `.variant("danger")`.
4. Aynı eylemi bir tuş da tetikliyorsa kısayol bölümü ekle: `.shortcut("ctrl s")`. Bu yalnızca tuşu gösterir; tuşu kısayol haritana bağla.
5. Eylem sürerken `.loading(true)` ver. Etiket kalır, ikonun yerine spinner döner ve basışlar yok sayılır.

## Nasıl çalışır

- **Şekli renk verir.** Buton, iki hücre iç boşluklu, kendi tonunda bir yüzeydir. Hiçbir karakter modunda parantez ya da çerçeve yoktur.
- **Çubuk en başta durur.** Hover ve odakta vurgu çubuğu `▌` butonun ilk hücresinde, kısayol bölümünden ve ikondan önce çıkar: `▌ ⏎   Kaydet`, asla `⏎ ▌ Kaydet`. Bu hücre buton dururken de onun parçasıdır, çubuk çıkınca hiçbir şey kaymaz. Butonlar kaymaz.
- **Durumlar temadan gelir.** Hover yüzeyi yükseltir ve soluk bir çubuk gösterir. Klavyeyle gelen odak da yüzeyi yükseltir, çubuk iki vurgu tonu arasında nefes alır; tıklanarak odaklanan buton farenin altında sakin kalır. Basış butonu temanın `motion.flash` süresi kadar bir ton parlatır.
- **Klavye ve fare eşittir.** Enter ya da Boşluk odaklı butona basar; tıklama, fare butonun üzerinde bırakıldığında basar, yani bırakmadan önce uzaklaşmak basışı iptal eder.
- **Basılı tutulan tuş tekrar etmez.** Enter'a basılı tutmak tek basış sayılır; basılı tutmayı hızlı ardışık basış olarak bildiren terminallerde bile. Başka bir tuştan sonra yeniden basılan Enter ne kadar çabuk gelirse gelsin yeni bir basıştır.
- **Pasif, girdi için yok demektir.** Pasif ya da yükleniyor durumundaki buton Tab'da atlanır ve tıklamaları yok sayar.

## Temayla özelleştirme

```toml
[style."button.primary"]
bg = "$accent"
fg = "$ink"

[style."button:focus"]
bg     = "$active"
pillar = "pulse($accent, $accent-2)"

[style."button.warning"]
bg = "mix($warning, $surface, 22%)"
fg = "$warning"
```

Varyant adları serbesttir: bir tema `button.warning` stilini tanımladığı anda `.variant("warning")` çalışır.

## Sık yapılan hatalar

- **Yan yana iki birincil buton.** Kullanıcının en büyük ihtimalle istediğini seç.
- **Etiketi koda yazmak.** Dil dosyasına koy ki buton dili takip etsin.
- **Butonu gezinmek için kullanmak.** Sayfalar arası geçiş listelerin, sekmelerin ve router'ın işidir.
