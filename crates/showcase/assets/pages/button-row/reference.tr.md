## Metotlar

- `ButtonRow::new()` — içi henüz boş bir sıra.
- `.button(Button)` — sıranın sonuna bir düğme ekler, her düğme için bir kez; eklenme sırası okuma sırasıdır ve Tab'ın gezdiği sıra da odur.
- Sade metotlar aynen kalır: her `Button` metodu ve bir düğümün yerleşim metotları (`.fill_width()`, `.width(Length::Cells(n))`, `.id(..)`).

## Davranış

- Düğmeler soldan başlayarak, iki hücre arayla, her birinin istediği genişlikte yerleşir.
- Her düğme sığdığı sürece satır sade bir düğme sırasıdır ve içinde başka denetim yoktur.
- Sığmadığında, menüyü açan denetim sığmayan ilk düğmenin duracağı yerde durur ve son düğmeler, sondan başlayarak, onda listelenir. İlk düğmeye bile yer olmayan satır denetimi yalnız, eline düşen genişliğe kesilmiş olarak gösterir.
- Bir girdi seçmek, o düğmenin basılınca göndereceği mesajı gönderir ve menüyü kapatır. `disabled` olan bir düğme herkes gibi listelenir ve seçilince yine bir şey göndermez.
- Satır düğmeleri kadar geniş ölçülür; sığmadığı yere verilen genişliği alır ve bir satır yüksektir (temada düğmelere dikey iç boşluk varsa en yüksek düğme kadar).

## Tuşlar

- `tab` düğmeleri soldan başlayarak birer birer, sonra menüyü açan denetimi bulur; bir sonraki `tab` satırdan çıkar.
- Denetimde `enter` ya da `space` menüyü açar. Menüde `up` `down` `home` `end` gezer, harf yazmak bir girdiye atlar, `enter` onu alır ve `esc` kapatır.

## Fare

- Düğmeye tıkla basılsın, denetime tıkla menü açılsın, menüde bir girdiye tıkla o düğmenin mesajı gitsin.
- Menü açıkken bir düğmenin yanına tıklamak menüyü kapatır ve yine o düğmeye basar.

## Tema anahtarları

- `button` ve çeşitleri — düğmeler ve menüyü açan denetim; denetim her buton gibi `hover`, `focus` (çubuk nefes alır) ve `pressed` durumlarını alır.
- `popup-menu`, `popup-item` (`hover`), `popup-check` — sığmayan düğmeler menüsü.

## İkonlar

- Denetimin etiketinden önce `chevron-down`; sözcükleri `quvyta.button-row.more` dil anahtarıdır.
