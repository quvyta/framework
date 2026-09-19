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
- `NodeMut::on_action(kapsam, eylem, mesaj)` — odak düğümde ya da içindeyken eylem `App::action`'a gitmez, `mesaj`'ı gönderir. Cevap veren en içteki düğüm kazanır; odaklı bileşenin kullandığı tuşlar ve çalışma motorunun kendi eylemleri (`quit`, `focus-next`, `focus-prev`, `debug`, `copy`, `paste`, `toggle-panel`) hiç cevaplanmaz. Her eylem için bir kez çağrılır.
- `Keymap::parse`, `.overlay`, `.bind`, `.action_for(tuş)`, `.chords_for(kapsam, eylem)`, `.iter()`, `.conflicts()`.
- `Scope::Global`, `Scope::App`, `scope.label_key(eylem)`.
- `KeyChord`, `"ctrl+shift+p"` metnini okur; `.label()` `ctrl shift p` verir.
- `Runtime::keymap_file(yol)` — gömülü haritanın üstüne bir dosya ekler.
- `Runtime::keymap_source(dosya, metin)` — metin olarak verilen, örneğin bir `include_str!` olan haritayı ekler; kurulan ikili tuşlarını kendi taşır, yanında dosyaya gerek kalmaz. `keymap_file`'dan sonra gelir ve kazanır; ayrıca verilen dosya artık zorunlu değildir: okunamadığında metin onun yerine geçer ve sebep, programı durdurmak yerine bir tanılamaya dönüşür.

## Dil anahtarları

- Global eylemler için `quvyta.keys.<eylem>`, uygulama eylemleri için `keys.<eylem>`.
