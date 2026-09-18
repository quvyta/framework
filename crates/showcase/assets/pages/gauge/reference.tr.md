## Metotlar

- `Gauge::new(değer)` — 0 ile 100 aralığında `değer`de bir gösterge.
- `.range(en_az, en_çok)` — değerin aralığı.
- `.label(metin)` — ölçerden önce bir ad.
- `.label_width(hücre)` — etiket sütununu ayırır; alt alta göstergeler hizalanır.
- `.value_text(metin)` — yüzde yerine ölçerden sonra yazılan metin.
- `.thresholds(uyarı, tehlike)` — değere göre ton: `uyarı`nın altında başarı, ondan itibaren uyarı, `tehlike`den itibaren tehlike.

## Davranış

- Aldığı tüm genişliği ve bir satırı ölçer.
- Ölçer hücrenin sekizde biri kadar dolar; ASCII modunda tam hücre.
- Eşik varsa izin her sınırdan sonraki bölgesi boyanır ve değer `dot`, `warning` ya da `error` ikonunu taşır.
- Ölçere dört hücreden az kalırsa yalnızca etiket ve değer çizilir.
- Bir okumadır: odak almaz, tuşlara ve fare olaylarına cevap vermez, mesaj göndermez. Tab onu atlar. Değer ayarlatmak için `Slider`, kesilen etiket için `Tooltip`, geçmişten değer okumak için `Sparkline` kullan.

## Tema anahtarları

- `gauge`, `gauge.success`, `gauge.warning`, `gauge.danger` — `track`, `fill`, `zone`.
- `gauge-label` — `fg`.
- `gauge-value`, `gauge-value.<seviye>` — `fg`, `bold`.
