## Ne zaman kullanılır

Panel birbirine ait şeyleri gruplar: bir form, bir seçenekler kümesi, canlı bir önizleme. Arka planından bir kademe yüksek bir yüzeydir; gruplama çerçeve olmadan görünür.

## Adım adım

1. İçeriğiyle ekle: `ui.add_with(Panel::new(), |ui| { ... })`.
2. Grubun bir ada ihtiyacı varsa kısa bir başlık ver: `.title(t!("settings.network"))`. Başlıklar bilerek sessizdir; önemli olan içeriktir.
3. Çocuklar arasına `.gap(satır)` koy; varsayılan bir satır.
4. Kullanıcının seçebileceği bir kart için `.on_press(msg)` ve `.selected(bool)` ekle.
5. Yüzey içinde yüzey için `.variant("inset")` kullan, yukarıdaki kartlar gibi: bir panelin üstündeki kart görünmek için bir kademe yüksek olmalı.

## Nasıl çalışır

- **Çerçeve değil, yükselme.** Canvas, surface, raised, active: her kademe daha açık bir tondur. Panel kendi tonunu, seçili panel active tonunu boyar.
- **Seçimi çubuk işaretler.** Seçili panel, diyaloglar gibi sol kenarının tamamında vurgu çubuğunu gösterir: seçim kalıcı bir durumdur. Duran paneller asla göstermez; vurgu her yerde olursa vurgu olmaktan çıkar.
- **Basılabilir paneller fareye buton gibi cevap verir.** Üzerine gelince yüzey belirgin bir ton açılır, başlık uyanır ve panelin tüm sol kenarı boyunca yumuşak bir çubuk çıkar. Basınca bir ton daha parlar. Klavyeyle gelen odak hover gibi görünür, çubuğu nefes alır; tıklamadan sonra bu iz kalmaz.
- **Basılabilir paneller gerçek kontrollerdir.** Odak sırasına girer, Enter, Boşluk ve tıklamaya tepki verir. `.disabled(true)` onları devreden çıkarır: hover, odak, basış yok.
- **Düz paneller kıpırdamaz.** `on_press` yoksa fare altında hiçbir şey değişmez; kullanıcı hiçbir şey yapmayan bir grubun üstünde tepki görmez.

## Temayla özelleştirme

```toml
[style.panel]
bg = "$surface"
padding = [1, 3]

[style."panel:hover"]
bg = "mix($text, $surface, 8%)"
pillar = "mix($accent, $active, 45%)"

[style."panel:selected"]
bg = "$active"
pillar = "pulse($accent, $accent-2)"
```

## Sık yapılan hatalar

- **Her şeyin etrafına panel.** Yalnızca birbirine ait olanı grupla; canvas da geçerli bir arka plandır.
- **Üç kat iç içe panel.** İki seviye net okunur; ötesinde ekranı yeniden düzenle.
- **Uzun başlıklar.** Başlık grubu adlandırır; açıklamalar içeride yer alır.
