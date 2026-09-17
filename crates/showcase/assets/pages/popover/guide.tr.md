## Ne zaman kullanılır

Tek bir kontrole ait, ekranı ele geçirmemesi gereken küçük ve isteğe bağlı bir panel için açılır kutu kullan: Filtreler butonunun altındaki filtreler, bir dağıtımın ayrıntıları, bir alanın yanındaki renk seçici. Kapandığında ekranın geri kalanı olduğu gibi görünür ve kullanılır. Listeden seçim için açılır liste; kullanıcının devam etmeden vermesi gereken bir karar için diyalog kullan.

## Adım adım

1. Açık olup olmadığını durumunda tut: `filters_open: bool`.
2. Çapa ve içerikle kur: `Popover::new(self.filters_open).anchor(|ui| …).content(|ui| …).show(ui)`.
3. Çapadan, genellikle bir butondan aç kapa: `Button::new(t!("filters")).on_press(Msg::ToggleFilters)`.
4. İstendiğinde kapat: `.on_dismiss(Msg::CloseFilters)` Esc'te ve dışarı tıklanınca gelir.
5. İçerik bir formsa `.focus_inside(true)` ekle; klavye katmanın içinde başlasın.

## Nasıl çalışır

- **Bir katmandır.** İçerik her şeyin üstüne, çerçevesiz, katman yüzeyinde çizilir; kenarı ton farkıdır. Alttaki hiçbir şey kaymaz.
- **Yer bulur.** Varsayılan olarak çapanın altında ya da `.placement(…)` ile seçilen tarafta açılır. O tarafta yer yoksa karşı tarafa geçer ve her zaman ekranın içine geri itilir.
- **Çapadan açılır**, temanın `motion.enter` süresince satır satır (yan yerleşimlerde sütun sütun). Hareket azaltılmışsa anında belirir.
- **Kapatmak senin elinde.** Esc ve dışarı tıklama kapatma mesajını gönderir.
- **Tıklama boşa gitmez.** Açılır kutu ekranı soldurmaz, bu yüzden ekranı tutmaz da: onu kapatan tıklama, düştüğü yerdeki şeye de ulaşır. Filtreler açıkken başka bir butona tek tıklama filtreleri kapatır ve o butona basar; menüdeki bir sayfaya tek tıklama sayfayı değiştirir. Çapanın kendisine tıklamak çapaya ulaşır; o da genellikle aç kapa yapar. Kutuyu çapanın dışındaki bir buton açtıysa, o butona tıklamak yalnızca kapatır; kutu aynı tıklamayla kapanıp yeniden açılmaz. Pencereler farklıdır: ekranı soldururlar ve kapanana kadar her tıklamayı yutarlar.
- **İçeride odak isteğe bağlıdır.** `.focus_inside(true)` ile açılınca odak katmandaki ilk odaklanabilir bileşene geçer, kapanınca eski yerine döner.
- **Katmanlar üst üste biner.** Açılır kutunun içindeki bir açılır liste kendi listesini onun üstünde açar; o listenin dışına tıklamak listeyi kapatır, sonra tıklama açılır kutunun içinde her zamanki gibi işler.

## Sık yapılan hatalar

- **`on_dismiss`'i unutmak.** O olmadan Esc ve dışarı tıklama bir şey yapmaz; kutu çapaya yeniden basılana kadar açık kalır.
- **İçine koca bir sayfa koymak.** Açılır kutu birkaç kontrol içindir; uzun içerik kendi ekranını ya da bir yan paneli ister.
- **Satırı kırılan metin.** Uzun satırlara `.no_wrap()` ya da sabit genişlik ver; yoksa katman ekran kadar genişler.
