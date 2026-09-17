## Ne zaman kullanılır

Bir kaynağın şu an ne kadar dolu olduğunu, sorun olmaya başladığı bir nokta varsa gösterge kullan: bellek, disk, bağlantı havuzu, istek sınırı. İlerleme çubuğu işin ne kadar ilerlediğini söyler; gösterge bir kaynağın sınırına ne kadar yaklaştığını.

## Adım adım

1. Değeriyle bir gösterge ekle: `Gauge::new(81.0)`. Aralık varsayılan olarak 0 ile 100 arasıdır.
2. Birim yüzde değilse gerçek aralığı ver: 8 GiB için `.range(0.0, 8.0)`.
3. Adlandır: `.label("Bellek")`.
4. Sınırları koy: `.thresholds(6.0, 7.2)`. İlk değerden itibaren uyarı tonuna, ikinciden itibaren tehlike tonuna döner.
5. Değeri insanların düşündüğü biçimde göster: `.value_text("6,4 / 8 GiB")`.
6. Birkaç göstergeyi aynı genişlikte ve `.label_width(8)` ile alt alta koy; ölçerleri hizalanır.

## Nasıl çalışır

- **Tek satır.** Solda etiket, ortada ölçer, sağda değer. Ölçer hücrenin sekizde biri hassasiyetle dolar.
- **Görünen sınırlar.** Eşik varsa, izin her sınırdan sonraki bölümü orada bekleyen tonla hafifçe boyanır; bir şey ters gitmeden önce ne kadar yer kaldığını görürsün.
- **Renk asla tek başına kalmaz.** Eşik varsa değer bir işaret taşır: her şey yolundaysa nokta, uyarıda `▲`, tehlikede `✕`.
- **Dar alanlar.** Ölçere dört hücreden az kalırsa ölçer çizilmez, etiket ve değer kalır; önce etiket `…` ile kesilir.

## Sık yapılan hatalar

- **Sınırı olmayan şeylere eşik.** Bir istek sayacının tehlike bölgesi yoktur; sayı ya da sparkline kullan.
- **Anlamsız uyarı renkleri.** Sisteminde %90 disk normalse, insanlara uyarı rengini görmezden gelmeyi öğretmek yerine eşikleri taşı.
- **Farklı genişlikler.** Farklı genişlikteki göstergeler zor karşılaştırılır; bir gruba aynı genişliği ver.
