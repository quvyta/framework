## Ne zaman kullanılır

`Text`, bir kontrolün parçası olmayan ekrandaki her kelimedir: başlıklar, paragraflar, etiketler, durum satırları, ipuçları.

## Adım adım

1. `Text::new(t!("anahtar"))` gövde metni çizer.
2. Hiyerarşi için bir rol seç: `.role("title")`, `"secondary"` ya da `"faint"`. Roller temanın `[typography]` bölümünde tanımlıdır; hiyerarşi temayla değişir.
3. Tek satırda stilleri `Text::rich([...])` ve parçalarla karıştır: `Span::new(..).color("success").bold()`, işaretli arka plan için `.on("raised")`, `.role("faint")`.
4. Uzun metin kendisine verilen genişliğe göre sarılır. Genişliği yerleşime bırakırsan (`.fill_width()` ya da hiçbir şey) paragraf bütün alanı kullanır ve terminal boyut değiştirince yeniden sarılır; `.width(Length::Cells(60))` yalnızca sabit bir ölçü istediğinde ver. Tek satır isteyip `…` ile kesmek için `.no_wrap()` çağır.
5. Başı da sonu da önemli olan bir yol ya da metin ortasından kısaltılır: `Text::new(qframe::text::truncate_middle(yol, en_fazla))` 24 hücrede `~/.config/quvyta/launcher.conf` yolunu `~/.config/q…auncher.conf` yapar; klasör ağacı da dosya adı da kalır.
6. `.align(Align::Center)` ya da `Align::End` her satırı hizalar.
7. Metin fareyle seçilemez. Bir mesaj ya da adres gibi kopyalanmaya değer içerikse düğümüne `.selectable(true)` ekle.

## Nasıl çalışır

- **Genişlikler gerçek hücre genişliğidir.** Türkçe harfler, birleşen aksanlar, geniş Doğu Asya karakterleri ve emojiler görünen genişlikleriyle ölçülür; sütunlar hizalı kalır.
- **Sarma kelimeleri bütün tutar.** Satır kelimeler arasında kırılır, noktalama kelimesiyle kalır, virgül asla satır başına düşmez; yalnızca satırdan geniş tek bir kelime karakterler arasından bölünür. Kırılmadaki boşluklar atılır.
- **Her yazı kendi kuralıyla.** Çince ve Japonca boşluksuz yazılır; satır herhangi iki karakter arasında kırılabilir, ama `。` `、` `」` gibi kapanış işaretlerinden ve uzatma işareti `ー`'den önce asla, açılan `「` ya da `（`'dan sonra asla. Bölünmez boşluk (U+00A0, U+202F, U+2007) kelimenin parçasıdır: Fransızca `?` ve `:` önüne bir tane koyar ve bu işaretler hiçbir zaman satır başına düşmez. BAŞKA YAZILAR paneli üçünü seçtiğin genişlikte gösterir; ayarlanacak bir şey yok.
- **Başlıklar ve etiketler içerik değildir.** İstenmedikçe hiçbir şey seçilemez; başlıkların ve ipuçlarının üzerinde sürüklemek bir şey seçmez. Kopyalamanın anlamlı olduğu düğümde seçimi aç.
- **Kesme dürüsttür.** Metin sarılmayacaksa son görünen hücre üç noktadır; okuyan bir şeyin gizli olduğunu anlar.
- **Kesme işareti glif kipine uyar.** ASCII bir terminal `…` gösteremez; bu yüzden ASCII kipinde her kesme `~` ile biter, kısaltılmış dosya adlarının eskiden beri kullandığı işaret. Aynı tek hücreyi kaplar, kip değişince hiçbir şey kaymaz.
- **Ortadan kısaltma iki ucu da korur.** `truncate_middle`, `…`'dan sonra kalan hücreleri baş ile son arasında paylaştırır, tek kalan hücreyi sona verir; geniş bir karakteri ya da bir harfi aksanından asla ayırmaz, bir tarafın kullanamadığı hücre öbür tarafa geçer.
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
- **Fransızca noktalamadan önce sıradan boşluk.** `dossier ?` değil `dossier\u{202F}?` yaz; yoksa işaret satır başında tek başına kalabilir.
- **Önemli metni kesmek.** Açıklamalar sarılsın; yalnızca başı yeterince şey söyleyen satırları, başlıklar gibi, kes. Sonundan kesilen bir yol dosya adını kaybeder; onu ortasından kısalt.
