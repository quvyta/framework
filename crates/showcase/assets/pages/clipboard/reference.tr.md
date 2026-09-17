## Metotlar

- `CopyValue::new(değer)` — kendini kopyalayan, yükseltilmiş zeminde bir değer.
- `.masked(bool)` — maske noktaları çizer ama gerçek değeri kopyalar. Varsayılan: `false`.
- `.disabled(bool)` — soluk, odaklanamaz, kopyalamaz. Varsayılan: `false`.
- `.on_copy(msg)` — her kopyadan sonra gönderilir.
- `Command::copy(metin)` — terminal panosuna (OSC 52) ve uygulama içi panoya kopyalar.
- `Command::read_clipboard(|Option<String>| msg)` — panodaki metni sonraki bir güncellemede getirir: önce sistem panosu, sonra terminalin panosu (OSC 52), en son uygulama içindeki son kopya; hiçbirinde metin yoksa `None`.
- `EventCx::copy(metin)` — aynı kopya, bir bileşenin içinden; `App::clipboard`'a bildirilir.
- `App::clipboard(&self, &ClipboardEvent) -> Option<Msg>` — bileşenlerden, menülerden ve kopyalama tuşundan `Copied(metin)`, hiçbir bileşenin almadığı yapıştırmada `Pasted(metin)`.
- `Harness::clipboard()` ve `Harness::copied()` — testler için uygulama içi kopya ve şimdiye kadarki bütün kopyalar; `Harness::paste(metin)` terminalin yapıştırmasını taklit eder; `Harness::set_system_clipboard(Some(metin))` sistem panosunun yerine geçer; test sürücüsü gerçek panoyu hiç okumaz.

## Davranış

- Enter, Space, `c` ya da tıklama CopyValue'yu kopyalar, parlatır ve başarı işaretini 1,4 saniye gösterir; işaret `motion.enter` süresinde eski rengine döner.
- Dar alan: değer `…` ile kısalır; işaret hep tam görünür.
- `paste` eylemi (`ctrl+v`) ve Yapıştır seçenekleri metni olan ilk kaynaktan odaklı bileşene yapıştırır: sistem aracı (`wl-paste --no-newline --type text`, `xclip -o -selection clipboard`, `xsel --clipboard --output`, `pbpaste`; kabuk yok, en fazla 500 ms, ayrı iş parçacığında), terminalin OSC 52 sorusuna cevabı (200 ms), uygulama içindeki son kopya. Hiçbirinde metin yoksa bir şey olmaz ve Yapıştır seçenekleri pasiftir.
- Fareyle seçip bırakmak bir şey kopyalamaz; `copy` eylemi seçimi temiz kopyalar, seçimin sağ tık menüsü Kopyala ve Ham kopyala sunar.
- Uygulamanın `Command::copy` ile istediği kopyalar `App::clipboard`'a bildirilmez.

## Tuşlar

- `[global] paste = "ctrl+v"`, etiketi `quvyta.keys.paste`.
- `[global] copy = "ctrl+c"`, etiketi `quvyta.keys.copy` — fareyle yapılan seçimi temiz kopyalar.

## Tema anahtarları

- `copy-value`; `hover`, `focus`, `pressed`, `disabled` durumlarıyla — `bg`, `fg`, `padding`.
- `copy-value-marker` (`fg`) ve `copy-value-marker.copied`.

## Dil anahtarları

- `quvyta.copy-value.copy`, `quvyta.copy-value.copied`; onay işareti `check` ikonundan gelir.
- Sağ tık menüleri için `quvyta.edit.copy`, `quvyta.edit.raw-copy`, `quvyta.edit.cut`, `quvyta.edit.paste`, `quvyta.edit.select-all`.
