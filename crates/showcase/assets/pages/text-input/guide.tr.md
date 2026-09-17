## Ne zaman kullanılır

Kullanıcının yazdığı tek satırlık metin için metin kutusu kullan: isimler, arama terimleri, yollar, kodlar. Değer uygulamana aittir; düzenlemeyle ilgili her şey bileşene aittir.

## Adım adım

1. Değerini göster: `TextInput::new(&self.name)`.
2. Her değişikliği al: `.on_change(Msg::NameChanged)`. Yeni değeri `update` içinde sakla; alan durumunda ne varsa onu gösterir.
3. Oraya ne yazılacağını söylemek için `.placeholder(t!("..."))` ekle.
4. Kodunda doğrula ve alanı işaretle: `.invalid(name.len() < 3)`, altına kısa bir mesajla.
5. Gizli bilgiler için `.password(true)`, sınırlar için `.max_length(n)`, Enter'da işlem için `.on_submit(msg)` kullan.

## Nasıl çalışır

- **Gerçek düzenleme.** İmleç karakterler arasında gezinir, asla birleşik bir harfin içine girmez. Ctrl ve oklar kelime atlar, Shift seçer, Home ve End kenarlara atlar.
- **Doğru hissettiren geri alma.** Bir kelime yazmak tek adımda geri alınır; bir kelime silmek kendi adımıdır. Ctrl+Z geri alır, Ctrl+Y ya da Ctrl+Shift+Z yineler.
- **Pano.** Ctrl+C ve Ctrl+X seçimi kopyalar ve keser; Ctrl+V önce sistem panosundan, sonra terminalin panosundan, en son uygulama içindeki son kopyadan yapıştırır. Terminalin kendi yapıştırması da metni ekler, satır sonları boşluğa döner.
- **Oklar seçimi arkada bırakır.** Seçim varken ← ve → onu kaldırır ve sol ya da sağ ucundan bir adım öteye gider; bir metin düzenleyicideki gibi.
- **Fare.** İmleci yerleştirmek için tıkla, seçmek için sürükle. Sağ tık Kes, Kopyala, Yapıştır ve Tümünü seç menüsünü açar: seçimin içindeyse seçimi korur, başka yerdeyse önce imleci oraya koyar. Seçim yokken Kes ve Kopyala, yapıştıracak bir şey yokken Yapıştır pasiftir. Shift+F10 ve menü tuşu aynı menüyü açar.
- **Parola kopyalanmaz.** Parola alanında Kes, Kopyala, Ctrl+C ve Ctrl+X hiçbir şey yapmaz.
- **Yazarken imleç sabit kalır**, yalnızca durduğunda temanın `motion.cursor-blink` hızında yanıp söner.
- **Uzun değerler** imleci görünür tutmak için yatayda kayar.
- **Dışarıdan değişiklik kazanır.** Uygulaman değeri değiştirirse alan onu gösterir ve imleci sona koyar.

## Temayla özelleştirme

```toml
[style."text-input:focus"]
bg = "$active"

[style."text-input:invalid"]
bg = "mix($danger, $surface, 14%)"

[style.text-input-cursor]
bg = "$accent"
fg = "$ink"
```

Geçersiz alan yüzeyini renklendirir; asla kırmızı bir çerçeve çizmez.

## Sık yapılan hatalar

- **Her tuşta yüksek sesli doğrulama.** Boş alan henüz hata değildir; mesajları kullanıcı bir şey yazdıktan sonra göster.
- **İmleci kendi durumunda tutmak.** Motor bunu senin için tutar.
- **Seçimler için metin kutusu.** Cevaplar belliyse açılır liste ya da liste kullan.
