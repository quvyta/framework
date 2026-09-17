## Metotlar

- `SidePanel::new(genişlik)` — sola yerleşmiş açık panel; `.panel(|ui| ...)`, `.body(|ui| ...)`, `.show(ui)`.
- `.side(Side::Left | Side::Right)` — yerleştiği kenar.
- `.open(bool)` — panelin açık olup olmadığı.
- `.on_toggle(|açık| msg)` — kenardaki düğme, kenarda Enter ve `toggle-panel` açıp kapatır.
- `.on_resize(|genişlik| msg)` — sürükleme ya da ok tuşları boyutlar.
- `.limits(min, max)` — `max` bir sayı ya da üst sınır olmaması için `None`; varsayılan 8 ve `None`.
- `.strip(ikonlar, |sıra| msg)` — dış kenarda görünüm ikonlarından bir etkinlik çubuğu; mesajın anlamı "bu görünümü göster".
- `.active_view(sıra)` — panelin gösterdiği görünüm; panel açıkken ikonu vurgu çubuğuyla yükselir, tıklamak paneli kapatır.
- `.closed(Closed::Collapse | Closed::Hide)` — kapanınca şerit kalır (varsayılan) ya da o da gizlenir ve tek bir kenar sütunu kalır.

## Tuşlar

- Panelin ya da gövdenin içinde: `alt+b` (genel `toggle-panel` eylemi) açıp kapatır.
- Odaktaki kenarda: `enter` `space` açıp kapatır; `left` `right` boyutlar, `shift` ile beş hücre; `home` `end` sınırlara atlar.
- Odaktaki şeritte: `up` `down` gezer, `home` `end` ilk ve son ikona atlar; `enter` `space` tıklama gibi çalışır.

## Fare

- Düğme için kenara gel; açıp kapatmak için düğmeye tıkla; boyutlamak için kenarın geri kalanını sürükle; açmak için kapalı kenarı gövdeye doğru sürükle.
- Şerit ikonları: başka bir ikon görünümü değiştirir (kapalı paneli de açar); gösterilen görünümün ikonu paneli kapatır.

## Davranış

- Sınırlar ne olursa olsun gövde alanın en az dörtte birini korur; `min`'den küçük bir `max` `min` sayılır.
- Düğme kenarın orta satırındadır: önce çubuk hücresi, sonra ok; kenar hücresini ve gövdenin bir hücresini kaplar. Kenara ya da düğmeye gelinince, sürüklerken ve kenar klavyeyle odaklanınca görünür. Vurgu çubuğu `▌` yalnızca imleç düğmenin üstündeyken ya da klavye odağı görünürken çizilir; yalnızca kenara gelinmişse ilk hücre düğmenin tonunda boş kalır.
- Ton basamakları: kenara gelinmiş (`side-toggle`) < düğmeye gelinmiş (`hover`) < basılı ya da yeni etkinleşmiş (`pressed`); klavye odağında (`focus`) çubuk nefes alır. Düğmede başlayan basış sürükleme başlatmaz.
- Açılış ve kapanış iki `motion.enter` süresince kayar; içerik kayarken genişliğini korur.
- `on_toggle` ve `on_resize` yoksa kenar tepkisizdir ve odaklanmaz.
- Şerit 4 sütundur: vurgu çubuğu, iki hücrelik ikon ve panele bakan boş bir sütun. Panel açıkken de dış kenarda kalır; panelin genişliğine dahil değildir, gövde yine alanın dörtte birini korur.
- Şerit ton basamakları: ikon (`side-strip-item`) < üstüne gelinmiş (`hover`) < gösterilen görünüm (`selected`, sabit çubuk). Klavye odağındaki ikonun (`focus`) çubuğu nefes alır; Enter onu bir kez parlatır (`pressed`). Seçili görünüş yalnızca panel açıkken çizilir.
- `Closed::Hide`: kapanışta panel ve şerit birlikte kayarak çıkar; gövde alanı alır, yalnızca kenarda bir sütun kalır. O sütun dururken gövdenin tonundadır, imleç gelince `split-handle` görünüşünü ve düğmeyi alır. Gizlenen şerit klavye odağını kenara bırakır.
- Kapalı kenar en küçük genişliğin yarısı kadar gövdeye sürüklenince panel sürüklenen genişlikte açılır.
- Tab sırası: kenar, şerit, panelin içeriği, gövde.

## Tema anahtarları

- `side-panel` — `bg`. `side-strip` — `bg`. `side-strip-item` — `fg`, `bg`, `pillar`; durumlar `hover`, `selected`, `focus`, `pressed`.
- `split-handle` — `bg`, `fg`; durumlar `hover`, `focus`, `active`.
- `side-toggle` — `bg`, `fg`, `pillar`; durumlar `hover`, `focus`, `pressed`.
- İkonlar — `edge-left`, `edge-right`. Kısayol — `[global] toggle-panel`; etiket `quvyta.keys.toggle-panel`.
