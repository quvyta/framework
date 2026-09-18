## Uptime

- `Uptime::now()` — iki tekdüze saati birden okur.
- `.awake` — makine uyurken duran saatin saydığı süre.
- `.elapsed` — durmayan saatin saydığı süre; ikinci saat yoksa `.awake` ile aynıdır.
- `.suspended_since(&earlier)` — iki ölçüm arasında makinenin uykuda geçirdiği süre; ölçümlerin sırası tersse ve uyku ayırt edilemiyorsa sıfır.
- `Uptime::detects_suspend()` — Linux ve Android'de `true`, diğer her yerde `false`.
- Kullanılan saatler: Linux ve Android'de `CLOCK_MONOTONIC` ile `CLOCK_BOOTTIME`, `rustix` üzerinden okunur, böylece `unsafe` gerekmez. Diğer Unix sistemlerinde yalnızca `CLOCK_MONOTONIC` okunur; macOS'un `CLOCK_UPTIME_RAW`'ı ve Windows'un `QueryUnbiasedInterruptTime`'ı yabancı işlev çağrısı istediği için okunmaz. Unix dışında `.awake` süreçteki ilk ölçümden sayılır.

## Date

- `Date::new(yıl, ay, gün) -> Option<Date>`, `Date::today_utc()`, `Date::today_local()`, `Date::from_days(n)`.
- `Date::parse(metin) -> Result<Date, String>` ve `metin.parse::<Date>()` — `YYYY-MM-DD`; yıl dört ya da daha çok hane, ay ve gün ikişer hane, başta isteğe bağlı bir `-`, çevresindeki boşluklar yok sayılır. Takvim gerçekten denetlenir.
- `Display` — `parse`'ın okuduğu biçimi, `YYYY-MM-DD`, geri yazar.
- `.year()`, `.month()`, `.day()`, `.weekday()`, `.to_days()` — 1970-01-01'den beri gün.
- `.add_days(n)`, `.add_months(n)`, `.first_of_month()`, `.start_of_week(Weekday)`.
- `date::is_leap_year(yıl)`, `date::days_in_month(yıl, ay)`.

## TimeOfDay

- `TimeOfDay::new(saat, dakika, saniye)` — her parça en büyük değerinde durdurulur; `TimeOfDay::LARGEST` 23:59:59'dur.
- `TimeOfDay::parse(metin) -> Option<TimeOfDay>` — `s:dd` ya da `s:dd:ss`; aralık dışı değer durdurulmaz, reddedilir.
- `.seconds_since_midnight()`, `TimeOfDay::from_seconds_since_midnight(n)` — bir günü aşan değer başa sarar.
- `Display` — `ss:dd:ss`. `hour`, `minute`, `second` alanları herkese açıktır; saatler saat sırasına göre karşılaştırılır.
- Tip `date` içinde durur ve `widgets`'tan yeniden dışa verilir, böylece `TimeInput` ile `date` aynı tipten söz eder.

## DateTime ve yerel fark

- `DateTime { date, time, offset_minutes }` — yerel bir tarih ve saat, yanında onlara ait UTC farkı.
- `DateTime::now_local()`, `DateTime::from_unix(saniye, fark_dakika)`, `.to_unix()`.
- `date::local_offset() -> Option<i16>` — yerel saatin UTC'den kaç dakika ileride olduğu; sistem söylemiyorsa `None`.
- `date::local_offset_minutes() -> i16` — aynısı, bilinmiyorsa 0.
- Kaynak: `TZ`'nin gösterdiği ya da `/etc/localtime` olan TZif dosyası, süreç başına bir kez okunur. `EST5EDT,M3.2.0,M11.1.0` gibi POSIX kuralı tutan bir `TZ` ayrıştırılmaz ve bilinmiyor sayılır; dosyanın kaydettiği son değişimden sonraki anlar onun verdiği son farkı korur.

## Girdisiz geçen süre

- `ui.idle_for() -> Duration` — bu terminale ne kadar süredir girdi gelmediği; ilk girdiye kadar uygulamanın başlangıcından sayılır. Onu okumak, görünüm okuduğu sürece, sessizliğin her tam saniyesinde görünümü yeniden çizdirir.
- `ui.on_idle(after, |away| …)` — sessizlik `after`'a ulaştığı anda bir kez `message(true)`; ondan sonraki ilk girdide, girdi bir bileşene ulaşmadan önce `message(false)`. Etkin kalması gereken her karede tanımla; `after`'ı farklı izleyiciler birbirinden bağımsızdır.
- Girdi sayılanlar: tuş basma, tekrar ve bırakma; fare tuşları, tekerlek, sürükleme ve imlecin pencere üzerinde gezinmesi; yapıştırma; bir `Handoff`'un bitişi.
- Girdi sayılmayanlar: terminalin yeniden boyutlandırılması, mesajlar, perform işleri ve görevler, çalışma zamanının kendi ürettiği olaylar.
- Testler: `Harness::advance` sessizliği ilerletir ve zamanı gelen izleyicilere haber verir; `press`, `click`, `hover`, `paste` ve `events` onu yeniden başlatır; `send` ve `resize` başlatmaz.
- Kapsam dışı: makinenin tamamının, başka terminallerin ve başka programların boşta kalması.

## Tuşlar

- Tarih alanı her metin girişi gibi metin alır; bu sayfada kendine ait tuşu olan bir şey yok.
