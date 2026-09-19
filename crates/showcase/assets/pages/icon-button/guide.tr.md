## Ne zaman kullanılır

İkon butonunu bir başlığın ya da satırın ucundaki küçük, herkesin bildiği bir eylem için kullan: ayarlar, arama, ekleme. Tek bir gliftir, bu yüzden etiketli bir butonun satırı kalabalıklaştıracağı yere sığar. Eylem anlaşılmak için söze ihtiyaç duyuyorsa ya da ekranın ana eylemiyse onun yerine etiketli bir `Button` kullan.

## Adım adım

1. İkon setinden bir ikon anahtarıyla oluştur: `IconButton::new("settings")`. Glif, glif kipine kendiliğinden uyar: Nerd Font'ta bir dişli, Unicode'da `▤`, ASCII'de `*`.
2. Göndereceği mesajı ver: `.on_press(Msg::OpenSettings)`. Mesaj olmadan çizilir ama odak almaz ve basılmaz.
3. Adını söyle: `.tooltip(t!("header.settings"))`. Tek başına bir glif tahmindir; imleç üzerinde durunca sözler altında görünür, Tab ile gelince de hemen.
4. Bu sayfadaki gibi, genişleyen bir başlığın ardından satırın sonuna koy: `ui.add(Text::new(başlık)).fill_width();` sonra butonlar.

## Nasıl çalışır

- **Her kipte üç hücre.** Bir boşluk, glif ve bir boşluk. Üç hücrenin hepsi hedeftir; glifin yanındaki boşluğa tıklamak da sayılır.
- **Dururken zemin yok.** Çevresindeki zeminin üstünde durur, böylece yan yana birkaçı bir buton sırası gibi değil, sessiz işaretler gibi okunur.
- **Durumu ton söyler.** İmleç gelince üç hücre birlikte hover tonuna açılır, klavye odağı bir ton daha açar, basış bir ton daha parlatır: dinlenme < hover < odak < basılı, hiç ters dönmez. Vurgu çubuğu yok: üç hücrede glifin önüne çubuk sığmaz, sığsa da glifi ortadan kaydırırdı.
- **Fare ve klavye eşittir.** Enter ya da Space odaktaki butona basar; tıklama, fare butonun üstünde bırakılınca basar, bırakmadan önce uzaklaşmak iptal eder.
- **Pasif, girdiye kapalı demektir.** Pasif bir ikon butonu soluktur, Tab onu atlar ve tıklamaları yok sayar.

## Neden Button'un bir seçeneği değil

`Button`, iç boşluğu ve içeriğinden önce bir çubuk hücresi olan yükselmiş bir yüzeydir. İkon butonunda bunların hiçbiri yok: zemin yok, temadan gelen iç boşluk yok, çubuk yok. Bir butonu buna çevirmek, her butonda olan bir şeyi kapatan üç ayrı seçenek isterdi; bu yüzden kendi stil anahtarı olan ayrı bir bileşendir.

## Temayla biçimlendirme

```toml
[style.icon-button]
fg = "$dim"

[style."icon-button:hover"]
bg = "$active"
fg = "$text"

[style."icon-button:focus"]
bg = "mix($accent, $active, 22%)"
fg = "$text"
```

## Sık yapılan hatalar

- **İpucunu unutmak.** Bir glif herkese başka şey söyler; sözle de söyle.
- **Ana eylem için kullanmak.** Ekranın birincil eylemi etiketli bir butonu hak eder.
- **İpucunu kodda yazmak.** Dil dosyasına koy ki dile uysun.
- **İki hücre genişliğinde bir ikon seçmek.** İkon setindeki her glif tek hücredir; uygulamanın kendi ikonu da öyle olmalı, yoksa buton büyür.
