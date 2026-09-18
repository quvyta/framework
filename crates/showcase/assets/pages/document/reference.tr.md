## Shape

- `Shape::new()` — henüz hiçbir şey tanımlamayan bir tablo; `Default` da aynısını verir.
- `.required(key, kind)` — belgenin taşımak zorunda olduğu anahtar. Eksikse tablonun başında hata, başka türden bir değerse durduğu yerde hata olur.
- `.optional(key, kind)` — taşıyabileceği anahtar. Eksik olması sorun değildir, başka türden bir değer uyarıdır.
- `.table(key, shape)` — bunun içindeki tablo. Tablonun kendisi isteğe bağlıdır; zorunlu anahtarları o tablo varsa zorunlu olur. `[key]` yazımını da noktalı `key.name` yazımını da aynı okur.
- `.entries(key, shape)` — tablo dizisi, tekrarlanan `[[key]]`; her kayıt kendi başına denetlenir. Hiç kayıt listelemeyen belgede hiç kayıt yoktur.
- Anahtarlar tek addır, noktalı yol değil. Bir adı yeniden tanımlamak, dört kurucudan hangisiyle tanımlanmış olursa olsun öncekini siler.

## ValueKind

- `ValueKind::text()` — herhangi bir metin.
- `ValueKind::choice(["podman", "docker"])` — sabit bir listeden bir metin; başkası `one of podman, docker` diye bildirilir.
- `ValueKind::integer()` — TOML'un yazdığı her tabanda tam sayı (`0x1f`, 31 olarak okunur).
- `ValueKind::flag()` — `true` ya da `false`.
- Ondalık sayı, düz değer dizisi ve doğrulayıcı yok, çünkü ailenin yazdığı hiçbir veri dosyasında henüz yok. Yalnızca uygulamanın karar verebildiğini okuduktan sonra denetle ve framework'ün bir değer hakkındaki kendi bulgularını bildirdiği `Table::value_location(key)` konumunda bildir.

## Document

- `Document::parse(file, text, &shape)` — TOML metnini okur; `file`, tanıların taşıdığı addır.
- `Document::open(path, &shape) -> io::Result<Document>` — `path`'teki dosyayı okur ve tanıları onun adıyla bildirir. Olmayan dosya bir `io::Error`'dur, boş belge değil.
- `.root() -> &Table` — belgenin kök tablosu.
- `.diagnostics() -> &[Diagnostic]` — bulunma sırasıyla her sorun: önce sözdizimi hataları, sonra şekil denetimi.
- `.is_clean() -> bool` — hiçbir sorun olmadı mı.
- Okumak hiç yazmaz, onarmaz, yedek bırakmaz ve panik yapmaz.

## Table

- `.text(key)`, `.integer(key)`, `.flag(key)` — anahtar varsa ve tanımlı türdeyse değeri, değilse `None`. `choice` da `text` ile okunur.
- `.table(key) -> Option<&Table>` — belgede varsa içteki tablo.
- `.entries(key) -> &[Table]` — dosya sırasıyla kayıtlar; yoksa boş.
- `.location(key) -> Option<&Location>` — anahtarın yazıldığı yer.
- `.value_location(key) -> Option<&Location>` — değerin yazıldığı yer: `mode = "halb"` satırında `"halb"` değerinin sütunu; yanlış tür ya da seçim de orada bildirilir. Yalnızca uygulamanın görebildiği bir sorunu bildirmek için. Eksik ya da okunamayan bir anahtar için `None`.

## Tanılar

- `` `colour` is not part of the document; it is ignored `` — anahtarın durduğu yerde bir uyarı.
- `` `name` must be a string, found 7; it is ignored `` — zorunlu anahtar için hata, isteğe bağlı için uyarı; değerin durduğu yerde.
- `` `id` is required and missing `` — eksik olduğu tablonun başında bir hata.
- `` `profile[1].name` is required and missing `` — dizi kaydının içinde aynısı, kaydı açan satırda.
- Sözdizimi hatası, ayrıştırıcının kendi sözlerini satır ve sütunuyla korur.

## Tuşlar

- Oyun alanı belgeleri bir segment denetimiyle değiştirir; bu sayfanın kendine ait tuşu yoktur.
