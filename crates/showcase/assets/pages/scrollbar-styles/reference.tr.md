## Tema

- `[style.scrollbar]` — `style`: `block` (varsayılan), `half`, `thin` ya da `dots`; `track`, `thumb` renkleri. Başka her sözcük, geçerli sözcükleri sayan bir uyarı verir.
- `[style."scrollbar:hover"]` — çubuğun üstüne gelinince ya da sürüklenirken renkler.
- `[style."scrollbar.<stil>"]` — yalnızca tek bir stilin renkleri, örneğin `[style."scrollbar.dots"] track = "$muted"`.

## Kod

- `ScrollbarStyle` — `Block` (varsayılan), `Half`, `Thin`, `Dots`; bu sırayla `ScrollbarStyle::ALL`, `.name()`, `ScrollbarStyle::from_name(sözcük)`.
- `List::scrollbar(stil)`, `ScrollView::scrollbar(stil)` — tek bir bileşende stili sabitler.
- `WidgetStyle::word(anahtar)` ve `StyleProps::word(anahtar)` — kendi bileşeninde sözcük özelliğini okur.

## Davranış

- Sağ kenarda tek sütun; yalnızca içerik görünümden uzunsa çizilir.
- Başparmak her stilde üstüne gelinince ya da sürüklenirken parlar.
- `block` her karakter modunda yalnızca arka plan rengi çizer. ASCII modunda `half` ve `thin` başparmakları renkli hücre olur; `dots` izini `.` ile korur.

## İkonlar

- `scroll-track`, `scroll-thumb` (half), `scroll-thin`, `scroll-dot`, `scroll-dot-thumb`. `block` ikon kullanmaz.
