## Metotlar

- `IconButton::new(anahtar)` — ikon setinden `anahtar` ikonunu gösteren bir buton; örneğin `settings`, `search` ya da `add`.
- `.on_press(msg)` — basıldığında gönderilen mesaj. Etkileşimli olması için gerekli.
- `.tooltip(metin)` — temanın `motion.hover-delay` süresinden sonra, klavye odağındayken de hemen altında görünen sözler. Varsayılan: yok.
- `.disabled(bool)` — soluk; odak almaz, girdiyi yok sayar. Varsayılan: `false`.

## Mesajlar

- `on_press` mesajı, her basışta bir kez.

## Klavye ve fare

- Odaktayken `enter` ya da `space` — basar.
- Sol butonun üç hücreden birinin üstünde bırakılması — basar. Başka yerde bırakmak iptal eder.

## Yerleşim

- Bir boşluk, glif ve bir boşluk: tek hücrelik her glif için Nerd Font, Unicode ve ASCII kipinde üç hücre. Bir satır yüksekliğinde.
- Dururken yüzey yok: hücreler üstünde durdukları zemini korur.

## Tema anahtarları

- `hover`, `focus`, `pressed`, `disabled` durumlarıyla `icon-button` — `bg`, `fg`, `bold`. `focus` yalnızca odak klavyeyle geldiyse uygulanır. Hazır temalar dururken `bg` vermez.
- `tooltip` — sözlerin `bg`, `fg` ve `padding` değeri.

## İkonlar ve metin

- İkon setindeki her ikon anahtarı; `settings` Nerd Font'ta bir dişli, Unicode'da `▤`, ASCII'de `*`.
