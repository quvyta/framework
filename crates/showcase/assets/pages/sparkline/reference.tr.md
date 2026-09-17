## Metotlar

- `Sparkline::new(değerler)` — `f32` değerlerden bir sparkline, en eskisi önde.
- `.range(en_az, en_çok)` — görünen değerler yerine sabit bir aralıkta ölçekler.
- `.highlight_extremes()` — en son en yüksek ve en düşük sütunu renklendirir.
- `.baseline(değer)` — `değer` seviyesindeki satırı boyar.

## Davranış

- Her değer için bir sütun ve bir satır ölçer; doldurması için düğüme genişlik, uzun sütunlar için yükseklik ver.
- Sığan en yeni değerleri gösterir; en düşük değer sekizde bir hücre tutar, en yüksek sütunu doldurur.
- Sabit aralığın dışındaki değerler sınıra çekilir.
- Odak almaz, mesaj göndermez.

## Tema anahtarları

- `sparkline` — `fg` (sütunlar), `peak`, `low`, `baseline` (bant), `track` (ASCII zemini).
