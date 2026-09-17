## Ne zaman kullanılır

`Text`, bir kontrolün parçası olmayan ekrandaki her kelimedir: başlıklar, paragraflar, etiketler, durum satırları, ipuçları.

## Adım adım

1. `Text::new(t!("anahtar"))` gövde metni çizer.
2. Hiyerarşi için bir rol seç: `.role("title")`, `"secondary"` ya da `"faint"`. Roller temanın `[typography]` bölümünde tanımlıdır; hiyerarşi temayla değişir.
3. Tek satırda stilleri `Text::rich([...])` ve parçalarla karıştır: `Span::new(..).color("success").bold()`, işaretli arka plan için `.on("raised")`, `.role("faint")`.
4. Uzun metin kendisine verilen genişliğe göre sarılır. Genişliği yerleşime bırakırsan (`.fill_width()` ya da hiçbir şey) paragraf bütün alanı kullanır ve terminal boyut değiştirince yeniden sarılır; `.width(Length::Cells(60))` yalnızca sabit bir ölçü istediğinde ver. Tek satır isteyip `…` ile kesmek için `.no_wrap()` çağır.
5. `.align(Align::Center)` ya da `Align::End` her satırı hizalar.
6. Metin fareyle seçilemez. Bir mesaj ya da adres gibi kopyalanmaya değer içerikse düğümüne `.selectable(true)` ekle.

## Nasıl çalışır

- **Genişlikler gerçek hücre genişliğidir.** Türkçe harfler, birleşen aksanlar, geniş Doğu Asya karakterleri ve emojiler görünen genişlikleriyle ölçülür; sütunlar hizalı kalır.
- **Sarma kelimeleri bütün tutar.** Satır kelimeler arasında kırılır, noktalama kelimesiyle kalır, virgül asla satır başına düşmez; yalnızca satırdan geniş tek bir kelime karakterler arasından bölünür. Kırılmadaki boşluklar atılır.
- **Başlıklar ve etiketler içerik değildir.** İstenmedikçe hiçbir şey seçilemez; başlıkların ve ipuçlarının üzerinde sürüklemek bir şey seçmez. Kopyalamanın anlamlı olduğu düğümde seçimi aç.
- **Kesme dürüsttür.** Metin sarılmayacaksa son görünen hücre üç noktadır; okuyan bir şeyin gizli olduğunu anlar.
- **Hiyerarşi renk ve ağırlıktır.** Tirelerden alt çizgi yok, bağıran büyük harfler yok. Başlıklar kalın yazı rengidir, ikincil metin daha soluk, silik metin daha da soluktur.

## Temayla özelleştirme

```toml
[typography]
title     = { fg = "$text", bold = true }
secondary = { fg = "$dim" }
faint     = { fg = "$muted" }
```

## Sık yapılan hatalar

- **Hiyerarşiyi renkle göstermek.** Rolleri kullan; rengi anlama sakla.
- **Durumu yalnızca renkle göstermek.** Kelimeyi de yaz: tek başına yeşil bir nokta değil, “başarılı”.
- **Önemli metni kesmek.** Açıklamalar sarılsın; yalnızca başı yeterince şey söyleyen satırları, dosya adları gibi, kes.
