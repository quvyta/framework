## Ne zaman kullanılır

Konum yolu, kullanıcının bir hiyerarşide nerede olduğunu gösterir ve yukarı çıkmasını sağlar: dosya gezginindeki klasörler, bir kümedeki namespace'ler, iç içe ayarlar. Geri gitmenin ya da sekmelerin yerini tutmaz.

## Adım adım

1. Yolu kökten bulunduğun yere kadar durumunda tut.
2. Göster: `Breadcrumb::new(self.path.clone())`.
3. Bir seviye açılınca yukarı çık: `.on_select(Msg::Up)`, `update` içinde `self.path.truncate(index + 1)`.
4. Kullanabileceği genişliği ver, örneğin bir başlık satırında `.fill_width()`.

## Nasıl çalışır

- **Buton değil yazı.** Seviyeler soluk, sade yazılardır; üstüne gelince yükselir. Son seviye bulunduğun yerdir: kalındır ve tıklanmaz.
- **Ayırıcı bir ikondur.** Seviyeler arasında ikon setinden silik, küçük bir ok durur; asla `/` ya da `>` değil. Temalar bunu `crumb-separator` ile değiştirebilir.
- **Dar yolda orta kısım katlanır.** Yol sığmayınca kök ve sığdığı kadar son seviye kalır, arada `…` durur. `…` açılınca gizli seviyeler listelenir. Çok dar alanda yalnızca `…` ve bulunduğun yer görünür. ASCII kipinde katlanan kısım `~` olarak çizilir.
- **Klavye.** Yola odaklan, ← ve → ile gez, Home ve End ile uçlara git, Enter ya da Boşluk ile aç.

## Sık yapılan hatalar

- **Bütün yolu etikete yazmak.** Her parça tek bir seviyenin adıdır, yolun tamamı değil.
- **Tek seviyelik yol göstermek.** Tek parçada dönülecek yer yoktur; yol o zaman odak almaz.
- **Kardeş klasörler menüsü gibi kullanmak.** Konum yolu yalnızca yukarı çıkar; kardeşleri bir listede ya da menüde göster.
