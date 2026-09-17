## Metotlar

- `HelpLayer::new(kapatma_mesajı)` — katman; Esc ve × işareti `kapatma_mesajı`nı gönderir.
- `.dismissable(bool)` — Esc ve × işareti birlikte kapatır mı. Varsayılan: `true`. `false` işareti gizler; katmanı uygulama kapatır.
- `.hint(tuş, etiket)` — o ekrana özgü bir tuş; "Bu ekran" altında ilk sırada listelenir.
- `.width(hücre)` — iç boşluk dahil genişlik. Varsayılan: 64; dar ekranda küçülür.

## Tuşlar

- Yazmak, `backspace`, `ctrl w`, `ctrl u`, `left` `right` `home` `end` süzgeci düzenler; yapıştırmak da çalışır.
- `up` `down` bir satır, `pgup` `pgdn` bir sayfa kaydırır.
- `esc` katman kapatılabilirken kapatma mesajını gönderir.
- Genel `help` eylemi (varsayılan `?`) `App::action("help")` çağrısına ulaşır; katmanı orada aç.

## Fare

- Tekerlek listeyi kaydırır; kaydırma çubuğu basılıp sürüklenebilir.
- Yüzeyin sağ üst köşesindeki, başlığın bir üst satırındaki × işareti kapatır: fare üstüne gelince aydınlanan üç hücre, yalnızca katman kapatılabilirken çizilir.
- Karartılmış ekrandaki diğer her şey yutulur.

## Davranış

- Gruplar: ekran ipuçları, `[app]` eylemleri, `[global]` eylemleri; eşleşmesi olmayan gruplar gizlenir.
- Etiketler: etkin dilde `keys.<eylem>` ve `quvyta.keys.<eylem>`; bir eylemin tüm tuş birleşimleri gösterilir.
- Kaydırmadan önce en çok 18 satır, kısa ekranda daha az; yeniden açılınca süzgeç ve kaydırma sıfırlanır.
- Modal bir katman: sol kenar boyunca çubuk, odak tuzağı, duraklatılmış uygulama kısayolları, kapanınca geri dönen odak.
- İpucu satırı katman kapatılabilirken `esc kapat`, liste kayarken `↑↓ kaydır` gösterir.

## Tema anahtarları

- `modal` (`bg`, `padding`, `pillar`), `modal-title`, `layer-backdrop` — yüzey; `close-mark` — × işareti.
- `layer-filter` (`bg`, `fg`), `layer-filter-mark`, `layer-filter-placeholder`, `layer-filter-cursor`, `layer-match`.
- `help-group`, `help-key` (`bg`, `fg`, `bold`), `help-label`.
- `layer-hint-key`, `layer-hint-label`, `scrollbar`.
- `[icons]` — `search`, `close`, `pillar`, `scroll-thumb`, `scroll-track`.
- Dil — `quvyta.help.title`, `.screen`, `.app`, `.global`, `.empty`; `quvyta.layer.filter`, `.close`, `.scroll`.
