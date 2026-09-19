## Metotlar

- `Tabs::new(etiketler)` — her etiket için bir sekme.
- `.active(sıra)` — açık sekme. Varsayılan: 0.
- `.numbered(bool)` — etiketlerden önce numara ve numara tuşları. Varsayılan: `false`.
- `.on_select(|sıra| msg)` — başka bir sekme açılınca gönderilir.
- `.badge(sıra, sayı)` — `sıra` numaralı sekmenin adından bir boşluk sonra bir sayı; 99'un üstü `99+` okunur, sıfır hiçbir şey göstermez ve yer tutmaz. Son sekmeden sonraki bir sıra yok sayılır. Varsayılan: sayı yok.

## Tuşlar

- Odaktayken: `left` `right` ya da `h` `l` komşuyu açar; `1`–`9` numaralı sekmeyi açar; `ctrl pgup` / `ctrl pgdn` taşan şeridi bir sekme kaydırır.

## Fare

- Açmak için sekmeye tıkla.
- Taşan şeritte bir sekme kaydırmak için oka tıkla ya da tekerleği çevir.

## Tema anahtarları

- `hover`, `selected`, `focus` ile `tab` — `bg`, `fg`, `bold`, `pillar`.
- Aynı durumlarla `tab-index`.
- Aynı durumlarla `tab-badge` — sayının `fg` ve `bold` değeri.
- `tab-arrow` — `bg`, `fg`, `pillar`; `hover`, `pressed`, gösterecek sekmesi kalmayan ok için `disabled`.

## Yerleşim

- Bir sekme: iki hücre boşluk, numaralıysa numara ve bir boşluk, ad, sayısı varsa bir boşluk ve sayı, kapatılabiliyorsa kapatma işareti, kayma için bir yedek hücre ve iki hücre boşluk.
- Her şeye yetmeyen dar bir sekmede önce ad `…` ile kısalır; sayı ve kapatma işareti hücrelerini korur. Sayı adla birlikte kaymaz.
- Gizli sekmelerin menüsünde sayı, adından iki boşluk sonra gelir.

## İkonlar

- Oklar için `chevron-left` ve `chevron-right`; üstüne gelinen ve açık sekmeler ile üstüne gelinen ok için `pillar`.
