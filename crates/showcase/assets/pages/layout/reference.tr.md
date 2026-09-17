## View

- `ui.add(bileşen)` — bir bileşen ekler ve düğümünü döndürür.
- `ui.add_with(kapsayıcı, |ui| ...)` — `Panel` ya da `ScrollView` gibi çocuk tutan bir bileşen ekler.
- `ui.column(|ui| ...)`, `ui.row(|ui| ...)`, `ui.stack(|ui| ...)` — kapsayıcılar.
- `ui.page(anahtar, |ui| ...)` — gizliyken odağı ve kaydırmayı hatırlayan bir sütun.
- `ui.spacer()` — dolduran boş alan.
- `ui.env()` — tema, ikonlar, dil ve kısayol haritası.

## Düğüm

- `.id(isim)` — kalıcı bir isim; durum ve odak onu takip eder.
- `.width(Length)`, `.height(Length)`, `.fill()`, `.fill_width()`, `.fill_height()`.
- `.padding(Padding)`, `.gap(hücre)`, `.justify(Align)`, `.align(Align)`.
- `.selectable(bool)` — `true` düğümü fareyle sürükleyince metin seçilen bir bölge yapar; `false` seçimi düğümün ve içindeki her şeyin dışında tutar.

## Length ve Align

- `Length::Auto`, `Length::Cells(n)`, `Length::Fill(ağırlık)`.
- `Align::Start`, `Align::Center`, `Align::End`.

## Router

- `Router::new(kök)`, `.current()`, `.push(sayfa)`, `.back() -> bool`, `.replace(sayfa)`, `.can_go_back()`, `.history()`, `.direction()` (sayfa geçişleri için `Navigation::Forward` ya da `Navigation::Back`).

## AppShell

- `.header`, `.sidebar`, `.body`, `.footer` kapanışları ve `.show(ui)` ile `AppShell::new()`.
- `.sidebar_width(sütun)` (varsayılan 28), `.collapse_below(sütun)` (varsayılan 90), `.sidebar_open(bool)`.
- Tema anahtarları: `shell-header`, `shell-sidebar`, `shell-body`, `shell-footer` (`bg`).
