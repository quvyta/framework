## Metotlar

- `Slider::new(değer)` — 0'dan 100'e birer adımla giden, `değer`i gösteren sürgü.
- `.range(en_küçük, en_büyük)` — aralığın uçları; ters verilen aralık sıraya sokulur.
- `.step(adım)` — bir adımın büyüklüğü; değerler en küçük değerden sayılan adımlara oturur. Pozitif olmayan adım 1 olur.
- `.suffix(metin)` — değerin hemen ardına yazılan metin, örneğin `"%"`.
- `.format(|değer| metin)` — değeri adımın ondalıklarıyla değil kendi yazdığınla gösterir; son ek yine eklenir.
- `.disabled(bool)` — sürgüyü soluklaştırır; odak alamaz, değiştirilemez.
- `.on_change(|değer| mesaj)` — topuz başka bir adıma geldiğinde yeni değerle mesaj. Verilmezse sürgü görünür ama odak alamaz.

## Davranış

- Verilen genişliğin tamamını ve bir satırı kullanır. Değer; en küçük, en büyük ve güncel değerden en genişinin sığacağı alanda sağa yaslanır, ardından iki hücre boşluk ve ray gelir.
- Alan, değerin dört hücre fazlasına yetmiyorsa yalnızca değer çizilir.
- ← → bir adım, Page Up / Page Down aralığın onda biri (en az bir adım), Home / End uçlar.
- Raya basmak imlecin altındaki değere atlar ve bırakana kadar imleci yakalar; sürükleme izlenir. Değere basmak yalnızca odaklar.
- Sürgünün üstünde tekerlek bir adım değiştirir (yukarı artırır, aşağı azaltır), uçlarda durur ve tekerleği sürgü alır, böylece çevredeki kaydırma alanı kaymaz; odak değişmez. Pasif sürgü tekerleği yok sayar.
- Klavye değişikliklerinde topuz her `motion.step` süresinde bir hücre, en çok sekiz adımda ilerler; fare ve tekerlek değişiklikleri ile azaltılmış hareket anında taşır.

## Tema anahtarları

- `slider` — `fill` (dolu kısım), `fill-cell` (ASCII modunda dolu kısım), `track`, `knob`; durumlar `hover`, `focus`, `disabled`.
- `slider-value` — `fg`, `bold`; aynı durumlar.
- `[motion]` — `step` ve nefes alan topuz için `pulse-period`.
- `[icons]` — `slider-rail`, `slider-knob`; boş karakter rayı hücre renkleriyle çizdirir.
