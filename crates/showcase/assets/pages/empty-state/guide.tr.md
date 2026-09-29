## Ne zaman kullanılır

Bir liste, tablo ya da panelin gösterecek bir şeyi kalmayabiliyorsa boş durum kullan: henüz container yok, arama sonucu yok, uyarı yok. Hiçbir şey yazmayan boş bir alan bozuk görünür; boş durum neden boş olduğunu ve şimdi ne yapılacağını söyler.

## Adım adım

1. Durumu söyleyen bir başlıkla başla: `EmptyState::new("Henüz container yok")`.
2. Bir iki cümleyle burada ne görüneceğini ya da neden bir şey görünmediğini açıkla: `.message(..)`.
3. Bir çıkış yolu varsa sun: `.action(Button::new("Container çalıştır").variant("primary").on_press(..))`. Eşit seçeneklerin her biri için `.action(..)` çağır; ilk eklenen seçenek birincil seçenektir.
4. Boş durum başarı, uyarı, hata ya da nötr bir haber bildiriyorsa `.tone(ToastKind::Danger)` ya da uygun türü ekle. İkon ve başlık durum rengini alır; ikon da rengin anlamını taşıyan işareti gösterir.
5. Durum tonu olmayan büyük alanlara sakin bir ikon ekle: `.icon("inbox")`.
6. Ona eksik içeriğin alanını ver, genellikle `.fill()` ya da sabit bir yükseklik; kendini ortalar.

## Nasıl çalışır

- **Ortalanmış blok.** İkon, başlık, açıklama ve eylemler aldıkları alanda iki yönde de tek bir blok olarak ortalanır.
- **Birden çok eylem.** Butonlar, diyalog eylemlerindeki gibi iki hücrelik boşlukla ortalanmış tek bir satırda yan yana durur. Satır fazla genişse her buton kendi ortalanmış satırında, aralarında boş bir satır bırakarak durur. Okuma ve odak sırası ilk eklenen butondan başlar; bu yüzden ilk buton birincil seçenektir.
- **Okunur genişlik.** Açıklama en fazla 52 hücrede sarılır; geniş ekranlarda kısa bir paragraf olarak kalır.
- **Durum tonu.** `ToastKind`, `Toast`'un kullandığı renkleri ve işaretleri yeniden kullanır. İşaret on altı renkli terminalde ve ASCII ikon kipinde de görünür; anlam yalnızca renkle taşınmaz.
- **Küçük alanlar.** Satır yetmezse önce ikon, sonra eylemlerin üstündeki boşluk, sonra açıklama satırları gider (görünen son satır `…` ile biter). Yer olduğu sürece başlık ve eylemler kalır.
- **Gerçek butonlar.** Her eylem sıradan bir Button'dır: Tab ve ok tuşlarıyla odak alır; Enter, Space ve fareyle çalışır.

## Sık yapılan hatalar

- **Kullanıcıyı suçlamak.** "Hiç container'ın yok" sitem gibi okunur; "Henüz container yok" yalnızca durumu söyler.
- **İşe yaramayan eylem.** Gerçek bir sonraki adım yoksa eylemi koyma.
- **İşaretsiz renk.** Anlamın sınırlı terminalde de okunması için durumda `.tone(..)` kullan.
- **Yüklenirken kullanmak.** Boş olmak bir sonuçtur. Veri yoldayken iskelet ya da spinner göster; yoksa veri gelmeden boş durum bir an görünüp kaybolur.
