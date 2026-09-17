## Ne zaman kullanılır

Kullanıcının devam etmeden önce küçük bir işi bitirmesi ya da vazgeçmesi gerekiyorsa pencere kullan: projeyi yeniden adlandırmak, yıkıcı bir işlemi gözden geçirmek, kısa bir listeyi düzenlemek. Kullanıcı yanında çalışmayı sürdürecekse yan panel ya da açılır kutu daha uygundur. Düz bir evet/hayır sorusu için `Command::confirm` daha kısadır (Onay sayfasına bak).

## Adım adım

1. Açık olup olmadığını durumunda tut: `rename_open: bool` ya da pencerelerini sayan bir enum.
2. `view` içinde açıkken ağacın herhangi bir yerine ekle: `ui.add_with(Modal::new().title(t!("rename")), |ui| { … })`. Eklendiği yerde yer kaplamaz.
3. Kapatılabilir yap: `.on_close(Msg::CloseRename)`; bayrağı `update` içinde geri al. Artık hem Esc hem sağ üstteki × bu mesajı gönderir.
4. Butonları ekle, güvenli olan önce: `.action(Button::new(t!("cancel")).on_press(Msg::CloseRename))`, sonra asıl işlem.
5. Yıkıcı pencerelerde `.variant("danger")` ekle; onaylayan butona da tehlike varyantını ver.
6. Pencereden çıkılmaması gereken anlarda, örneğin kayıt sürerken, `.dismissable(false)` ekle: Esc, × ve dışarı tıklama birlikte durur, `on_close` sonrası için yerinde kalır.

## Nasıl çalışır

- **Karartılmış ekranın üstünde bir katman.** Pencere ekranın ortasında bir katman yüzeyidir; arkasındaki her şey zemin rengine doğru karıştırılır. Çerçeve yoktur, onu ton ayırır; sol kenarı boyunca baştan sona bir `▌` çubuğu uzanır: vurgu renginin sönük hali, yıkıcı pencerelerde tehlike rengi.
- **Hafifçe belirir.** Yüzey temadaki `motion.enter` süresinde birkaç hücre büyüyerek ve solarak gelir. Hareket azaltılmışsa doğrudan görünür.
- **Odak içeride kalır.** Açılınca içindeki ilk odaklanabilir bileşen odağı alır. Tab ve Shift+Tab yalnızca onun bileşenleri arasında döner, tuşlar alttaki bileşenlere ulaşmaz, uygulama kısayolları (showcase'teki geri için Esc gibi) durur.
- **Odak geri döner.** Pencereyi kaldırınca odak, önceden onu tutan bileşene, çoğunlukla pencereyi açan butona döner.
- **Pencereler üst üste biner.** Başka bir pencerenin içine ya da görünümde daha sonra eklenen pencere üstte çizilir ve tuşları o alır; Esc yalnızca üsttekini kapatır.
- **Kapatılabilir demek Esc ve × birlikte demek.** `on_close` verilmiş pencere hem Esc ile hem sağ üst köşedeki üç hücrelik × işaretiyle kapanır; işaret, fare üstüne gelince küçük bir buton gibi aydınlanır. Biri ötekisiz gelmez: `.dismissable(false)` ikisini birden kapatır ve işareti gizler.
- **İşaretçi engellenir.** Karartılmış ekrana tıklamak, `.close_on_click_outside(true)` verilmedikçe hiçbir şey yapmaz; verilse de yalnızca pencere kapatılabilirken kapatır. Açılır listelerden farkı şu: pencere tıklamaları yutmayı sürdürür, kapanana kadar alttaki hiçbir şey tepki vermez.
- **İpuçları tuşları söyler.** Sol altta silik bir satır, Esc kapatıyorsa `esc kapat`, odaklanacak birden fazla bileşen varsa `tab geç` yazar.

## Sık yapılan hatalar

- **Engellemeyen şeyler için pencere açmak.** "Kaydedildi" ya da biten bir dağıtım bir bildirimdir, pencere değil.
- **Yıkıcı butonu öne koymak.** Odağı ilk odaklanabilir bileşen alır; kazara basılan Enter zarar vermesin diye onu güvenli olan yap.
- **Çıkışı olmayan pencere.** `on_close` yoksa ya da `.dismissable(false)` verdiysen pencereyi kapatan bir buton ekle.
- **Esc'i × olmadan, ×'i Esc olmadan istemek.** Bilerek tek bir seçenektir: klavyeyle çalışan da fareyle çalışan da aynı çıkışı bulur.
- **İmleç ya da kaydırma durumunu uygulamada tutmak.** Pencere açıkken içindeki bileşenler kendi durumlarını tutar.
