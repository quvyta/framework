## Ne zaman kullanılır

Kesin sayıdan çok değerin aralıktaki yeri önemliyse sürgü kullan: canary sürümüne giden trafik, CPU sınırı, logların ne kadar saklanacağı. Kullanıcı aralığın tamamını bir bakışta görür ve sürükleyerek değeri seçer. Kesin sayı önemliyse ya da aralık çok genişse sayı girişi kullan.

## Adım adım

1. Değeri uygulamanda tut: `canary: f64`.
2. Çiz: `Slider::new(state.canary)`. Seçeneksiz sürgü 0'dan 100'e birer birer gider.
3. Değişiklikleri işle: `.on_change(|value| Msg::Canary(value))`, değeri `update` içinde sakla.
4. Aralığı yalnızca gerekiyorsa değiştir: `.range(0.5, 8.0).step(0.5)`.
5. Sayının ne olduğunu söyle: `.suffix(t!("cores"))` ya da `.format(|mb| …)` ile kendin yaz.
6. Genişliği yerleşimle ver, örneğin `.width(Length::Cells(48))`; sürgü kendisine verilen genişliğin tamamını kullanır.

## Nasıl çalışır

- **Önce değer gelir.** Rayın önünde vurgu renginde yazılır; en geniş değerin sığacağı alanda sağa yaslanır, böylece sayı değişirken ray kıpırdamaz.
- **Şekli ray verir.** Değere kadarki kısım vurgu renginde, topuz `◆` değerin olduğu hücrede, rayın kalanı sakin, yükseltilmiş bir tondadır. ASCII modunda aynı ray karakter yerine hücre renkleriyle çizilir.
- **Değerler adımlara oturur.** Adımlar en küçük değerden sayılır, değer aralığın dışına çıkmaz. Ondalık basamaklar adıma uyar: `0.5` adımda `2.5`, `1` adımda `3` yazılır.
- **Klavye:** ← ve → bir adım, Page Up ve Page Down aralığın onda biri kadar gider; Home ve End uçlara atlar.
- **Fare:** raya basınca topuz oraya atlar; sürükleyince imleç raydan çıksa bile topuz izler. Değerin üstüne basmak yalnızca odaklar.
- **Tekerlek:** imleç sürgünün herhangi bir yerindeyken tekerleği yukarı çevirmek bir adım artırır, aşağı çevirmek bir adım azaltır; odak değişmez. Tekerleği sürgü alır, yani imleç sürgünün üstündeyken çevredeki kayan sayfa kıpırdamaz; sayfayı kaydırmak için imleci sürgüden çek. Pasif sürgü tekerleği sayfaya bırakır.
- **Hareket:** klavyeyle yapılan atlamada topuz her `motion.step` süresinde bir hücre ilerler; uzun atlamalar en çok sekiz adım sürer. Fareyle topuz anında yerini alır; hareket azaltılmışsa her değişiklik anında olur.
- **Odaklanınca** topuz vurgu rengiyle ikinci tonu arasında nefes alır.

## Sık yapılan hatalar

- **Kesin sayılar için sürgü.** Kimse tam 1 337'ye sürükleyemez; sayı girişi sun.
- **Birimi yalnızca etikette yazmak.** "CPU sınırı 2.5" okuyanı düşündürür; `.suffix(" çekirdek")` ekle.
- **İmleci koyacak başka yer bırakmayan uzun kayan formlar.** Tekerlek, imlecin altındaki sürgüyü değiştirir; insanlar sayfayı kaydırabilsin diye sürgülerin yanında boş yer bırak.
- **Çok dar ray.** Birkaç hücrelik rayda her hücre birçok adımı kapsar; sürgüye yer aç ya da adımı büyüt.
