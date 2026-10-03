## Metotlar

- `TextInput::new(değer)` — `değer` gösteren bir alan.
- `.on_change(|metin| msg)` — her düzenlemeden sonra yeni değer.
- `.on_submit(|metin| msg)` — Enter'a basıldığında değer.
- `.on_cancel(mesaj)` — alanda Esc'e basılınca gönderilir, tuş öteye geçmez; açık bir öneri listesi ilk Esc'i alır.
- `.placeholder(metin)` — boşken silik metin.
- `.password(bool)` — karakterleri `mask` ikonuyla gizler. Varsayılan: `false`.
- `.max_length(n)` — en fazla `n` karakter.
- `.invalid(bool)` — geçersiz görünüm. Varsayılan: `false`.
- `.disabled(bool)` — salt okunur, odak almaz. Varsayılan: `false`.
- `.select_on_focus(aralık)` — alan her odak aldığında `aralık` içindeki karakterleri seçer, imleç aralığın sonundadır; metinle sınırlanır. Varsayılan olarak yoktur.
- `.select_all_on_focus()` — alan her odak aldığında bütün metni seçer.
- `.suggestions(satırlar)` — alanın kendi altında sunduğu `Suggestion`'lar, okunacak sırayla. Liste yalnızca `.on_suggestion` ile birlikte görünür; ikisini de vermeyen alan önce gibi davranır. Varsayılan olarak yoktur.
- `.max_suggestions(n)` — listenin kaç tanesini bir arada tuttuğu, varsayılan `8`; ilk `n` tanesi gösterilir.
- `.on_suggestion(|index| msg)` — seçilen satırın yeri, `.suggestions`'a verilen ilk satırdan sayılır.

## Öneriler

- `Suggestion::new(etiket)` — `etiket` yazan bir satır; satırın gösterdiği metin odur.
- `.detail(metin)` — satırın sağındaki soluk not, örneğin bir yolun nereye gittiği.
- `.icon(anahtar)` — etiketten önce çizilen ikon anahtarı; etiketler listedeki en geniş ikonun ardından hizalanır.
- `.label()` — satırın metni.

## Tuşlar

- `left` `right`, `ctrl` ile kelime kelime, `shift` ile seçerek; `home` `end`.
- `backspace` `delete`; `ctrl backspace` ve `ctrl w` bir kelime siler; `ctrl u` başa kadar siler.
- `ctrl a` tümünü seç; `ctrl z` geri al; `ctrl y` ya da `ctrl shift z` yinele.
- `ctrl c` kopyala, `ctrl x` seçimi kes (parola alanında asla); `ctrl v` ve terminalin yapıştırması ekler.
- Seçim varken `left` ve `right` onu kaldırır ve sol ya da sağ ucunun bir adım ötesine gider; `ctrl` ile o uçtan bir kelime.
- `shift f10` ya da `menu` düzenleme menüsünü alanın altında açar.
- `enter` gönderir; `tab` odağı taşır.
- Öneri listesi açıkken `up` `down` bir satır seçer, `enter` onu alır; ilk satırdan `up` ve son satırdan `down` yazdığını bırakır, `enter` o zaman gönderir. Yazmak hiçbir şey seçmez, `esc` listeyi kapatır ve metni korur.

## Fare

- Tıklama imleci yerleştirir; sürükleme seçer. `select_on_focus` verilmiş bir alana odağı getiren tıklama imleci yerleştirir, hiçbir şey seçmez.
- Sağ tık düzenleme menüsünü imlecin yanında açar: Kes, Kopyala, Yapıştır, Tümünü seç. Seçimin içindeyse seçimi korur, başka yerdeyse önce imleci oraya koyar. Kes ve Kopyala seçim ister; Yapıştır sistem panosunda, terminalin panosunda ya da uygulama içindeki bir kopyada metin ister.
- Öneri listesindeki bir satıra tıklamak onu seçer. Başka bir yere basmak listeyi kapatır ve tıklanan yine basılır.

## Tema anahtarları

- `hover`, `focus`, `invalid`, `disabled` ile `text-input` — `bg`, `fg`, `padding`.
- `text-input-prompt`, `text-input-placeholder`, `text-input-selection`, `text-input-cursor`.
- Öneri listesi bir menü gibi çizilir: yüzeyi `context-menu`, bir satır ve seçilenin `▌` sütunu `hover` ile `context-item`, not `context-item-detail`.

## İkonlar

- Metinden önce `prompt`; parolalar için `mask`; bir önerinin `.icon(anahtar)` dediği ikon.

## Dil anahtarları

- `quvyta.edit.cut`, `quvyta.edit.copy`, `quvyta.edit.paste`, `quvyta.edit.select-all`.
