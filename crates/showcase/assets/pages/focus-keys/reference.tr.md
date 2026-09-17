## Global eylemler

- `quit` — `ctrl+q`; uygulamadan çıkar.
- `focus-next` — `tab`; `focus-prev` — `shift+tab`.
- `debug` — `f12`; hata ayıklama katmanını açıp kapatır.
- `copy` — `ctrl+c`; fareyle yapılan seçimi temiz kopyalar.
- `paste` — `ctrl+v`; önce sistem panosundan, sonra terminalin panosundan, en son uygulama içindeki son kopyadan odaklı bileşene yapıştırır.
- `toggle-panel` — `alt+b`; yan paneli, panelin ya da gövdesinin içindeki her yerden açıp kapatır.
- `help` — `?` ve `palette` — `ctrl+p`; çalışma motoru bunları kendisi işlemez, `App::action`'a iletir; uygulama orada yardım katmanını ve komut paletini açar.

## Kısayol dosyası

- `[global]` ve `[app]` tabloları; değerler `"ctrl+s"` gibi tek bir tuş ya da tuş listesi.
- Tuş adları: karakterler, `f1`–`f24`, `enter esc tab space backspace delete insert home end pgup pgdn up down left right menu`.
- Değiştiriciler: `ctrl`, `alt`, `shift`. Shift simgelerin içindedir: `shift+/` değil `?` yaz. Harflerde ayrı yazılır: `shift+s` ya da kısaca `S`; büyük harf, shift ile o harf demektir. Değiştirici adları ve adı olan tuşlar (`Enter`, `Tab`, `PgDn`) büyük küçük harf ayırmaz.

## Kod

- `App::action(isim) -> Option<Msg>` — uygulama eylemleri.
- `Command::focus(isim)` — adlandırılmış bileşene odaklan.
- `Keymap::parse`, `.overlay`, `.bind`, `.action_for(tuş)`, `.chords_for(kapsam, eylem)`, `.iter()`, `.conflicts()`.
- `Scope::Global`, `Scope::App`, `scope.label_key(eylem)`.
- `KeyChord`, `"ctrl+shift+p"` metnini okur; `.label()` `ctrl shift p` verir.
- `Runtime::keymap_file(yol)` — gömülü haritanın üstüne bir dosya ekler.

## Dil anahtarları

- Global eylemler için `quvyta.keys.<eylem>`, uygulama eylemleri için `keys.<eylem>`.
