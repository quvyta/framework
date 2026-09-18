## Metotlar

- `Sparkline::new(değerler)` — `f32` değerlerden bir sparkline, en eskisi önde.
- `.range(en_az, en_çok)` — görünen değerler yerine sabit bir aralıkta ölçekler.
- `.highlight_extremes()` — en son en yüksek ve en düşük sütunu renklendirir.
- `.baseline(değer)` — `değer` seviyesindeki satırı boyar.
- `.reading(sıra)` — okunan örnek: verilen değerler içindeki sırası; yokken `None`.
- `.on_read(mesaj)` — bir örneğin fareyle ve klavyeyle okunmasını açar; mesaj okunan sırayı, okuma bırakıldığında `None` taşır.

## Davranış

- Her değer için bir sütun ve bir satır ölçer; doldurması için düğüme genişlik, uzun sütunlar için yükseklik ver.
- Sığan en yeni değerleri gösterir; en düşük değer sekizde bir hücre tutar, en yüksek sütunu doldurur.
- Sabit aralığın dışındaki değerler sınıra çekilir.
- `on_read` yoksa odak almaz ve mesaj göndermez.
- `on_read` varsa: sol tuşa basmak basılan sütunu okur, sürüklemek gezer ve uçlarda durur; odakta ←/→ bir örnek ilerler, Home ve End gösterilen en eski ve en yeni örneğe gider, Esc `None` gönderir. İlk tuş en yeni örneği okur. Mesaj yalnızca okunan örnek değiştiğinde gönderilir.
- Gösterilen örneklerin dışındaki bir `reading` işaretlenmez; tuşlar gösterilen pencerenin içinde işini sürdürür.
- Okunan sütun active zemininde vurgu renginde, üzerine gelinen sütun bir hover adımı aydınlanmış zeminde çizilir; ikisinde de hiçbir hücre yer değiştirmez.

## Tema anahtarları

- `sparkline` — `fg` (sütunlar), `peak`, `low`, `reading` (okunan sütun, varsayılan vurgu rengi), `baseline` (bant), `track` (ASCII zemini).
