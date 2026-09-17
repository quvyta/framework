## Ne zaman kullanılır

Kullanıcı bilinen bir kümeden, hepsini birden göstermek için fazla uzun olan bir seçenek seçecekse ya da yer darsa açılır liste kullan: bir tema, bir dil, bir bölge. Hepsinin görünür kalması gereken iki üç seçenek için segment seçici ya da radyo grubu daha hızlı okunur.

## Adım adım

1. Seçenekleri ver: `Select::new([t!("runtime.podman"), t!("runtime.docker")])`.
2. Seçimi göster: `Option<usize>` olarak `.selected(self.runtime)`.
3. Seçimleri al: `.on_select(Msg::Runtime)`.
4. Bir şey seçilmemişken ne seçileceğini söyle: `.placeholder(t!("runtime.choose"))`.
5. Uzun listelerde yüksekliği `.max_visible(satır)` ile sınırla; liste kayar.

## Nasıl çalışır

- **Liste bir katmandır.** Açılınca seçenekler her şeyin üstüne çizilir; yer varsa alanın altına, yoksa üstüne. İçeriği asla itmez.
- **Açıkken klavye içeride kalır.** Oklar gezinir, Home ve End atlar, PgUp ve PgDn sayfa sayfa gider, bir harf yazmak o harfle başlayan sonraki seçeneğe atlar, Enter ya da Boşluk seçer, Esc kapatır. Tab kapatıp ilerler.
- **Başka yere tıklamak kapatır, tıklama da boşa gitmez.** Dil listesi açıkken tema alanına tek tıklama onu kapatır ve tema listesini açar; menüdeki bir sayfaya tek tıklama sayfayı değiştirir. Açık listenin kendi alanına tıklamak yalnızca kapatır.
- **Tek vurgu.** Klavye de fare de aynı satırı vurgular. Liste açılırken yerinde duran fare, hareket edene kadar vurguyu almaz; tuşlar farenin bıraktığı satırdan devam eder.
- **Uzun listeler kayar**: tekerlekle, tuşlarla ya da kaydırma çubuğuna basıp sürükleyerek. Kaydırma çubuğunun görünüp görünmeyeceğine listenin tamamen açılmış yüksekliği karar verir; açılırken bir görünüp bir kaybolmaz.
- **Satırlar liste satırı gibi davranır.** Vurgulanan satır yüzeyini yükseltir, çubuğu gösterir ve etiketini bir hücre kaydırır; seçili seçeneğin sağında bir onay işareti vardır.
- **Yalnızca gerçek değişiklikler mesaj gönderir.** Zaten seçili olanı seçmek listeyi sessizce kapatır.

## Temayla özelleştirme

```toml
[style.select-menu]
bg = "$overlay"

[style."select-option:hover"]
bg = "$active"
pillar = "pulse($accent, $accent-2)"
```

## Sık yapılan hatalar

- **Seçenek metnini değer olarak kullanmak.** Kimlikleri kodunda tut ve sırayı eşle; etiketler çevrilir.
- **Yüzlerce seçenek.** Arama ekle ya da filtre alanlı bir liste kullan.
- **Kısa bir ekranın en altına koymak.** Yukarı doğru açılır, ama elinden geldiğince yer bırak.
