## Ne zaman kullanılır

Aynı şeyin görünümleri arasında geçiş için sekme kullan: bir sayfanın bölümleri, bir editörde açık dosyalar. Her sekme bir yerdir, bir eylem değil.

## Adım adım

1. Etiketleri ver: `Tabs::new([t!("tabs.overview"), t!("tabs.activity")])`.
2. Açık olanı durumundan göster: `.active(self.tab)`.
3. Geçişleri al: `.on_select(Msg::Tab)`.
4. Numaralar işe yarıyorsa `.numbered(true)` ekle, bu showcase'teki Demo, Kod, Rehber, Referans sekmeleri gibi.
5. Bir sekmeye `.badge(sıra, sayı)` ile sayı koy, örneğin bir Güncellemeler sekmesinin arkasında bekleyen güncellemeler. Sayıyı olduğu gibi ver: sıfır hiçbir şey göstermez.

## Nasıl çalışır

- **Açık sekme bir yüzeydir.** `active` tonunda, kalın yazı ve vurgu çubuğuyla durur; şerit klavyeyle odaklıyken çubuk nefes alır. Diğerleri sessiz metindir. Üstüne gelinen sekme `surface` tonuna yükselir ve soluk bir çubuk gösterir. Kutu yok, parantez yok, alt çizgi karakteri yok.
- **Sayı okunur kalır.** Rozet adın bir boşluk sonrasında, addan bir ton sessiz durur; 99'un üstü bir `Badge` sayısı gibi `99+` okunur. Sekme darsa önce ad `…` ile kısalır, sayı hücrelerini korur; sayı adla birlikte kaymaz. Taşma menüsüne gizlenen bir sekme sayısını da yanında götürür.
- **Kayma.** Kayma açıkken dururken etiket bir hücre solda bekler, sekmenin üstüne gelinince ya da sekme açılınca yerine kayar; sekme o hücreyi sağında tuttuğu için hiçbir sekmenin genişliği değişmez.
- **Klavye.** Odaktayken ← ve → (ya da h ve l) komşu sekmeyi açar, 1–9 numaralı sekmeyi doğrudan açar.
- **Taşınca oklar çıkar.** Sekmeler sığmazsa şeridin iki ucunda birer ok düğmesi durur. Düğme üç hücredir: boşluk, ok, boşluk; zemini bir ton yüksektir. İmleç üstüne gelince aydınlanır ve ilk hücresinde vurgu çubuğu belirir; basınca bir ton daha parlar ve şeridi hiçbir sekmeyi açmadan bir sekme kaydırır. Gösterecek sekmesi kalmayan ok zemine iner ve basışları yok sayar. Açılan sekme her zaman görünür hale gelir. Sıralanabilir bir şeritte (gelişmiş sekmeler sayfasına bak) sürükleyip okun üstünde tuttuğun sekme de şeridi birer sekme kaydırır.
- **Fare.** Açmak için sekmeye, kaydırmak için oka tıkla ya da şeridin üstünde tekerleği çevir. Oklara yer kalmayacak kadar dar bir şerit açık sekmeyi kısaltır, tekerlekle kaymaya devam eder.
- **Kaydırma tuşları.** ctrl+PgUp ve ctrl+PgDn oklar gibi kaydırır ve oku parlatır. Sığan bir şerit bu tuşları uygulamana bırakır.
- **Yalnızca değişiklikler mesaj gönderir.** Zaten açık olan sekmeyi açmak bir şey yapmaz.

## Temayla özelleştirme

```toml
[style."tab:selected"]
bg = "$active"
fg = "$text"
bold = true

[style."tab-index:selected:focus"]
fg = "pulse($accent, $accent-2)"
```

## Sık yapılan hatalar

- **Buton yerine sekme.** Sekme bir görünüm gösterir; bir şey yapmak için buton kullan.
- **Sıralı adımlar için sekme.** Kullanıcının sırayla izlemesi gereken adımlar için adım göstergesi ya da sihirbaz kullan.
