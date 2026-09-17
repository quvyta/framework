## Metotlar

- `Segmented::new(seçenekler)` — ilki seçili bölümler.
- `.selected(usize)` — seçili bölüm.
- `.disabled(bool)` — odak alamaz, değiştirilemez.
- `.on_select(|index| mesaj)` — yeni seçilen bölüm için mesaj.

## Davranış

- Her bölüm, etiketi ve iki yanında temanın yatay iç boşluğu kadardır; bir satır.
- Sol ve Sağ komşuyu, Home ve End uçları seçer; tıklama işaretçinin altındaki bölümü seçer.
- Zaten seçili olanı seçmek mesaj göndermez.

## Tema anahtarları

- `segment` — `bg`, `fg`, `bold`, `padding`, `pillar` (üzerine gelinen bölümün, klavye odağında ise seçili bölümün ilk hücresinde çizilir); durumlar `hover`, `focus`, `checked`, `disabled`.
