## Metotlar

- `CellAnimation::new()` — karesiz, `FrameTime::Motion("spinner")`, `Playback::Loop`, `ColorMode::Step`.
- `.frame(AnimationFrame)`, `.frame_time(FrameTime)`, `.playback(Playback)`, `.colors(ColorMode)`, `.rest(indeks)` — kodda kurmak için; `rest` 0'dan sayar.
- `.sample(tema, fg, now, since) -> CellFrame` — kare indeksi, renk, tek oynatmanın bitip bitmediği, karenin ne zaman değişeceği ve rengin hareket edip etmediği; `since = None` duruş karesinde sabit durur.
- `.glyph(indeks, GlyphMode)` — yedeklerden sonra bir karenin karakteri. `.to_toml(ad)` — `[animations.<ad>]` bloğu.
- `AnimationFrame::new(ascii)`, ardından `.unicode(karakter)`, `.nerd(karakter)`, `.color(CellColor)`, `.duration(FrameTime)`.
- `CellColor::parse("mix($success, $fg, 50%)")`, `CellColor::from(Rgb)`; `FrameTime::parse("step" | "80ms")`.
- `check_glyph(karakter, mod)` — yükleyicinin yaptığı tek hücre denetimi. `parse_animations(dosya, metin)` — animasyon dosyasını konumlu tanılarla okur.
- `PaintCx::animation(ad, stil, since) -> AnimatedCell` — kayıtlı bir animasyonun şu anki karakteri ve stili, terminalin karakter modunda; sonraki kareyi planlar.
- `Icons::animation(ad)`, `Icons::animation_names()` — kayıtlı animasyonlar.
- `Spinner::animation(ad)`, `SpinnerStyle::animation()`, `Toast::icon_motion(stil ya da ad)`.

## Davranış

- **Dosya biçimi:** `frame`, `playback`, `colors`, `rest` (1'den sayılır) ve `frames = [{ nerd, unicode, ascii, color, duration }]` içeren `[animations.<ad>]`. Adlar küçük harf, rakam ve `-` kullanır. En çok 256 kare.
- **Yedek:** nerd → unicode → ascii; `ascii` zorunlu; her karakter tek grafem, tek hücre, parantez yok; ASCII yalnızca yazdırılabilir.
- **Renkler:** `$token`, `#RRGGBB`, `mix(a, b, N%)` (a'nın %N'si), yalnızca bütün değer olarak `pulse(a, b)`; `$fg` bileşenin rengidir; bilinmeyen bir token bileşenin rengini alır.
- **Zamanlama:** kareler `since` anından sayılan tam milisaniye sınırlarında değişir; dönen spinner'lar sıfırdan sayar, böylece birlikte döner. Nabızlar `motion.pulse-period` izler.
- **Blend:** karenin başında kendi rengi, sonra oynatma sırasındaki bir sonraki kareye doğru düz karışım; `once` animasyonunun son karesi karışmaz.
- **Katmanlar:** gömülü varsayılan ikon seti, sonra seçilen ikon seti, sonra tema zinciri. Bir katmanda önce eski spinner ikon anahtarları (`spinner`, `spinner-arc`, `spinner-done`, `spinner-orbit`, `spinner-pop`, `spinner-quarters`, `spinner-slices`) kendi animasyonlarının karakterlerini değiştirir, sonra `[animations]` uygulanır.
- **Yükleyici:** asla çökmez; her sorun dosya, satır ve sütunlu bir tanıdır ve animasyonun tamamı atlanır.
- **Stüdyo kaydı:** showcase ayarlarında açık `studio` önekinin altında, değişen ve yeni animasyonların TOML metni olan `studio.animations`.

## Tema anahtarları

- İkon seti ve tema dosyalarında `[animations.<ad>]`.
- Gömülü adlar: `spinner-arc`, `spinner-done`, `spinner-dots`, `spinner-orbit`, `spinner-pop`, `spinner-pulse`, `spinner-quarters`, `spinner-slices`.
- Kare süresi olarak kullanılabilen `[motion]` anahtarları: `spinner`, `step`, `flash`, `enter`, `shimmer`, `pulse-period`, `cursor-blink`, `page`, `hover-delay`.
- Buradaki önizlemeler `spinner` ve `spinner-label` stilleriyle çizilir.
