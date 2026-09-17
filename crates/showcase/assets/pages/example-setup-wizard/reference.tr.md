## Metotlar

Örnek şu framework bileşenlerini kullanır; her birinin tam referansı kendi sayfasındadır.

- `Wizard` — `current`, `on_back`, `on_next`, `on_finish`, `on_cancel`, `on_step`, `page_height`.
- `Form` ve `Field` — `label_width`, `required`, `hint`, `error`; `check` ve `focus_first` ile `FormErrors`.
- `TextInput` — `placeholder`, `max_length`, `invalid`, `on_change`.
- `RadioGroup` — container motoru.
- `SettingsList` ve `SettingRow` — tema `Select`, ikonlar `Segmented`, animasyonlar `Switch`.
- `Checkbox` — isteğe bağlı özellikler.
- `CodeView` — proje dosyası önizlemesi, `Language::Toml`.
- `Steps` (`vertical`, `running`) ve `ProgressBar` (`percent`) — oluşturma ilerlemesi.
- `Command::focus`, `Command::perform` — sonraki sayfaya odak, arka plan işi.

## Davranış

- 1. adım en az iki küçük harf, rakam ya da tireden oluşan ve tireyle başlamayan bir ad, bir de konum ister.
- 2. adım bir motor ister.
- İleri odağı yeni adımın ilk kontrolüne taşır; engellenen İleri ilk soruna odaklanır.
- Bitir dört aşamayı arka planda, her biri bir mesajla çalıştırır, sonra projenin nasıl çalıştırılacağını gösterir.
- Vazgeç, Esc ve "Başka bir proje kur" bütün yanıtları sıfırlar.

## Tema anahtarları

- Örneğin kendi anahtarı yoktur; tema bileşenlerini nasıl biçimliyorsa öyle görünür.
