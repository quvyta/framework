## Metotlar

- `Tabs::new(etiketler)` — her etiket için bir sekme.
- `.active(sıra)` — açık sekme. Varsayılan: 0.
- `.numbered(bool)` — etiketlerden önce numara ve numara tuşları. Varsayılan: `false`.
- `.on_select(|sıra| msg)` — başka bir sekme açılınca gönderilir.

## Tuşlar

- Odaktayken: `left` `right` ya da `h` `l` komşuyu açar; `1`–`9` numaralı sekmeyi açar; `ctrl pgup` / `ctrl pgdn` taşan şeridi bir sekme kaydırır.

## Fare

- Açmak için sekmeye tıkla.
- Taşan şeritte bir sekme kaydırmak için oka tıkla ya da tekerleği çevir.

## Tema anahtarları

- `hover`, `selected`, `focus` ile `tab` — `bg`, `fg`, `bold`, `pillar`.
- Aynı durumlarla `tab-index`.
- `tab-arrow` — `bg`, `fg`, `pillar`; `hover`, `pressed`, gösterecek sekmesi kalmayan ok için `disabled`.

## İkonlar

- Oklar için `chevron-left` ve `chevron-right`; üstüne gelinen ve açık sekmeler ile üstüne gelinen ok için `pillar`.
