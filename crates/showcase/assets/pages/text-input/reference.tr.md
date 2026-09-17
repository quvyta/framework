## Metotlar

- `TextInput::new(değer)` — `değer` gösteren bir alan.
- `.on_change(|metin| msg)` — her düzenlemeden sonra yeni değer.
- `.on_submit(|metin| msg)` — Enter'a basıldığında değer.
- `.placeholder(metin)` — boşken silik metin.
- `.password(bool)` — karakterleri `mask` ikonuyla gizler. Varsayılan: `false`.
- `.max_length(n)` — en fazla `n` karakter.
- `.invalid(bool)` — geçersiz görünüm. Varsayılan: `false`.
- `.disabled(bool)` — salt okunur, odak almaz. Varsayılan: `false`.

## Tuşlar

- `left` `right`, `ctrl` ile kelime kelime, `shift` ile seçerek; `home` `end`.
- `backspace` `delete`; `ctrl backspace` ve `ctrl w` bir kelime siler; `ctrl u` başa kadar siler.
- `ctrl a` tümünü seç; `ctrl z` geri al; `ctrl y` ya da `ctrl shift z` yinele.
- `ctrl c` kopyala, `ctrl x` seçimi kes (parola alanında asla); `ctrl v` ve terminalin yapıştırması ekler.
- Seçim varken `left` ve `right` onu kaldırır ve sol ya da sağ ucunun bir adım ötesine gider; `ctrl` ile o uçtan bir kelime.
- `shift f10` ya da `menu` düzenleme menüsünü alanın altında açar.
- `enter` gönderir; `tab` odağı taşır.

## Fare

- Tıklama imleci yerleştirir; sürükleme seçer.
- Sağ tık düzenleme menüsünü imlecin yanında açar: Kes, Kopyala, Yapıştır, Tümünü seç. Seçimin içindeyse seçimi korur, başka yerdeyse önce imleci oraya koyar. Kes ve Kopyala seçim ister; Yapıştır sistem panosunda, terminalin panosunda ya da uygulama içindeki bir kopyada metin ister.

## Tema anahtarları

- `hover`, `focus`, `invalid`, `disabled` ile `text-input` — `bg`, `fg`, `padding`.
- `text-input-prompt`, `text-input-placeholder`, `text-input-selection`, `text-input-cursor`.

## İkonlar

- Metinden önce `prompt`; parolalar için `mask`.

## Dil anahtarları

- `quvyta.edit.cut`, `quvyta.edit.copy`, `quvyta.edit.paste`, `quvyta.edit.select-all`.
