## Metotlar

- `Toast::new(ToastKind, başlık)`, `Toast::success(başlık)`, `Toast::warning(başlık)`, `Toast::danger(başlık)`, `Toast::info(başlık)`.
- `.body(metin)` — başlığın altında silik, satır kıran ayrıntı.
- `.action(etiket, msg)` — başlık satırında bir buton; `msg` gönderir ve kapatır.
- `.duration(Duration)` — imleç üstünde değilken ekranda kalma süresi. Varsayılan: 5 sn, eylemle 8 sn.
- `.key(ad)` — aynı anahtarlı bildirim onu yerinde değiştirir.
- `.on_press(msg)` — bildirimi basılabilir yapar: eylemi ve kapatma işareti dışında bir yere tıklamak `msg` gönderir, bildirim kalır. Her basış bir kopya gönderdiği için `Msg: Clone` ister.
- `.icon_motion(SpinnerStyle)` — o tek hücrelik animasyonu ikon hücresinde türün renginde oynatır; hareket azaltılmışsa türün ikonu görünür.

## Komutlar

- `Command::toast(bildirim)` — gösterir.
- `Command::dismiss_toast(anahtar)` — o anahtarlı bildirimi kaldırır.
- `Command::toast_corner(Corner)` — `TopRight`, `BottomRight` (varsayılan), `BottomLeft`, `TopLeft`. `Corner::name()` ve `ToastKind::name()` ayar ekranları için kısa adlar verir.

## Fare

- Üstüne gelmek geri sayımı durdurur. Kapatma işareti boştaki tonunda kalır; yalnızca imleç işaretin üstündeyken üç hücresi aydınlanır.
- İşarete tıklamak kapatır. Eyleme tıklamak mesajını gönderir ve kapatır. Başka bir yere tıklamak hiçbir şey yapmaz ya da `on_press` mesajını gönderip bildirimi yerinde bırakır. Bildirime tıklamak altındakine asla ulaşmaz.

## Tema anahtarları

- `toast` — `bg` (varsayılan `$overlay`), `padding` (varsayılan `[1, 2]`); imleç basılabilir bildirimin üstündeyken `hover` (varsayılan `bg = $active`).
- `toast-title`, `toast-body`.
- `hover` ile `toast-action`; basılabilir bildirim yükseldiğinde `active` (varsayılan `$active` üstüne biraz vurgu, `active:hover` bir kademe yukarıda).
- `close-mark` — `fg`, `bg`, `bold`; işaretin üstünde `hover`. Sekmeler, sekme rayı ve diyaloglarla ortaktır.
- Animasyonlu ikon için `success`, `warning`, `danger`, `info` varyantlarıyla `spinner`.
- İşaret ve ikon `success`, `warning`, `danger` ve `info` renklerini kullanır.
- `[motion] enter` — bildirimin kayarak girme ve çıkma süresi.

## İkonlar

- Türler için `success`, `warning`, `error`, `info`; kapatma işaretinde `close`; animasyonlu ikon için spinner kareleri.
