## Metotlar

- `CodeView::new(kod, dil)` — `Language::Rust`, `Language::Toml`, `Language::Shell` ya da `Language::Plain`.
- `.line_numbers(bool)` — Varsayılan: `true`.
- `.on_copy(msg)` — `c` ile kopyalandıktan sonra gönderilir.
- `.line_marks(işaretler)` — 1. satırdan başlayarak her satıra bir `LineMark`: `Added` (yeşil ton, `+`), `Removed` (kırmızı ton, `−`), `Unchanged`. Varsayılan: yok. Satır numaraları o zaman metni değil dosyaları izler: silinen satır eski dosyanın numarasını, eklenen satır yeni dosyanınkini, ikisinde de olan satır yeni dosyanınkini taşır.
- `.highlight_lines(aralık, ton)` — satırlar 1'den sayılır, `4..=6` ya da `9..` gibi her aralık olur; `LineTone::Accent` (dikey çubuk) ya da `LineTone::Warning` (uyarı ikonu). Fark işaretini geçer; aralıklar çakışınca sonraki çağrı kazanır.
- `.reveal(satır)` — satır değişince çevreleyen `ScrollView` satırı iki satır bağlamla gösterir; kayarak, hareket azaltılmışsa bir anda. Sonu geçerse son satır.
- `.reveal_number(numara)` — aynısı, satırın metindeki yeri yerine yanında yazan numarasıyla. Silinen ve eklenen bir satır aynı numarayı taşıdığında yeni dosyanın o numarayı verdiği satıra gidilir; hiçbir satırın taşımadığı numara hiçbir yere kaydırmaz. `.reveal(...)` ile birlikte verilirse bu kazanır.
- `.line_numbers_from(numaralar)` — 1. satırdan başlayarak her satıra bir `Option<usize>`; `None` o satırın sütununu boş bırakır, son numaradan sonraki satırlar da boştur. `.line_marks(...)`'ın ima ettiği numaraları geçer. Varsayılan: yok.

## Tuşlar

- Odaktayken: `c` tüm kodu kopyalar ve parlar.
- Fare: kodun içinde sürüklemek kodu seçer, seçim iç boşluğa taşmaz; satır numarası sütunu süstür, temiz kopyaya girmez.

## Dil

- `Language::from_tag("rust" | "rs" | "toml" | "sh" | "bash" | "shell" | "zsh" | "pkgbuild" | diğer)` — kod bloğu etiketleri için.
- `Language::from_file_name(ad)` — `PKGBUILD`, `.sh`, `.bash`, `.zsh`, `.install` kabuk; `.rs` Rust; `.toml` TOML; gerisi düz metin.
- `TextArea::language(dil)` — aynı diller, düzenlenen metin alanını da aynı `code-token` stilleriyle renklendirir; kod görünümü bütün dosyaların görüntüleyicisi olarak kalır.

## Kendi metnini renklendirmek

- `qframe::text::highlight(kod, dil)` — kodun belirteçlerinin bayt aralıkları, sırayla, metnin tamamını kapsayarak: ilk aralık sıfırdan başlar, her aralık bir öncekinin bittiği yerden başlar, sonuncusu metnin uzunluğunda biter. `Language::Plain` metnin tamamını tek bir `Token::Plain` olarak verir.
- `Token::style_variant()` — bir belirteci boyayan `code-token` stilin varyantı: `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `variable`, `plain`. Kodu kendi bileşeninde çizen bir uygulama temadan `code-token.<varyant>` ister ve her aralığı onunla boyar; bu bileşen de tam olarak böyle yapar.
- `Token` `#[non_exhaustive]`: yeni bir belirteç türü eklenebilir, bu yüzden eşlemede her türü saymak yerine joker kullan.
- `Language`, `qframe::widgets::Language` ile aynı türdür; `qframe::text::Language` adresinde de yeniden yayımlanır.

## Tema anahtarları

- `focus` ve `pressed` ile `code` — `bg`, `padding`.
- `code-line-number` — `fg`.
- `code-token.<tür>` — `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `variable`, `plain`.
- `code-line.<görünüm>` — sıra için `bg`, işareti için `fg`; görünüm `added`, `removed`, `accent` ya da `warning`.

## İkonlar

- `line-added`, `line-removed` — fark işaretleri; vurgular için `warning` ve dikey çubuk.
