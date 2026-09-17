## Metotlar

Bu örnek kendine ait bir API eklemez. Şunları kullanır:

- Her bölüm için `.title(..)` ile `Panel`.
- Sunucu durumu, container sayısı ve yüklü servisler için `Badge::new(..).variant(..).count(..)`.
- Saat için `BigText::new("14:32").variant("accent")`.
- İşlemci geçmişi için `Sparkline::new(..).range(0.0, 100.0).highlight_extremes().baseline(80.0)`.
- Kaynaklar için `Gauge::new(..).range(..).label(..).label_width(8).thresholds(..).value_text(..)`.
- Servisler için `Bar::new(..).value_text(..).variant("danger")` çubuklarıyla `BarChart::new(çubuklar).max(100.0)`.
- Uyarı yokken `EmptyState::new(..).icon("success").message(..)`.
- Yüklenirken `Skeleton::block()`, `Skeleton::lines(..)`.

## Davranış

- Yenile, örnek saatini bir tik ilerletir: saat bir dakika ilerler ve her grafik yeni bir örnek alır.
- İşlemcisi %70'i aşan servisin çubuğu tehlike tonuna döner ve sayılı bir "Yüklü servisler" rozeti eklenir.
- Yükleniyor, grafiklerin yerine aynı boyutta iskeletler koyar; uyarılar, boş durumun yerine bir uyarı listesi ve Tümünü onayla butonu koyar.
- Her etkileşim olay günlüğüne yazılır.

## Tema anahtarları

Sayfa bileşenlerinin tema anahtarlarını kullanır: `panel`, `badge`, `badge-count`, `big-text`, `sparkline`, `gauge`, `gauge-label`, `gauge-value`, `bar-chart`, `bar-chart-label`, `bar-chart-value`, `empty-state-*`, `skeleton` ve `button`.
