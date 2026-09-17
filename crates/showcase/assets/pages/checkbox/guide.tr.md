## Ne zaman kullanılır

Açık ya da kapalı olan bir ayar veya öğe için onay kutusu kullan; özellikle birlikte uygulanan bağımsız seçimlerin listesinde (bir butonla kaydedilen form gibi). Değiştiği anda uygulanan bir ayar için anahtar daha iyi okunur.

## Adım adım

1. Durumu uygulamanda tut: `autosave: bool`.
2. Kutuyu etiketiyle çiz: `Checkbox::new(state.autosave).label(t!("autosave"))`.
3. Tıklamaları mesaja çevir: `.on_toggle(|on| Msg::Autosave(on))` ve `update` içinde `on` değerini sakla.
4. Birkaç kutunun üstündeki kutu için işaretli çocukları say ve `.partial(bazısı && !hepsi)` ver. Üst kutuya basınca `true` istenir; `update` içinde her çocuğu ayarla.
5. Seçim şu an kullanılamıyorsa `.disabled(true)` kullan.
6. Varsayılan sade kutudur. Tikli kutu için `.style(CheckboxStyle::Check)` ekle.

## Nasıl çalışır

- **Kutu bir çizim değil, renktir.** İşaretsizken boş tonda iki hücre, işaretliyken vurgu rengiyle dolu, kısmen işaretliyken yalnızca sol hücresi dolu. Hiç karakter yok; ASCII terminalde de kutu birebir aynı görünür.
- **Değişim renkle geçer.** İşaretleyince ya da kaldırınca renk üç `motion.step` boyunca, yani bir anahtar topuzunun gittiği sürede karışarak değişir; hareket azaltılmışsa hemen değişir.
- **Radyo grubuyla aynı kutu.** Onay kutusu ve radyo grubu bu kutuyu bilerek paylaşır. Fark anlamdadır: her onay kutusu kendi başına açık ya da kapalıdır, radyo grubunda ise her zaman tek bir seçenek seçilidir. İnsanlar hangisine baktığını anlasın diye seçenekleri iyi etiketle ve grupla.
- **Tikli stil.** `CheckboxStyle::Check`, tikli `✓` ve kısmen işaretlide çizgili üç hücrelik kutuyu korur.
- **Satırın tamamı tıklanır.** Etikete tıklamak da değiştirir; odaktayken Enter ve Boşluk da.
- **Hover ve odak renktir.** Üstüne gelince boş kutu bir ton açılır; klavyeyle gelinen kutu vurguya doğru ısınır, odaklanan işaretli kutu nefes alır. Vurgu çubuğu yok.
- **Senin durumun geçerlidir.** Kutu yalnızca ister; `update` mesajı yok sayarsa kutu olduğu gibi kalır.

## Sık yapılan hatalar

- **Olumsuz etiketler.** "Veri gönderme" insanı iki kez düşündürür. İşaretlemenin ne yaptığını söyle.
- **Üst kutuyu unutmak.** Çocuklar değişince üst kutunun kısmi durumunu saklamak yerine çocuklardan yeniden hesapla.
