## Metotlar

- `SettingsList::show(ui, |list| …)` — listeyi genişliği doldurarak ekler.
- `list.heading(başlık)` — silik bir grup başlığı; ilkinden sonraki her başlığın üstünde boş bir satır olur.
- `list.row(satır, |ui| …)` — kapanışın eklediği tek kontrolle bir satır.
- `SettingRow::new(etiket)` — tek satırlık ayar.
- `.description(metin)` — etiketin altında silik ikinci satır.
- `.disabled(bool)` — soluk görünür, klavye atlar.
- `.nested(bool)` — üstteki satıra ait bir satır, örneğin onu niteleyen bir seçim: metni iki hücre içeriden başlar; çubuk ve kontrol yerinde kalır.
- Klavye satırının denetimi odaklı çizilir ve tuşları alır; saat ya da süre alanı yazılan bölümü tutar: `12` on iki olur, ← → dakikaya ulaşır.
- `.on_activate(mesaj)` — kontrolün kullanmadığı Enter ya da Boşluk'ta veya etikete tıklanınca gönderilir.

## Davranış

- Etiket iki hücre içeriden başlar; kontrol sağ kenardan iki hücre önce biter.
- Etiket bütçesi: kontrolden iki hücre öncesine kadar olan alan, kayma için bir hücre pay eksik.
- Odaktayken tuşlar: ↑/↓ etkin satırlar arasında gezer; kontrol kullanmıyorsa Home ve End uçlara atlar; diğer her şey önce seçili satırın kontrolüne gider. Listeden kısa bir `ScrollView` içinde tuşla gidilen satır görünecek kadar kaydırılır, fazlası değil; tıklamak hiç kaydırmaz.
- Odak gelince hatırlanan satır, yoksa ilk etkin satır seçilir.
- Fare satırın kontrolü üzerindeyken de satır aydınlık kalır.
- Odaktaki liste yalnızca bir satırı yükseltir: fareyi etkin bir satıra götürmek onu klavyenin satırı yapar, oklar oradan devam eder.

## Tema anahtarları

- `setting-row` — `bg`, `pillar`; durumlar `hover`, `selected`, `focus`, `disabled`.
- `setting-label` — `fg`, `bold`; aynı durumlar.
- `setting-description` — `fg`; aynı durumlar.
- `settings-heading` — `fg`, `bold`.
- `[motion]` — `slide`, `pulse-period`.
- `[icons]` — `pillar`.
