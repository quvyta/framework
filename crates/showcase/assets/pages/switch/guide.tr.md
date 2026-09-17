## Ne zaman kullanılır

Hemen etkili olan bir ayar için anahtar kullan: animasyonlar açık, sesler kapalı. Değişiklik ancak Kaydet'e basınca uygulanıyorsa onay kutusu kullan.

## Adım adım

1. Durumu uygulamanda tut: `animations: bool`.
2. Çiz: `Switch::new(state.animations).label(t!("animations"))`.
3. Değişiklikleri işle: `.on_toggle(|on| Msg::Animations(on))`, ayarı `update` içinde uygula.
4. Varsayılan uymuyorsa görünüm seç: sürgülerin yanında `.style(SwitchStyle::Rail)`, durumun kelimeyle okunması gereken yerde `.style(SwitchStyle::Labeled)`.

## Nasıl çalışır

- **Varsayılan kapsüldür.** Beş düz hücre ve iki hücrelik topuz. Kapalıyken topuz solda, yükseltilmiş iz üzerinde silik renkte; açıkken sağda, renklenmiş iz üzerinde vurgu renginde durur.
- **Topuz adım atar, renkler karışır.** Topuz her `motion.step` süresinde bir hücre ilerler; iz ve topuz rengi de onunla birlikte değişir, böylece ortadaki karede ara renk olur. Hareket azaltılmışsa bir anda geçer.
- **Topuz her zaman en parlak kısımdır**; durum her temada bir bakışta okunur.
- **Tek ton merdiveni.** Kapalı anahtar sakin durur: topuzu, açık anahtarın yanan izinin epey altındadır; alt alta dizilen anahtarlar dolu ve boş kutular gibi okunmaz. Üzerine gelmek, açık da olsa kapalı da olsa iki yarıyı bir basamak aydınlatır. Her ton tema renklerinin karışımıdır (`raised`, `muted`, `accent`); tema bunları `switch` anahtarlarıyla değiştirir.
- **Etiketli** stil, etkin dildeki `quvyta.switch.on` / `quvyta.switch.off` sözcüklerini yarım blok yuvarlak uçlu kapsülün içine yazar.
- **Ray** stili ince bir ray üzerinde topuz çizer; ASCII modunda kapsüle döner.

## Sık yapılan hatalar

- **Kaydet'i bekleyen anahtarlar.** İnsanlar anahtarın hemen çalışmasını bekler.
- **Durumu anlatan etiketler.** "Animasyonlar açık" değil "Animasyonlar" yaz; durumu anahtar gösterir.
