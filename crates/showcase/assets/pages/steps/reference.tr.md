## Metotlar

- `Steps::new(etiketler)` — bir satır adım; ilki güncel, seçilemez.
- `.current(sıra)` — güncel adım; öncekiler bitmiştir, sondan büyük bir değer hepsini bitirir.
- `.vertical(bool)` — her satıra bir adım.
- `.running(bool)` — güncel işaret nefes alır.
- `.failed(bool)` — güncel işaret tehlike renginde hata ikonuna döner.
- `.on_select(|sıra| mesaj)` — bitmiş adımlar seçilebilir; mesaj adımı taşır.

## Davranış

- Bir adım; işareti, iki hücre boşluk ve etiketidir. Satırdaki adımlar arasında üç hücre vardır.
- Satır sığmazsa işaretler birer hücre arayla kalır, güncel etiket onların ardından gelir.
- Seçilebilir adımlar, hover yüzeyine yer açmak için iki yanda birer hücre iç boşluk alır; fare ya da klavye üstündeyken bu hücrelerin ilkinde vurgu çubuğu belirir.
- Odaktayken tuşlar: satırda ←/→, sütunda ↑/↓, Home ve End bitmiş adımlar arasında gezer; Enter ya da Boşluk seçer. Bitmiş bir adıma tıklamak onu seçer; diğer adımlar tıklamayı yok sayar.
- Adımlar yalnızca `on_select` verilmişken ve en az bir adım bitmişken odak alır.

## Tema anahtarları

- `step` — `bg`, `pillar`; durumlar `hover`, `focus` (seçilebilir bitmiş adımlar).
- `step-marker` — `fg`; durumlar `checked` (bitmiş), `active` (güncel); varyantlar `running`, `failed`.
- `step-label` — `fg`, `bold`; aynı durum ve varyantlar, ayrıca `hover` ve `focus`.
- `[icons]` — `check`, `dot`, `dot-outline`, `error`.
