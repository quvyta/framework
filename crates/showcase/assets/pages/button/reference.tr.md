## Metotlar

- `Button::new(etiket)` — `etiket` gösteren bir buton.
- `.on_press(msg)` — basıldığında gönderilen mesaj. Etkileşimli olması için gerekli.
- `.variant(ad)` — `primary` ya da `danger` gibi tema varyantı. Varsayılan: yok.
- `.shortcut(etiket)` — solda koyu bir bölümde tuş etiketi, örneğin `⏎` ya da `ctrl s`. Bölüm çubuk hücresiyle başlar. Varsayılan: yok.
- `.icon(anahtar)` — etiketten önce ikon setinden bir ikon. Varsayılan: yok.
- `.disabled(bool)` — soluklaştırır; odak almaz, girdiyi yok sayar. Varsayılan: `false`.
- `.loading(bool)` — ikonun yerine spinner; basışları yok sayar. Varsayılan: `false`.
- `.selected(bool)` — butonu birkaç görünüm modundan biri gibi bir seçim butonu yapar: seçiliyken yükselmiş yüzey ve sabit bir çubukla görünür. Seçim butonu basınca parlamaz. Varsayılan: sıradan buton.

## Mesajlar

- `on_press` mesajı, her basışta bir kez.

## Klavye ve fare

- Odaklıyken `enter` ya da `space` — basar.
- Sol tuş butonun üzerinde bırakılırsa — basar. Başka yerde bırakmak iptal eder.
- Tuşu basılı tutmak — tek basış.

## Yerleşim

- Çubuk hücresi, kısayol bölümü (boşluk, tuş, boşluk), sol iç boşluk, ikon ve bir boşluk, etiket, sağ iç boşluk.
- Çubuk hücresi yalnızca kısayol varsa ve sol iç boşluk en az bir hücreyse eklenir; kısayol yoksa çubuk sol iç boşluğun ilk hücresindedir. Her iki durumda da en soldaki hücredir ve hover ile odakta hiçbir şeyi kaydırmadan belirir.

## Tema anahtarları

- `hover`, `focus`, `pressed`, `selected`, `disabled` durumlarıyla `button` — `bg`, `fg`, `bold`, `padding`, `pillar`. `focus` yalnızca odak klavyeyle geldiğinde uygulanır.
- Aynı durumlarla `button.<varyant>`.
- `button-key` ve `button-key.<varyant>` — kısayol bölümü.

## İkonlar ve metin

- Yüklenirken varsayılan spinner stilinin kareleri (animasyon `spinner-arc`); `.icon` için herhangi bir ikon anahtarı.
