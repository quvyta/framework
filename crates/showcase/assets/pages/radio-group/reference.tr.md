## Metotlar

- `RadioGroup::new(seçenekler)` — hiçbir şey seçili olmayan dikey grup.
- `.selected(Option<usize>)` — seçili seçenek.
- `.horizontal(bool)` — seçenekler arasında dört hücre olan tek satır.
- `.style(RadioStyle)` — `Square` (varsayılan): her seçenekte ortada küçük bir kare; seçili olan seçili renge bürünür. `Mark`: aynı kare, seçilince iki hücrelik dolu kutuya büyür. `Box`: onay kutusununkiyle aynı iki hücrelik renk kutusu. `Dot`: nokta ve halka.
- `.disabled(bool)` — odak alamaz, değiştirilemez.
- `.on_select(|index| mesaj)` — yeni seçilen seçenek için mesaj.

## Davranış

- Dikey: en uzun seçenek artı dört (`Square`, `Mark`, `Box`) ya da üç (`Dot`) hücre genişliğinde, seçenek başına bir satır. Yatay: bir satır. İşaretin genişliği hiçbir durumda ve karede değişmez.
- `Square`: her zaman `🬇🬃` (U+1FB07 ve U+1FB03). Seçim yeni kareyi iki `motion.step` boyunca sakin tondan seçili tona karıştırır, eskisi aynı anda geri döner; biçim hiç değişmez. `Mark` biçimleri: küçük `🬇🬃` ve dolu (iki hücre renk), arada başka boy yok; aynı karışım, biçim yolun yarısında değişir. Hareket azaltılmışsa hemen değişir.
- Sekstantları kitty, WezTerm, Ghostty ve foot kendisi çizer; diğer terminallerde Symbols for Legacy Computing içeren bir yazı tipi gerekir ya da ikonlar değiştirilir.
- ASCII'de kare ikonu boştur: işaret, tonu silik kutudan seçili kutuya karışan bir kutudur.
- `Box` her kutuyu boş renk ile dolu renk arasında `motion.step` × 3 sürede, her seçeneği ayrı ayrı karıştırır; hareket azaltılmışsa hemen geçer.
- Yukarı/Aşağı (dikey) ya da Sol/Sağ (yatay) komşuyu seçer; Home ve End uçları; Boşluk ya da Enter geçerli ya da ilk seçeneği seçer; tıklama işaretçinin altındaki seçeneği seçer.
- Zaten seçili olanı seçmek mesaj göndermez.

## Tema anahtarları

- `radio.mark` — işaret stilinin `fg` rengi: kare ve `checked` durumunda dolu kutunun rengi; durumlar `hover`, `focus`, `checked`, `disabled`.
- `radio.box` — kutu stilinin ve boy ikonu boş olan işaretlerin `bg` rengi; aynı durumlar. Yerleşik temalar ona `checkbox` ile aynı tonları verir.
- `radio` — nokta stilinin `fg` rengi; aynı durumlar.
- `radio-label` — `fg`, `bold`; aynı durumlar.
- `[motion]` — `step` (işaretin boy adımları ve kutu geçişi).
- `[icons]` — `radio-mark-small` (işaret stili, iki hücre; tema değiştirebilir), `dot`, `dot-outline` (nokta stili).
