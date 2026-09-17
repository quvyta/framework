## Ne zaman kullanılır

Kesin sayı önemliyse sayı girişi kullan: container portu, kopya sayısı, ayrılacak CPU çekirdeği. Kullanıcı bildiği sayıyı yazabilir ya da ok tuşlarıyla azar azar değiştirebilir. Kesin sayıdan çok aralıktaki yer önemliyse sürgü kullan.

## Adım adım

1. Sayıyı uygulamanda tut: `replicas: f64`.
2. Çiz: `NumberInput::new(state.replicas)`. Seçeneksiz alanın sınırı yoktur, birer birer değişir.
3. Değişiklikleri işle: `.on_change(|value| Msg::Replicas(value))`, değeri `update` içinde sakla.
4. Sayının sınırları varsa belirt: `.range(1.0, 12.0)`.
5. Adımı seç: `.step(0.25)`. Adımın ondalıkları sayının nasıl yazılacağını ve ondalık nokta yazılıp yazılamayacağını belirler.
6. Alan çoğunlukla fareyle kullanılacaksa `.steppers(true)` ekle.

## Nasıl çalışır

- **Bu bir metin girişidir.** Yüzey, istem işareti, odak, imleç, seçim, geri alma ve pano `TextInput` ile aynıdır; yeni bir şey öğrenmek gerekmez.
- **Yalnızca sayı yazılır.** Rakamlar her zaman; eksi işareti yalnızca aralık sıfırın altına iniyorsa; ondalık nokta yalnızca adımın ondalığı varsa. Yapıştırılan metinden yalnızca bu karakterler kalır.
- **Uygulamana yalnızca geçerli sayılar gelir.** Metin yarımken (`-`, `2.`), sayı değilken ya da aralığın dışındayken alan geçersiz tonuna bürünür ve hiçbir şey göndermez; durumunda son geçerli değer kalır. Boş alan yer tutucuyu gösterir ve işaretlenmez.
- **Adım adım değiştirme:** ↑ ve ↓ bir adım, Page Up ve Page Down on adım gider; sonuç her zaman aralıkta kalır. Adım yazılan sayıdan başlar: 7 yazıp ↑'e basınca 8 olur.
- **Adım bölümleri** sağda, butonun kısayol bölümü gibi iki yüzey parçasıdır. Birine tıklamak bir adım değiştirir ve bölümü parlatır; sınıra gelince işareti soluklaşır.
- **Son söz uygulamanındır.** Uygulaman değeri değiştirdiğinde alan yeni sayıyı hemen gösterir.
- **Tekerlek adımlar.** Alanın üzerindeyken tekerlek her çentikte bir adım ilerletir ve çevresindeki sayfa kaymaz; pasif alan tekerleği yok sayar.
- **Düzenleme menüsü.** Sağ tık, metin kutusundaki gibi Kes, Kopyala, Yapıştır ve Tümünü seç menüsünü açar; yapıştırılan metinden yalnızca sayıya uyan karakterler kalır.

## Sık yapılan hatalar

- **İki kez doğrulamak.** Aralık, dışındaki sayıları zaten `update`'ten uzak tutar; `.invalid(true)`'yu yalnızca kendi kuralların için kullan, örneğin portun dolu olması.
- **Seçenekler için sayı girişi.** İki ile beş arasındaki sabit seçenek, segment seçiciyle ya da radyo grubuyla daha iyi okunur.
- **`update` içinde yuvarlamak.** Bunun yerine adım seç; yazılan sayılar kendi yazılışını korur, adım adım değiştirme adımın ondalıklarıyla yazar.
