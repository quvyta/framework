## Ne zaman kullanılır

Bir şeyin nasıl gösterildiğini değiştiren iki ile beş arası kısa ve birbirini dışlayan seçim için segment seçici kullan: liste, ızgara ya da ağaç; son bir saat, gün ya da hafta. Radyo grubundan daha hızlı okunur ve tek satır kaplar.

## Adım adım

1. Seçimi uygulamanda tut: `view: usize`.
2. Çiz: `Segmented::new(["Liste", "Izgara", "Ağaç"]).selected(state.view)`.
3. Seçimleri işle: `.on_select(|index| Msg::View(index))` ve içeriği yeni seçime göre çiz.
4. Etiketleri bir iki kelimede tut; her metin gibi çevir.

## Nasıl çalışır

- **Tek yüzey, tek dolu bölüm.** Bölümler boşluk ya da ayırıcı olmadan yan yana durur; seçili olan vurgu rengiyle dolu ve kalındır, diğerleri yükseltilmiş tonu paylaşır.
- **Hover bölümü yükseltir** ve vurgu çubuğu `▌` kontrolün en solunda değil, o bölümün ilk hücresinde çıkar; tıklanacak yer belli olur. Hiçbir şey kaymaz.
- **Klavye.** Tab kontrole bir kez gelir; Sol ve Sağ komşuyu hemen seçer, Home ve End uçları. Klavye kontrole odaklanınca seçili bölüm çubuğuyla nefes alır.

## Sık yapılan hatalar

- **Bölümlerde eylem.** Bölümler bir durum seçer; "Yenile" bir butondur.
- **Çok fazla ya da çok uzun bölüm.** Beşten fazlaysa ya da cümle gibiyse açılır liste veya sekme kullan.
