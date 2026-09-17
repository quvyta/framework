## Ne zaman kullanılır

Bir liste, tablo ya da panelin gösterecek bir şeyi kalmayabiliyorsa boş durum kullan: henüz container yok, arama sonucu yok, uyarı yok. Hiçbir şey yazmayan boş bir alan bozuk görünür; boş durum neden boş olduğunu ve şimdi ne yapılacağını söyler.

## Adım adım

1. Durumu söyleyen bir başlıkla başla: `EmptyState::new("Henüz container yok")`.
2. Bir iki cümleyle burada ne görüneceğini ya da neden bir şey görünmediğini açıkla: `.message(..)`.
3. Bir çıkış yolu varsa sun: `.action(Button::new("Container çalıştır").variant("primary").on_press(..))`.
4. Büyük alanlar için sakin bir ikon ekle: `.icon("inbox")`.
5. Ona eksik içeriğin alanını ver, genellikle `.fill()` ya da sabit bir yükseklik; kendini ortalar.

## Nasıl çalışır

- **Ortalanmış blok.** İkon, başlık, açıklama ve eylem, aldığı alanda iki yönde de tek bir blok olarak ortalanır.
- **Okunur genişlik.** Açıklama en fazla 52 hücrede sarılır; geniş ekranlarda kısa bir paragraf olarak kalır.
- **Küçük alanlar.** Satır yetmezse önce ikon, sonra eylemin üstündeki boşluk, sonra açıklama satırları gider (görünen son satır `…` ile biter). Yer olduğu sürece başlık ve eylem kalır.
- **Gerçek bir buton.** Eylem sıradan bir Button'dır: tab ile odak alır; Enter, Space ve fareyle çalışır.

## Sık yapılan hatalar

- **Kullanıcıyı suçlamak.** "Hiç container'ın yok" sitem gibi okunur; "Henüz container yok" yalnızca durumu söyler.
- **İşe yaramayan eylem.** Gerçek bir sonraki adım yoksa eylemi koyma.
- **Yüklenirken kullanmak.** Boş olmak bir sonuçtur. Veri yoldayken iskelet ya da spinner göster; yoksa veri gelmeden boş durum bir an görünüp kaybolur.
