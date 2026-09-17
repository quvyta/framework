## Metotlar

- `CommandPalette::new(komutlar, kapatma_mesajı)` — palet; Esc, × işareti, dışarı tıklamak ve bir komutu çalıştırmak `kapatma_mesajı`nı gönderir.
- `.dismissable(bool)` — Esc, × işareti ve dışarı tıklama birlikte kapatır mı. Varsayılan: `true`. `false` işareti gizler; komut çalıştırmak yine kapatır.
- `.keymap(bool)` — kısayol haritasının eylemlerini de listeler (odak geçişleri ve paletin kendisi hariç). Kimlikleri `action:<ad>`. Varsayılan: `false`.
- `.recent(kimlikler)` — süzgeç boşken en yenisi önde listelenen kimlikler.
- `.on_run(|kimlik| mesaj)` — çalışan her komuttan sonra gönderilir.
- `.placeholder(metin)` — boş süzgeçteki silik metin. Varsayılan: `quvyta.palette.placeholder`.
- `.width(hücre)` — iç boşluk dahil genişlik. Varsayılan: 72.
- `PaletteCommand::new(kimlik, etiket, mesaj)` — bir komut.
- `.chord(etiket)` — sağda gösterilen tuş, örneğin `"ctrl r"`.

## Tuşlar

- Yazmak, `backspace`, `ctrl w`, `ctrl u`, `left` `right` `home` `end` süzgeci düzenler; yapıştırmak da çalışır.
- `up` `down` ya da `ctrl p` `ctrl n` vurguyu taşır; `pgup` `pgdn` bir sayfa taşır.
- `enter` vurgulanan komutu çalıştırır; `esc` palet kapatılabilirken kapatır.

## Fare

- Fareyi bir satırın üstüne getirmek tek vurguyu oraya taşır; palet açılırken yerinde duran fare, hareket edene kadar bekler.
- Bir satıra tıklamak onu çalıştırır; tekerlek kaydırır, kaydırma çubuğu basılıp sürüklenebilir.
- Yüzeyin sağ üst köşesindeki, süzgecin bir üst satırındaki × işareti kapatır: fare üstüne gelince aydınlanan üç hücre. Karartılmış ekrana tıklamak da kapatır. İkisi de yalnızca palet kapatılabilirken.

## Davranış

- Süzgeç doluyken girdiler eşleşme kalitesine göre sıralanır (eşitler sırasını korur), başlık olmadan.
- Çalıştırmak önce `kapatma_mesajı`nı, sonra komutun mesajını ya da kısayol eylemini, en son `on_run` mesajını gönderir.
- Kaydırmadan önce en çok 10 satır; yeniden açılınca süzgeç, vurgu ve kaydırma sıfırlanır.
- İpucu satırı `esc kapat` (palet kapatılabilirken), `↑↓ gez`, `⏎ çalıştır` ve eşleşen komut sayısını gösterir.
- Yüzeyin sol kenarı boyunca bir çubuk uzanır; vurgulanan satırın kendi çubuğu o satırda onu parlatır.
- Ekranın üst kısmına yerleşen modal bir katman; kapanınca odak geri döner.

## Tema anahtarları

- `modal` (`bg`, `padding`, `pillar`), `layer-backdrop` — yüzey; `close-mark` — × işareti.
- `layer-filter`, `layer-filter-mark`, `layer-filter-placeholder`, `layer-filter-cursor`, `layer-match`.
- `hover` durumlu `palette-item` (`bg`, `fg`, `pillar`), `hover` durumlu `palette-chord`, `palette-header`.
- `layer-hint-key`, `layer-hint-label`, `scrollbar`; `[motion]` — `enter`, `slide`.
- Dil — `quvyta.palette.placeholder`, `.recent`, `.all`, `.empty`; `quvyta.layer.close`, `.move`, `.run`.
