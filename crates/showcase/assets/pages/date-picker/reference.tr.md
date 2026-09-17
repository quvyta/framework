## Metotlar

- `DatePicker::new(Option<Date>)` — seçili tarih.
- `.on_change(|tarih| msg)` — farklı bir gün seçildiğinde gönderilir.
- `.placeholder(metin)` — tarih seçilmemişken gösterilir.
- `.today(Date)` — bugün olarak işaretlenen gün. Varsayılan: `Date::today_utc()`.
- `.disabled(bool)` — odak almaz, açılamaz. Varsayılan: `false`.

## Date

- `Date::new(yıl, ay, gün) -> Option<Date>`, `Date::today_utc()`, `Date::from_days(n)`.
- `.year()`, `.month()`, `.day()`, `.weekday()`, `.to_days()` (1970-01-01'den beri geçen gün).
- `.add_days(n)`, `.add_months(n)` (ayın son gününe kırpar), `.first_of_month()`, `.start_of_week(Weekday)`.
- `date::is_leap_year(yıl)`, `date::days_in_month(yıl, ay)`; `Weekday::ALL`, `.number()`, `Weekday::from_number(n)`, `.days_since(başlangıç)` (`başlangıç` gününden ileri doğru gün sayısı, `0..7`).

## Tuşlar

- Kapalı: `enter`, `space` ya da `down` açar.
- Açık: `left` `right` bir gün, `up` `down` bir hafta, `pgup` `pgdn` bir ay, `shift pgup` `shift pgdn` bir yıl, `home` `end` haftanın uçları, `enter` ya da `space` seçer, `esc` kapatır, `tab` kapatıp odağı taşır.

## Fare

- Açıp kapatmak için alana tıkla; seçmek için bir güne tıkla; ayı değiştirmek için oklara tıkla ya da tekerleği kullan.
- Fareyi günlerin üstünde gezdirmek tek vurgulu günü taşır.
- Alan, iki ay oku ve vurgulu gün, en soldaki ve hep boş olan hücrelerinde çubuğu gösterir. Kaydırma ayarı ne olursa olsun hiçbir şey kaymaz. Çubuk yalnızca vurguyu en son klavye taşıdıysa nefes alır.
- Dışarı tıklamak kapatır ve tıklama düştüğü yerdeki şeye de ulaşır.

## Dil anahtarları

- `quvyta.date.month-1` … `month-12`, `weekday-1` (pazartesi) … `weekday-7` (pazar).
- `quvyta.date.first-weekday` — `1` pazartesi ile `7` pazar arası.
- `quvyta.date.format` — `{day}`, `{month}`, `{year}` ile alan metni; `quvyta.date.title` — takvim başlığı.

## Tema anahtarları

- Alan: `select`, `select-placeholder`, `select-chevron`.
- `calendar` — `bg` (varsayılan `$overlay`), `padding` (varsayılan `[1, 2]`).
- `calendar-title`, `hover` ile `calendar-arrow` (`bg` üç hücresini aydınlatır, `pillar` ilk hücresinde), `calendar-weekday`.
- `calendar-day`: `hover` (vurgulu gün: `bg`, `pillar`), `hover:focus` (klavye taşıdığında `pillar`), `selected`, `selected:hover` ve `selected:hover:focus` (vurgu dolgusu üstündeki `pillar`); `outside` ve `today` varyantları.
