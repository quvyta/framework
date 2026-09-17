## Metotlar

- `Divider::new()` — tek bir boş satır.
- `.space(hücre)` — boşluk satırı sayısı, dikeyse sütun sayısı; varsayılan 1.
- `.label(metin)` — boşluğun ardından kendi satırında bir başlık; yalnızca yatay ayırıcılarda.
- `.band()` — ayırıcıyı bant tonuyla boyar.
- `.vertical()` — yan yana içeriği ayırır; `.fill_height()` ile birlikte kullan.

## Davranış

- Yatay: tüm genişliği ve `space` satırı ölçer, başlık varsa bir satır daha.
- Dikey: `space` sütun ve bir satır ölçer; `fill_height` onu satır boyunca uzatır.
- Başlık sığmazsa `…` ile kesilir.
- Hiçbir karakter modunda çizgi karakteri çizmez. Odak almaz.

## Tema anahtarları

- `divider` — `band`.
- `divider-label` — `fg`, `bold`.
